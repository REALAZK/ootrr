use std::collections::BTreeSet as BS;

use crate::ainb::{hand_hash, Ainb};
use crate::byml::{Byml, Node};
use crate::error::{Error as E, Res};
use crate::pickup_pack::build::pool_of;
use crate::pickup_pack::closure::{actor_file, chain, dangling_with, local, norm, select, work, Chain, Pool, COMPONENTS, PARENT};
use crate::pickup_pack::kit::{graph, inherited, name_of};
use crate::pickup_pack::is_actor_name;
use crate::pickup_persist::{TAIL, TABLES};
use crate::sarc::{self, Entry};
use crate::pickup_pack::rows::{self, ACTOR_INFO, GAME_ACTOR_INFO};
use crate::pickup_pack::sizes;
use crate::pickup_pack::write::{base_dir, earlier, size_table, table, Done, File, Opts};
use crate::pickup_pack::{pack_of, PACK_DICT};
use crate::flow_locations::dict_id;
use crate::modpack::Pack;
use crate::pickup_persist::chain as death_chain;
use crate::romfs::Romfs;
use crate::rsizetable::{Rst, TABLE_PATH};
use crate::zs;
use crate::TITLE_ID;
use std::fmt::Write as W;
use std::path::Path as P;

pub const AI_REF: &str = "AIInfoRef";
pub const DEATH_REF: &str = "DeathRef";
pub const ROOT_REF: &str = "RootAIRef";
pub const PROCEDURES: &str = "DeathProcedures";
pub const CELLS: &str = "CellList";
pub const CELL_TYPE: &str = "game__death__DeathReasonProceduresCell";
pub const KEEP: &str = "IsSetGmdRespawnFlagToFalse";
pub const DEFAULT_TABLE: &str = "Default";
const AI_DIR: &str = "AI/AIInfo/";
const AI_EXT: &str = ".engine__actor__AIInfo.bgyml";
const DEATH_DIR: &str = "Component/DeathParam/";
const DEATH_EXT: &str = ".game__component__DeathParam.bgyml";
const ROOT_DIR: &str = "AI/Root/";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PotPlan {
    pub name: String,
    pub base: String,
    pub files: Vec<String>,
    pub fresh: Vec<(String, usize)>,
    pub peers: Vec<(String, usize)>,
    pub sarc: Vec<u8>,
}

fn stem<'a>(path: &'a str, ext: &str) -> Res<&'a str> {
    path.rsplit('/').next().and_then(|f| f.strip_suffix(ext)).filter(|s| !s.is_empty()).ok_or_else(|| E::Msg(format!("{path} is not a {ext} file")))
}

fn comp<'a>(c: &'a Chain, k: &str, who: &str) -> Res<&'a str> {
    c.comps.get(k).map(|s| s.as_str()).ok_or_else(|| E::Msg(format!("{who} has no {k}")))
}

fn text(p: &Pool, first: &str, k: &str) -> Res<String> {
    inherited(p, first, k)?.and_then(|v| v.as_str().map(|s| s.to_string())).ok_or_else(|| E::Msg(format!("{first} sets no {k}")))
}

fn table_file(name: &str) -> String {
    format!("{TABLES}{name}{TAIL}.bgyml")
}

fn put(p: &mut Pool, fresh: &mut Vec<(String, usize)>, f: &str, d: Vec<u8>) -> Res<()> {
    if let Some(old) = p.get(f) {
        if old != d.as_slice() {
            return Err(E::Msg(format!("{f} already exists with other contents")));
        }
        return Ok(());
    }
    p.add(f, &d)?;
    fresh.push((f.to_string(), d.len()));
    Ok(())
}

fn byml(root: Node, version: u16) -> Res<Vec<u8>> {
    Byml { root, version, big: false }.write()
}

pub fn keep_table(parent_ref: &str, cells: &[String], version: u16) -> Res<Vec<u8>> {
    let mut list: Node = Node::Map(Vec::new());
    for c in cells {
        let mut cell: Node = Node::Map(Vec::new());
        cell.set("$type", Node::Str(CELL_TYPE.to_string()));
        cell.set(KEEP, Node::Bool(false));
        list.set(&format!("{c}___Data"), cell);
    }
    let mut root: Node = Node::Map(Vec::new());
    root.set(PARENT, Node::Str(parent_ref.to_string()));
    root.set(CELLS, list);
    byml(root, version)
}

