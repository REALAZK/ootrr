use std::collections::BTreeSet as BS;
use std::fs;
use std::path::{Path as P, PathBuf as PB};

use ootrr_core::bfevfl::flow::NONE;
use ootrr_core::bfevfl::{Bf, Flow, Kind};
use ootrr_core::byml::{Byml, Node};
use ootrr_core::error::{Error, Res};
use ootrr_core::romfs::Romfs;
use ootrr_core::rsizetable::{self, Rst};
use ootrr_core::teleport::{self, Cfg, FLOW, Mode};
use ootrr_core::teleport_dest::{self, Dest, start_name};
use ootrr_core::teleport_flow::{self, STEPS};
use ootrr_core::teleport_sim::{self, Sweep};

use crate::teleport_cmd::{arg, cfg, opt};

const ARRIVE: &str = "OcarinaWarp_Arrive";
const START_FILE: &str = "Banc/BancBasedResource/StartPos.byml.zs";

fn changed(a: &Flow, b: &Flow) -> Vec<String> {
    a.events.iter().enumerate().filter(|(i, e)| b.events[*i] != **e).map(|(_, e)| e.name.clone()).collect()
}

fn entry_main(f: &Flow, n: &str) -> Res<u16> {
    f.entries.iter().find(|e| e.name == n).map(|e| e.main).ok_or_else(|| Error::Msg(format!("check: no entry {n}")))
}

fn orphans(f: &Flow, first: usize) -> Vec<usize> {
    let mut seen: BS<usize> = BS::new();
    for e in &f.entries {
        seen.extend(teleport_sim::reach(f, e.main));
    }
    (first..f.events.len()).filter(|i| !seen.contains(i)).collect()
}

fn ranges(f: &Flow) -> usize {
    let mut bad: usize = 0;
    for e in &f.events {
        let t: Vec<u16> = match &e.kind {
            Kind::Action { next, .. } | Kind::Join { next } | Kind::Sub { next, .. } => vec![*next],
            Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => cases.iter().map(|c| c.ev).collect(),
            Kind::Fork { join, forks } => forks.iter().copied().chain(std::iter::once(*join)).collect(),
            Kind::Label { .. } => Vec::new(),
        };
        bad += t.iter().filter(|x| **x != NONE && **x as usize >= f.events.len()).count();
    }
    bad
}

fn vec3(n: Option<&Node>) -> Vec<f32> {
    let Some(Node::Arr(a)) = n else {
        return Vec::new();
    };
    a.iter().filter_map(|x| if let Node::F32(v) = x { Some(*v) } else { None }).collect()
}

fn starts(orig: &Byml, new: &Byml, ds: &[Dest]) -> Res<(usize, usize, usize)> {
    let Node::Map(og) = &orig.root else {
        return Err(Error::Format("check: StartPos root is not a map"));
    };
    let mut total: usize = 0;
    let mut same: usize = 0;
    for (g, n) in og {
        let Node::Map(es) = n else {
            continue;
        };
        for (k, v) in es {
            total += 1;
            if new.root.get(g).and_then(|x| x.get(k)) == Some(v) {
                same += 1;
            }
        }
    }
    let mut good: usize = 0;
    for d in ds {
        let e: Option<&Node> = new.root.get(&d.banc).and_then(|g| g.get(&start_name(d)));
        let t: Vec<f32> = vec3(e.and_then(|x| x.get("Trans")));
        let r: Vec<f32> = vec3(e.and_then(|x| x.get("Rot")));
        if t == d.pos.to_vec() && r == vec![0.0, d.yaw, 0.0] {
            good += 1;
        }
    }
    Ok((total, same, good))
}

fn orig(rf: &mut Romfs, base: Option<&P>, path: &str) -> Res<Vec<u8>> {
    match base.map(|b| b.join("romfs").join(path)).filter(|p| p.is_file()) {
        Some(p) => rf.zs.decompress(&fs::read(p)?),
        None => rf.read_zs(path),
    }
}

fn flow_part(rf: &mut Romfs, modd: &P, base: Option<&P>) -> Res<(Bf, Bf, usize, usize)> {
    let mz: Vec<u8> = fs::read(modd.join("romfs").join(teleport::FLOW_FILE))?;
    let oraw: Vec<u8> = orig(rf, base, teleport::FLOW_FILE)?;
    let mraw: Vec<u8> = rf.zs.decompress(&mz)?;
    let mb: Bf = Bf::parse(&mraw)?;
    println!("flow bytes {} -> {}, rewrite byte identical: {}", oraw.len(), mraw.len(), mb.write()? == mraw);
    Ok((Bf::parse(&oraw)?, mb, oraw.len(), mraw.len()))
}

