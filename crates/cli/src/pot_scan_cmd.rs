use std::fs;
use std::path::Path as P;

use ootrr_core::error::{Error, Res};
use ootrr_core::pot_slice::scan::{scan, summary, to_json, PotScan};

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = a.first().map(|s| s.as_str()).ok_or(Error::Format("missing romfs_root"))?;
    let out: &str = a.get(1).map(|s| s.as_str()).ok_or(Error::Format("missing out file"))?;
    let s: PotScan = scan(P::new(root))?;
    let mut t: String = to_json(&s).pretty();
    t.push('\n');
    fs::write(out, t)?;
    print!("{}", summary(&s));
    Ok(())
}