pub fn plan(packs: &[Vec<Entry>], skull_graph: &[u8], cells: &[String], base: &str, peers: &[(String, usize)]) -> Res<PotPlan> {
    if !is_actor_name(base) {
        return Err(E::Msg(format!("{base} is not a plain actor name")));
    }
    let name: String = name_of(base);
    let mut p: Pool = pool_of(packs)?;
    let c: Chain = chain(&p, base)?;
    let version: u16 = p.byml(&actor_file(base))?.version;
    let ai: String = norm(comp(&c, AI_REF, base)?).ok_or_else(|| E::Msg(format!("{base} keeps its AI info outside its pack")))?;
    let death: String = norm(comp(&c, DEATH_REF, base)?).ok_or_else(|| E::Msg(format!("{base} keeps its death parameters outside its pack")))?;
    let root: String = text(&p, &ai, ROOT_REF)?;
    let old_graph: String = norm(&root).filter(|g| g.ends_with(".ainb")).ok_or_else(|| E::Msg(format!("{root} is not an AI graph")))?;
    let ai_stem: String = format!("{}{}", stem(&ai, AI_EXT)?, crate::pickup_pack::kit::SUFFIX);
    let new_root: String = format!("{ROOT_DIR}{}.root.ain", stem(&old_graph, ".root.ainb")?.to_string() + crate::pickup_pack::kit::SUFFIX);
    let new_graph: String = graph(&new_root).ok_or_else(|| E::Msg(format!("{new_root} does not name a graph")))?;
    let procs: String = text(&p, &death, PROCEDURES).unwrap_or_else(|_| work(&table_file(DEFAULT_TABLE)));
    let old_table: String = norm(&procs).ok_or_else(|| E::Msg(format!("{procs} is not a table reference")))?;
    let table_stem: &str = stem(&old_table, &format!("{TAIL}.bgyml"))?;
    let new_table_stem: String = format!("{table_stem}{}", crate::pickup_pack::kit::SUFFIX);
    let new_table: String = table_file(&new_table_stem);
    let death_stem: String = format!("{}{}", stem(&death, DEATH_EXT)?, crate::pickup_pack::kit::SUFFIX);
    let new_death: String = format!("{DEATH_DIR}{death_stem}{DEATH_EXT}");
    let new_ai: String = format!("{AI_DIR}{ai_stem}{AI_EXT}");
    let mut fresh: Vec<(String, usize)> = Vec::new();

    let mut g: Ainb = Ainb::parse(p.get(&old_graph).ok_or_else(|| E::Msg(format!("{old_graph} is not in the source packs")))?)?;
    let s: Ainb = Ainb::parse(skull_graph)?;
    hand_hash(&mut g, &s)?;
    put(&mut p, &mut fresh, &new_graph, g.write()?)?;

    let mut a: Byml = p.byml(&ai)?;
    a.root.set(ROOT_REF, Node::Str(new_root.replace("AI/Root/", "Work/AI/Root/")));
    put(&mut p, &mut fresh, &new_ai, a.write()?)?;

    let mut d: Byml = p.byml(&death)?;
    d.root.set(PROCEDURES, Node::Str(work(&new_table)));
    put(&mut p, &mut fresh, &new_death, d.write()?)?;

    let parent_ref: String = work(&old_table);
    put(&mut p, &mut fresh, &new_table, keep_table(&parent_ref, cells, version)?)?;

    let mut comps: Node = Node::Map(Vec::new());
    comps.set(AI_REF, Node::Str(local(&new_ai)));
    comps.set(DEATH_REF, Node::Str(local(&new_death)));
    let mut actor: Node = Node::Map(Vec::new());
    actor.set(PARENT, Node::Str(work(&actor_file(base))));
    actor.set(COMPONENTS, comps);
    put(&mut p, &mut fresh, &actor_file(&name), byml(actor, version)?)?;

    let files: Vec<String> = select(&p, &name)?;
    if let Some((f, _)) = fresh.iter().find(|f| !files.contains(&f.0)) {
        return Err(E::Msg(format!("{f} did not make it into the pack")));
    }
    let known: BS<String> = p.files.keys().cloned().collect();
    let cn: Chain = chain(&p, &name)?;
    if let Some(f) = dangling_with(&p, &files, &known, &cn)?.first() {
        return Err(E::Msg(format!("{f} is referenced but left out of the pack")));
    }
    let es: Vec<Entry> = files.iter().map(|f| Entry { name: f.clone(), data: p.get(f).unwrap_or(&[]).to_vec() }).collect();
    let mut all: Vec<(String, usize)> = packs.iter().flatten().map(|e| (e.name.clone(), e.data.len())).collect();
    all.extend(peers.iter().cloned());
    Ok(PotPlan { name, base: base.to_string(), files, fresh, peers: all, sarc: sarc::write(&es)? })
}

