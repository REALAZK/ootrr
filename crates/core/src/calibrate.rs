use std::collections::{BTreeMap as BM, BTreeSet as BS, HashMap as HM};
use std::fmt::Write as W;
use std::path::{Path as P, PathBuf as PB};

use crate::TITLE_ID;
use crate::byml::{Byml, Node};
use crate::catalog::{Catalog, Conf, Via};
use crate::error::{Error as E, Res};
use crate::modpack::Pack;
use crate::placement::hash_of;
use crate::romfs::Romfs;
use crate::rsizetable::{self, Rst};
use crate::slice::banc::{actors, find, Found};
use crate::slice::edit::{edit_file, Edited, Target, MARGIN};
use crate::zs;

pub const CHEST_ITEMS: [(&str, &str); 12] = [
    ("TBox_Rupee_Green", "green rupee"),
    ("TBox_Rupee_Blue", "blue rupee"),
    ("TBox_Rupee_Red", "red rupee"),
    ("TBox_Rupee_Purple", "purple rupee"),
    ("TBox_Rupee_Gold", "gold rupee"),
    ("Item_BombBundle_Small", "bombs"),
    ("Item_BombchuBundle_Small", "bombchus"),
    ("TBox_ArrowBundle_Medium", "medium arrows"),
    ("TBox_ArrowBundle_Large", "large arrows"),
    ("TBox_DekuNutBundle", "deku nuts"),
    ("Item_DekuStickBundle_Small", "deku sticks"),
    ("Item_ScrubBusinessSeed", "deku seeds"),
];

pub const POT_ITEMS: [(&str, &str); 9] = [
    ("Obj_Rupee_Green", "green rupee"),
    ("Obj_Rupee_Blue", "blue rupee"),
    ("Obj_Rupee_Red", "red rupee"),
    ("Obj_Heart", "recovery heart"),
    ("Obj_MagicJar_Small", "small magic jar"),
    ("Obj_Fairy", "fairy"),
    ("Obj_Bomb", "bomb"),
    ("Obj_DekuStick", "deku stick"),
    ("Obj_ArrowBundle_Small", "arrows"),
];

pub const CHEST_GYAML: [&str; 3] = ["Obj_TBox_A_01", "Obj_TBox_A_01_NoBake", "Obj_TBox_GlassTruth_A_01"];

