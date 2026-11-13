use std::collections::BTreeMap as BM;
use std::fmt::Write as W;
use std::fs;
use std::path::{Path as P, PathBuf as PB};

use crate::byml::{Byml, Node};
use crate::error::{Error as E, Res};
use crate::flow_locations::dict_id;
use crate::modpack::{Host, Pack};
use crate::pickup_pack::actor::pouch_file;
use crate::pickup_pack::build::{plan, pool_of, Plan};
use crate::pickup_pack::closure::{actor_file, select, texts, Pool};
use crate::pickup_pack::guard;
use crate::pickup_pack::place;
use crate::pickup_pack::rows::{self, ACTOR_INFO, GAME_ACTOR_INFO, POUCH_INFO};
use crate::pickup_pack::sizes;
use crate::pickup_pack::{is_actor_name, pack_of, Spec, BASE, DONOR, PACK_DICT};
use crate::chest_slot::LIST;
use crate::placement::banc_files;
use crate::pots::KEPT;
use crate::slice::data::Data;
use crate::romfs::Romfs;
use crate::rsizetable::{self, Rst, TABLE_PATH};
use crate::sarc::{self, Entry};
use crate::staging::{BANC_DICT, MARGIN};
use crate::zs;
use crate::TITLE_ID;

#[derive(Clone, Debug)]
pub struct Opts {
    pub host: Host,
    pub base: Option<PB>,
    pub rst_from: Option<PB>,
    pub places: Vec<u64>,
    pub dry: bool,
}

#[derive(Clone, Debug)]
pub struct File {
    pub path: String,
    pub raw: Vec<u8>,
    pub z: Vec<u8>,
    pub old: usize,
    pub fresh: bool,
}

#[derive(Clone, Debug)]
pub struct Done {
    pub report: String,
    pub root: Option<PB>,
    pub files: Vec<File>,
    pub carried: Vec<File>,
    pub heap: i32,
}

fn base_file(base: Option<&PB>, path: &str) -> Option<PB> {
    base.map(|b| b.join(path)).filter(|p| p.is_file())
}

fn start(rf: &mut Romfs, base: Option<&PB>, path: &str) -> Res<Vec<u8>> {
    match base_file(base, path) {
        Some(p) => rf.zs.decompress(&fs::read(p)?),
        None => rf.read_zs(path),
    }
}

fn old_len(rf: &mut Romfs, modded: bool, path: &str, first: &[u8]) -> Res<usize> {
    if modded {
        return Ok(first.len());
    }
    Ok(rf.read_zs(path)?.len())
}

fn table(rf: &mut Romfs, o: &Opts, modded: bool, path: &str, edit: impl FnOnce(&mut Byml) -> Res<()>) -> Res<File> {
    let first: Vec<u8> = start(rf, o.base.as_ref(), path)?;
    let mut y: Byml = Byml::parse(&first)?;
    edit(&mut y)?;
    let raw: Vec<u8> = y.write()?;
    let id: Option<u32> = dict_id(&rf.read(path)?);
    Ok(File { path: path.to_string(), z: zs::compress_raw(&raw, id)?, old: old_len(rf, modded, path, &first)?, raw, fresh: false })
}

fn tables(rf: &mut Romfs, o: &Opts, modded: bool, s: &Spec) -> Res<(Vec<File>, i32)> {
    let seen: Byml = Byml::parse(&start(rf, o.base.as_ref(), ACTOR_INFO)?)?;
    let heap: i32 = match s.heap {
        Some(h) => h,
        None => rows::heap(&seen, &s.item)?,
    };
    let mut at: [usize; 2] = [0, 0];
    let a: File = table(rf, o, modded, ACTOR_INFO, |y: &mut Byml| {
        let r: Node = rows::actor_row(rows::need(y, &s.item, ACTOR_INFO)?, &s.name, heap);
        at[0] = rows::insert(y, r)?;
        Ok(())
    })?;
    let g: File = table(rf, o, modded, GAME_ACTOR_INFO, |y: &mut Byml| {
        let r: Node = rows::game_row(rows::need(y, BASE, GAME_ACTOR_INFO)?, rows::need(y, &s.item, GAME_ACTOR_INFO)?, &s.name);
        at[1] = rows::insert(y, r)?;
        Ok(())
    })?;
    if at[0] != at[1] {
        return Err(E::Msg(format!("the new row sits at {} in the actor table and at {} in the game actor table, the two tables no longer line up", at[0], at[1])));
    }
    let q: File = table(rf, o, modded, POUCH_INFO, |y: &mut Byml| {
        let r: Node = rows::pouch_row(rows::need(y, &s.item, POUCH_INFO)?, &s.name);
        rows::insert(y, r)?;
        Ok(())
    })?;
    Ok((vec![a, g, q], heap))
}

