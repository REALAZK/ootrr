use std::path::Path as P;

use ootrr_core::error::{Error, Res};
use ootrr_core::progressive::{build, Opts, FAMILIES};

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(Error::Format(what))
}

fn plan(s: &str) -> Res<(String, Vec<u64>)> {
    let (f, h): (&str, &str) = s
        .split_once('=')
        .ok_or_else(|| Error::Msg(format!("{s} is not Family=hash,hash")))?;
    let mut out: Vec<u64> = Vec::new();
    for x in h.split(',') {
        out.push(x.trim().parse().map_err(|_| Error::Msg(format!("{x} is not an actor hash")))?);
    }
    if out.is_empty() {
        return Err(Error::Format("a family needs at least one chest hash"));
    }
    Ok((f.trim().to_string(), out))
}

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let load: &str = arg(a, 1, "missing load_dir")?;
    let name: &str = arg(a, 2, "missing mod_name")?;
    let mut o: Opts = Opts { scale: None, rst_from: None, banc_from: None, grants: false };
    let mut plans: Vec<(String, Vec<u64>)> = Vec::new();
    let mut i: usize = 3;
    while i < a.len() {
        if a[i] == "--scale" {
            let f: f32 = arg(a, i + 1, "missing factor")?.parse().map_err(|_| Error::Format("bad factor"))?;
            let m: i32 = arg(a, i + 2, "missing min_frames")?.parse().map_err(|_| Error::Format("bad min_frames"))?;
            if !(0.0..=1.0).contains(&f) {
                return Err(Error::Format("the factor must be from 0 to 1"));
            }
            o.scale = Some((f, m));
            i += 3;
            continue;
        }
        if a[i] == "--rst-from" {
            o.rst_from = Some(arg(a, i + 1, "missing size table file")?.into());
            i += 2;
            continue;
        }
        if a[i] == "--grants" {
            o.grants = true;
            i += 1;
            continue;
        }
        if a[i] == "--banc-from" {
            o.banc_from = Some(arg(a, i + 1, "missing mod romfs folder")?.into());
            i += 2;
            continue;
        }
        plans.push(plan(&a[i])?);
        i += 1;
    }
    if plans.is_empty() {
        let names: Vec<&str> = FAMILIES.iter().map(|f| f.0).collect();
        return Err(Error::Msg(format!("need at least one Family=hash list, families: {}", names.join(" "))));
    }
    let b = build(P::new(root), P::new(load), name, &plans, &o)?;
    print!("{}", b.report);
    println!("wrote {} files, {} needed a resource size table update", b.files, b.grown);
    Ok(())
}
