use std::collections::BTreeMap as BM;
use std::fmt::Write as W;
use std::fs;
use std::path::{Path as P, PathBuf as PB};

use crate::TITLE_ID;
use crate::error::{Error, Res};
use crate::flow_locations::dict_id;
use crate::modpack::Pack;
use crate::romfs::Romfs;
use crate::rsizetable::{self, Rst};
use crate::staging::Out;
use crate::teleport::{self, Cfg, MARGIN, Mode};
use crate::teleport_dest::Dest;
use crate::teleport_start;
use crate::zs;

pub const ROMFS: &str = "romfs";

#[derive(Clone, Debug, Default)]
pub struct Merged {
    pub root: PB,
    pub copied: usize,
    pub shared: Vec<String>,
    pub added: Vec<String>,
    pub report: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Verify {
    pub seed_files: usize,
    pub same: usize,
    pub changed: Vec<String>,
    pub missing: Vec<String>,
    pub extra: Vec<String>,
    pub shared: Vec<String>,
    pub sized: usize,
    pub no_entry: usize,
    pub too_small: Vec<String>,
    pub below_seed: usize,
    pub below_tp: usize,
}

impl Verify {
    pub fn ok(&self) -> bool {
        self.changed.is_empty() && self.missing.is_empty() && self.extra.is_empty() && self.too_small.is_empty() && self.below_seed == 0 && self.below_tp == 0
    }
}

pub fn mod_root(load: &P, name: &str) -> PB {
    load.join(TITLE_ID).join(name)
}

pub fn walk(root: &P) -> Res<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    if !root.is_dir() {
        return Ok(out);
    }
    let mut stack: Vec<PB> = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d)? {
            let e: fs::DirEntry = e?;
            let p: PB = e.path();
            if e.file_type()?.is_dir() {
                stack.push(p);
            } else {
                let r: &P = p.strip_prefix(root).map_err(|_| Error::BadPath(p.display().to_string()))?;
                out.push(r.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    out.sort();
    Ok(out)
}

pub fn tp_paths(c: &Cfg) -> Vec<&'static str> {
    let mut v: Vec<&'static str> = Vec::new();
    if c.mode == Mode::Start {
        v.push(teleport_start::FILE);
    }
    v.push(teleport::FLOW_FILE);
    v
}

pub fn union(a: &Rst, b: &Rst) -> Res<Rst> {
    if a.version != b.version || a.name_len != b.name_len {
        return Err(Error::Format("merge: the two size tables have different layouts"));
    }
    let mut crc: BM<u32, u32> = a.crc.iter().copied().collect();
    for (k, v) in &b.crc {
        let e: &mut u32 = crc.entry(*k).or_insert(*v);
        *e = (*e).max(*v);
    }
    let mut names: BM<Vec<u8>, u32> = a.names.iter().cloned().collect();
    for (k, v) in &b.names {
        let e: &mut u32 = names.entry(k.clone()).or_insert(*v);
        *e = (*e).max(*v);
    }
    Ok(Rst { version: a.version, name_len: a.name_len, crc: crc.into_iter().collect(), names: names.into_iter().collect() })
}

pub fn edit(raw: &[u8], path: &str, c: &Cfg, ds: &[Dest]) -> Res<Vec<u8>> {
    if path == teleport::FLOW_FILE {
        teleport::checked_bf(raw, c, ds)
    } else if path == teleport_start::FILE {
        teleport::checked_byml(raw, ds)
    } else {
        Err(Error::Msg(format!("merge: {path} is not a teleport file")))
    }
}

pub fn seed_table(rf: &mut Romfs, seed: &P) -> Res<Option<Rst>> {
    let p: PB = seed.join(ROMFS).join(rsizetable::TABLE_PATH);
    if !p.is_file() {
        return Ok(None);
    }
    Ok(Some(Rst::parse(&rf.zs.decompress(&fs::read(p)?)?)?))
}

pub fn tp_table(rf: &mut Romfs, c: &Cfg, ds: &[Dest]) -> Res<Rst> {
    let outs: Vec<Out> = teleport::stage(rf, c, ds)?;
    let mut t: Rst = rsizetable::load(rf)?;
    for x in &outs {
        t.grow(&x.path, x.old as u64, x.new as u64, MARGIN)?;
    }
    Ok(t)
}

fn stage_on(rf: &mut Romfs, seed: &P, path: &str, c: &Cfg, ds: &[Dest]) -> Res<(Out, bool)> {
    let sp: PB = seed.join(ROMFS).join(path);
    let from: bool = sp.is_file();
    let z: Vec<u8> = if from { fs::read(&sp)? } else { rf.read(path)? };
    let id: Option<u32> = match dict_id(&z) {
        Some(i) => Some(i),
        None => dict_id(&rf.read(path)?),
    };
    let raw: Vec<u8> = rf.zs.decompress(&z)?;
    let out: Vec<u8> = edit(&raw, path, c, ds)?;
    Ok((Out { path: path.to_string(), z: zs::compress_raw(&out, id)?, old: raw.len(), new: out.len() }, from))
}

pub fn merge(romfs: &P, seed_load: &P, seed_name: &str, out_load: &P, out_name: &str, c: &Cfg, ds: &[Dest]) -> Res<Merged> {
    c.check()?;
    let seed: PB = mod_root(seed_load, seed_name);
    if !seed.join(ROMFS).is_dir() {
        return Err(Error::Msg(format!("merge: {} holds no romfs folder, build the seed first", seed.display())));
    }
    let out: PB = mod_root(out_load, out_name);
    if fs::canonicalize(&seed).ok() == fs::canonicalize(&out).ok() && out.exists() {
        return Err(Error::Format("merge: the combined mod must not overwrite the seed mod"));
    }
    let mut rf: Romfs = Romfs::open(romfs)?;
    let base: Rst = match seed_table(&mut rf, &seed)? {
        Some(t) => t,
        None => rsizetable::load(&mut rf)?,
    };
    let mut grown: Rst = base.clone();
    let mut m: Merged = Merged::default();
    let mut outs: Vec<Out> = Vec::new();
    for path in tp_paths(c) {
        let (o, from): (Out, bool) = stage_on(&mut rf, &seed, path, c, ds)?;
        let was: Option<u32> = grown.get(path);
        let now: Option<u32> = grown.grow(path, o.old as u64, o.new as u64, MARGIN)?;
        let _ = writeln!(m.report, "{path}: base {} {} -> {} bytes, size table {:?} -> {:?}", if from { "seed" } else { "vanilla" }, o.old, o.new, was, now);
        if from {
            m.shared.push(path.to_string());
        } else {
            m.added.push(path.to_string());
        }
        outs.push(o);
    }
    let tpt: Rst = tp_table(&mut rf, c, ds)?;
    let table: Rst = union(&grown, &tpt)?;
    let pk: Pack = Pack::begin(out_load, TITLE_ID, out_name)?;
    let skip: Vec<String> = outs.iter().map(|o| format!("{ROMFS}/{}", o.path)).chain(std::iter::once(format!("{ROMFS}/{}", rsizetable::TABLE_PATH))).collect();
    for r in walk(&seed)? {
        if skip.contains(&r) || r.starts_with(crate::modpack::WORK) {
            continue;
        }
        let b: Vec<u8> = fs::read(seed.join(&r))?;
        let p: PB = pk.work.join(&r);
        if let Some(d) = p.parent() {
            fs::create_dir_all(d)?;
        }
        fs::write(&p, &b)?;
        m.copied += 1;
    }
    for o in &outs {
        pk.write_romfs(&o.path, &o.z)?;
    }
    pk.write_romfs(rsizetable::TABLE_PATH, &table.to_zs()?)?;
    pk.commit()?;
    m.shared.push(rsizetable::TABLE_PATH.to_string());
    let _ = writeln!(m.report, "seed files copied unchanged: {}, shared files rebuilt: {:?}, teleport files added: {:?}", m.copied, m.shared, m.added);
    m.root = pk.root;
    Ok(m)
}

fn real_len(rf: &mut Romfs, p: &P, r: &str) -> Res<usize> {
    let b: Vec<u8> = fs::read(p)?;
    if r.ends_with(".zs") { Ok(rf.zs.decompress(&b)?.len()) } else { Ok(b.len()) }
}

fn below(a: &Rst, b: &Rst) -> usize {
    let n: usize = b.crc.iter().filter(|(k, v)| a.crc.iter().find(|e| e.0 == *k).map(|e| e.1 < *v).unwrap_or(true)).count();
    n + b.names.iter().filter(|(k, v)| a.names.iter().find(|e| e.0 == *k).map(|e| e.1 < *v).unwrap_or(true)).count()
}

pub fn verify(rf: &mut Romfs, seed: &P, out: &P, c: &Cfg, ds: &[Dest]) -> Res<Verify> {
    let mut v: Verify = Verify::default();
    let tps: Vec<String> = tp_paths(c).iter().map(|p| format!("{ROMFS}/{p}")).collect();
    let tab: String = format!("{ROMFS}/{}", rsizetable::TABLE_PATH);
    let sf: Vec<String> = walk(seed)?;
    let of: Vec<String> = walk(out)?;
    v.seed_files = sf.len();
    for r in &sf {
        if tps.contains(r) || *r == tab {
            v.shared.push(r.clone());
            continue;
        }
        if !of.contains(r) {
            v.missing.push(r.clone());
        } else if fs::read(seed.join(r))? == fs::read(out.join(r))? {
            v.same += 1;
        } else {
            v.changed.push(r.clone());
        }
    }
    for r in &of {
        if !sf.contains(r) && !tps.contains(r) && *r != tab {
            v.extra.push(r.clone());
        }
    }
    let t: Rst = Rst::parse(&rf.zs.decompress(&fs::read(out.join(&tab))?)?)?;
    for r in &of {
        let Some(rel) = r.strip_prefix(&format!("{ROMFS}/")) else {
            continue;
        };
        if *r == tab {
            continue;
        }
        match t.get(rel) {
            Some(e) => {
                v.sized += 1;
                if (e as usize) < real_len(rf, &out.join(r), rel)? {
                    v.too_small.push(rel.to_string());
                }
            }
            None => v.no_entry += 1,
        }
    }
    let st: Rst = match seed_table(rf, seed)? {
        Some(x) => x,
        None => rsizetable::load(rf)?,
    };
    v.below_seed = below(&t, &st);
    v.below_tp = below(&t, &tp_table(rf, c, ds)?);
    Ok(v)
}

pub fn text(v: &Verify) -> String {
    let mut s: String = String::new();
    let _ = writeln!(s, "seed files {}: byte identical {}, changed {} {:?}, missing {} {:?}", v.seed_files, v.same, v.changed.len(), v.changed, v.missing.len(), v.missing);
    let _ = writeln!(s, "shared with the teleport mod: {:?}", v.shared);
    let _ = writeln!(s, "files that are in neither mod: {} {:?}", v.extra.len(), v.extra);
    let _ = writeln!(s, "size table: {} files have an entry, {} have none, entries smaller than the real size: {} {:?}", v.sized, v.no_entry, v.too_small.len(), v.too_small);
    let _ = writeln!(s, "size table entries below the seed table: {}, below the teleport only table: {}", v.below_seed, v.below_tp);
    let _ = writeln!(s, "merge verified: {}", v.ok());
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rsizetable::hash;

    fn name(s: &str) -> Vec<u8> {
        let mut b: Vec<u8> = s.as_bytes().to_vec();
        b.resize(16, 0);
        b
    }

    fn table(c: &[(&str, u32)], n: &[(&str, u32)]) -> Rst {
        let mut crc: Vec<(u32, u32)> = c.iter().map(|(k, v)| (hash(k), *v)).collect();
        crc.sort();
        Rst { version: 1, name_len: 16, crc, names: n.iter().map(|(k, v)| (name(k), *v)).collect() }
    }

    #[test]
    fn union_takes_the_larger_entry_and_keeps_both_sides() {
        let a: Rst = table(&[("a.bin", 10), ("b.bin", 50), ("c.bin", 30)], &[("x/one.bin", 5)]);
        let b: Rst = table(&[("a.bin", 40), ("b.bin", 20), ("d.bin", 7)], &[("x/one.bin", 9), ("x/two.bin", 3)]);
        let u: Rst = union(&a, &b).unwrap();
        assert_eq!(u.get("a.bin"), Some(40));
        assert_eq!(u.get("b.bin"), Some(50));
        assert_eq!(u.get("c.bin"), Some(30));
        assert_eq!(u.get("d.bin"), Some(7));
        assert_eq!(u.get("x/one.bin"), Some(9));
        assert_eq!(u.get("x/two.bin"), Some(3));
        assert_eq!(u.sorted(), (true, true));
        assert_eq!(below(&u, &a), 0);
        assert_eq!(below(&u, &b), 0);
        assert_eq!(below(&a, &b), 4);
        assert_eq!(Rst::parse(&u.write().unwrap()).unwrap(), u);
    }

    #[test]
    fn union_refuses_tables_of_another_layout() {
        let a: Rst = table(&[("a.bin", 1)], &[]);
        let mut b: Rst = a.clone();
        b.version = 2;
        assert!(union(&a, &b).is_err());
    }

    #[test]
    fn union_of_grown_tables_matches_growing_one_after_the_other() {
        let v: Rst = table(&[("flow.bin", 1000), ("start.bin", 500), ("banc.bin", 200)], &[]);
        let mut s: Rst = v.clone();
        s.grow("banc.bin", 100, 400, 0x400).unwrap();
        s.grow("flow.bin", 900, 900, 0x400).unwrap();
        let mut t: Rst = v.clone();
        t.grow("flow.bin", 900, 5000, 0x2000).unwrap();
        t.grow("start.bin", 400, 800, 0x2000).unwrap();
        let u: Rst = union(&s, &t).unwrap();
        assert_eq!(u.get("banc.bin"), s.get("banc.bin"));
        assert_eq!(u.get("flow.bin"), t.get("flow.bin"));
        assert_eq!(u.get("start.bin"), t.get("start.bin"));
    }

    #[test]
    fn paths_follow_the_mode() {
        let mut c: Cfg = Cfg::new();
        assert_eq!(tp_paths(&c), vec![teleport_start::FILE, teleport::FLOW_FILE]);
        c.mode = Mode::Coords;
        assert_eq!(tp_paths(&c), vec![teleport::FLOW_FILE]);
        assert!(edit(b"", "Other/File.bin", &c, &[]).is_err());
    }

    #[test]
    fn walk_lists_nested_files_with_forward_slashes() {
        let d: PB = std::env::temp_dir().join(format!("ootrr_tpmerge_walk_{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("romfs").join("A")).unwrap();
        fs::write(d.join("romfs").join("A").join("b.bin"), b"x").unwrap();
        fs::write(d.join("top.txt"), b"y").unwrap();
        assert_eq!(walk(&d).unwrap(), vec!["romfs/A/b.bin".to_string(), "top.txt".to_string()]);
        assert!(walk(&d.join("none")).unwrap().is_empty());
        let _ = fs::remove_dir_all(&d);
    }

    fn dump() -> Option<PB> {
        let p: PB = std::env::var_os("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|| PB::from("C:/o/nup dump/updates/131072/romfs"));
        if p.is_dir() {
            Some(p)
        } else {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            None
        }
    }

    fn dests(rf: &mut Romfs) -> Vec<Dest> {
        let st: crate::teleport_dest::Stands = crate::teleport_dest::Stands::new();
        let all: Vec<Dest> = crate::teleport_dest::chests(rf, &st).unwrap();
        let mut v: Vec<Dest> = Vec::new();
        for m in ["InsideTheDekuTree", "KokiriForest_Child", "FireTemple"] {
            v.extend(all.iter().filter(|d| d.banc == m).take(4).cloned());
        }
        for (i, d) in v.iter_mut().enumerate() {
            d.id = (i + 1).to_string();
        }
        v
    }

    #[test]
    fn a_real_seed_mod_and_the_teleport_edit_merge_without_losing_either() {
        use crate::bfevfl::{Bf, Flow, Kind, Val};
        use crate::settings::{Settings, Value};
        use crate::slice::data::Data;
        use crate::slice::generate::{GenOpts, Generated, generate_data};
        use crate::teleport::Layout;
        let Some(root) = dump() else {
            return;
        };
        let mut s: Settings = Settings::default();
        s.set("shuffle_songs", Value::s("own")).unwrap();
        let d: Data = Data::load().unwrap();
        let g: Generated = generate_data(&d, &s, "tp merge test", &GenOpts::default()).unwrap();
        let load: PB = PB::from(env!("CARGO_MANIFEST_DIR")).join("../../target/tp_merge_mods");
        crate::slice::edit::build(&g.seed_file, &root, &load, "OOTRR Merge Seed").unwrap();
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let ds: Vec<Dest> = dests(&mut rf);
        let mut c: Cfg = Cfg::new();
        c.layout = Layout::Direct;
        c.autosave = true;
        let m: Merged = merge(&root, &load, "OOTRR Merge Seed", &load, "OOTRR Merge Combined", &c, &ds).unwrap();
        assert!(m.shared.contains(&teleport::FLOW_FILE.to_string()), "{:?}", m.shared);
        assert!(m.added.contains(&teleport_start::FILE.to_string()), "{:?}", m.added);
        assert!(m.copied > 20, "{}", m.copied);
        let seed: PB = mod_root(&load, "OOTRR Merge Seed");
        let v: Verify = verify(&mut rf, &seed, &m.root, &c, &ds).unwrap();
        assert!(v.ok(), "{}", text(&v));
        assert_eq!(v.same + v.shared.len(), v.seed_files);
        let raw: Vec<u8> = rf.zs.decompress(&fs::read(m.root.join(ROMFS).join(teleport::FLOW_FILE)).unwrap()).unwrap();
        let b: Bf = Bf::parse(&raw).unwrap();
        let f: &Flow = b.flows.iter().find(|f| f.name == teleport::FLOW).unwrap();
        let zeroed: usize = f.events.iter().filter(|e| match &e.kind {
            Kind::Action { params: Some(p), .. } => p.get("Actor").is_some_and(|a| matches!(a, Val::Str(x) if x.starts_with(crate::songs::PREFIX))) && p.get("Num") == Some(&Val::Int(0)),
            _ => false,
        }).count();
        assert!(zeroed > 0);
        let sw: crate::teleport_sim::Sweep = crate::teleport_sim::sweep(f, &ds, &c.flag, &c.mflag, c.groups).unwrap();
        assert!(sw.bad.is_empty(), "{:?}", sw.bad);
        assert!(merge(&root, &load, "OOTRR Merge Combined", &load, "OOTRR Merge Combined", &c, &ds).is_err());
        let _ = fs::remove_dir_all(load.join(TITLE_ID).join("OOTRR Merge Combined"));
        let _ = fs::remove_dir_all(load.join(TITLE_ID).join("OOTRR Merge Seed"));
    }
}
