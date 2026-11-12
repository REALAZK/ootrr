use std::path::Path as P;

use ootrr_core::error::{Error, Res};
use ootrr_core::pot_slice::persist::{build, Built};

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = a.first().map(|s| s.as_str()).ok_or(Error::Format("missing romfs_root"))?;
    let load: &str = a.get(1).map(|s| s.as_str()).ok_or(Error::Format("missing load_dir"))?;
    let name: &str = a.get(2).map(|s| s.as_str()).ok_or(Error::Format("missing mod_name"))?;
    let b: Built = build(P::new(root), P::new(load), name)?;
    print!("{}", b.report);
    Ok(())
}
