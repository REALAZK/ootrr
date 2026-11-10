use std::collections::BTreeMap as BM;
use std::fs;
use std::path::{Path as P, PathBuf as PB};
use std::time::Instant;

use ootrr_core::TITLE_ID;
use ootrr_core::entrance_swap::{self as sw, FileEdit};
use ootrr_core::entrances::{self as en, Analysis, Ent, Scan, StartIdx};
use ootrr_core::error::{Error, Res};
use ootrr_core::modpack::Pack;
use ootrr_core::romfs::Romfs;
use ootrr_core::zs;

use crate::entrance_json as ej;

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(Error::Format(what))
}

pub fn catalog(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let out: &str = arg(a, 1, "missing out_json")?;
    let t0: Instant = Instant::now();
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let s: Scan = en::scan(&mut rf)?;
    let an: Analysis = en::analyze(&s);
    let bad: Vec<String> = en::verify(&s, &an);
    fs::write(out, ej::render(&s, &an))?;
    let c: BM<String, usize> = ej::counts(&s, &an);
    let n = |k: &str| -> usize { c.get(k).copied().unwrap_or(0) };
    println!(
        "entrance actors: {}  with a linked preload anchor: {}  standalone param anchors: {}",
        n("entrances"),
        n("with_linked_anchor"),
        n("param_anchors")
    );
    println!(
        "anchors in total: {}  start positions: {}  maps: {}",
        n("preload_anchors"),
        n("start_positions"),
        n("maps")
    );
    for k in ej::cat_order() {
        println!("  {}: {}", k.name(), n(&format!("cat_{}", k.name())));
    }
    println!(
        "by file age: adult {}  child {}  both {}",
        n("file_age_adult"),
        n("file_age_child"),
        n("file_age_both")
    );
    println!(
        "returns: strict {}  weak {}  none {}  no destination {}",
        n("return_strict"),
        n("return_weak"),
        n("return_none"),
        n("no_destination")
    );
    println!(
        "unresolved destination fields: {}  (file_age {}, map_age {}, live {})",
        n("broken_fields"),
        n("broken_file_age"),
        n("broken_map_age"),
        n("broken_live")
    );
    for b in &an.broken {
        let e: &Ent = &s.ents[b.i];
        println!("  [{}] {} {} {} -> {}/{} ({})", b.why, e.map, e.name, b.field, b.param, b.start, b.st.name());
    }
    println!("unclassified (no destination):");
    for r in an.rows.iter().filter(|r| r.no_dest) {
        let e: &Ent = &s.ents[r.i];
        println!("  {} {} {} as {}", e.map, e.name, e.hash, r.cat.name());
    }
    println!("swappable without force: {}", n("swappable"));
    println!("actor types that carry a map or start position name:");
    for (k, v) in &s.banc_keys {
        println!("  {k}: {v}");
    }
    println!("verification issues: {}", bad.len());
    for m in bad.iter().take(20) {
        println!("  {m}");
    }
    println!("wrote {out}  elapsed {:.1}s", t0.elapsed().as_secs_f32());
    if bad.is_empty() { Ok(()) } else { Err(Error::Format("entrance verification failed")) }
}

pub fn list(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let pat: &str = a.get(1).map(|s| s.as_str()).unwrap_or("");
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let s: Scan = en::scan(&mut rf)?;
    let an: Analysis = en::analyze(&s);
    let mut k: usize = 0;
    for r in &an.rows {
        let e: &Ent = &s.ents[r.i];
        let line: String = format!(
            "{}/{}  {}  {}  {} -> {}/{}  {}",
            en::base(&e.map),
            e.name,
            e.map,
            e.hash,
            r.cat.name(),
            en::field(e, if e.age == "child" { "child" } else { "adult" }),
            e.start,
            if ej::swappable(e, r) { "swappable" } else { "needs force" }
        );
        if line.contains(pat) {
            println!("{line}");
            k += 1;
        }
    }
    println!("{k} entrance(s)");
    Ok(())
}

fn desc(s: &Scan, i: usize) -> String {
    let e: &Ent = &s.ents[i];
    format!("{}/{} ({}, {})", en::base(&e.map), e.name, e.age, e.hash)
}

pub fn swap(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let load: &str = arg(a, 1, "missing load_dir")?;
    let name: &str = arg(a, 2, "missing mod_name")?;
    let sa: &str = arg(a, 3, "missing first entrance")?;
    let sb: &str = arg(a, 4, "missing second entrance")?;
    let force: bool = a.get(5).is_some_and(|s| s == "force");
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let s: Scan = en::scan(&mut rf)?;
    let ix: StartIdx = en::start_index(&s);
    let pr: Vec<(usize, usize)> = sw::pairs(&s, sa, sb)?;
    for (i, j) in &pr {
        println!("swap {}  <->  {}", desc(&s, *i), desc(&s, *j));
        for m in sw::check(&s, &ix, *i, *j, force)? {
            println!("  note: {m}");
        }
        for (x, y) in [(*i, *j), (*j, *i)] {
            let ex: &Ent = &s.ents[x];
            let ey: &Ent = &s.ents[y];
            println!(
                "  {} now leads to {}/{} (was {}/{}), arrival event {} (was {})",
                desc(&s, x),
                en::field(ey, if ex.age == "child" { "child" } else { "adult" }),
                ey.start,
                en::field(ex, if ex.age == "child" { "child" } else { "adult" }),
                ex.start,
                ey.next,
                ex.next
            );
        }
    }
    let edits: Vec<FileEdit> = sw::plan(&mut rf, &s, &pr)?;
    let pk: Pack = Pack::create(P::new(load), TITLE_ID, name)?;
    for e in &edits {
        let z: Vec<u8> = zs::compress_raw(&e.bytes, Some(3))?;
        if rf.zs.decompress(&z)? != e.bytes {
            return Err(Error::Format("compressed output does not round trip"));
        }
        let at: PB = pk.write_romfs(&e.file, &z)?;
        println!("{}", e.file);
        println!(
            "  size {} -> {} bytes decompressed, bytes differing {}, strings changed {}",
            e.old_len,
            e.bytes.len(),
            e.bytes_differing,
            e.changed.len()
        );
        for c in &e.changed {
            println!("    {c}");
        }
        if e.bytes.len() != e.old_len {
            println!("  warning: size changed, the resource size table may need an update");
        }
        println!("  wrote {}", at.display());
    }
    println!("mod folder: {}", pk.root.display());
    Ok(())
}