fn spots(rf: &mut Romfs, o: &Opts, modded: bool, name: &str, note: &mut String) -> Res<Vec<File>> {
    let mut out: Vec<File> = Vec::new();
    if o.places.is_empty() {
        return Ok(out);
    }
    let list: Byml = Byml::parse(&start(rf, o.base.as_ref(), LIST)?)?;
    let d: Data = Data::load()?;
    for h in &o.places {
        place::single(&d, *h)?;
        place::permanent(&list, *h)?;
        let _ = writeln!(note, "spot {h} has a permanent saved row (reset type {KEPT}, no extra byte)");
    }
    let mut hit: BM<u64, usize> = BM::new();
    for f in banc_files(rf)? {
        let first: Vec<u8> = start(rf, o.base.as_ref(), &f)?;
        if !o.places.iter().any(|h| place::has_hash(&first, *h)) {
            continue;
        }
        let mut y: Byml = Byml::parse(&first)?;
        let mut n: usize = 0;
        for h in &o.places {
            if let Some(old) = place::swap(&mut y, *h, name)? {
                *hit.entry(*h).or_insert(0) += 1;
                n += 1;
                let _ = writeln!(note, "spot {h} in {f}: {old} -> {name}");
            }
        }
        if n == 0 {
            continue;
        }
        let raw: Vec<u8> = y.write()?;
        let id: u32 = dict_id(&rf.read(&f)?).unwrap_or(BANC_DICT);
        out.push(File { path: f.clone(), z: zs::compress_raw(&raw, Some(id))?, old: old_len(rf, modded, &f, &first)?, raw, fresh: false });
    }
    if let Some(h) = o.places.iter().find(|h| !hit.contains_key(h)) {
        return Err(E::Msg(format!("no placed actor has the hash {h}")));
    }
    Ok(out)
}

fn earlier(rf: &mut Romfs, base: Option<&PB>, own: &str) -> Res<Vec<File>> {
    let mut out: Vec<File> = Vec::new();
    let Some(dir) = base.map(|b| b.join("Pack").join("Actor")).filter(|p| p.is_dir()) else {
        return Ok(out);
    };
    let mut names: Vec<String> = Vec::new();
    for e in fs::read_dir(&dir)? {
        names.push(e?.file_name().to_string_lossy().to_string());
    }
    names.sort();
    for n in names.iter().filter(|n| n.ends_with(".pack.zs")) {
        let path: String = format!("Pack/Actor/{n}");
        if path == own || rf.root.join("Pack").join("Actor").join(n).is_file() {
            continue;
        }
        let z: Vec<u8> = fs::read(dir.join(n))?;
        let raw: Vec<u8> = rf.zs.decompress(&z)?;
        out.push(File { path, old: raw.len(), raw, z, fresh: false });
    }
    Ok(out)
}

pub fn actor_of(path: &str) -> &str {
    path.trim_start_matches("Pack/Actor/").trim_end_matches(".pack.zs")
}

fn size_table(rf: &mut Romfs, o: &Opts) -> Res<(Rst, bool)> {
    let from: Option<PB> = o.rst_from.clone().or_else(|| base_file(o.base.as_ref(), TABLE_PATH));
    match from {
        Some(p) => Ok((Rst::parse(&rf.zs.decompress(&fs::read(p)?)?)?, true)),
        None => Ok((rsizetable::load(rf)?, false)),
    }
}

fn sized(rst: &mut Rst, pl: &Plan, files: &[File], carried: &[File], note: &mut String) -> Res<()> {
    for c in carried {
        let n: u32 = sizes::pack_size(c.raw.len())?;
        rst.set_size(&c.path, n)?;
        let _ = writeln!(note, "{}: earlier pack carried from the base, {} bytes, size table entry {n}", c.path, c.raw.len());
    }
    for f in files {
        if f.fresh {
            let n: u32 = sizes::pack_size(f.raw.len())?;
            rst.set_size(&f.path, n)?;
            let _ = writeln!(note, "{}: new, {} bytes, size table entry {n}", f.path, f.raw.len());
            continue;
        }
        let was: Option<u32> = rst.get(&f.path);
        let now: Option<u32> = rst.grow(&f.path, f.old as u64, f.raw.len() as u64, MARGIN)?;
        let _ = writeln!(note, "{}: {} -> {} bytes, size table entry {:?} -> {:?}", f.path, f.old, f.raw.len(), was, now);
    }
    for (f, len) in &pl.fresh {
        let n: u32 = sizes::file_size(rst, &pl.peers, f, *len)?;
        rst.set_size(f, n)?;
        let _ = writeln!(note, "{f}: new file in the pack, {len} bytes, size table entry {n}");
    }
    Ok(())
}