fn start_part(rf: &mut Romfs, modd: &P, ds: &[Dest], base: Option<&P>) -> Res<()> {
    let po: Byml = Byml::parse(&orig(rf, base, START_FILE)?)?;
    let pz: Vec<u8> = fs::read(modd.join("romfs").join(START_FILE))?;
    let raw: Vec<u8> = rf.zs.decompress(&pz)?;
    let pm: Byml = Byml::parse(&raw)?;
    let (total, same, good): (usize, usize, usize) = starts(&po, &pm, ds)?;
    println!("StartPos: vanilla entries {total} unchanged {same}, new entries correct {good} of {}, rewrite byte identical: {}", ds.len(), pm.write()? == raw);
    Ok(())
}

fn table_part(rf: &mut Romfs, modd: &P, real: usize, base: Option<&P>) -> Res<()> {
    let has: bool = base.is_some_and(|b| b.join("romfs").join(rsizetable::TABLE_PATH).is_file());
    let ro: Rst = if has { Rst::parse(&orig(rf, base, rsizetable::TABLE_PATH)?)? } else { rsizetable::load(rf)? };
    let z: Vec<u8> = fs::read(modd.join("romfs").join(rsizetable::TABLE_PATH))?;
    let rn: Rst = Rst::parse(&rf.zs.decompress(&z)?)?;
    let mut diffs: Vec<String> = Vec::new();
    for (k, v) in &rn.crc {
        if ro.crc.iter().find(|e| e.0 == *k).map(|e| e.1) != Some(*v) {
            diffs.push(format!("{k:#010x}={v}"));
        }
    }
    println!("size table: {} entries differ from the original {:?}, entry counts {} and {} against {} and {}", diffs.len(), diffs, rn.crc.len(), rn.names.len(), ro.crc.len(), ro.names.len());
    println!("flow resource: table {:?} -> {:?} for {} real bytes", ro.get(teleport::FLOW_FILE), rn.get(teleport::FLOW_FILE), real);
    Ok(())
}

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let modd: PB = PB::from(arg(a, 1, "missing mod folder (the one that holds romfs)")?);
    let tsv: &str = arg(a, 2, "missing destinations tsv")?;
    let mode: Mode = Mode::parse(opt(a, "--mode").unwrap_or("start"))?;
    let c: Cfg = cfg(a, mode)?;
    let ds: Vec<Dest> = teleport_dest::parse(&fs::read_to_string(tsv)?)?;
    let base: Option<PB> = opt(a, "--base").map(PB::from);
    let bp: Option<&P> = base.as_deref();
    if let Some(b) = bp {
        println!("base: the seed mod {} (files it holds replace the vanilla originals)", b.display());
    }
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let (ob, mb, _, real): (Bf, Bf, usize, usize) = flow_part(&mut rf, &modd, bp)?;
    let of: &Flow = ob.flows.iter().find(|f| f.name == FLOW).ok_or(Error::Format("check: no original flow"))?;
    let mf: &Flow = mb.flows.iter().find(|f| f.name == FLOW).ok_or(Error::Format("check: no modded flow"))?;
    let n0: usize = of.events.len();
    let ch: Vec<String> = changed(of, mf);
    println!("events {} -> {} (added {}), original events changed: {} {:?}", n0, mf.events.len(), mf.events.len() - n0, ch.len(), ch);
    println!("entries added: {:?}", mf.entries.iter().skip(of.entries.len()).map(|e| e.name.as_str()).collect::<Vec<&str>>());
    println!("targets out of range: {}", ranges(mf));
    println!("actor, action or query indexes out of range: {}", teleport_sim::indexes_wrong(mf));
    println!("entries whose subs list is not the calls they reach: {} {:?} (original {:?})", teleport_sim::subs_wrong(mf).len(), teleport_sim::subs_wrong(mf), teleport_sim::subs_wrong(of));
    let lost: Vec<usize> = orphans(mf, n0);
    println!("added events not reachable from any entry: {} {:?}", lost.len(), &lost[..lost.len().min(10)]);
    let sw: Sweep = teleport_sim::sweep(mf, &ds, &c.flag, &c.mflag, c.groups)?;
    println!("simulated walks: {} wrong: {} deepest: {}", sw.runs, sw.bad.len(), sw.deepest);
    for b in sw.bad.iter().take(10) {
        println!("  {b}");
    }
    let clean_o: bool = teleport_sim::arrival_clean(of, entry_main(of, ARRIVE)?)?;
    let clean_m: bool = teleport_sim::arrival_clean(mf, entry_main(mf, ARRIVE)?)?;
    println!("arrival event sets the song flags: original {} modded {}", !clean_o, !clean_m);
    let gs: Vec<(usize, usize)> = teleport_flow::groups_of(&ds)?;
    println!("destinations {} groups {} songs hooked {}", ds.len(), gs.len(), STEPS.iter().filter(|s| c.groups || !s.extra()).count());
    if mode == Mode::Start {
        start_part(&mut rf, &modd, &ds, bp)?;
    }
    table_part(&mut rf, &modd, real, bp)
}
