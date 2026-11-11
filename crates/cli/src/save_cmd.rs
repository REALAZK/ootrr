use std::collections::BTreeMap as Map;
use std::fs;
use std::path::Path as P;

use ootrr_core::entrances::{STARTPOS, Sp, read_startpos};
use ootrr_core::error::{Error, Res};
use ootrr_core::flags::{self, Flag, FlagList, KEYED, hash_name};
use ootrr_core::json;
use ootrr_core::romfs::Romfs;
use ootrr_core::save::diff::diff;
use ootrr_core::save::info::report;
use ootrr_core::save::pos;
use ootrr_core::save::presets::{self, Applied, Op};
use ootrr_core::save::text::{hash_of, parse as parse_val, show, target};
use ootrr_core::save::{self, Save, Val};

const FLAGS: &str = "data/flags.json";

struct Args {
    pos: Vec<String>,
    opt: Map<String, String>,
}

fn split(a: &[String]) -> Res<Args> {
    let mut pos: Vec<String> = Vec::new();
    let mut opt: Map<String, String> = Map::new();
    let mut i: usize = 0;
    while i < a.len() {
        if let Some(k) = a[i].strip_prefix("--") {
            let v: &String = a
                .get(i + 1)
                .ok_or(Error::Format("save: option without a value"))?;
            opt.insert(k.to_string(), v.clone());
            i += 2;
        } else {
            pos.push(a[i].clone());
            i += 1;
        }
    }
    Ok(Args { pos, opt })
}

impl Args {
    fn need(&self, i: usize, what: &'static str) -> Res<&str> {
        self.pos
            .get(i)
            .map(|s| s.as_str())
            .ok_or(Error::Format(what))
    }

    fn flags(&self) -> Res<FlagList> {
        let p: &str = self.opt.get("flags").map(|s| s.as_str()).unwrap_or(FLAGS);
        let t: String = fs::read_to_string(p)?;
        flags::from_json(&json::parse_with(
            &t,
            &json::Limits {
                depth: 64,
                bytes: 64 << 20,
            },
        )?)
    }
}

fn load(p: &str, l: &FlagList) -> Res<(Save, Vec<u8>)> {
    let b: Vec<u8> = fs::read(p)?;
    Ok((save::parse(&b, l)?, b))
}

fn guard(inp: &str, out: &str) -> Res<()> {
    let o: String = out.replace('\\', "/").to_lowercase();
    if o.contains("/nand/") || (o.contains("/eden/") && o.contains("/save/")) {
        return Err(Error::Format(
            "save: refusing to write into an emulator save folder, copy the file by hand",
        ));
    }
    if P::new(inp) == P::new(out) {
        return Err(Error::Format("save: the output must differ from the input"));
    }
    Ok(())
}

fn emit(s: &Save, l: &FlagList, out: &str) -> Res<()> {
    let b: Vec<u8> = save::write(s);
    let back: Save = save::parse(&b, l)?;
    if back != *s || save::write(&back) != b {
        return Err(Error::Format(
            "save: the written file does not parse back identically",
        ));
    }
    fs::write(out, &b)?;
    println!("wrote {out} ({} bytes)", b.len());
    Ok(())
}

pub fn info(a: &[String]) -> Res<()> {
    let g: Args = split(a)?;
    let l: FlagList = g.flags()?;
    let (s, b): (Save, Vec<u8>) = load(g.need(0, "missing progress.sav")?, &l)?;
    for x in report(&s, &l, b.len()) {
        println!("{x}");
    }
    Ok(())
}

fn get_one(s: &Save, f: &Flag, i: Option<usize>) -> String {
    if f.ty == KEYED {
        return s.has_key(f.hash).to_string();
    }
    let h: u32 = f.hash as u32;
    match (s.get(h), i) {
        (None, _) => format!("not stored in this file (save index {})", f.save),
        (Some(Val::List(l)), Some(k)) => match l.get(k) {
            Some(v) => show(f, &v),
            None => format!("index out of range, length {}", l.len()),
        },
        (Some(v), _) => show(f, &v),
    }
}

pub fn get(a: &[String]) -> Res<()> {
    let g: Args = split(a)?;
    let l: FlagList = g.flags()?;
    let (s, _): (Save, Vec<u8>) = load(g.need(0, "missing progress.sav")?, &l)?;
    if g.pos.len() < 2 {
        return Err(Error::Format("save: give at least one flag name or hash"));
    }
    for q in &g.pos[1..] {
        let (f, i): (&Flag, Option<usize>) = target(&l, q)?;
        println!("{q} [{}] = {}", f.ty, get_one(&s, f, i));
    }
    Ok(())
}

