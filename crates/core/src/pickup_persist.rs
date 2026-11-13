use std::collections::{BTreeSet as BS, HashMap as HM};
use std::fs;
use std::path::{Path as P, PathBuf as PB};

use crate::byml::{Byml, Node};
use crate::chest_slot::LIST;
use crate::error::{Error as E, Res};
use crate::placement::hash_of;
use crate::pots;
use crate::romfs::Romfs;
use crate::sarc::{self, Entry};

pub const INFO: &str = "RSDB/GameActorInfo.Product.111.rstbl.byml.zs";
pub const TABLES: &str = "System/CombinationDataTableData/";
pub const TAIL: &str = "_game__death__DeathReasonProcedures.pp__CombinationDataTableData";
pub const TAKEN: [&str; 2] = ["PickedUp", "LifeDepleted"];
pub const ALWAYS: &str = "AlwaysRespawn";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub dead: bool,
    pub saved: bool,
    pub ignored: bool,
}

impl Default for Cell {
    fn default() -> Cell {
        Cell { dead: false, saved: true, ignored: false }
    }
}

impl Cell {
    pub fn stays(&self) -> bool {
        !self.ignored && self.dead && self.saved
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Verdict {
    pub actor: String,
    pub respawn: bool,
    pub setting: String,
    pub table: String,
    pub cells: Vec<(String, Cell)>,
}

impl Verdict {
    pub fn stays(&self) -> bool {
        self.respawn && self.setting != ALWAYS && !self.cells.is_empty() && self.cells.iter().all(|(_, c)| c.stays())
    }

    pub fn why(&self) -> String {
        let mut out: Vec<String> = Vec::new();
        if !self.respawn {
            out.push("GameActorInfo has no IsRespawn".to_string());
        }
        if self.setting == ALWAYS {
            out.push(format!("ActorGameDataSetting is {ALWAYS}"));
        }
        for (r, c) in &self.cells {
            if !c.stays() {
                out.push(format!("death reason {r} keeps the placement alive"));
            }
        }
        out.join(", ")
    }
}

fn last(path: &str, cut: &str) -> String {
    let n: &str = path.rsplit('/').next().unwrap_or(path);
    n.split(cut).next().unwrap_or(n).to_string()
}

pub fn table_name(r: &str) -> String {
    last(r, TAIL)
}

fn flag(m: &Node, k: &str) -> Option<bool> {
    match m.get(k) {
        Some(Node::Bool(b)) => Some(*b),
        _ => None,
    }
}

pub fn cell_of(chain: &[Byml], reason: &str) -> Cell {
    let key: String = format!("{reason}___Data");
    let mut c: Cell = Cell::default();
    for y in chain {
        let Some(m) = y.root.get("CellList").and_then(|l| l.get(&key)) else {
            continue;
        };
        if let Some(b) = flag(m, "IsSetRespawnFlagToFalse") {
            c.dead = b;
        }
        if let Some(b) = flag(m, "IsSetGmdRespawnFlagToFalse") {
            c.saved = b;
        }
        if let Some(b) = flag(m, "IsIgnoreThisDeathReason") {
            c.ignored = b;
        }
    }
    c
}

pub fn chain(rf: &mut Romfs, name: &str) -> Res<Vec<Byml>> {
    let mut out: Vec<Byml> = Vec::new();
    let mut seen: BS<String> = BS::new();
    let mut cur: Option<String> = Some(name.to_string());
    while let Some(n) = cur {
        if !seen.insert(n.clone()) {
            return Err(E::Msg(format!("death table {n} has a parent loop")));
        }
        let y: Byml = rf.read_byml(&format!("{TABLES}{n}{TAIL}.bgyml"))?;
        cur = y.root.get("$parent").and_then(|p| p.as_str()).map(table_name);
        out.push(y);
    }
    out.reverse();
    Ok(out)
}

pub fn procs_of(b: &[u8]) -> Option<String> {
    let k: &[u8] = b"Work/System/CombinationDataTableData/";
    let i: usize = b.windows(k.len()).position(|w| w == k)?;
    let n: usize = b[i..].iter().position(|c| *c == 0)?;
    String::from_utf8(b[i..i + n].to_vec()).ok().filter(|s| s.contains(TAIL))
}

fn entry<'a>(es: &'a [Entry], r: &str) -> Option<&'a Entry> {
    let p: String = r.trim_start_matches('?').trim_start_matches("Work/").replace(".gyml", ".bgyml");
    sarc::get(es, &p)
}