pub const SKULL_PACK: &str = "Enemy_Skulltula_Gold";
pub const SKULL_UTIL: &str = "AI/Enemy_Skulltula_Gold.utility.root.ainb";
pub const BASE_PACK: &str = "Obj_Pot_A_01";
pub const ACTORS: [&str; 8] = ["Obj_Pot_A_01", "Obj_Pot_A_02", "Obj_Pot_A_03", "Obj_Pot_A_04", "Obj_Pot_A_05", "Obj_Pot_A_06", "Obj_Pot_A_07", "Obj_Pot_A_08"];

fn cells_of(rf: &mut Romfs) -> Res<Vec<String>> {
    let c: Vec<Byml> = death_chain(rf, DEFAULT_TABLE)?;
    let top: &Byml = c.last().ok_or(E::Format("pot kit: the default death table is missing"))?;
    match top.root.get(CELLS) {
        Some(Node::Map(m)) => Ok(m.iter().filter_map(|e| e.0.strip_suffix("___Data").map(|s| s.to_string())).collect()),
        _ => Err(E::Format("pot kit: the default death table has no cell list")),
    }
}

fn loose_peers(rf: &Romfs) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = Vec::new();
    for f in rf.list(TABLES).unwrap_or_default() {
        if let Ok(d) = rf.read(&f) {
            out.push((f, d.len()));
        }
    }
    out
}

fn clone_row(y: &mut Byml, from: &str, to: &str, t: &str) -> Res<usize> {
    let mut r: Node = rows::need(y, from, t)?.clone();
    r.set(rows::ROW_ID, Node::Str(to.to_string()));
    rows::insert(y, r)
}