fn set_one(s: &mut Save, l: &FlagList, spec: &str) -> Res<()> {
    let (q, v): (&str, &str) = spec
        .split_once('=')
        .ok_or(Error::Format("save: use name=value"))?;
    let (f, i): (&Flag, Option<usize>) = target(l, q)?;
    if f.ty == KEYED {
        let on: bool = matches!(v, "true" | "1");
        s.set_key(f.hash, on);
        return Ok(());
    }
    let x: Val = parse_val(f, v, i.is_some())?;
    match i {
        Some(k) => s.set_at(f.hash as u32, k, &x),
        None => s.set(f.hash as u32, &x),
    }
}

pub fn set(a: &[String]) -> Res<()> {
    let g: Args = split(a)?;
    let l: FlagList = g.flags()?;
    let inp: &str = g.need(0, "missing input progress.sav")?;
    let out: &str = g.need(1, "missing output path")?;
    guard(inp, out)?;
    let (mut s, _): (Save, Vec<u8>) = load(inp, &l)?;
    if g.pos.len() < 3 {
        return Err(Error::Format("save: give at least one name=value"));
    }
    for spec in &g.pos[2..] {
        set_one(&mut s, &l, spec)?;
        println!("set {spec}");
    }
    emit(&s, &l, out)
}

fn start_of(g: &Args, spec: &str) -> Res<(String, [f32; 3], f32)> {
    let root: &str = g
        .opt
        .get("romfs")
        .map(|s| s.as_str())
        .ok_or(Error::Format("save: --start needs --romfs <dump root>"))?;
    let (m, n): (&str, &str) = spec
        .split_once('/')
        .ok_or(Error::Format("save: use --start Map/StartPosName"))?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let list: Vec<Sp> = read_startpos(&rf.read_byml(STARTPOS)?);
    let p: &Sp = list
        .iter()
        .find(|p| p.map == m && p.name == n)
        .ok_or_else(|| Error::Msg(format!("save: no start position {spec}")))?;
    if p.trans.len() < 3 || p.rot.len() < 3 {
        return Err(Error::Format("save: start position has no Trans or Rot"));
    }
    let r: [f32; 3] = [p.rot[0], p.rot[1], p.rot[2]];
    Ok((
        p.map.clone(),
        [p.trans[0], p.trans[1], p.trans[2]],
        pos::yaw(r),
    ))
}

fn xyz(s: &str) -> Res<[f32; 3]> {
    let v: Vec<f32> = s
        .split(',')
        .map(|x| x.trim().parse::<f32>())
        .collect::<Result<Vec<f32>, _>>()
        .map_err(|_| Error::Format("save: --xyz needs x,y,z"))?;
    if v.len() != 3 {
        return Err(Error::Format("save: --xyz needs x,y,z"));
    }
    Ok([v[0], v[1], v[2]])
}

fn place_ops(g: &Args) -> Res<Vec<Op>> {
    if let Some(sp) = g.opt.get("start") {
        let (m, t, r): (String, [f32; 3], f32) = start_of(g, sp)?;
        return Ok(pos::ops(&m, t, r));
    }
    if let Some(m) = g.opt.get("map") {
        let t: [f32; 3] = xyz(g.opt.get("xyz").map(|s| s.as_str()).unwrap_or("0,0,0"))?;
        let r: f32 = g
            .opt
            .get("rad-y")
            .map(|s| s.parse::<f32>())
            .transpose()
            .map_err(|_| Error::Format("save: --rad-y needs a number"))?
            .unwrap_or(0.0);
        return Ok(pos::ops(m, t, r));
    }
    Ok(Vec::new())
}

fn report_applied(tag: &str, r: &Applied) {
    println!(
        "{tag}: changed {}, already set {}, not stored in this file {}",
        r.changed,
        r.same,
        r.missing.len()
    );
    for m in &r.missing {
        println!("  missing: {m}");
    }
}