fn text<'a>(n: &'a Node, k: &str) -> &'a str {
    n.get(k).and_then(|v| v.as_str()).unwrap_or("")
}

fn audit_carried(rst: &Rst, tables: &[Byml], carried: &[File]) -> Res<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for c in carried {
        let name: &str = actor_of(&c.path);
        if rst.get(&c.path) != Some(sizes::pack_size(c.raw.len())?) {
            return Err(E::Msg(format!("audit: the earlier pack {name} has the wrong size table entry")));
        }
        if let Some(f) = [actor_file(name), pouch_file(name)].iter().find(|f| rst.get(f).is_none()) {
            return Err(E::Msg(format!("audit: {f} of the earlier pack {name} has no size table entry, the size table does not come from the mod that made that pack")));
        }
        if tables.iter().any(|y| rows::find(y, name).is_none()) {
            return Err(E::Msg(format!("audit: the earlier pack {name} has lost a table row")));
        }
        out.push(format!("earlier pack {name} keeps its three rows and its size table entries"));
    }
    Ok(out)
}

pub struct Scope<'a> {
    pub carried: &'a [File],
    pub places: &'a [u64],
    pub base: Option<&'a PB>,
}

pub fn audit(rf: &mut Romfs, s: &Spec, pl: &Plan, files: &[File], rst_z: &[u8], sc: &Scope) -> Res<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    let get = |p: &str| files.iter().find(|f| f.path == p).ok_or_else(|| E::Msg(format!("audit: {p} was not written")));
    let rst: Rst = Rst::parse(&rf.zs.decompress(rst_z)?)?;
    let pk: &File = get(&pack_of(&s.name))?;
    let raw: Vec<u8> = rf.zs.decompress(&pk.z)?;
    let es: Vec<Entry> = sarc::parse(&raw)?;
    let p: Pool = pool_of(&[es.clone()])?;
    let mut names: Vec<String> = es.iter().map(|e| e.name.clone()).collect();
    names.sort();
    if select(&p, &s.name)? != names {
        return Err(E::Format("audit: the written pack holds a file its actor does not reach"));
    }
    for e in &es {
        if texts(&e.data).is_err() {
            return Err(E::Msg(format!("audit: {} does not read back", e.name)));
        }
    }
    if rst.get(&pk.path) != Some(sizes::pack_size(raw.len())?) {
        return Err(E::Format("audit: the pack has the wrong size table entry"));
    }
    out.push(format!("pack reads back: {} files, {} bytes, dictionary {:?}, size entry {:?}", es.len(), raw.len(), dict_id(&pk.z), rst.get(&pk.path)));
    for (f, len) in &pl.fresh {
        let n: u32 = rst.get(f).ok_or_else(|| E::Msg(format!("audit: {f} has no size table entry")))?;
        if (n as usize) < *len {
            return Err(E::Msg(format!("audit: {f} has a size table entry below its size")));
        }
    }
    let mut now: Vec<Byml> = Vec::new();
    for t in [ACTOR_INFO, GAME_ACTOR_INFO, POUCH_INFO] {
        let f: &File = get(t)?;
        let y: Byml = Byml::parse(&rf.zs.decompress(&f.z)?)?;
        let was: Byml = Byml::parse(&start(rf, sc.base, t)?)?;
        let n: usize = y.root.as_arr().map(|a| a.len()).unwrap_or(0);
        let m: usize = was.root.as_arr().map(|a| a.len()).unwrap_or(0);
        let had: usize = usize::from(rows::find(&was, &s.name).is_some());
        let r: &Node = rows::need(&y, &s.name, t)?;
        if !rows::sorted(&y)? || n + had != m + 1 {
            return Err(E::Msg(format!("audit: {t} is not the table it started from plus one sorted row")));
        }
        let kept: usize = was
            .root
            .as_arr()
            .unwrap_or(&[])
            .iter()
            .filter(|x| rows::id(x) != Some(s.name.as_str()))
            .filter(|x| rows::id(x).and_then(|i| rows::find(&y, i)) == Some(*x))
            .count();
        if kept + had != m {
            return Err(E::Msg(format!("audit: {t} changed {} rows it started with", m - had - kept)));
        }
        if (rst.get(t).unwrap_or(0) as usize) < f.raw.len() {
            return Err(E::Msg(format!("audit: {t} has a size table entry below its size")));
        }
        let fact: String = match t {
            ACTOR_INFO => format!("ActorName {} heap {:?} model {}", text(r, "ActorName"), r.get(rows::HEAP), text(r, "FmdbName")),
            GAME_ACTOR_INFO => format!("IsRespawn {:?} layer {}", r.get(rows::RESPAWN), text(r, rows::LAYER)),
            _ => format!("deal {} category {}", text(r, "PouchSpecialDeal"), text(r, "PouchCategory")),
        };
        out.push(format!("{t}: {m} -> {n} rows, {kept} earlier rows unchanged, new row {fact}"));
        now.push(y);
    }
    out.extend(audit_carried(&rst, &now, sc.carried)?);
    for h in sc.places {
        let n: usize = files
            .iter()
            .filter(|f| f.path.starts_with("Banc/"))
            .filter(|f| Byml::parse(&f.raw).ok().and_then(|y| place::current(&y, *h)).as_deref() == Some(s.name.as_str()))
            .count();
        if n == 0 {
            return Err(E::Msg(format!("audit: spot {h} does not hold {}", s.name)));
        }
        out.push(format!("spot {h} holds {} in {n} file(s)", s.name));
    }
    Ok(out)
}

