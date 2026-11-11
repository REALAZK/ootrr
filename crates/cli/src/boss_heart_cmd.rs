use std::path::Path as P;

use ootrr_core::boss_heart::{build, Opts, CHEST};
use ootrr_core::error::{Error, Res};

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(Error::Format(what))
}

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let load: &str = arg(a, 1, "missing load_dir")?;
    let name: &str = arg(a, 2, "missing mod_name")?;
    let mut o: Opts = Opts { rst_from: None, banc_from: None, chest: CHEST.to_string(), slot: false };
    let mut picks: Vec<(String, String)> = Vec::new();
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
            "--slot" => {
                o.slot = true;
                i += 1;
            }
            "--chest" => {
                o.chest = arg(a, i + 1, "missing chest actor")?.to_string();
                i += 2;
            }
            s => {
                let (l, v): (&str, &str) = s
                    .rsplit_once('=')
                    .ok_or_else(|| Error::Msg(format!("{s} is not Location=Item_Actor")))?;
                picks.push((l.trim().to_string(), v.trim().to_string()));
                i += 1;
            }
        }
    }
    if picks.is_empty() {
        return Err(Error::Format("need at least one Location=Item_Actor pair"));
    }
    let b = build(P::new(root), P::new(load), name, &picks, &o)?;
    print!("{}", b.report);
    println!("wrote {} files, {} needed a resource size table update", b.files, b.grown);
    Ok(())
}
