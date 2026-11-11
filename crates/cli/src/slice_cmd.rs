use std::fs;
use std::path::Path as P;

use ootrr_core::error::{Error, Res};
use ootrr_core::json::{self, Json};
use ootrr_core::settings::presets::{self, Preset};
use ootrr_core::settings::seed::SeedFile;
use ootrr_core::settings::{Loaded, Settings};
use ootrr_core::slice::{self, plando, scan, BuildCounts, Built, GenOpts, Generated};

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(Error::Format(what))
}

fn resolve(t: &str) -> Res<Loaded> {
    if t.starts_with("OOTRR") {
        return Settings::decode(t);
    }
    if let Some(p) = presets::builtin()?.into_iter().find(|p| p.id == t) {
        return Ok(Loaded { settings: p.apply(&Settings::default())?, warnings: Vec::new() });
    }
    let p: &P = P::new(t);
    if !p.is_file() {
        return Err(Error::Msg(format!("{t} is not a settings string, a preset id or a file")));
    }
    let text: String = fs::read_to_string(p)?;
    let j: Json = json::parse(&text)?;
    if j.get("id").is_some() {
        let pr: Preset = presets::parse(&text)?;
        return Ok(Loaded { settings: pr.apply(&Settings::default())?, warnings: Vec::new() });
    }
    Settings::from_json(&j)
}

pub fn generate(a: &[String]) -> Res<()> {
    let s: Loaded = resolve(arg(a, 0, "missing settings string, preset id or file")?)?;
    let seed: &str = arg(a, 1, "missing seed")?;
    let out: &str = arg(a, 2, "missing out file")?;
    let mut o: GenOpts = GenOpts::default();
    let mut spoiler: Option<&str> = None;
    let mut i: usize = 3;
    while i < a.len() {
        match a[i].as_str() {
            "--spoiler" => {
                i += 1;
                spoiler = Some(arg(a, i, "--spoiler needs a file")?);
                o.spoiler = true;
            }
            "--plando" => {
                i += 1;
                o.plando.push(plando::parse(arg(a, i, "--plando needs Location=Item")?)?);
            }
            "--plando-file" => {
                i += 1;
                let t: String = fs::read_to_string(arg(a, i, "--plando-file needs a file")?)?;
                for line in t.lines().map(|l| l.trim()).filter(|l| !l.is_empty() && !l.starts_with('#')) {
                    o.plando.push(plando::parse(line)?);
                }
            }
            other => return Err(Error::Msg(format!("unknown option {other}"))),
        }
        i += 1;
    }
    for w in &s.warnings {
        eprintln!("warning: {w}");
    }
    let g: Generated = slice::generate_with(&s.settings, seed, &o)?;
    g.seed_file.write(P::new(out))?;
    if let (Some(p), Some(t)) = (spoiler, &g.spoiler) {
        fs::write(p, t)?;
    }
    print!("{}", g.report);
    println!("seed file: {out}");
    Ok(())
}

pub fn build(a: &[String]) -> Res<()> {
    let seed: SeedFile = SeedFile::read(P::new(arg(a, 0, "missing seed file")?))?;
    let romfs: &str = arg(a, 1, "missing romfs_root")?;
    let load: &str = arg(a, 2, "missing load_dir")?;
    let name: &str = arg(a, 3, "missing mod_name")?;
    let b: Built = slice::build(&seed, P::new(romfs), P::new(load), name)?;
    let c: &BuildCounts = &b.counts;
    print!("{}", b.report);
    println!("changed {} of {} placed chests", c.changed, c.placed);
    Ok(())
}

pub fn scan_cmd(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let out: &str = arg(a, 1, "missing out file")?;
    let s = scan(P::new(root))?;
    let mut t: String = scan::to_json(&s).pretty();
    t.push('\n');
    fs::write(out, t)?;
    print!("{}", scan::summary(&s));
    Ok(())
}