pub fn components(es: &[Entry], actor: &str) -> Res<Vec<(String, String)>> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut cur: Option<String> = Some(actor.to_string());
    let mut seen: BS<String> = BS::new();
    while let Some(n) = cur {
        if !seen.insert(n.clone()) {
            break;
        }
        let Some(e) = sarc::get(es, &format!("Actor/{n}.engine__actor__ActorParam.bgyml")) else {
            break;
        };
        let y: Byml = Byml::parse(&e.data)?;
        if let Some(Node::Map(m)) = y.root.get("Components") {
            for (k, v) in m {
                if let Some(s) = v.as_str()
                    && !out.iter().any(|(a, _)| a == k)
                {
                    out.push((k.clone(), s.to_string()));
                }
            }
        }
        cur = y.root.get("$parent").and_then(|p| p.as_str()).map(|p| last(p, ".engine__actor__"));
    }
    if out.is_empty() {
        return Err(E::Msg(format!("actor pack {actor} has no ActorParam")));
    }
    Ok(out)
}

pub fn flags_of(y: &Byml) -> Res<HM<String, bool>> {
    let rows: &[Node] = y.root.as_arr().ok_or(E::Format("GameActorInfo is not a row list"))?;
    let mut out: HM<String, bool> = HM::new();
    for r in rows {
        if let Some(id) = r.get("__RowId").and_then(|v| v.as_str()) {
            out.insert(id.to_string(), flag(r, "IsRespawn") == Some(true));
        }
    }
    Ok(out)
}

pub fn respawn_flags(rf: &mut Romfs) -> Res<HM<String, bool>> {
    flags_of(&rf.read_byml(INFO)?)
}

#[derive(Debug, Clone, Default)]
pub struct Extra {
    pub packs: HM<String, Vec<Entry>>,
    pub flags: HM<String, bool>,
}

impl Extra {
    pub fn add_pack(&mut self, actor: &str, sarc_raw: &[u8]) -> Res<()> {
        self.packs.insert(actor.to_string(), sarc::parse(sarc_raw)?);
        Ok(())
    }

    pub fn add_table(&mut self, game_actor_info: &[u8]) -> Res<()> {
        self.flags.extend(flags_of(&Byml::parse(game_actor_info)?)?);
        Ok(())
    }

    pub fn has(&self, actor: &str) -> bool {
        self.packs.contains_key(actor)
    }

    pub fn load(rf: &mut Romfs, mod_romfs: &P) -> Res<Extra> {
        let mut out: Extra = Extra::default();
        let t: PB = mod_romfs.join(INFO);
        if t.is_file() {
            out.add_table(&rf.zs.decompress(&fs::read(t)?)?)?;
        }
        let dir: PB = mod_romfs.join("Pack").join("Actor");
        if !dir.is_dir() {
            return Ok(out);
        }
        for e in fs::read_dir(&dir)? {
            let e: fs::DirEntry = e?;
            let n: String = e.file_name().to_string_lossy().to_string();
            if let Some(actor) = n.strip_suffix(".pack.zs") {
                out.add_pack(actor, &rf.zs.decompress(&fs::read(e.path())?)?)?;
            }
        }
        Ok(out)
    }
}

pub fn verdict(rf: &mut Romfs, flags: &HM<String, bool>, actor: &str) -> Res<Verdict> {
    verdict_with(rf, flags, &Extra::default(), actor)
}

