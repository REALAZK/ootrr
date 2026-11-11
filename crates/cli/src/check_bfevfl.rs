use std::collections::BTreeMap as Map;
use std::path::Path as P;
use std::time::Instant;

use ootrr_core::bfevfl::{replace_bytes, rewards, scale_waits, show, Bf, Kind, Paced, Replaced, Val};
use ootrr_core::modpack::Pack;
use ootrr_core::TITLE_ID;
use ootrr_core::error::{Error, Res};
use ootrr_core::romfs::Romfs;
use ootrr_core::zs;

const PLACEHOLDER: &str = "Item_Placeholder_Test";

fn dict_id(z: &[u8]) -> Option<u32> {
    let d: u8 = *z.get(4)?;
    let n: usize = [0, 1, 2, 4][(d & 3) as usize];
    let mut id: u32 = 0;
    for i in 0..n {
        id |= (*z.get(5 + i)? as u32) << (8 * i);
    }
    if n == 0 { None } else { Some(id) }
}

fn explain(a: &[u8], b: &[u8]) -> String {
    let n: usize = a.len().min(b.len());
    let at: usize = (0..n).find(|&i| a[i] != b[i]).unwrap_or(n);
    format!("sizes {} vs {}, first difference at {:#x}", a.len(), b.len(), at)
}

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = a.first().ok_or(Error::Format("missing romfs_root"))?;
    let sub: &str = a.get(1).ok_or(Error::Format("missing subfolder"))?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let files: Vec<String> = rf
        .list(sub)?
        .into_iter()
        .filter(|p| p.ends_with(".bfevfl.zs") || p.ends_with(".bfevfl"))
        .collect();
    let t0: Instant = Instant::now();
    let mut same: usize = 0;
    let mut parsed: usize = 0;
    let mut zsok: usize = 0;
    let mut swaps: usize = 0;
    let mut swaps_ok: usize = 0;
    let mut kinds: Map<String, usize> = Map::new();
    let mut shown: Vec<String> = Vec::new();
    for f in &files {
        let zraw: Vec<u8> = rf.read(f)?;
        let raw: Vec<u8> = if f.ends_with(".zs") { rf.zs.decompress(&zraw)? } else { zraw.clone() };
        let bf: Bf = match Bf::parse(&raw) {
            Ok(b) => b,
            Err(e) => {
                *kinds.entry(format!("parse: {e}")).or_insert(0) += 1;
                shown.push(format!("{f}: parse: {e}"));
                continue;
            }
        };
        parsed += 1;
        let out: Vec<u8> = match bf.write() {
            Ok(o) => o,
            Err(e) => {
                *kinds.entry(format!("write: {e}")).or_insert(0) += 1;
                shown.push(format!("{f}: write: {e}"));
                continue;
            }
        };
        if out == raw {
            same += 1;
        } else {
            let why: String = explain(&raw, &out);
            *kinds.entry("bytes differ".to_string()).or_insert(0) += 1;
            shown.push(format!("{f}: {why}"));
        }
        if let Some(en) = rewards::entry(f, &bf) {
            let mut items: Vec<&str> = en.hits.iter().map(|h| h.item.as_str()).collect();
            items.sort_unstable();
            items.dedup();
            for it in items {
                swaps += 1;
                let ok: bool = replace_bytes(&raw, it, PLACEHOLDER)
                    .and_then(|(o, _)| replace_bytes(&o, PLACEHOLDER, it))
                    .is_ok_and(|(b, _)| b == raw);
                if ok {
                    swaps_ok += 1;
                } else {
                    shown.push(format!("{f}: replacement of {it} did not restore the original"));
                }
            }
        }
        if let Some(id) = dict_id(&zraw) {
            let packed: Vec<u8> = zs::compress_raw(&out, Some(id))?;
            if rf.zs.decompress(&packed)? == out && Bf::parse(&out).is_ok_and(|x| x == bf) {
                zsok += 1;
            }
        }
    }
    println!(
        "files: {}  parsed: {}  byte-identical rewrites: {}  differing or failed: {}  zstd and reparse round trips: {}",
        files.len(),
        parsed,
        same,
        files.len() - same,
        zsok
    );
    println!("replace and restore round trips: {swaps_ok} of {swaps}");
    for (k, v) in &kinds {
        println!("  {k}: {v}");
    }
    for m in shown.iter().take(12) {
        println!("  {m}");
    }
    println!("elapsed: {:.1}s", t0.elapsed().as_secs_f32());
    Ok(())
}

