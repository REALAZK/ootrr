use std::path::Path as P;

use ootrr_core::dungeon_credit::{build, Opts, DUNGEONS, PLAIN};
use ootrr_core::error::{Error, Res};
use ootrr_core::progressive::Built;

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(Error::Format(what))
}

fn plan(s: &str) -> Res<(u64, Option<String>)> {
    let (h, d): (&str, &str) = s
        .split_once('=')
        .ok_or_else(|| Error::Msg(format!("{s} is not ChestActorHash=Dungeon")))?;
    let hash: u64 = h.trim().parse().map_err(|_| Error::Msg(format!("{h} is not an actor hash")))?;
    let d: &str = d.trim();
    Ok((hash, if d == PLAIN { None } else { Some(d.to_string()) }))
}

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let load: &str = arg(a, 1, "missing load_dir")?;
    let name: &str = arg(a, 2, "missing mod_name")?;
    let mut o: Opts = Opts { rst_from: None, banc_from: None, count: Vec::new() };
    let mut plans: Vec<(u64, Option<String>)> = Vec::new();
    let mut i: usize = 3;
    while i < a.len() {
        match a[i].as_str() {
            "--rst-from" => {
                o.rst_from = Some(arg(a, i + 1, "missing size table file")?.into());
                i += 2;
            }
            "--banc-from" => {
                o.banc_from = Some(arg(a, i + 1, "missing mod romfs folder")?.into());
                i += 2;
            }
            "--count" => {
                o.count.push(arg(a, i + 1, "missing dungeon for --count")?.to_string());
                i += 2;
            }
            s => {
                plans.push(plan(s)?);
                i += 1;
            }
        }
    }
    if plans.is_empty() {
        return Err(Error::Msg(format!("need at least one ChestActorHash=Dungeon pair, dungeons: {} or {PLAIN} for a key that credits nothing", DUNGEONS.join(" "))));
    }
    let b: Built = build(P::new(root), P::new(load), name, &plans, &o)?;
    print!("{}", b.report);
    println!("wrote {} files, {} needed a resource size table update", b.files, b.grown);
    Ok(())
}
