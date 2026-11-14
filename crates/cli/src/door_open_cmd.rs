use std::path::Path as P;

use ootrr_core::door_open::build;
use ootrr_core::error::{Error, Res};

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(Error::Format(what))
}

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let load: &str = arg(a, 1, "missing load_dir")?;
    let name: &str = arg(a, 2, "missing mod_name")?;
    let report: String = build(P::new(root), P::new(load), name)?;
    print!("{report}");
    Ok(())
}
