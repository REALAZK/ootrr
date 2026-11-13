use std::fs;
use std::path::{Path as P, PathBuf as PB};

use ootrr_core::error::{Error, Res};
use ootrr_core::romfs::Romfs;
use ootrr_core::teleport::{Cfg, Mode};
use ootrr_core::teleport_dest::{self, Dest};
use ootrr_core::teleport_merge::{self, Merged, Verify};

use crate::teleport_cmd::{arg, cfg, opt};

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let sload: &str = arg(a, 1, "missing seed_load_dir")?;
    let sname: &str = arg(a, 2, "missing seed_mod_name")?;
    let tsv: &str = arg(a, 3, "missing destinations tsv")?;
    let oload: &str = arg(a, 4, "missing out_load_dir")?;
    let oname: &str = arg(a, 5, "missing out_mod_name")?;
    let mode: Mode = Mode::parse(opt(a, "--mode").unwrap_or("start"))?;
    let c: Cfg = cfg(a, mode)?;
    let ds: Vec<Dest> = teleport_dest::parse(&fs::read_to_string(tsv)?)?;
    let m: Merged = teleport_merge::merge(P::new(root), P::new(sload), sname, P::new(oload), oname, &c, &ds)?;
    print!("{}", m.report);
    println!("{} destinations, combined mod at {}", ds.len(), m.root.display());
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let seed: PB = teleport_merge::mod_root(P::new(sload), sname);
    let v: Verify = teleport_merge::verify(&mut rf, &seed, &m.root, &c, &ds)?;
    print!("{}", teleport_merge::text(&v));
    if !v.ok() {
        return Err(Error::Format("tp-merge: the combined mod failed its checks"));
    }
    Ok(())
}