pub fn rewrite(a: &[String]) -> Res<()> {
    let root: &str = a.first().ok_or(Error::Format("missing romfs_root"))?;
    let r: &str = a.get(1).ok_or(Error::Format("missing relative path"))?;
    let out: &str = a.get(2).ok_or(Error::Format("missing out_file"))?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let raw: Vec<u8> = if r.ends_with(".zs") { rf.read_zs(r)? } else { rf.read(r)? };
    std::fs::write(out, Bf::parse(&raw)?.write()?)?;
    Ok(())
}

pub fn dump(a: &[String]) -> Res<()> {
    let root: &str = a.first().ok_or(Error::Format("missing romfs_root"))?;
    let r: &str = a.get(1).ok_or(Error::Format("missing relative path"))?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let raw: Vec<u8> = if r.ends_with(".zs") { rf.read_zs(r)? } else { rf.read(r)? };
    print!("{}", show::show(&Bf::parse(&raw)?));
    Ok(())
}

fn load(rf: &mut Romfs, r: &str) -> Res<Vec<u8>> {
    if r.ends_with(".zs") { rf.read_zs(r) } else { rf.read(r) }
}

pub fn replace(a: &[String]) -> Res<()> {
    let root: &str = a.first().ok_or(Error::Format("missing romfs_root"))?;
    let r: &str = a.get(1).ok_or(Error::Format("missing relative path"))?;
    let from: &str = a.get(2).ok_or(Error::Format("missing old item name"))?;
    let to: &str = a.get(3).ok_or(Error::Format("missing new item name"))?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let raw: Vec<u8> = load(&mut rf, r)?;
    let (out, n): (Vec<u8>, Replaced) = replace_bytes(&raw, from, to)?;
    println!("replaced {} actor name(s) and {} value(s)", n.actors, n.values);
    println!("decompressed size {} -> {}", raw.len(), out.len());
    let before: String = show::show(&Bf::parse(&raw)?);
    let after: String = show::show(&Bf::parse(&out)?);
    let (x, y): (Vec<&str>, Vec<&str>) = (before.lines().collect(), after.lines().collect());
    for (i, (p, q)) in x.iter().zip(y.iter()).enumerate() {
        if p != q {
            println!("line {i}
- {p}
+ {q}");
        }
    }
    let d: usize = raw.iter().zip(out.iter()).filter(|(p, q)| p != q).count();
    println!("bytes differing over the common length: {d}");
    if let Some(o) = a.get(4) {
        std::fs::write(o, &out)?;
        println!("wrote {o}");
    }
    Ok(())
}

pub fn catalog(a: &[String]) -> Res<()> {
    let root: &str = a.first().ok_or(Error::Format("missing romfs_root"))?;
    let out: &str = a.get(1).ok_or(Error::Format("missing out_file"))?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let files: Vec<String> = rf.list("Event/EventFlow")?.into_iter().filter(|p| p.ends_with(".bfevfl.zs")).collect();
    let mut v: Vec<rewards::Entry> = Vec::new();
    for f in &files {
        let raw: Vec<u8> = load(&mut rf, f)?;
        let bf: Bf = Bf::parse(&raw)?;
        let name: &str = f.rsplit('/').next().unwrap_or(f).trim_end_matches(".zs");
        v.extend(rewards::entry(name, &bf));
    }
    let n: usize = v.iter().map(|e| e.hits.len()).sum();
    std::fs::write(out, rewards::to_json("Event/EventFlow", &v))?;
    println!("{} flow(s) with grants, {} event(s), wrote {out}", v.len(), n);
    Ok(())
}

pub fn force(a: &[String]) -> Res<()> {
    let root: &str = a.first().ok_or(Error::Format("missing romfs_root"))?;
    let load_dir: &str = a.get(1).ok_or(Error::Format("missing load_dir"))?;
    let name: &str = a.get(2).ok_or(Error::Format("missing mod_name"))?;
    let r: &str = a.get(3).ok_or(Error::Format("missing relative path"))?;
    let rest: &[String] = a.get(4..).unwrap_or(&[]);
    if rest.is_empty() || rest.len() % 2 != 0 {
        return Err(Error::Format("need event_name case_value pairs"));
    }
    let mut pairs: Vec<(&str, u32)> = Vec::new();
    for c in rest.chunks(2) {
        let v: u32 = c[1].parse().map_err(|_| Error::Format("bad case value"))?;
        pairs.push((c[0].as_str(), v));
    }
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let z: Vec<u8> = rf.read(r)?;
    let id: Option<u32> = dict_id(&z).or(Some(1));
    let raw: Vec<u8> = rf.read_zs(r)?;
    let mut bf: Bf = Bf::parse(&raw)?;
    let mut done: usize = 0;
    for (event, want) in &pairs {
        let mut hit: bool = false;
        for f in bf.flows.iter_mut() {
            for e in f.events.iter_mut() {
                if e.name != *event {
                    continue;
                }
                if let Kind::Switch { cases, .. } = &mut e.kind {
                    let to: u16 = cases
                        .iter()
                        .find(|c| c.val == *want)
                        .map(|c| c.ev)
                        .ok_or(Error::Format("no case with that value"))?;
                    for c in cases.iter_mut() {
                        c.ev = to;
                    }
                    hit = true;
                    done += 1;
                }
            }
        }
        if !hit {
            return Err(Error::Format("no switch event with that name"));
        }
    }
    let out: Vec<u8> = bf.write()?;
    let pk: Pack = Pack::create(P::new(load_dir), TITLE_ID, name)?;
    let at = pk.write_romfs(r, &zs::compress_raw(&out, id)?)?;
    println!("forced {done} switch(es), wrote {}", at.display());
    Ok(())
}

pub fn durations(a: &[String]) -> Res<()> {
    let root: &str = a.first().ok_or(Error::Format("missing romfs_root"))?;
    let prefix: &str = a.get(1).map(|s| s.as_str()).unwrap_or("Dm_");
    let top: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(40);
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let files: Vec<String> = rf
        .list("Event/EventFlow")?
        .into_iter()
        .filter(|p| p.ends_with(".bfevfl.zs") && !p.contains(".110."))
        .filter(|p| p.rsplit('/').next().map(|n| n.starts_with(prefix)).unwrap_or(false))
        .collect();
    let mut rows: Vec<(f32, f32, f32, usize, String)> = Vec::new();
    for f in &files {
        let raw: Vec<u8> = load(&mut rf, f)?;
        let Ok(bf) = Bf::parse(&raw) else {
            continue;
        };
        let tl: f32 = bf.tls.iter().map(|t| t.dur).sum();
        let tl_max: f32 = bf.tls.iter().map(|t| t.dur).fold(0.0, f32::max);
        let mut waits: f32 = 0.0;
        let mut texts: usize = 0;
        for fl in &bf.flows {
            for e in &fl.events {
                if let Kind::Action { params: Some(c), .. } = &e.kind {
                    if let Some(Val::Int(n)) = c.get("Frames") {
                        waits += *n as f32;
                    }
                    if c.get("MessageId").is_some() {
                        texts += 1;
                    }
                }
            }
        }
        let name: String = f.rsplit('/').next().unwrap_or(f).trim_end_matches(".bfevfl.zs").to_string();
        rows.push((tl, tl_max, waits, texts, name));
    }
    rows.sort_by(|x, y| (y.0 + y.2).partial_cmp(&(x.0 + x.2)).unwrap_or(std::cmp::Ordering::Equal));
    println!("{} flows starting with {prefix}. timeline total, longest timeline, wait frames, text boxes, name", rows.len());
    for r in rows.iter().take(top) {
        println!("{:9.0} {:9.0} {:7.0} {:4}  {}", r.0, r.1, r.2, r.3, r.4);
    }
    Ok(())
}

pub fn scale(a: &[String]) -> Res<()> {
    let root: &str = a.first().ok_or(Error::Format("missing romfs_root"))?;
    let load_dir: &str = a.get(1).ok_or(Error::Format("missing load_dir"))?;
    let name: &str = a.get(2).ok_or(Error::Format("missing mod_name"))?;
    let factor: f32 = a
        .get(3)
        .ok_or(Error::Format("missing factor"))?
        .parse()
        .map_err(|_| Error::Format("bad factor"))?;
    let min: i32 = a
        .get(4)
        .ok_or(Error::Format("missing min_frames"))?
        .parse()
        .map_err(|_| Error::Format("bad min_frames"))?;
    let rels: &[String] = a.get(5..).unwrap_or(&[]);
    if rels.is_empty() || !(0.0..=1.0).contains(&factor) {
        return Err(Error::Format("need a factor from 0 to 1 and at least one flow file"));
    }
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let pk: Pack = Pack::create(P::new(load_dir), TITLE_ID, name)?;
    for r in rels {
        let z: Vec<u8> = rf.read(r)?;
        let id: Option<u32> = dict_id(&z).or(Some(1));
        let raw: Vec<u8> = rf.read_zs(r)?;
        let mut bf: Bf = Bf::parse(&raw)?;
        let p: Paced = scale_waits(&mut bf, factor, min);
        let out: Vec<u8> = bf.write()?;
        let at = pk.write_romfs(r, &zs::compress_raw(&out, id)?)?;
        println!("{r}: scaled {} waits, {} frames -> {}, wrote {}", p.waits, p.before, p.after, at.display());
    }
    Ok(())
}