pub fn run(romfs: &P, load: &P, mod_name: &str, s: &Spec, o: &Opts) -> Res<Done> {
    if !is_actor_name(&s.name) || !is_actor_name(&s.item) {
        return Err(E::Msg(format!("{} or {} is not a plain actor name", s.name, s.item)));
    }
    if let Some(b) = o.base.as_ref().filter(|b| !b.is_dir()) {
        return Err(E::Msg(format!("the base folder {} does not exist, it must be the romfs folder of a mod", b.display())));
    }
    let mut rf: Romfs = Romfs::open(romfs)?;
    if rf.root.join(pack_of(&s.name)).is_file() {
        return Err(E::Msg(format!("the game already ships {}, use that actor", s.name)));
    }
    if !rf.root.join(pack_of(&s.item)).is_file() {
        return Err(E::Msg(format!("the game has no actor pack for {}", s.item)));
    }
    let mut packs: Vec<Vec<Entry>> = Vec::new();
    for n in [BASE, DONOR, s.item.as_str()] {
        packs.push(sarc::parse(&start(&mut rf, o.base.as_ref(), &pack_of(n))?)?);
    }
    let pl: Plan = plan(&packs, s)?;
    let (mut rst, modded): (Rst, bool) = size_table(&mut rf, o)?;
    let mut note: String = String::new();
    let id: u32 = dict_id(&rf.read(&pack_of(BASE))?).unwrap_or(PACK_DICT);
    let mut files: Vec<File> = vec![File { path: pack_of(&s.name), z: zs::compress_raw(&pl.sarc, Some(id))?, raw: pl.sarc.clone(), old: 0, fresh: true }];
    let (ts, heap): (Vec<File>, i32) = tables(&mut rf, o, modded, s)?;
    rows::need(&Byml::parse(&ts[2].raw)?, &pl.identity, POUCH_INFO)?;
    let _ = writeln!(note, "pouch identity {}, the name the save and the get window use", pl.identity);
    files.extend(ts);
    files.extend(spots(&mut rf, o, modded, &s.name, &mut note)?);
    let carried: Vec<File> = earlier(&mut rf, o.base.as_ref(), &pack_of(&s.name))?;
    sized(&mut rst, &pl, &files, &carried, &mut note)?;
    let rst_z: Vec<u8> = rst.to_zs()?;
    let sc: Scope = Scope { carried: &carried, places: &o.places, base: o.base.as_ref() };
    for l in audit(&mut rf, s, &pl, &files, &rst_z, &sc)? {
        let _ = writeln!(note, "{l}");
    }
    let _ = writeln!(note, "{}", guard::check(&mut rf, &s.name, &files, &carried)?);
    let mut root: Option<PB> = None;
    if !o.dry {
        let pk: Pack = Pack::create_for(load, TITLE_ID, mod_name, o.host)?;
        for f in files.iter().chain(carried.iter()) {
            pk.write_romfs(&f.path, &f.z)?;
        }
        pk.write_romfs(TABLE_PATH, &rst_z)?;
        let _ = writeln!(note, "wrote {} files to {}", files.len() + carried.len() + 1, pk.root.display());
        root = Some(pk.root);
    }
    Ok(Done { report: note, root, files, carried, heap })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pickup_pack::Style;

    fn real() -> Option<PB> {
        let p: PB = std::env::var("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|_| PB::from("C:/o/nup dump/updates/131072/romfs"));
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Some(p)
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    fn hookshot() -> Spec {
        Spec { item: "Item_Hookshot".to_string(), name: "Obj_Hookshot".to_string(), style: Style::Touch, heap: None }
    }

    fn opts(places: Vec<u64>) -> Opts {
        Opts { host: Host::Hookshot, base: None, rst_from: None, places, dry: true }
    }

    const SPOT: u64 = 11219803099333988363;

    #[test]
    fn the_three_real_tables_round_trip_and_stay_in_step() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut ids: Vec<Vec<String>> = Vec::new();
        for t in [ACTOR_INFO, GAME_ACTOR_INFO, POUCH_INFO] {
            let raw: Vec<u8> = rf.read_zs(t).unwrap();
            let y: Byml = Byml::parse(&raw).unwrap();
            assert_eq!(y.write().unwrap(), raw, "{t}");
            assert!(rows::sorted(&y).unwrap(), "{t}");
            ids.push(y.root.as_arr().unwrap().iter().filter_map(rows::id).map(|s| s.to_string()).collect());
        }
        assert_eq!(ids[0].len(), 8363);
        assert_eq!(ids[0], ids[1]);
        assert_eq!(ids[2].len(), 395);
        assert!(ids[2].iter().all(|i| ids[0].contains(i)));
    }

    #[test]
    fn every_real_pack_entry_follows_the_pack_size_rule() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let rst: Rst = rsizetable::load(&mut rf).unwrap();
        let all: Vec<String> = rf.list("Pack/Actor").unwrap();
        let mut n: usize = 0;
        for f in all.iter().filter(|f| f.ends_with(".pack.zs")).step_by(7) {
            let len: usize = rf.read_zs(f).unwrap().len();
            assert_eq!(rst.get(f), Some(sizes::pack_size(len).unwrap()), "{f}");
            n += 1;
        }
        assert!(n > 1000, "{n}");
    }

    #[test]
    fn the_real_hookshot_build_adds_one_row_per_table_and_changes_nothing_else() {
        let Some(root) = real() else {
            return;
        };
        let d: Done = run(&root, P::new("unused"), "unused", &hookshot(), &opts(vec![SPOT])).unwrap();
        assert!(d.root.is_none());
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let paths: Vec<&str> = d.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(
            paths,
            vec!["Pack/Actor/Obj_Hookshot.pack.zs", ACTOR_INFO, GAME_ACTOR_INFO, POUCH_INFO, "Banc/KokiriForest/KokiriForest_Child_Dynamic.bcett.byml.zs"]
        );
        for f in &d.files {
            assert_eq!(rf.zs.decompress(&f.z).unwrap(), f.raw, "{}", f.path);
        }
        assert_eq!(dict_id(&d.files[0].z), Some(PACK_DICT));
        for (i, t) in [ACTOR_INFO, GAME_ACTOR_INFO, POUCH_INFO].iter().enumerate() {
            let was: Byml = rf.read_byml(t).unwrap();
            let mut now: Byml = Byml::parse(&d.files[i + 1].raw).unwrap();
            let at: usize = now.root.as_arr().unwrap().iter().position(|r| rows::id(r) == Some("Obj_Hookshot")).unwrap();
            let new: Node = now.root.as_arr_mut().unwrap().remove(at);
            assert_eq!(now, was, "{t}");
            let item: &Node = rows::find(&was, "Item_Hookshot").unwrap();
            match i {
                0 => {
                    assert_eq!(text(&new, "ActorName"), "Item_Hookshot");
                    assert_eq!(text(&new, "FmdbName"), "Item_Hookshot_Body");
                    assert_eq!(new.get(rows::HEAP), Some(&Node::I32(d.heap)));
                    assert_eq!(d.heap, 167160 + rows::HEAP_MARGIN);
                    assert_eq!(new.get("ModelAabbMax"), item.get("ModelAabbMax"));
                    assert_eq!(new.get("PreActorRenderInfoHeapSize"), item.get("PreActorRenderInfoHeapSize"));
                }
                1 => {
                    assert_eq!(new.get(rows::RESPAWN), Some(&Node::Bool(true)));
                    assert_eq!(text(&new, rows::LAYER), "SmallObject");
                    assert_eq!(new.get(rows::VARIATION), item.get(rows::VARIATION));
                    assert!(new.get(rows::VARIATION).is_some());
                }
                _ => {
                    let mut same: Node = new.clone();
                    same.set(rows::ROW_ID, Node::Str("Item_Hookshot".to_string()));
                    assert_eq!(&same, item);
                }
            }
        }
        let was: Byml = rf.read_byml(&d.files[4].path).unwrap();
        let now: Byml = Byml::parse(&d.files[4].raw).unwrap();
        let a: &[Node] = was.root.get("Actors").unwrap().as_arr().unwrap();
        let b: &[Node] = now.root.get("Actors").unwrap().as_arr().unwrap();
        assert_eq!(a.len(), b.len());
        let diff: Vec<usize> = (0..a.len()).filter(|i| a[*i] != b[*i]).collect();
        assert_eq!(diff.len(), 1);
        assert_eq!(text(&a[diff[0]], "Gyaml"), "Obj_Rupee_Green");
        assert_eq!(text(&b[diff[0]], "Gyaml"), "Obj_Hookshot");
        let mut back: Node = b[diff[0]].clone();
        back.set("Gyaml", Node::Str("Obj_Rupee_Green".to_string()));
        assert_eq!(back, a[diff[0]]);
        let mut rest: Byml = now.clone();
        rest.root.get_mut("Actors").unwrap().as_arr_mut().unwrap()[diff[0]] = a[diff[0]].clone();
        assert_eq!(rest, was);
        assert!(d.report.contains("pack reads back: 40 files"));
    }

    #[test]
    fn the_real_build_refuses_a_shipped_name_a_missing_item_and_a_chest() {
        let Some(root) = real() else {
            return;
        };
        let taken: Spec = Spec { name: "Obj_Arrow_Fire".to_string(), ..hookshot() };
        assert!(run(&root, P::new("unused"), "unused", &taken, &opts(Vec::new())).is_err());
        let none: Spec = Spec { item: "Item_Nothing_Here".to_string(), ..hookshot() };
        assert!(run(&root, P::new("unused"), "unused", &none, &opts(Vec::new())).is_err());
        assert!(run(&root, P::new("unused"), "unused", &hookshot(), &opts(vec![12786193094758345086])).is_err());
        assert!(run(&root, P::new("unused"), "unused", &hookshot(), &opts(vec![1])).is_err());
        let mut lost: Opts = opts(Vec::new());
        lost.base = Some(PB::from("no/such/mod/romfs"));
        assert!(run(&root, P::new("unused"), "unused", &hookshot(), &lost).unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn a_second_run_on_top_of_the_first_mod_is_stable() {
        let Some(root) = real() else {
            return;
        };
        let load: PB = std::env::temp_dir().join(format!("ootrr_pack_add_{}", std::process::id()));
        let _ = fs::remove_dir_all(&load);
        let mut o: Opts = opts(vec![SPOT]);
        o.dry = false;
        let one: Done = run(&root, &load, "one", &hookshot(), &o).unwrap();
        let first: PB = one.root.clone().unwrap();
        assert!(first.join("romfs").join("Pack").join("Actor").join("Obj_Hookshot.pack.zs").is_file());
        assert!(first.join("romfs").join(TABLE_PATH).is_file());
        assert!(first.ends_with("one"));
        o.base = Some(first.join("romfs"));
        let two: Done = run(&root, &load, "two", &hookshot(), &o).unwrap();
        for (a, b) in one.files.iter().zip(two.files.iter()) {
            assert_eq!(a.path, b.path);
            assert_eq!(a.raw, b.raw, "{}", a.path);
        }
        let ta: Vec<u8> = fs::read(first.join("romfs").join(TABLE_PATH)).unwrap();
        let tb: Vec<u8> = fs::read(two.root.unwrap().join("romfs").join(TABLE_PATH)).unwrap();
        assert_eq!(ta, tb);
        let _ = fs::remove_dir_all(&load);
    }
    fn snapshot(d: &P) -> BM<String, Vec<u8>> {
        let mut out: BM<String, Vec<u8>> = BM::new();
        let mut stack: Vec<PB> = vec![d.to_path_buf()];
        while let Some(x) = stack.pop() {
            for e in fs::read_dir(&x).unwrap() {
                let p: PB = e.unwrap().path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    out.insert(p.strip_prefix(d).unwrap().to_string_lossy().replace('\\', "/"), fs::read(&p).unwrap());
                }
            }
        }
        out
    }

    fn ids(rf: &mut Romfs, dir: &P, t: &str) -> (Byml, Vec<String>) {
        let y: Byml = Byml::parse(&rf.zs.decompress(&fs::read(dir.join(t)).unwrap()).unwrap()).unwrap();
        let v: Vec<String> = y.root.as_arr().unwrap().iter().filter_map(rows::id).map(|s| s.to_string()).collect();
        (y, v)
    }

    #[test]
    fn three_packs_chain_on_top_of_each_other_and_every_earlier_pack_stays_untouched() {
        let Some(root) = real() else {
            return;
        };
        let load: PB = std::env::temp_dir().join(format!("ootrr_pack_chain_{}", std::process::id()));
        let _ = fs::remove_dir_all(&load);
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let chain: [(&str, &str); 3] = [("Item_Hookshot", "Obj_Hookshot"), ("Item_Boomerang", "Obj_Boomerang"), ("Item_Hammer", "Obj_Hammer")];
        let mut base: Option<PB> = None;
        let mut first: BM<String, Vec<u8>> = BM::new();
        let mut sizes_seen: BM<String, u32> = BM::new();
        let mut rows_seen: Vec<BM<String, Node>> = vec![BM::new(), BM::new(), BM::new()];
        let mut frozen: Vec<(PB, BM<String, Vec<u8>>)> = Vec::new();
        for (i, (item, name)) in chain.iter().enumerate() {
            let s: Spec = Spec { item: item.to_string(), name: name.to_string(), style: Style::Touch, heap: None };
            let places: Vec<u64> = if i == 0 { vec![SPOT] } else { Vec::new() };
            let o: Opts = Opts { host: Host::Hookshot, base: base.clone(), rst_from: None, places, dry: false };
            let d: Done = run(&root, &load, &format!("step{i}"), &s, &o).unwrap();
            let out: PB = d.root.clone().unwrap().join("romfs");
            assert_eq!(d.carried.len(), i, "{name}");
            assert!(d.report.contains(&format!("{name} stays collected once taken")), "{}", d.report);
            first.insert(pack_of(name), fs::read(out.join(pack_of(name))).unwrap());
            for (path, z) in &first {
                assert_eq!(&fs::read(out.join(path)).unwrap(), z, "{path} in step {i}");
            }
            let rst: Rst = Rst::parse(&rf.zs.decompress(&fs::read(out.join(TABLE_PATH)).unwrap()).unwrap()).unwrap();
            let stock: Rst = rsizetable::load(&mut rf).unwrap();
            for f in [pack_of(name), actor_file(name), pouch_file(name)] {
                assert!(stock.get(&f).is_none(), "{f}");
                sizes_seen.insert(f.clone(), rst.get(&f).unwrap());
            }
            for (f, n) in &sizes_seen {
                assert_eq!(rst.get(f), Some(*n), "{f} in step {i}");
            }
            for path in first.keys() {
                let len: usize = rf.zs.decompress(&first[path]).unwrap().len();
                assert_eq!(rst.get(path), Some(sizes::pack_size(len).unwrap()), "{path}");
            }
            let mut all: Vec<Vec<String>> = Vec::new();
            for (k, t) in [ACTOR_INFO, GAME_ACTOR_INFO, POUCH_INFO].iter().enumerate() {
                let (y, v): (Byml, Vec<String>) = ids(&mut rf, &out, t);
                let was: Byml = rf.read_byml(t).unwrap();
                assert!(rows::sorted(&y).unwrap(), "{t}");
                assert_eq!(v.len(), was.root.as_arr().unwrap().len() + i + 1, "{t} in step {i}");
                rows_seen[k].insert(name.to_string(), rows::find(&y, name).unwrap().clone());
                for (n, r) in &rows_seen[k] {
                    assert_eq!(rows::find(&y, n), Some(r), "{n} in {t} in step {i}");
                }
                let mut rest: Byml = y.clone();
                rest.root.as_arr_mut().unwrap().retain(|r| !rows_seen[k].contains_key(rows::id(r).unwrap_or("")));
                assert_eq!(rest, was, "{t} in step {i}");
                assert!(rst.get(t).unwrap() as usize >= rf.zs.decompress(&fs::read(out.join(t)).unwrap()).unwrap().len(), "{t}");
                all.push(v);
            }
            assert_eq!(all[0], all[1]);
            let seen: crate::pickup_persist::Extra = crate::pickup_persist::Extra::load(&mut rf, &out).unwrap();
            assert_eq!(seen.packs.len(), i + 1);
            for (n, snap) in &frozen {
                assert_eq!(&snapshot(n), snap, "step {i} changed {}", n.display());
            }
            frozen.push((out.clone(), snapshot(&out)));
            base = Some(out);
        }
        let banc: &str = "Banc/KokiriForest/KokiriForest_Child_Dynamic.bcett.byml.zs";
        assert!(frozen[0].1.contains_key(banc));
        assert!(!frozen[2].1.contains_key(banc));
        assert_eq!(frozen[2].1.keys().filter(|k| k.starts_with("Pack/Actor/")).count(), 3);
        let again: Spec = Spec { item: "Item_Boomerang".to_string(), name: "Obj_Boomerang".to_string(), style: Style::Touch, heap: None };
        let o: Opts = Opts { host: Host::Hookshot, base: base.clone(), rst_from: None, places: Vec::new(), dry: false };
        let d: Done = run(&root, &load, "step2", &again, &o).unwrap();
        assert_eq!(d.carried.len(), 2);
        assert_eq!(snapshot(&d.root.unwrap().join("romfs")), frozen[2].1);
        let _ = fs::remove_dir_all(&load);
    }

    #[test]
    fn a_base_table_that_lost_or_changed_a_row_fails_the_audit() {
        let Some(root) = real() else {
            return;
        };
        let load: PB = std::env::temp_dir().join(format!("ootrr_pack_audit_{}", std::process::id()));
        let _ = fs::remove_dir_all(&load);
        let mut o: Opts = opts(Vec::new());
        o.dry = false;
        let one: Done = run(&root, &load, "one", &hookshot(), &o).unwrap();
        let dir: PB = one.root.unwrap().join("romfs");
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let boom: Spec = Spec { item: "Item_Boomerang".to_string(), name: "Obj_Boomerang".to_string(), style: Style::Touch, heap: None };
        o.base = Some(dir.clone());
        o.dry = true;
        let pl: Plan = {
            let mut packs: Vec<Vec<Entry>> = Vec::new();
            for n in [BASE, DONOR, "Item_Boomerang"] {
                packs.push(sarc::parse(&rf.read_zs(&pack_of(n)).unwrap()).unwrap());
            }
            plan(&packs, &boom).unwrap()
        };
        let good: Done = run(&root, &load, "two", &boom, &o).unwrap();
        let rst_z: Vec<u8> = fs::read(dir.join(TABLE_PATH)).unwrap();
        let mut rst: Rst = Rst::parse(&rf.zs.decompress(&rst_z).unwrap()).unwrap();
        sized(&mut rst, &pl, &good.files, &good.carried, &mut String::new()).unwrap();
        let z: Vec<u8> = rst.to_zs().unwrap();
        let sc: Scope = Scope { carried: &good.carried, places: &[], base: Some(&dir) };
        assert!(audit(&mut rf, &boom, &pl, &good.files, &z, &sc).is_ok());
        let stock: Scope = Scope { carried: &good.carried, places: &[], base: None };
        let e: String = audit(&mut rf, &boom, &pl, &good.files, &z, &stock).unwrap_err().to_string();
        assert!(e.contains("plus one sorted row"), "{e}");
        let mut bad: Vec<File> = good.files.clone();
        let at: usize = bad.iter().position(|f| f.path == ACTOR_INFO).unwrap();
        let mut y: Byml = Byml::parse(&bad[at].raw).unwrap();
        let row: &mut Node = y.root.as_arr_mut().unwrap().iter_mut().find(|r| rows::id(r) == Some("Obj_Hookshot")).unwrap();
        row.set(rows::HEAP, Node::I32(8));
        bad[at].raw = y.write().unwrap();
        bad[at].z = zs::compress_raw(&bad[at].raw, dict_id(&bad[at].z)).unwrap();
        let e: String = audit(&mut rf, &boom, &pl, &bad, &z, &sc).unwrap_err().to_string();
        assert!(e.contains("changed 1 rows"), "{e}");
        let _ = fs::remove_dir_all(&load);
    }
}
