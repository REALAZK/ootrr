use std::collections::BTreeMap as Map;
use std::path::Path as P;
use std::time::Instant;

use ootrr_core::error::{Error, Res};
use ootrr_core::msbt::{self, Msbt, Tok};
use ootrr_core::romfs::Romfs;
use ootrr_core::sarc;

#[derive(Default)]
struct Tally {
    files: usize,
    not_msbt: usize,
    same: usize,
    labels: usize,
    strings: usize,
    codes: usize,
    tok_same: usize,
    markup_same: usize,
    edit_same: usize,
    edit_changed: usize,
    fails: Vec<String>,
}

fn strings_of(m: &Msbt, t: &mut Tally, kinds: &mut Map<(u16, u16), usize>) -> Res<()> {
    let v: &Vec<Vec<u8>> = m.strings().ok_or(Error::Format("no TXT2 section"))?;
    for s in v {
        t.strings += 1;
        let k: Vec<Tok> = m.decode(s)?;
        if m.encode(&k)? == *s {
            t.tok_same += 1;
        }
        if msbt::from_markup(&msbt::to_markup(&k))? == k {
            t.markup_same += 1;
        }
        for x in &k {
            if let Tok::Code { grp, kind, .. } = x {
                t.codes += 1;
                *kinds.entry((*grp, *kind)).or_insert(0) += 1;
            }
        }
    }
    Ok(())
}

fn one(name: &str, raw: &[u8], t: &mut Tally, kinds: &mut Map<(u16, u16), usize>) -> Res<()> {
    let m: Msbt = msbt::parse(raw)?;
    let out: Vec<u8> = m.write()?;
    if out == raw {
        t.same += 1;
    } else {
        t.fails.push(format!("{name}: rewrite differs"));
    }
    t.labels += m.labels().map_or(0, |g| g.iter().map(|v| v.len()).sum());
    strings_of(&m, t, kinds)?;
    match m.names().first() {
        Some((n, _)) => {
            let n: String = n.to_string();
            let mk: String = msbt::to_markup(&m.text(&n)?);
            if msbt::replace_label(raw, &n, &mk)? == raw {
                t.edit_same += 1;
            } else {
                t.fails.push(format!("{name}: identity edit of {n} differs"));
            }
            let ch: Msbt = msbt::parse(&msbt::replace_label(raw, &n, "edited {{x}}")?)?;
            let want: Vec<Tok> = vec![Tok::Text("edited {x}".to_string())];
            let rest: bool = ch.names().iter().skip(1).all(|(k, _)| ch.raw(k) == m.raw(k));
            if ch.text(&n)? == want && rest && ch.secs.len() == m.secs.len() {
                t.edit_changed += 1;
            } else {
                t.fails.push(format!("{name}: changed edit of {n} disturbed the file"));
            }
        }
        None => t.edit_same += 1,
    }
    Ok(())
}

fn lang(rf: &mut Romfs, l: &str) -> Res<Tally> {
    let path: String = format!("Mals/{l}.Product.110.sarc.zs");
    let es: Vec<sarc::Entry> = sarc::parse(&rf.read_zs(&path)?)?;
    let mut t: Tally = Tally::default();
    let mut kinds: Map<(u16, u16), usize> = Map::new();
    let mut encs: Map<(bool, u8, u8), usize> = Map::new();
    for e in &es {
        if e.data.get(..8) != Some(b"MsgStdBn".as_slice()) {
            t.not_msbt += 1;
            continue;
        }
        t.files += 1;
        if let Ok(m) = msbt::parse(&e.data) {
            *encs.entry((m.be, m.enc, m.ver)).or_insert(0) += 1;
        }
        if let Err(x) = one(&e.name, &e.data, &mut t, &mut kinds) {
            t.fails.push(format!("{}: {x}", e.name));
        }
    }
    println!(
        "{l}: archive entries: {}  msbt files: {}  not msbt: {}  byte-identical rewrites: {}  identity label edits: {}  changed label edits: {}  failures: {}",
        es.len(),
        t.files,
        t.not_msbt,
        t.same,
        t.edit_same,
        t.edit_changed,
        t.fails.len()
    );
    println!(
        "  labels: {}  strings: {}  control codes: {}  distinct code kinds: {}  token round trips: {}  markup round trips: {}",
        t.labels,
        t.strings,
        t.codes,
        kinds.len(),
        t.tok_same,
        t.markup_same
    );
    for ((be, enc, ver), n) in &encs {
        println!("  big endian {be}, encoding {enc}, version {ver}: {n} file(s)");
    }
    for m in t.fails.iter().take(8) {
        println!("  {m}");
    }
    Ok(t)
}

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = a.first().ok_or(Error::Format("missing romfs_root"))?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let mut ls: Vec<String> = a[1..].to_vec();
    if ls.iter().any(|l| l == "all") {
        ls = rf
            .list("Mals")?
            .into_iter()
            .filter_map(|f| f.strip_prefix("Mals/")?.strip_suffix(".Product.110.sarc.zs").map(|s| s.to_string()))
            .collect();
    }
    if ls.is_empty() {
        ls.push("USen".to_string());
    }
    let t0: Instant = Instant::now();
    let mut files: usize = 0;
    let mut same: usize = 0;
    let mut fails: usize = 0;
    for l in &ls {
        match lang(&mut rf, l) {
            Ok(t) => {
                files += t.files;
                same += t.same;
                fails += t.fails.len();
            }
            Err(e) => {
                fails += 1;
                println!("{l}: {e}");
            }
        }
    }
    println!("total msbt files: {files}  byte-identical rewrites: {same}  failures: {fails}");
    println!("elapsed: {:.1}s", t0.elapsed().as_secs_f32());
    Ok(())
}
