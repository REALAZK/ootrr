use std::fs;
use std::path::Path as P;
use std::sync::Arc;

use ootrr_core::colors::{self, Colors};
use ootrr_core::cosmetic::{self, Cosmetics, Music};
use ootrr_core::sfx::Sfx;
use ootrr_core::error::{Error, Res};
use ootrr_core::modpack::Host;
use ootrr_core::json::{self, Json};
use ootrr_core::settings::presets::{self, Preset};
use ootrr_core::settings::seed::SeedFile;
use ootrr_core::settings::{Loaded, Settings};
use ootrr_core::slice::generate::LIMIT;
use ootrr_core::slice::watch::Live;
use ootrr_core::slice::{self, plando, scan, BuildCounts, Built, GenOpts, Generated};

use crate::gen_progress;

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
    let live: Arc<Live> = Arc::new(Live::default());
    o.limit = Some(LIMIT);
    o.live = Some(Arc::clone(&live));
    let line: gen_progress::Line = gen_progress::start(live);
    let r: Res<Generated> = slice::generate_with(&s.settings, seed, &o);
    line.stop();
    let g: Generated = r?;
    g.seed_file.write(P::new(out))?;
    if let (Some(p), Some(t)) = (spoiler, &g.spoiler) {
        fs::write(p, t)?;
    }
    print!("{}", g.report);
    println!("seed file: {out}");
    Ok(())
}

pub fn build(a: &[String]) -> Res<()> {
    let mut rest: Vec<String> = Vec::new();
    let mut host: Host = Host::Eden;
    let mut cos: Cosmetics = Cosmetics::default();
    let mut i: usize = 0;
    while i < a.len() {
        if a[i] == "--music" {
            let v: &str = a.get(i + 1).map(|s| s.as_str()).ok_or(Error::Format("missing mode after --music"))?;
            cos.music = Music::parse(v).ok_or(Error::Format("--music is off, grouped or anywhere"))?;
            i += 2;
            continue;
        }
        if a[i] == "--sfx" {
            let v: &str = a.get(i + 1).map(|s| s.as_str()).ok_or(Error::Format("missing mode after --sfx"))?;
            cos.sfx = Sfx::parse(v).ok_or(Error::Format("--sfx is off, grouped or anywhere"))?;
            i += 2;
            continue;
        }
        if a[i] == "--colors" {
            let v: &str = a.get(i + 1).map(|s| s.as_str()).ok_or(Error::Format("missing mode after --colors"))?;
            cos.colors = Colors::parse(v).ok_or(Error::Format("--colors is off, random or shared"))?;
            i += 2;
            continue;
        }
        if a[i] == "--cosmetic-seed" {
            cos.seed = a.get(i + 1).cloned().ok_or(Error::Format("missing text after --cosmetic-seed"))?;
            i += 2;
            continue;
        }
        if a[i] == "--host" {
            let v: &str = a.get(i + 1).map(|s| s.as_str()).ok_or(Error::Format("missing host after --host"))?;
            host = Host::parse(v).ok_or(Error::Format("--host is eden, hookshot, longshot or nx2navi"))?;
            i += 2;
            continue;
        }
        rest.push(a[i].clone());
        i += 1;
    }
    let seed: SeedFile = SeedFile::read(P::new(arg(&rest, 0, "missing seed file")?))?;
    let romfs: &str = arg(&rest, 1, "missing romfs_root")?;
    let load: &str = arg(&rest, 2, "missing load_dir")?;
    let name: &str = arg(&rest, 3, "missing mod_name")?;
    let b: Built = slice::build_for(&seed, P::new(romfs), P::new(load), name, host)?;
    let c: &BuildCounts = &b.counts;
    print!("{}", b.report);
    for n in cosmetic::apply(&b.root, P::new(romfs), &cos, &seed.seed)? {
        println!("{n}");
    }
    println!("changed {} of {} placed chests", c.changed, c.placed);
    Ok(())
}

pub fn preview(a: &[String]) -> Res<()> {
    let romfs: &str = arg(a, 0, "missing romfs_root")?;
    let out: &str = arg(a, 1, "missing out_dir")?;
    let mode: Colors = Colors::parse(arg(a, 2, "missing mode")?).ok_or(Error::Format("the mode is random or shared"))?;
    for l in colors::preview(P::new(romfs), P::new(out), mode, arg(a, 3, "missing seed text")?)? {
        println!("{l}");
    }
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
