use std::collections::BTreeMap as Map;
use std::fs;
use std::path::Path as P;
use std::time::Instant;

use ootrr_core::error::{Error, Res};
use ootrr_core::flags::{self, Body, Flag, FlagList};
use ootrr_core::flags_build::{self, Built};
use ootrr_core::flags_harvest::Evidence;
use ootrr_core::flags_map::{self, Concept, Row};
use ootrr_core::json::{self, Json};
use ootrr_core::romfs::Romfs;

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(Error::Format(what))
}

fn read_list(p: &str) -> Res<FlagList> {
    let t: String = fs::read_to_string(p)?;
    flags::from_json(&json::parse_with(
        &t,
        &json::Limits {
            depth: 64,
            bytes: 64 << 20,
        },
    )?)
}

fn by_type(l: &FlagList) -> Map<String, (usize, usize)> {
    let mut out: Map<String, (usize, usize)> = Map::new();
    for f in &l.flags {
        let e: &mut (usize, usize) = out.entry(f.ty.clone()).or_default();
        e.0 += 1;
        if f.name.is_some() {
            e.1 += 1;
        }
    }
    out
}

fn by_method(l: &FlagList) -> Map<String, usize> {
    let mut out: Map<String, usize> = Map::new();
    for f in &l.flags {
        if let Some(h) = &f.how {
            *out.entry(h.clone()).or_default() += 1;
        }
    }
    out
}

fn counts_json(l: &FlagList) -> Json {
    let mut t: Json = Json::obj();
    for (k, v) in by_type(l) {
        t.put(
            &k,
            Json::obj()
                .with("total", Json::Int(v.0 as i64))
                .with("named", Json::Int(v.1 as i64)),
        );
    }
    let mut m: Json = Json::obj();
    for (k, v) in by_method(l) {
        m.put(&k, Json::Int(v as i64));
    }
    Json::obj()
        .with("total", Json::Int(l.len() as i64))
        .with("named", Json::Int(l.named() as i64))
        .with("by_type", t)
        .with("by_method", m)
}

fn print_counts(l: &FlagList) {
    println!("flags: {}  labelled: {}", l.len(), l.named());
    println!("by type (total, labelled):");
    for (k, v) in by_type(l) {
        println!("  {k}: {} {}", v.0, v.1);
    }
    println!("by method:");
    for (k, v) in by_method(l) {
        println!("  {k}: {v}");
    }
}

fn print_concepts(l: &FlagList, ev: &Evidence) {
    for (k, rs) in flags_map::rows(l, ev) {
        let n: usize = rs.iter().filter(|r| r.found).count();
        println!(
            "{} [{} {}] {}/{} flags",
            k.id,
            flags_map::status(&rs, k.conf),
            k.conf,
            n,
            rs.len()
        );
        print_rows(&k, &rs);
    }
}

fn print_rows(k: &Concept, rs: &[Row]) {
    for r in rs {
        let f: &str = r.files.first().map(|s| s.as_str()).unwrap_or("-");
        let m: &str = if r.found { "ok" } else { "MISSING" };
        println!("    {m} {} {} {} {}", r.name, r.ty, r.how, f);
    }
    println!("    basis: {}", k.basis);
}

pub fn build(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let out: &str = arg(a, 1, "missing out_json")?;
    let t0: Instant = Instant::now();
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let b: Built = flags_build::build(&mut rf)?;
    println!("elapsed: {:.1}s", t0.elapsed().as_secs_f32());
    println!(
        "walked {} files, {} MB, strings {}, candidates tested {}",
        b.walked.files,
        b.walked.bytes >> 20,
        b.strings,
        b.tested
    );
    let n: u64 = b.list.flags.iter().filter(|f| f.ty != flags::KEYED).count() as u64;
    println!(
        "expected random hash collisions across all candidates: {:.1}",
        (b.tested as f64) * (n as f64) / 4294967296.0
    );
    for (k, v) in &b.stages {
        println!("  after {k}: {v}");
    }
    print_counts(&b.list);
    let extra: Vec<(String, Json)> = vec![
        ("counts".to_string(), counts_json(&b.list)),
        (
            "concepts".to_string(),
            flags_map::to_json(&b.list, &b.evidence),
        ),
    ];
    fs::write(out, flags::render(&b.list, extra))?;
    println!("wrote {out}");
    print_concepts(&b.list, &b.evidence);
    Ok(())
}

pub fn lookup(a: &[String]) -> Res<()> {
    let src: &str = arg(a, 0, "missing flags_json")?;
    let l: FlagList = read_list(src)?;
    if a.len() < 2 {
        return Err(Error::Format("missing name or hash"));
    }
    for q in &a[1..] {
        match l.find(q) {
            Some(f) => println!("{}", describe(f, &l)),
            None => println!("{q}: not in the list"),
        }
    }
    Ok(())
}

fn describe(f: &Flag, l: &FlagList) -> String {
    let name: &str = f.name.as_deref().unwrap_or("-");
    let how: &str = f.how.as_deref().unwrap_or("-");
    let mut s: String = format!(
        "{} {} {} default={} how={}",
        flags::hex(f.hash, &f.ty),
        f.ty,
        name,
        f.def,
        how
    );
    if let Body::Members(m) = &f.body {
        let n: usize = m
            .iter()
            .filter(|x| l.by_hash(x.1 as u64).is_some_and(|y| y.name.is_some()))
            .count();
        s.push_str(&format!(" members={} named={}", m.len(), n));
    }
    if let Body::Values(v) = &f.body {
        s.push_str(&format!(" values={}", v.join(",")));
    }
    s
}

pub fn check(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let src: &str = arg(a, 1, "missing flags_json")?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let game: FlagList = flags_build::load(&mut rf)?;
    let l: FlagList = read_list(src)?;
    let mut bad: usize = 0;
    let mut rehashed: usize = 0;
    let mut labelled: usize = 0;
    let mut missing: usize = 0;
    let mut differ: usize = 0;
    for f in &l.flags {
        let Some(g) = game.by_hash(f.hash) else {
            missing += 1;
            continue;
        };
        if g.ty != f.ty
            || g.def != f.def
            || g.reset != f.reset
            || g.save != f.save
            || g.body != f.body
        {
            differ += 1;
        }
        let Some(n) = &f.name else {
            continue;
        };
        if f.ty == flags::KEYED {
            labelled += 1;
        } else if flags::hash_name(n) as u64 == f.hash {
            rehashed += 1;
        } else {
            bad += 1;
        }
    }
    let absent: usize = game
        .flags
        .iter()
        .filter(|g| l.by_hash(g.hash).is_none())
        .count();
    println!("game flags: {}  json flags: {}", game.len(), l.len());
    println!("json flags missing from the game list: {missing}");
    println!("game flags absent from the json: {absent}");
    println!("entries whose type, default, reset, save or body differ from the game: {differ}");
    println!("names that re-hash to the stored hash: {rehashed}");
    println!("names that do not re-hash: {bad}");
    println!("keyed labels (placement references, not hashed): {labelled}");
    if bad + missing + absent + differ > 0 {
        return Err(Error::Format("flags json does not match the game list"));
    }
    Ok(())
}

pub fn report(a: &[String]) -> Res<()> {
    let src: &str = arg(a, 0, "missing flags_json")?;
    let l: FlagList = read_list(src)?;
    print_counts(&l);
    print_concepts(&l, &Evidence::new());
    Ok(())
}
