use std::fs;
use std::path::Path as P;

use ootrr_core::error::{Error, Res};
use ootrr_core::romfs::Romfs;
use ootrr_core::teleport::{self, Cfg, Layout, Mode, Unit};
use ootrr_core::teleport_dest::{self, Dest, Stands};

pub fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(Error::Format(what))
}

pub fn opt<'a>(a: &'a [String], k: &str) -> Option<&'a str> {
    let i: usize = a.iter().position(|s| s == k)?;
    a.get(i + 1).map(|s| s.as_str())
}

fn onoff(a: &[String], k: &str, d: bool) -> Res<bool> {
    match opt(a, k) {
        None => Ok(d),
        Some("on") => Ok(true),
        Some("off") => Ok(false),
        Some(_) => Err(Error::Format("teleport: an on or off option has another value")),
    }
}

pub fn cfg(a: &[String], mode: Mode) -> Res<Cfg> {
    let mut c: Cfg = Cfg::new();
    c.mode = mode;
    if let Some(f) = opt(a, "--flag") {
        c.flag = f.to_string();
    }
    if let Some(f) = opt(a, "--mflag") {
        c.mflag = f.to_string();
    }
    c.unit = Unit::parse(opt(a, "--rot").unwrap_or("deg"))?;
    c.layout = match opt(a, "--layout") {
        Some(s) => Layout::parse(s)?,
        None if mode == Mode::Coords => Layout::Direct,
        None => Layout::Shared,
    };
    c.groups = onoff(a, "--groups", true)?;
    c.autosave = onoff(a, "--autosave", false)?;
    Ok(c)
}

fn stands(a: &[String]) -> Res<Stands> {
    let mut st: Stands = Stands::new();
    if let Some(v) = opt(a, "--stand") {
        st.parse(v)?;
    }
    Ok(st)
}

pub fn chests(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let out: &str = arg(a, 1, "missing out_tsv")?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let mut v: Vec<Dest> = teleport_dest::chests(&mut rf, &stands(a)?)?;
    if let Some(f) = a.get(2).filter(|s| !s.starts_with("--")) {
        v.retain(|d| d.banc.contains(f.as_str()));
    }
    fs::write(out, teleport_dest::to_text(&v))?;
    println!("{} chest destinations", v.len());
    Ok(())
}

pub fn picks(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let out: &str = arg(a, 1, "missing out_tsv")?;
    let pairs: &str = arg(a, 2, "missing pairs_tsv")?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let v: Vec<Dest> = teleport_dest::picks(&mut rf, &fs::read_to_string(pairs)?, &stands(a)?)?;
    fs::write(out, teleport_dest::to_text(&v))?;
    println!("{} destinations", v.len());
    Ok(())
}

pub fn build(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let load: &str = arg(a, 1, "missing load_dir")?;
    let name: &str = arg(a, 2, "missing mod_name")?;
    let mode: Mode = Mode::parse(arg(a, 3, "missing mode")?)?;
    let tsv: &str = arg(a, 4, "missing destinations tsv")?;
    let c: Cfg = cfg(a, mode)?;
    let ds: Vec<Dest> = teleport_dest::parse(&fs::read_to_string(tsv)?)?;
    let report: String = teleport::build(P::new(root), P::new(load), name, &c, &ds)?;
    print!("{report}");
    println!("{} destinations", ds.len());
    Ok(())
}