pub fn run(romfs: &P, load: &P, mod_name: &str, actors: &[&str], o: &Opts) -> Res<Done> {
    let o: &Opts = &Opts { base: base_dir(o)?, ..o.clone() };
    let mut rf: Romfs = Romfs::open(romfs)?;
    let skull: Vec<Entry> = sarc::parse(&rf.read_zs(&pack_of(SKULL_PACK))?)?;
    let util: Vec<u8> = sarc::get(&skull, SKULL_UTIL).ok_or_else(|| E::Msg(format!("{SKULL_UTIL} is not in the skulltula pack")))?.data.clone();
    let cells: Vec<String> = cells_of(&mut rf)?;
    let peers: Vec<(String, usize)> = loose_peers(&rf);
    let base: Vec<Entry> = sarc::parse(&rf.read_zs(&pack_of(BASE_PACK))?)?;
    let (mut rst, modded): (Rst, bool) = size_table(&mut rf, o)?;
    let mut note: String = String::new();
    let id: u32 = dict_id(&rf.read(&pack_of(BASE_PACK))?).unwrap_or(PACK_DICT);
    let mut files: Vec<File> = Vec::new();
    let mut plans: Vec<PotPlan> = Vec::new();
    for a in actors {
        if rf.root.join(pack_of(&name_of(a))).is_file() {
            return Err(E::Msg(format!("the game already ships {}", name_of(a))));
        }
        let own: Vec<Entry> = sarc::parse(&rf.read_zs(&pack_of(a))?)?;
        let packs: Vec<Vec<Entry>> = if *a == BASE_PACK { vec![own] } else { vec![base.clone(), own] };
        let pl: PotPlan = plan(&packs, &util, &cells, a, &peers)?;
        files.push(File { path: pack_of(&pl.name), z: zs::compress_raw(&pl.sarc, Some(id))?, raw: pl.sarc.clone(), old: 0, fresh: true });
        plans.push(pl);
    }
    let mut at: [usize; 2] = [0; 2];
    for (i, t) in [ACTOR_INFO, GAME_ACTOR_INFO].iter().enumerate() {
        let f: File = table(&mut rf, o, modded, t, |y: &mut Byml| {
            for pl in &plans {
                at[i] = clone_row(y, &pl.base, &pl.name, t)?;
            }
            Ok(())
        })?;
        files.push(f);
    }
    if at[0] != at[1] {
        return Err(E::Msg(format!("the new rows sit at {} in the actor table and at {} in the game actor table, the two tables no longer line up", at[0], at[1])));
    }
    for pl in &plans {
        let n: u32 = sizes::pack_size(pl.sarc.len())?;
        rst.set_size(&pack_of(&pl.name), n)?;
        let _ = writeln!(note, "{}: {} files, {} bytes, size table entry {n}", pl.name, pl.files.len(), pl.sarc.len());
        for (f, len) in &pl.fresh {
            let n: u32 = sizes::file_size(&rst, &pl.peers, f, *len)?;
            rst.set_size(f, n)?;
        }
    }
    for f in files.iter().filter(|f| !f.fresh) {
        let was: Option<u32> = rst.get(&f.path);
        let now: Option<u32> = rst.grow(&f.path, f.old as u64, f.raw.len() as u64, crate::staging::MARGIN)?;
        let _ = writeln!(note, "{}: {} -> {} bytes, size table entry {:?} -> {:?}", f.path, f.old, f.raw.len(), was, now);
    }
    let carried: Vec<File> = earlier(&mut rf, o.base.as_ref(), "")?;
    let rst_z: Vec<u8> = rst.to_zs()?;
    let mut root: Option<std::path::PathBuf> = None;
    if !o.dry {
        let pk: Pack = Pack::create_for(load, TITLE_ID, mod_name, o.host)?;
        for f in files.iter().chain(carried.iter()) {
            pk.write_romfs(&f.path, &f.z)?;
        }
        pk.write_romfs(TABLE_PATH, &rst_z)?;
        let _ = writeln!(note, "wrote {} files to {}", files.len() + carried.len() + 1, pk.root.display());
        root = Some(pk.root);
    }
    Ok(Done { report: note, root, files, carried, heap: 0, kept: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_keep_table_clears_the_saved_row_bit_on_every_cell() {
        let b: Vec<u8> = keep_table("Work/T.gyml", &["Burned".to_string(), "LifeDepleted".to_string()], 7).unwrap();
        let y: Byml = Byml::parse(&b).unwrap();
        assert_eq!(y.root.get(PARENT).and_then(|v| v.as_str()), Some("Work/T.gyml"));
        let list: &Node = y.root.get(CELLS).unwrap();
        for k in ["Burned___Data", "LifeDepleted___Data"] {
            assert_eq!(list.get(k).and_then(|c| c.get(KEEP)), Some(&Node::Bool(false)));
        }
    }

    #[test]
    fn stems_come_off_the_ends_and_a_wrong_end_is_refused() {
        assert_eq!(stem("AI/AIInfo/Obj_Pot.engine__actor__AIInfo.bgyml", AI_EXT).unwrap(), "Obj_Pot");
        assert!(stem("AI/AIInfo/Obj_Pot.bgyml", AI_EXT).is_err());
    }

    fn real() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var("OOTRR_ROMFS").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::PathBuf::from("C:/o/nup dump/updates/131072/romfs"));
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Some(p)
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    #[test]
    fn every_pot_type_gets_a_variant_whose_graph_hands_its_hash_and_whose_table_keeps_nothing() {
        use crate::pickup_pack::pack_of;
        use crate::romfs::Romfs;
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let skull: Vec<Entry> = sarc::parse(&rf.read_zs(&pack_of(SKULL_PACK)).unwrap()).unwrap();
        let g: Vec<u8> = sarc::get(&skull, SKULL_UTIL).unwrap().data.clone();
        let cells: Vec<String> = cells_of(&mut rf).unwrap();
        assert!(cells.len() > 10);
        let base: Vec<Entry> = sarc::parse(&rf.read_zs(&pack_of(BASE_PACK)).unwrap()).unwrap();
        for n in ACTORS {
            let own: Vec<Entry> = sarc::parse(&rf.read_zs(&pack_of(n)).unwrap()).unwrap();
            let packs: Vec<Vec<Entry>> = if n == BASE_PACK { vec![own] } else { vec![base.clone(), own] };
            let pl: PotPlan = plan(&packs, &g, &cells, n, &[]).unwrap();
            let es: Vec<Entry> = sarc::parse(&pl.sarc).unwrap();
            assert!(sarc::get(&es, &actor_file(&pl.name)).is_some(), "{n}");
            let graph: &Entry = es.iter().find(|e| e.name.ends_with("_GS.root.ainb")).unwrap();
            let a: Ainb = Ainb::parse(&graph.data).unwrap();
            assert_eq!(a.find_els(crate::ainb::graft::SET).len(), 1, "{n}");
            assert!(crate::ainb::audit(&a).is_empty(), "{n}");
            let t: &Entry = es.iter().find(|e| e.name.contains("_GS_game__death__")).unwrap();
            let y: Byml = Byml::parse(&t.data).unwrap();
            let list: &Node = y.root.get(CELLS).unwrap();
            for c in &cells {
                assert_eq!(list.get(&format!("{c}___Data")).and_then(|x| x.get(KEEP)), Some(&Node::Bool(false)), "{n} {c}");
            }
        }
    }
}