pub fn verdict_with(rf: &mut Romfs, flags: &HM<String, bool>, extra: &Extra, actor: &str) -> Res<Verdict> {
    let es: Vec<Entry> = match extra.packs.get(actor) {
        Some(es) => es.clone(),
        None => sarc::parse(&rf.read_zs(&format!("Pack/Actor/{actor}.pack.zs"))?)?,
    };
    let cs: Vec<(String, String)> = components(&es, actor)?;
    let get = |k: &str| cs.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
    let setting: String = get("ActorGameDataSetting").map(|s| last(&s, ".game__component__")).unwrap_or_default();
    let death: String = get("DeathRef").ok_or_else(|| E::Msg(format!("{actor} has no DeathRef")))?;
    let param: &Entry = entry(&es, &death).ok_or_else(|| E::Msg(format!("{actor} does not carry its death parameter {death}")))?;
    let procs: String = procs_of(&param.data).ok_or_else(|| E::Msg(format!("{actor} death parameter names no table")))?;
    let table: String = table_name(&procs);
    let ch: Vec<Byml> = chain(rf, &table)?;
    let cells: Vec<(String, Cell)> = TAKEN.iter().map(|r| (r.to_string(), cell_of(&ch, r))).collect();
    let respawn: bool = extra.flags.get(actor).or_else(|| flags.get(actor)).copied().unwrap_or(false);
    Ok(Verdict { actor: actor.to_string(), respawn, setting, table, cells })
}

pub fn rows_of(y: &Byml) -> Res<HM<u64, (i64, i64)>> {
    let list: &[Node] = y.root.get("Data").and_then(|d| d.get(pots::TABLE)).and_then(|t| t.as_arr()).ok_or(E::Format("flag list has no 64-bit key table"))?;
    let mut out: HM<u64, (i64, i64)> = HM::new();
    for r in list {
        if let Some(h) = hash_of(r) {
            out.insert(h, pots::saved(r));
        }
    }
    Ok(out)
}

pub fn rows(rf: &mut Romfs) -> Res<HM<u64, (i64, i64)>> {
    rows_of(&rf.read_byml(LIST)?)
}

pub fn kept(rows: &HM<u64, (i64, i64)>, h: u64) -> bool {
    rows.get(&h).is_some_and(|r| pots::permanent(*r))
}

pub fn check_swaps(rf: &mut Romfs, actors: &[&str]) -> Res<()> {
    check_swaps_with(rf, actors, &Extra::default())
}

pub fn check_swaps_with(rf: &mut Romfs, actors: &[&str], extra: &Extra) -> Res<()> {
    if actors.is_empty() {
        return Ok(());
    }
    let flags: HM<String, bool> = respawn_flags(rf)?;
    let uniq: BS<&str> = actors.iter().copied().collect();
    for a in uniq {
        let v: Verdict = verdict_with(rf, &flags, extra, a)?;
        if !v.stays() {
            return Err(E::Msg(format!("{a} would not stay collected once taken ({}), so no freestanding spot may become it", v.why())));
        }
    }
    Ok(())
}