pub fn unlock(a: &[String]) -> Res<()> {
    let g: Args = split(a)?;
    let l: FlagList = g.flags()?;
    let inp: &str = g.need(0, "missing input progress.sav")?;
    let out: &str = g.need(1, "missing output path")?;
    let spec: &str = g.need(2, "missing preset list")?;
    guard(inp, out)?;
    let (mut s, _): (Save, Vec<u8>) = load(inp, &l)?;
    let names: Vec<&str> = spec.split(',').collect();
    let ops: Vec<Op> = presets::ops_of(&names)?;
    let r: Applied = presets::apply(&mut s, &ops)?;
    report_applied(spec, &r);
    let p: Vec<Op> = place_ops(&g)?;
    if !p.is_empty() {
        let r: Applied = presets::apply(&mut s, &p)?;
        report_applied("start position", &r);
    }
    if let Some(k) = g.opt.get("keys") {
        if k != "clear" && k != "keep" {
            return Err(Error::Format("save: --keys must be clear or keep"));
        }
        if k == "clear" {
            let n: usize = s.keys.len();
            s.keys.clear();
            println!("cleared {n} collected pickup keys, every chest and pickup starts unopened");
        }
    }
    for (name, on) in [("key-on", true), ("key-off", false)] {
        let Some(list) = g.opt.get(name) else {
            continue;
        };
        for t in list.split(',') {
            let k: u64 = t.trim().parse().map_err(|_| Error::Format("save: a key is a decimal actor hash"))?;
            let changed: bool = s.set_key(k, on);
            println!("key {k} {} ({})", if on { "set" } else { "cleared" }, if changed { "changed" } else { "already so" });
        }
    }
    for sp in g.pos.iter().skip(3) {
        set_one(&mut s, &l, sp)?;
        println!("set {sp}");
    }
    emit(&s, &l, out)
}

pub fn check(a: &[String]) -> Res<()> {
    let g: Args = split(a)?;
    let l: FlagList = g.flags()?;
    let mut bad: usize = 0;
    for p in &g.pos {
        let b: Vec<u8> = fs::read(p)?;
        let s: Save = match save::parse(&b, &l) {
            Ok(s) => s,
            Err(e) => {
                println!("{p}: parse failed: {e}");
                bad += 1;
                continue;
            }
        };
        let w: Vec<u8> = save::write(&s);
        let same: bool = w == b;
        let sorted: bool = s.keys.windows(2).all(|x| x[0] < x[1]);
        let spare: usize = s.head[12..].iter().filter(|x| **x != 0).count();
        println!(
            "{p}: {} bytes, {} rows, {} keys (sorted {sorted}), trailing bytes {}, nonzero spare header bytes {spare}, rewrite byte-identical {same}",
            b.len(),
            s.rows.len(),
            s.keys.len(),
            s.tail.len()
        );
        if !same {
            bad += 1;
        }
    }
    if bad > 0 {
        return Err(Error::Format("save: at least one file failed the check"));
    }
    Ok(())
}

pub fn compare(a: &[String]) -> Res<()> {
    let g: Args = split(a)?;
    let l: FlagList = g.flags()?;
    let (x, _): (Save, Vec<u8>) = load(g.need(0, "missing first progress.sav")?, &l)?;
    let (y, _): (Save, Vec<u8>) = load(g.need(1, "missing second progress.sav")?, &l)?;
    let d: Vec<String> = diff(&x, &y, &l);
    for m in &d {
        println!("{m}");
    }
    println!("{} difference(s)", d.len());
    Ok(())
}

pub fn list_presets(a: &[String]) -> Res<()> {
    let g: Args = split(a)?;
    let l: FlagList = g.flags()?;
    let s: Option<Save> = match g.opt.get("in") {
        Some(p) => Some(load(p, &l)?.0),
        None => None,
    };
    for n in presets::NAMES {
        let ops: Vec<Op> = presets::ops(n)?;
        println!("preset {n}: {} flags", ops.len());
        if n == "everything" {
            println!("  = {}", presets::EVERYTHING.join(", "));
            continue;
        }
        for (k, v) in &ops {
            let h: u32 = hash_of(k);
            let f: Option<&Flag> = l.by_hash(h as u64);
            let in_list: &str = if f.is_some() { "listed" } else { "NOT IN LIST" };
            let in_save: &str = match &s {
                Some(sv) if sv.row(h).is_some() => "in save",
                Some(_) => "NOT IN SAVE",
                None => "",
            };
            let shown: String = match f {
                Some(f) => show(f, v),
                None => format!("{v:?}"),
            };
            println!(
                "  {k} = {shown}  (0x{:08x}, {in_list} {in_save})",
                hash_name(k)
            );
        }
    }
    Ok(())
}

pub fn starts(a: &[String]) -> Res<()> {
    let g: Args = split(a)?;
    let root: &str = g.need(0, "missing romfs_root")?;
    let pat: &str = g.pos.get(1).map(|s| s.as_str()).unwrap_or("");
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let mut n: usize = 0;
    for p in read_startpos(&rf.read_byml(STARTPOS)?) {
        let id: String = format!("{}/{}", p.map, p.name);
        if id.contains(pat) {
            println!("{id}  trans {:?}  rot {:?}", p.trans, p.rot);
            n += 1;
        }
    }
    println!("{n} start position(s)");
    Ok(())
}