#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub hash: u64,
    pub files: Vec<String>,
    pub old: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Raw {
    pub id: String,
    pub names: Vec<String>,
    pub members: Vec<Member>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Info {
    pub gyaml: String,
    pub pos: Vec<f32>,
    pub room: Option<i64>,
    pub event: bool,
    pub check: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spot {
    pub hash: u64,
    pub files: Vec<String>,
    pub old: String,
    pub new: String,
    pub label: String,
    pub pos: Vec<f32>,
    pub room: Option<i64>,
    pub gyaml: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub id: String,
    pub names: Vec<String>,
    pub spots: Vec<Spot>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Skip {
    pub id: String,
    pub hash: u64,
    pub pos: Vec<f32>,
    pub why: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Visit {
    pub id: String,
    pub names: Vec<String>,
    pub hash: u64,
    pub pos: Vec<f32>,
    pub room: Option<i64>,
    pub gyaml: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub area: String,
    pub groups: Vec<Group>,
    pub skipped: Vec<Skip>,
    pub visits: Vec<Visit>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Done {
    pub root: PB,
    pub files: usize,
    pub changed: usize,
    pub grown: usize,
    pub issues: Vec<String>,
}

pub fn folders(cat: &Catalog) -> Vec<String> {
    let mut s: BS<String> = BS::new();
    for l in &cat.locs {
        s.insert(l.folder.clone());
    }
    for g in &cat.gap_rem {
        s.insert(g.folder.clone());
    }
    s.into_iter().collect()
}

pub fn areas(cat: &Catalog) -> Vec<String> {
    folders(cat)
        .into_iter()
        .filter(|f| {
            let (a, b): (Vec<Raw>, Vec<Raw>) = select(cat, f);
            !a.is_empty() || !b.is_empty()
        })
        .collect()
}

pub fn select(cat: &Catalog, area: &str) -> (Vec<Raw>, Vec<Raw>) {
    let mut set: BM<String, Raw> = BM::new();
    let mut see: BM<String, Raw> = BM::new();
    for l in &cat.locs {
        if l.folder != area || l.conf != Conf::Pool {
            continue;
        }
        let id: String = l.pool.clone().unwrap_or_else(|| l.name.clone());
        let dst: &mut BM<String, Raw> = if l.via == Via::Drop { &mut set } else { &mut see };
        let g: &mut Raw = dst.entry(id.clone()).or_insert_with(|| Raw { id, names: Vec::new(), members: Vec::new() });
        g.names.push(l.name.clone());
        for r in &l.refs {
            g.members.push(Member { hash: r.hash, files: r.files.clone(), old: l.item.clone() });
        }
    }
    for g in &cat.gap_rem {
        if g.folder != area || g.via != Via::Drop || !g.reason.contains("ambiguous") {
            continue;
        }
        let id: String = format!("{}: {}", g.kind, g.reason);
        let e: &mut Raw = set.entry(id.clone()).or_insert_with(|| Raw { id, names: Vec::new(), members: Vec::new() });
        for n in &g.cands {
            if !e.names.contains(n) {
                e.names.push(n.clone());
            }
        }
        for r in &g.refs {
            e.members.push(Member { hash: r.hash, files: r.files.clone(), old: g.item.clone() });
        }
    }
    (set.into_values().collect(), see.into_values().collect())
}

fn room_of(a: &Node) -> Option<i64> {
    match a.get("Dynamic")?.get("DefaultDungeonRoomIndex")? {
        Node::I32(v) => Some(*v as i64),
        Node::U32(v) => Some(*v as i64),
        Node::I64(v) => Some(*v),
        Node::U64(v) => i64::try_from(*v).ok(),
        _ => None,
    }
}

pub fn inspect(y: &Byml, file: &str, hash: u64) -> Option<Info> {
    let f: Found = find(y, file, hash).into_iter().next()?;
    let a: &Node = actors(y).iter().find(|a| hash_of(a) == Some(hash))?;
    Some(Info { gyaml: f.gyaml, pos: f.pos, room: room_of(a), event: f.event, check: f.check })
}

fn palette(gyaml: &str) -> Option<(&'static [(&'static str, &'static str)], &'static str)> {
    if CHEST_GYAML.contains(&gyaml) {
        Some((&CHEST_ITEMS, "chest"))
    } else if gyaml.starts_with("Obj_Pot_") {
        Some((&POT_ITEMS, "pot"))
    } else {
        None
    }
}

fn key(i: &Info) -> (i64, i64, i64, i64) {
    let q = |v: Option<&f32>| (v.copied().unwrap_or(0.0) * 10.0).round() as i64;
    (i.room.unwrap_or(-1), -q(i.pos.get(1)), q(i.pos.first()), q(i.pos.get(2)))
}

pub fn assign(area: &str, raws: &[Raw], infos: &HM<(u64, String), Info>, visits: &[Raw]) -> Plan {
    let mut groups: Vec<Group> = Vec::new();
    let mut skipped: Vec<Skip> = Vec::new();
    let mut used: BM<&'static str, usize> = BM::new();
    for r in raws {
        let mut rows: Vec<(&Member, &Info)> = Vec::new();
        for m in &r.members {
            let Some(f) = m.files.first() else {
                continue;
            };
            let Some(i) = infos.get(&(m.hash, f.clone())) else {
                skipped.push(Skip { id: r.id.clone(), hash: m.hash, pos: Vec::new(), why: "actor not found in its file".to_string() });
                continue;
            };
            rows.push((m, i));
        }
        rows.sort_by_key(|(_, i)| key(i));
        let mut spots: Vec<Spot> = Vec::new();
        let mut taken: BS<&str> = BS::new();
        for (m, i) in rows {
            let why: Option<String> = if i.event {
                Some("runs a story event".to_string())
            } else if i.check {
                Some("checks or substitutes its item".to_string())
            } else if palette(&i.gyaml).is_none() {
                Some(format!("{} cannot be set safely", i.gyaml))
            } else {
                None
            };
            if let Some(w) = why {
                skipped.push(Skip { id: r.id.clone(), hash: m.hash, pos: i.pos.clone(), why: w });
                continue;
            }
            let Some((pal, tag)) = palette(&i.gyaml) else {
                continue;
            };
            let mut pick: Option<(&str, &str)> = None;
            let n: usize = used.get(tag).copied().unwrap_or(0);
            for k in 0..pal.len() {
                let c: (&str, &str) = pal[(n + k) % pal.len()];
                if !taken.contains(c.0) {
                    pick = Some(c);
                    break;
                }
            }
            let Some((new, label)) = pick else {
                skipped.push(Skip { id: r.id.clone(), hash: m.hash, pos: i.pos.clone(), why: "no distinct item left in the palette".to_string() });
                continue;
            };
            taken.insert(new);
            *used.entry(tag).or_insert(0) += 1;
            spots.push(Spot {
                hash: m.hash,
                files: m.files.clone(),
                old: m.old.clone(),
                new: new.to_string(),
                label: label.to_string(),
                pos: i.pos.clone(),
                room: i.room,
                gyaml: i.gyaml.clone(),
            });
        }
        if !spots.is_empty() {
            groups.push(Group { id: r.id.clone(), names: r.names.clone(), spots });
        }
    }
    let mut seen: Vec<Visit> = Vec::new();
    for r in visits {
        for m in &r.members {
            let Some(f) = m.files.first() else {
                continue;
            };
            let Some(i) = infos.get(&(m.hash, f.clone())) else {
                continue;
            };
            seen.push(Visit { id: r.id.clone(), names: r.names.clone(), hash: m.hash, pos: i.pos.clone(), room: i.room, gyaml: i.gyaml.clone() });
        }
    }
    seen.sort_by(|a, b| (&a.id, a.room, a.hash).cmp(&(&b.id, b.room, b.hash)));
    Plan { area: area.to_string(), groups, skipped, visits: seen }
}

pub fn plan(cat: &Catalog, rf: &mut Romfs, area: &str) -> Res<Plan> {
    let (raws, visits): (Vec<Raw>, Vec<Raw>) = select(cat, area);
    if raws.is_empty() && visits.is_empty() {
        return Err(E::Msg(format!(
            "nothing to calibrate in {area}, the areas with guesses are: {}",
            areas(cat).join(", ")
        )));
    }
    let mut cache: BM<String, Byml> = BM::new();
    let mut infos: HM<(u64, String), Info> = HM::new();
    for r in raws.iter().chain(visits.iter()) {
        for m in &r.members {
            let Some(f) = m.files.first() else {
                continue;
            };
            if !cache.contains_key(f) {
                cache.insert(f.clone(), rf.read_byml(f)?);
            }
            if let Some(y) = cache.get(f)
                && let Some(i) = inspect(y, f, m.hash)
            {
                infos.insert((m.hash, f.clone()), i);
            }
        }
    }
    Ok(assign(area, &raws, &infos, &visits))
}

fn same_but_drop(a: &Node, b: &Node, new: &str) -> bool {
    let (Node::Map(x), Node::Map(y)) = (a, b) else {
        return false;
    };
    if x.len() != y.len() {
        return false;
    }
    for (k, v) in x {
        let Some(w) = b.get(k) else {
            return false;
        };
        if k == "Dynamic" {
            let (Node::Map(dv), Node::Map(dw)) = (v, w) else {
                return false;
            };
            if dv.len() != dw.len() {
                return false;
            }
            for (dk, dx) in dv {
                let Some(dy) = w.get(dk) else {
                    return false;
                };
                let ok: bool = if dk == "Drop__DropActor" { dy.as_str() == Some(new) } else { dx == dy };
                if !ok {
                    return false;
                }
            }
        } else if v != w {
            return false;
        }
    }
    true
}

pub fn compare(file: &str, old: &Byml, new: &Byml, want: &HM<u64, &str>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if old.version != new.version || old.big != new.big {
        out.push(format!("{file}: header differs"));
    }
    let (Node::Map(a), Node::Map(b)) = (&old.root, &new.root) else {
        out.push(format!("{file}: root is not a map"));
        return out;
    };
    if a.len() != b.len() {
        out.push(format!("{file}: root keys differ"));
    }
    for (k, v) in a {
        let Some(w) = new.root.get(k) else {
            out.push(format!("{file}: key {k} is missing"));
            continue;
        };
        if k != "Actors" {
            if v != w {
                out.push(format!("{file}: key {k} changed"));
            }
            continue;
        }
        let (Some(x), Some(y)) = (v.as_arr(), w.as_arr()) else {
            out.push(format!("{file}: Actors is not a list"));
            continue;
        };
        if x.len() != y.len() {
            out.push(format!("{file}: actor count {} became {}", x.len(), y.len()));
            continue;
        }
        let mut hit: usize = 0;
        for (p, q) in x.iter().zip(y.iter()) {
            match hash_of(p).and_then(|h| want.get(&h)) {
                Some(n) => {
                    hit += 1;
                    if !same_but_drop(p, q, n) {
                        out.push(format!("{file}: an actor changed in more than its drop item"));
                    }
                }
                None => {
                    if p != q {
                        out.push(format!("{file}: an untargeted actor changed"));
                    }
                }
            }
        }
        let mut want_n: usize = 0;
        for p in x {
            if hash_of(p).is_some_and(|h| want.contains_key(&h)) {
                want_n += 1;
            }
        }
        if hit != want_n {
            out.push(format!("{file}: expected {want_n} edited actors, found {hit}"));
        }
    }
    out
}

fn targets(p: &Plan) -> Vec<Target> {
    let mut out: Vec<Target> = Vec::new();
    for g in &p.groups {
        for s in &g.spots {
            out.push(Target {
                name: g.id.clone(),
                hash: s.hash,
                files: s.files.clone(),
                old_item: s.old.clone(),
                new_item: s.label.clone(),
                old_actor: s.old.clone(),
                new_actor: s.new.clone(),
                ev: None,
                family: None,
                key: None,
            });
        }
    }
    out
}

pub fn build(p: &Plan, rf: &mut Romfs, load: &P, mod_name: &str) -> Res<Done> {
    let ts: Vec<Target> = targets(p);
    if ts.is_empty() {
        return Err(E::Msg(format!("{} has no chest or pot that can be calibrated", p.area)));
    }
    let mut files: BS<String> = BS::new();
    for t in &ts {
        for f in &t.files {
            files.insert(f.clone());
        }
    }
    let mut edits: Vec<Edited> = Vec::new();
    let mut origs: BM<String, Byml> = BM::new();
    let mut found: BS<(String, u64)> = BS::new();
    for f in &files {
        let raw: Vec<u8> = rf.read_zs(f)?;
        let here: Vec<Target> = ts.iter().filter(|t| t.files.contains(f)).cloned().collect();
        if let Some(e) = edit_file(f, &raw, &here)? {
            for (i, _) in &e.hits {
                found.insert((f.clone(), here[*i].hash));
            }
            edits.push(e);
            origs.insert(f.clone(), Byml::parse(&raw)?);
        }
    }
    for t in &ts {
        for f in &t.files {
            if !found.contains(&(f.clone(), t.hash)) {
                return Err(E::Msg(format!("{} was not found in {f}, the game files differ", t.hash)));
            }
        }
    }
    let pk: Pack = Pack::create(load, TITLE_ID, mod_name)?;
    let mut rst: Option<Rst> = None;
    let mut grown: usize = 0;
    let mut paths: Vec<(String, PB)> = Vec::new();
    for e in &edits {
        let z: Vec<u8> = zs::compress_raw(&e.bytes, Some(3))?;
        let at: PB = pk.write_romfs(&e.file, &z)?;
        paths.push((e.file.clone(), at));
        if e.bytes.len() > e.old_len {
            if rst.is_none() {
                rst = Some(rsizetable::load(rf)?);
            }
            if let Some(t) = rst.as_mut() {
                t.grow(&e.file, e.old_len as u64, e.bytes.len() as u64, MARGIN)?;
                grown += 1;
            }
        }
    }
    if let Some(t) = &rst {
        pk.write_romfs(rsizetable::TABLE_PATH, &t.to_zs()?)?;
    }
    let mut issues: Vec<String> = Vec::new();
    let mut changed: usize = 0;
    for (f, at) in &paths {
        let back: Vec<u8> = rf.zs.decompress(&std::fs::read(at)?)?;
        let new: Byml = Byml::parse(&back)?;
        let Some(old) = origs.get(f) else {
            continue;
        };
        let mut want: HM<u64, &str> = HM::new();
        for t in &ts {
            if t.files.contains(f) {
                want.insert(t.hash, t.new_actor.as_str());
            }
        }
        changed += want.len();
        issues.extend(compare(f, old, &new, &want));
    }
    Ok(Done { root: pk.root, files: edits.len(), changed, grown, issues })
}

fn at(p: &[f32]) -> String {
    match p {
        [x, y, z] => format!("({x:.1}, {y:.1}, {z:.1})"),
        _ => "(unknown)".to_string(),
    }
}

fn room(r: Option<i64>) -> String {
    match r {
        Some(n) => format!("room {n}"),
        None => "no room index".to_string(),
    }
}

pub fn report(p: &Plan, d: Option<&Done>) -> String {
    let mut out: String = String::new();
    let n: usize = p.groups.iter().map(|g| g.spots.len()).sum();
    let _ = writeln!(out, "Calibration for {}: {} chests or pots in {} groups, {} skipped, {} to visit.", p.area, n, p.groups.len(), p.skipped.len(), p.visits.len());
    if let Some(d) = d {
        let _ = writeln!(out, "Mod folder: {}", d.root.display());
        let _ = writeln!(out, "{} files written, {} actors changed, {} needed a resource size table update.", d.files, d.changed, d.grown);
        let _ = writeln!(out, "Independent decode check: {} issues.", d.issues.len());
        for i in d.issues.iter().take(10) {
            let _ = writeln!(out, "  {i}");
        }
    }
    let _ = writeln!(out, "Walk the area once. Open each chest or break each pot below and note what it gives.");
    let _ = writeln!(out, "Then say which listed location you were standing at, and the item you got.");
    for g in &p.groups {
        let _ = writeln!(out);
        let _ = writeln!(out, "Group {}", g.id);
        let _ = writeln!(out, "  could be: {}", g.names.join(" | "));
        for (i, s) in g.spots.iter().enumerate() {
            let _ = writeln!(out, "  {}. {} {} | {} | {} | was {}", i + 1, at(&s.pos), room(s.room), s.label, s.gyaml, s.old);
        }
    }
    if !p.skipped.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "Skipped, these stay as the game has them:");
        for s in &p.skipped {
            let _ = writeln!(out, "  {} | {} | {}", at(&s.pos), s.id, s.why);
        }
    }
    if !p.visits.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "Cannot be re-itemed, visit and note which listed location each one is:");
        let mut last: &str = "";
        for v in &p.visits {
            if v.id != last {
                let _ = writeln!(out, "Group {}", v.id);
                let _ = writeln!(out, "  could be: {}", v.names.join(" | "));
                last = &v.id;
            }
            let _ = writeln!(out, "  {} {} | {}", at(&v.pos), room(v.room), v.gyaml);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIX: &str = "{\"format\":1,\"note\":\"x\",\"locations\":[\
{\"name\":\"A\",\"area\":\"Z\",\"kind\":\"chest\",\"vanilla\":\"V\",\"item\":\"TBox_Rupee_Blue\",\"conf\":\"pool\",\"folder\":\"F\",\"via\":\"drop\",\"age\":\"both\",\"pool\":\"p1\",\"refs\":[{\"hash\":\"1\",\"files\":[\"Banc/F/a.zs\"]}]},\
{\"name\":\"B\",\"area\":\"Z\",\"kind\":\"chest\",\"vanilla\":\"V\",\"item\":\"TBox_Rupee_Blue\",\"conf\":\"pool\",\"folder\":\"F\",\"via\":\"drop\",\"age\":\"both\",\"pool\":\"p1\",\"refs\":[{\"hash\":\"2\",\"files\":[\"Banc/F/a.zs\"]}]},\
{\"name\":\"C\",\"area\":\"Z\",\"kind\":\"chest\",\"vanilla\":\"V\",\"item\":\"TBox_Rupee_Red\",\"conf\":\"item\",\"folder\":\"F\",\"via\":\"drop\",\"age\":\"both\",\"refs\":[{\"hash\":\"3\",\"files\":[\"Banc/F/a.zs\"]}]},\
{\"name\":\"S\",\"area\":\"Z\",\"kind\":\"gs\",\"vanilla\":\"T\",\"item\":\"Enemy_Gold\",\"conf\":\"pool\",\"folder\":\"F\",\"via\":\"actor\",\"age\":\"both\",\"pool\":\"p2\",\"refs\":[{\"hash\":\"4\",\"files\":[\"Banc/F/a.zs\"]}]},\
{\"name\":\"T\",\"area\":\"Z\",\"kind\":\"gs\",\"vanilla\":\"T\",\"item\":\"Enemy_Gold\",\"conf\":\"pool\",\"folder\":\"F\",\"via\":\"actor\",\"age\":\"both\",\"pool\":\"p2\",\"refs\":[{\"hash\":\"5\",\"files\":[\"Banc/F/a.zs\"]}]}\
],\"unmatched_oot\":[],\"unmatched_remake\":[\
{\"folder\":\"F\",\"kind\":\"pot\",\"item\":\"Obj_Heart\",\"via\":\"drop\",\"age\":\"both\",\"reason\":\"ambiguous: 2 oot vs 2 remake candidates\",\"refs\":[{\"hash\":\"6\",\"files\":[\"Banc/F/a.zs\"]}],\"cands\":[\"Pot 1\",\"Pot 2\"]},\
{\"folder\":\"F\",\"kind\":\"pot\",\"item\":\"Obj_Heart\",\"via\":\"drop\",\"age\":\"both\",\"reason\":\"ambiguous: 2 oot vs 2 remake candidates\",\"refs\":[{\"hash\":\"7\",\"files\":[\"Banc/F/a.zs\"]}],\"cands\":[\"Pot 1\",\"Pot 2\"]},\
{\"folder\":\"F\",\"kind\":\"chest\",\"item\":\"X\",\"via\":\"drop\",\"age\":\"both\",\"reason\":\"no oot counterpart in group\",\"refs\":[{\"hash\":\"8\",\"files\":[\"Banc/F/a.zs\"]}]}\
]}";

    fn info(g: &str, p: [f32; 3], room: Option<i64>) -> Info {
        Info { gyaml: g.to_string(), pos: p.to_vec(), room, event: false, check: false }
    }

    fn infos() -> HM<(u64, String), Info> {
        let f: String = "Banc/F/a.zs".to_string();
        let mut m: HM<(u64, String), Info> = HM::new();
        m.insert((1, f.clone()), info("Obj_TBox_A_01", [5.0, 1.0, 5.0], Some(2)));
        m.insert((2, f.clone()), info("Obj_TBox_A_01", [1.0, 1.0, 1.0], Some(1)));
        m.insert((4, f.clone()), info("Enemy_Skulltula_Gold", [3.0, 2.0, 3.0], None));
        m.insert((5, f.clone()), info("Enemy_Skulltula_Gold", [9.0, 2.0, 9.0], None));
        m.insert((6, f.clone()), info("Obj_Pot_A_01", [0.0, 0.0, 0.0], None));
        m.insert((7, f), info("Obj_Pot_A_01", [1.0, 0.0, 0.0], None));
        m
    }

    #[test]
    fn palettes_hold_distinct_known_items() {
        let c: BS<&str> = CHEST_ITEMS.iter().map(|x| x.0).collect();
        let l: BS<&str> = CHEST_ITEMS.iter().map(|x| x.1).collect();
        assert_eq!(c.len(), CHEST_ITEMS.len());
        assert_eq!(l.len(), CHEST_ITEMS.len());
        let p: BS<&str> = POT_ITEMS.iter().map(|x| x.0).collect();
        assert_eq!(p.len(), POT_ITEMS.len());
        assert!(CHEST_ITEMS.iter().all(|x| !x.0.contains("Key")));
    }

    #[test]
    fn selects_pools_and_ambiguous_gaps_only() {
        let c: Catalog = Catalog::parse(FIX).unwrap();
        let (set, see) = select(&c, "F");
        assert_eq!(set.len(), 2);
        assert_eq!(set[0].id, "p1");
        assert_eq!(set[0].members.len(), 2);
        assert_eq!(set[1].names, vec!["Pot 1".to_string(), "Pot 2".to_string()]);
        assert_eq!(set[1].members.len(), 2);
        assert_eq!(see.len(), 1);
        assert_eq!(see[0].members.len(), 2);
        assert_eq!(select(&c, "Other").0.len(), 0);
        assert_eq!(folders(&c), vec!["F".to_string()]);
        assert_eq!(areas(&c), vec!["F".to_string()]);
    }

    #[test]
    fn assigns_distinct_items_in_walking_order() {
        let c: Catalog = Catalog::parse(FIX).unwrap();
        let (set, see) = select(&c, "F");
        let p: Plan = assign("F", &set, &infos(), &see);
        assert_eq!(p.groups.len(), 2);
        let g: &Group = &p.groups[0];
        assert_eq!(g.spots[0].hash, 2);
        assert_eq!(g.spots[1].hash, 1);
        assert_ne!(g.spots[0].new, g.spots[1].new);
        assert_eq!(g.spots[0].new, "TBox_Rupee_Green");
        let pots: &Group = &p.groups[1];
        assert_eq!(pots.spots[0].new, "Obj_Rupee_Green");
        assert_eq!(pots.spots[1].new, "Obj_Rupee_Blue");
        assert_eq!(p.visits.len(), 2);
        assert!(p.skipped.is_empty());
    }

    #[test]
    fn events_and_odd_actors_are_skipped() {
        let c: Catalog = Catalog::parse(FIX).unwrap();
        let (set, see) = select(&c, "F");
        let mut m: HM<(u64, String), Info> = infos();
        m.get_mut(&(1, "Banc/F/a.zs".to_string())).unwrap().event = true;
        m.get_mut(&(6, "Banc/F/a.zs".to_string())).unwrap().gyaml = "Obj_Other".to_string();
        let p: Plan = assign("F", &set, &m, &see);
        assert_eq!(p.skipped.len(), 2);
        assert!(p.skipped.iter().any(|s| s.why.contains("story event")));
        assert!(p.skipped.iter().any(|s| s.why.contains("cannot be set safely")));
        assert_eq!(p.groups[0].spots.len(), 1);
    }

    #[test]
    fn a_group_larger_than_the_palette_is_cut_short() {
        let mut members: Vec<Member> = Vec::new();
        let mut m: HM<(u64, String), Info> = HM::new();
        for h in 0..(CHEST_ITEMS.len() as u64 + 2) {
            members.push(Member { hash: h, files: vec!["f".to_string()], old: "Old".to_string() });
            m.insert((h, "f".to_string()), info("Obj_TBox_A_01", [h as f32, 0.0, 0.0], Some(0)));
        }
        let r: Raw = Raw { id: "big".to_string(), names: Vec::new(), members };
        let p: Plan = assign("F", &[r], &m, &[]);
        assert_eq!(p.groups[0].spots.len(), CHEST_ITEMS.len());
        assert_eq!(p.skipped.len(), 2);
        let used: BS<&str> = p.groups[0].spots.iter().map(|s| s.new.as_str()).collect();
        assert_eq!(used.len(), CHEST_ITEMS.len());
    }

    #[test]
    fn items_stay_distinct_across_groups_while_the_palette_lasts() {
        let mut m: HM<(u64, String), Info> = HM::new();
        let mut raws: Vec<Raw> = Vec::new();
        for g in 0..3u64 {
            let mut members: Vec<Member> = Vec::new();
            for k in 0..4u64 {
                let h: u64 = g * 10 + k;
                members.push(Member { hash: h, files: vec!["f".to_string()], old: "Old".to_string() });
                m.insert((h, "f".to_string()), info("Obj_TBox_A_01", [h as f32, 0.0, 0.0], Some(0)));
            }
            raws.push(Raw { id: format!("g{g}"), names: Vec::new(), members });
        }
        let p: Plan = assign("F", &raws, &m, &[]);
        let all: BS<&str> = p.groups.iter().flat_map(|g| g.spots.iter().map(|s| s.new.as_str())).collect();
        assert_eq!(all.len(), 12);
    }

    fn actor(h: u64, drop: &str, y: f32) -> Node {
        Node::Map(vec![
            ("Dynamic".to_string(), Node::Map(vec![("Drop__DropActor".to_string(), Node::Str(drop.to_string())), ("Extra".to_string(), Node::I32(3))])),
            ("Gyaml".to_string(), Node::Str("Obj_TBox_A_01".to_string())),
            ("Hash".to_string(), Node::U64(h)),
            ("Translate".to_string(), Node::Arr(vec![Node::F32(1.0), Node::F32(y), Node::F32(3.0)])),
        ])
    }

    fn file(list: Vec<Node>) -> Byml {
        Byml { root: Node::Map(vec![("Actors".to_string(), Node::Arr(list))]), version: 7, big: false }
    }

    #[test]
    fn compare_accepts_only_the_intended_change() {
        let old: Byml = file(vec![actor(1, "A", 2.0), actor(2, "B", 4.0)]);
        let new: Byml = file(vec![actor(1, "Z", 2.0), actor(2, "B", 4.0)]);
        let mut want: HM<u64, &str> = HM::new();
        want.insert(1, "Z");
        assert!(compare("f", &old, &new, &want).is_empty());
        let moved: Byml = file(vec![actor(1, "Z", 9.0), actor(2, "B", 4.0)]);
        assert!(!compare("f", &old, &moved, &want).is_empty());
        let other: Byml = file(vec![actor(1, "Z", 2.0), actor(2, "C", 4.0)]);
        assert!(!compare("f", &old, &other, &want).is_empty());
        let wrong: Byml = file(vec![actor(1, "Y", 2.0), actor(2, "B", 4.0)]);
        assert!(!compare("f", &old, &wrong, &want).is_empty());
        let short: Byml = file(vec![actor(1, "Z", 2.0)]);
        assert!(!compare("f", &old, &short, &want).is_empty());
    }

    #[test]
    fn inspect_reads_position_room_and_flags() {
        let mut a: Node = actor(7, "A", 2.5);
        if let Some(d) = a.get_mut("Dynamic") {
            d.set("DefaultDungeonRoomIndex", Node::I32(4));
            d.set("GetEventName", Node::Str("E".to_string()));
        }
        let y: Byml = file(vec![a]);
        let i: Info = inspect(&y, "f", 7).unwrap();
        assert_eq!(i.room, Some(4));
        assert!(i.event && !i.check);
        assert_eq!(i.pos, vec![1.0, 2.5, 3.0]);
        assert!(inspect(&y, "f", 8).is_none());
    }

    #[test]
    fn the_report_lists_coordinates_items_and_visits() {
        let c: Catalog = Catalog::parse(FIX).unwrap();
        let (set, see) = select(&c, "F");
        let p: Plan = assign("F", &set, &infos(), &see);
        let t: String = report(&p, None);
        assert!(t.contains("(1.0, 1.0, 1.0) room 1 | green rupee"));
        assert!(t.contains("could be: A | B"));
        assert!(t.contains("could be: Pot 1 | Pot 2"));
        assert!(t.contains("Cannot be re-itemed"));
        assert!(t.contains("(3.0, 2.0, 3.0)"));
    }

    #[test]
    fn the_shipped_catalog_has_evidence_and_clean_calibration_groups() {
        let c: Catalog = Catalog::parse(crate::slice::data::CATALOG_JSON).unwrap();
        assert!(c.pinned() >= 162);
        for l in &c.locs {
            assert!(!l.ev.is_empty(), "{} has no evidence code", l.name);
            assert_eq!(l.conf == Conf::Pool, l.ev == vec!["pool".to_string()], "{}", l.name);
        }
        let known: Vec<String> = areas(&c);
        assert!(known.contains(&"DodongoCavern".to_string()));
        for a in &known {
            let (set, see) = select(&c, a);
            let mut hashes: BS<(String, u64)> = BS::new();
            for r in set.iter().chain(see.iter()) {
                assert!(!r.members.is_empty());
                for m in &r.members {
                    assert!(!m.files.is_empty());
                    assert!(hashes.insert((m.files[0].clone(), m.hash)), "{a}: hash {} listed twice", m.hash);
                }
            }
        }
    }

    #[test]
    fn an_unknown_area_lists_the_known_ones() {
        let c: Catalog = Catalog::parse(FIX).unwrap();
        let (a, b) = select(&c, "Nowhere");
        assert!(a.is_empty() && b.is_empty());
    }
}