pub fn check_rows(rows: &HM<u64, (i64, i64)>, spots: &[(u64, &str)]) -> Res<()> {
    for (h, a) in spots {
        if !kept(rows, *h) {
            return Err(E::Msg(format!("actor {h} has no permanent saved row ({:?}), so {a} placed there would come back", rows.get(h))));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::catalog::Loc;
    use crate::skull_death;
    use crate::slice::data::Data;
    use crate::slice::pickups::{is_world, world_actor};

    fn cell(pairs: &[(&str, bool)]) -> Node {
        Node::Map(pairs.iter().map(|(k, v)| (k.to_string(), Node::Bool(*v))).collect())
    }

    fn table(cells: Vec<(&str, Node)>) -> Byml {
        let list: Node = Node::Map(cells.into_iter().map(|(k, v)| (format!("{k}___Data"), v)).collect());
        Byml { root: Node::Map(vec![("CellList".to_string(), list)]), version: 7, big: false }
    }

    #[test]
    fn cells_merge_field_by_field_from_the_root_table_down() {
        let base: Byml = table(vec![("PickedUp", cell(&[("IsSetRespawnFlagToFalse", true)])), ("FallIntoAbyss", cell(&[]))]);
        let mid: Byml = table(vec![("PickedUp", cell(&[("IsSetGmdRespawnFlagToFalse", false)]))]);
        let top: Byml = table(vec![("PickedUp", cell(&[("IsSetGmdRespawnFlagToFalse", true)]))]);
        let c: Cell = cell_of(&[base.clone(), mid.clone()], "PickedUp");
        assert_eq!(c, Cell { dead: true, saved: false, ignored: false });
        assert!(!c.stays());
        assert!(cell_of(&[base.clone(), mid, top], "PickedUp").stays());
        assert!(!cell_of(&[base.clone()], "FallIntoAbyss").stays());
        assert_eq!(cell_of(&[base], "Nothing"), Cell::default());
    }

    #[test]
    fn a_verdict_needs_the_type_bit_a_normal_setting_and_both_pickup_cells() {
        let good: Cell = Cell { dead: true, saved: true, ignored: false };
        let mut v: Verdict = Verdict {
            actor: "Obj_X".to_string(),
            respawn: true,
            setting: "NoRespawn".to_string(),
            table: "DropItem".to_string(),
            cells: vec![("PickedUp".to_string(), good), ("LifeDepleted".to_string(), good)],
        };
        assert!(v.stays());
        assert!(v.why().is_empty());
        v.setting = ALWAYS.to_string();
        assert!(!v.stays());
        v.setting = "NoRespawn".to_string();
        v.respawn = false;
        assert!(!v.stays());
        assert!(v.why().contains("IsRespawn"));
        v.respawn = true;
        v.cells[1].1.dead = false;
        assert!(!v.stays());
        assert!(v.why().contains("LifeDepleted"));
        v.cells.clear();
        assert!(!v.stays());
    }

    #[test]
    fn names_are_cut_from_reference_paths() {
        assert_eq!(table_name("Work/System/CombinationDataTableData/DropItem_game__death__DeathReasonProcedures.pp__CombinationDataTableData.gyml"), "DropItem");
        assert_eq!(last("?Component/ActorGameDataSetting/NoRespawn.game__component__ActorGameDataSetting.bgyml", ".game__component__"), "NoRespawn");
        assert_eq!(last("Work/Actor/Obj_Item_Base.engine__actor__ActorParam.gyml", ".engine__actor__"), "Obj_Item_Base");
        let b: Vec<u8> = b"YB\x07\x00Death_PickedUp\x00Work/System/CombinationDataTableData/DropItem_game__death__DeathReasonProcedures.pp__CombinationDataTableData.gyml\x00".to_vec();
        assert_eq!(procs_of(&b).map(|s| table_name(&s)).as_deref(), Some("DropItem"));
        assert_eq!(procs_of(b"Work/System/CombinationDataTableData/Other.gyml\x00"), None);
        assert_eq!(procs_of(b"nothing"), None);
    }

    #[test]
    fn a_row_is_kept_only_at_type_24_without_an_extra_byte() {
        let mut r: HM<u64, (i64, i64)> = HM::new();
        r.insert(1, (24, 0));
        r.insert(2, (88, 7));
        r.insert(3, (24, 3));
        assert!(kept(&r, 1));
        assert!(!kept(&r, 2));
        assert!(!kept(&r, 3));
        assert!(!kept(&r, 4));
    }

    fn romfs() -> Option<Romfs> {
        let p: PB = std::env::var("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|_| PB::from("C:/o/nup dump/updates/131072/romfs"));
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Romfs::open(&p).ok()
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    #[test]
    fn every_class_a_freestanding_spot_can_become_stays_collected_in_the_dump() {
        let Some(mut rf) = romfs() else {
            return;
        };
        let d: Data = Data::load().unwrap();
        let flags: HM<String, bool> = respawn_flags(&mut rf).unwrap();
        let mut seen: BS<&str> = BS::new();
        for l in d.cat.locs.iter().filter(|l| is_world(l)) {
            seen.insert(l.item.as_str());
        }
        for i in ["Piece of Heart", "Recovery Heart", "Rupee (1)", "Rupees (5)", "Rupees (20)", "Rupees (50)"] {
            seen.insert(world_actor(i).unwrap());
        }
        assert!(seen.len() >= 7);
        for a in &seen {
            let v: Verdict = verdict(&mut rf, &flags, a).unwrap();
            assert!(v.stays(), "{a}: {}", v.why());
            assert_eq!(v.table, "DropItem", "{a}");
            assert_eq!(v.setting, "NoRespawn", "{a}");
        }
        let swaps: Vec<&str> = seen.iter().copied().collect();
        assert!(check_swaps(&mut rf, &swaps).is_ok());
    }

    #[test]
    fn the_dump_keeps_a_type_24_row_for_every_hash_of_every_freestanding_spot() {
        let Some(mut rf) = romfs() else {
            return;
        };
        let d: Data = Data::load().unwrap();
        let rs: HM<u64, (i64, i64)> = rows(&mut rf).unwrap();
        let mut n: usize = 0;
        for l in d.cat.locs.iter().filter(|l| is_world(l)) {
            for r in &l.refs {
                n += 1;
                assert!(kept(&rs, r.hash), "{} hash {} row {:?}", l.name, r.hash, rs.get(&r.hash));
            }
        }
        assert_eq!(n, 37);
    }

    #[test]
    fn classes_that_do_not_stay_are_refused_as_swap_targets() {
        let Some(mut rf) = romfs() else {
            return;
        };
        let flags: HM<String, bool> = respawn_flags(&mut rf).unwrap();
        let key: Verdict = verdict(&mut rf, &flags, "Obj_Dungeon_SmallKey").unwrap();
        assert!(!key.stays(), "{key:?}");
        let pot: Verdict = verdict(&mut rf, &flags, "Obj_Pot_A_02_Respawn").unwrap();
        assert!(!pot.stays(), "{pot:?}");
        assert!(check_swaps(&mut rf, &["Obj_Rupee_Green", "Obj_Dungeon_SmallKey"]).is_err());
        assert!(check_swaps(&mut rf, &[]).is_ok());
    }

    #[test]
    fn the_skulltula_table_keeps_a_killed_skulltula_only_after_the_mod_edit() {
        let Some(mut rf) = romfs() else {
            return;
        };
        let mut ch: Vec<Byml> = chain(&mut rf, "Enemy_Skulltula_Gold").unwrap();
        assert_eq!(ch.len(), 3);
        assert!(!cell_of(&ch, "LifeDepleted").stays());
        let raw: Vec<u8> = rf.read(skull_death::PATH).unwrap();
        let (out, _): (Vec<u8>, usize) = skull_death::persist(&raw).unwrap();
        let top: usize = ch.len() - 1;
        ch[top] = Byml::parse(&out).unwrap();
        assert!(cell_of(&ch, "LifeDepleted").stays());
        assert!(cell_of(&ch, "Drowned").stays());
        assert!(!cell_of(&ch, "ChemicalCriticalHitIce").dead);
        assert!(!cell_of(&ch, "FallIntoAbyss").dead);
    }

    #[test]
    fn pot_classes_die_with_a_saved_row_and_only_the_row_type_brings_them_back() {
        let Some(mut rf) = romfs() else {
            return;
        };
        let flags: HM<String, bool> = respawn_flags(&mut rf).unwrap();
        for a in ["Obj_Pot_A_01", "Obj_Pot_A_04", "Obj_Pot_A_08", "Obj_BrokenBox_A_03"] {
            let v: Verdict = verdict(&mut rf, &flags, a).unwrap();
            assert!(v.respawn, "{a}");
            assert_eq!(v.setting, "SupplyObject", "{a}");
            let ch: Vec<Byml> = chain(&mut rf, &v.table).unwrap();
            assert!(cell_of(&ch, "LifeDepleted").stays(), "{a}");
        }
        let d: Data = Data::load().unwrap();
        let rs: HM<u64, (i64, i64)> = rows(&mut rf).unwrap();
        let twice: Vec<&Loc> = d.cat.locs.iter().filter(|l| is_world(l) && crate::slice::copies::hashes(l) > 1).collect();
        assert_eq!(twice.len(), 7);
        for l in twice {
            let hs: BS<u64> = l.refs.iter().map(|r| r.hash).collect();
            assert!(hs.iter().all(|h| kept(&rs, *h)), "{}", l.name);
        }
    }
}
