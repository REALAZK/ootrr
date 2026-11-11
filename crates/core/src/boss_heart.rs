use std::collections::BTreeMap as BM;
use std::fmt::Write as W;
use std::path::{Path as P, PathBuf as PB};

use crate::ainb::Ainb;
use crate::byml::{Byml, Node};
use crate::chest_slot::{self, LIST};
use crate::error::{Error, Res};
use crate::placement::{drops, hash_of};
use crate::romfs::Romfs;
use crate::slice::data::Data;
use crate::staging::{banc_out, banc_raw, commit, rewrite, Committed, Out};

pub const HEART: &str = "Obj_Heart_Container";
pub const CHEST: &str = "Obj_TBox_A_02";
pub const UNLOCK: &str = "Unlock";
pub const NODE: &str = "InstanceName";

#[derive(Clone, Debug)]
pub struct Opts {
    pub rst_from: Option<PB>,
    pub banc_from: Option<PB>,
    pub chest: String,
    pub slot: bool,
}

#[derive(Clone, Debug)]
pub struct Built {
    pub report: String,
    pub files: usize,
    pub grown: usize,
}

pub fn swap(y: &mut Byml, hash: u64, drop: &str, chest: &str) -> usize {
    let Some(list) = y.root.get_mut("Actors").and_then(|a| a.as_arr_mut()) else {
        return 0;
    };
    let mut n: usize = 0;
    for a in list.iter_mut() {
        if hash_of(a) != Some(hash) || a.get("Gyaml").and_then(|v| v.as_str()) != Some(HEART) {
            continue;
        }
        a.set("Gyaml", Node::Str(chest.to_string()));
        match a.get_mut("Dynamic") {
            Some(d) => {
                d.set("Drop__DropActor", Node::Str(drop.to_string()));
            }
            None => {
                a.set("Dynamic", Node::Map(vec![("Drop__DropActor".to_string(), Node::Str(drop.to_string()))]));
            }
        }
        n += 1;
    }
    n
}

pub fn heart_ids(y: &Byml, hash: u64) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let Some(gs) = y.root.get("AiGroups").and_then(|g| g.as_arr()) else {
        return out;
    };
    for g in gs {
        for r in g.get("References").and_then(|r| r.as_arr()).unwrap_or(&[]) {
            let same: bool = match r.get("Reference") {
                Some(Node::U64(x)) => *x == hash,
                Some(Node::I64(x)) => *x as u64 == hash,
                _ => false,
            };
            if let (true, Some(id)) = (same, r.get("Id").and_then(|v| v.as_str())) {
                if !out.iter().any(|x| x == id) {
                    out.push(id.to_string());
                }
            }
        }
    }
    out
}

pub fn unlock(a: &mut Ainb, id: &str) -> Res<usize> {
    let mut n: usize = 0;
    for i in 0..a.els.len() {
        if a.prop_str(i, NODE) == Some(id) && a.add_bool_input(i, UNLOCK, true)? {
            n += 1;
        }
    }
    Ok(n)
}

fn logic_outs(rf: &mut Romfs, ids: &[String]) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    let mut done: Vec<String> = Vec::new();
    for f in rf.list("Logic")? {
        if !f.ends_with(".ainb") {
            continue;
        }
        let raw: Vec<u8> = rf.read(&f)?;
        let hit: Vec<&String> = ids.iter().filter(|id| raw.windows(id.len()).any(|w| w == id.as_bytes())).collect();
        if hit.is_empty() {
            continue;
        }
        let mut a: Ainb = Ainb::parse(&raw)?;
        let mut n: usize = 0;
        for id in hit {
            let k: usize = unlock(&mut a, id)?;
            if k > 0 {
                done.push(id.clone());
            }
            n += k;
        }
        if n > 0 {
            let z: Vec<u8> = a.write()?;
            outs.push(Out { path: f, old: raw.len(), new: z.len(), z });
        }
    }
    if let Some(m) = ids.iter().find(|i| !done.contains(i)) {
        return Err(Error::Msg(format!("no logic node of the heart container {m} was found, its chest would stay locked")));
    }
    Ok(outs)
}

pub fn slots(y: &mut Byml, all: &[u64], want: &[u64], names: &BM<u32, String>) -> Res<usize> {
    let free: Vec<u32> = chest_slot::spares(y);
    let mut n: usize = 0;
    for h in want {
        if chest_slot::has(y, *h) {
            continue;
        }
        let i: usize = all.iter().position(|x| x == h).ok_or(Error::Format("heart is not in the catalog"))?;
        let s: u32 = *free.get(i).ok_or(Error::Format("no spare flag left for this chest"))?;
        chest_slot::add(y, *h, s, names)?;
        n += 1;
    }
    Ok(n)
}

pub fn stage(rf: &mut Romfs, d: &Data, hearts: &[(u64, Vec<String>)], slot: bool) -> Res<Vec<Out>> {
    let ids: Vec<String> = hearts.iter().flat_map(|h| h.1.clone()).collect();
    let mut outs: Vec<Out> = logic_outs(rf, &ids)?;
    if outs.is_empty() {
        return Err(Error::Format("no logic file holds the heart container node, the chest would stay locked"));
    }
    if slot {
        let mut all: Vec<u64> = d.cat.locs.iter().filter(|l| l.kind == "bossheart").flat_map(|l| l.refs.iter().map(|r| r.hash)).collect();
        all.sort_unstable();
        all.dedup();
        let mut want: Vec<u64> = hearts.iter().map(|h| h.0).collect();
        want.sort_unstable();
        want.dedup();
        let names: BM<u32, String> = chest_slot::names_of(drops(rf)?.into_iter().map(|x| x.hash));
        outs.push(rewrite(rf, LIST, |raw| {
            let mut y: Byml = Byml::parse(raw)?;
            slots(&mut y, &all, &want, &names)?;
            y.write()
        })?);
    }
    Ok(outs)
}

pub fn build(romfs: &P, load: &P, name: &str, picks: &[(String, String)], o: &Opts) -> Res<Built> {
    let d: Data = Data::load()?;
    let mut rf: Romfs = Romfs::open(romfs)?;
    let mut by: BM<String, Vec<(u64, String, String)>> = BM::new();
    for (loc, item) in picks {
        let l = d.cat.by_name(loc).ok_or_else(|| Error::Msg(format!("{loc} is not in the catalog")))?;
        if l.kind != "bossheart" {
            return Err(Error::Msg(format!("{loc} is not a boss heart")));
        }
        if item == HEART {
            continue;
        }
        for r in &l.refs {
            for f in &r.files {
                by.entry(f.clone()).or_default().push((r.hash, item.clone(), loc.clone()));
            }
        }
    }
    let mut outs: Vec<Out> = Vec::new();
    let mut hearts: Vec<(u64, Vec<String>)> = Vec::new();
    let mut report: String = String::new();
    for (f, swaps) in &by {
        let raw: Vec<u8> = banc_raw(&mut rf, f, o.banc_from.as_ref())?;
        let mut y: Byml = Byml::parse(&raw)?;
        for (hash, item, loc) in swaps {
            hearts.push((*hash, heart_ids(&y, *hash)));
            if swap(&mut y, *hash, item, &o.chest) == 0 {
                return Err(Error::Msg(format!("{loc}: no {HEART} with hash {hash} in {f}")));
            }
            let _ = writeln!(report, "{loc}: {} now holds {item} in a {}", HEART, o.chest);
        }
        let bytes: Vec<u8> = y.write()?;
        outs.push(banc_out(&mut rf, f, raw.len(), &bytes)?);
    }
    if outs.is_empty() {
        return Err(Error::Format("nothing to change, every pick keeps its heart container"));
    }
    outs.extend(stage(&mut rf, &d, &hearts, o.slot)?);
    let c: Committed = commit(&mut rf, load, name, &outs, o.rst_from.as_ref())?;
    report.push_str(&c.report);
    Ok(Built { report, files: c.files, grown: c.grown })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(g: &str, h: u64, dynamic: Option<Node>) -> Node {
        let mut m: Vec<(String, Node)> = vec![
            ("Gyaml".to_string(), Node::Str(g.to_string())),
            ("Hash".to_string(), Node::U64(h)),
        ];
        if let Some(d) = dynamic {
            m.insert(0, ("Dynamic".to_string(), d));
        }
        Node::Map(m)
    }

    fn scene(actors: Vec<Node>) -> Byml {
        Byml { root: Node::Map(vec![("Actors".to_string(), Node::Arr(actors))]), version: 2, big: false }
    }

    #[test]
    fn a_heart_with_a_dynamic_block_becomes_a_chest() {
        let d: Node = Node::Map(vec![("DefaultDungeonRoomIndex".to_string(), Node::I32(10))]);
        let mut y: Byml = scene(vec![actor(HEART, 7, Some(d)), actor(HEART, 8, None)]);
        assert_eq!(swap(&mut y, 7, "Item_Hammer", CHEST), 1);
        let a: &Node = &y.root.get("Actors").unwrap().as_arr().unwrap()[0];
        assert_eq!(a.get("Gyaml").and_then(|v| v.as_str()), Some(CHEST));
        let d: &Node = a.get("Dynamic").unwrap();
        assert_eq!(d.get("Drop__DropActor").and_then(|v| v.as_str()), Some("Item_Hammer"));
        assert_eq!(d.get("DefaultDungeonRoomIndex"), Some(&Node::I32(10)));
        let b: &Node = &y.root.get("Actors").unwrap().as_arr().unwrap()[1];
        assert_eq!(b.get("Gyaml").and_then(|v| v.as_str()), Some(HEART));
    }

    #[test]
    fn a_heart_without_a_dynamic_block_gets_one() {
        let mut y: Byml = scene(vec![actor(HEART, 9, None)]);
        assert_eq!(swap(&mut y, 9, "Item_Longshot", "Obj_TBox_A_01"), 1);
        let a: &Node = &y.root.get("Actors").unwrap().as_arr().unwrap()[0];
        assert_eq!(a.get("Dynamic").and_then(|d| d.get("Drop__DropActor")).and_then(|v| v.as_str()), Some("Item_Longshot"));
    }

    #[test]
    fn each_heart_keeps_its_own_spare_flag_whatever_else_is_picked() {
        use crate::chest_slot::{key, member, OPEN, ROOMS, SPARE_MAP};
        let pair = |h: u32, v: u32| Node::Map(vec![("Hash".to_string(), Node::U32(h)), ("Value".to_string(), Node::U32(v))]);
        let st = |h: u32, m: Vec<Node>| Node::Map(vec![("DefaultValue".to_string(), Node::Arr(m)), ("Hash".to_string(), Node::U32(h))]);
        let flag = |h: u32| Node::Map(vec![("Hash".to_string(), Node::U32(h))]);
        let structs: Vec<Node> = vec![
            st(key(OPEN), Vec::new()),
            st(key(ROOMS), vec![pair(key(SPARE_MAP), 50)]),
            st(50, vec![pair(1, 51)]),
            st(51, vec![pair(2, 901), pair(3, 902), pair(4, 903)]),
        ];
        let data: Node = Node::Map(vec![
            ("Bool".to_string(), Node::Arr(vec![flag(901), flag(902), flag(903)])),
            ("Struct".to_string(), Node::Arr(structs)),
        ]);
        let mut y: Byml = Byml { root: Node::Map(vec![("Data".to_string(), data)]), version: 2, big: false };
        let names: BM<u32, String> = chest_slot::names_of([30u64].into_iter());
        assert_eq!(slots(&mut y, &[10, 20, 30], &[30], &names).unwrap(), 1);
        let open: &Node = &y.root.get("Data").unwrap().get("Struct").unwrap().as_arr().unwrap()[0];
        let m: &[Node] = open.get("DefaultValue").unwrap().as_arr().unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0], pair(member(30), 903));
        assert_eq!(slots(&mut y, &[10, 20, 30], &[30], &names).unwrap(), 0);
    }

    #[test]
    fn the_logic_ids_of_a_heart_come_from_the_ai_groups() {
        let r = |id: &str, h: u64| Node::Map(vec![("Id".to_string(), Node::Str(id.to_string())), ("Reference".to_string(), Node::U64(h))]);
        let g: Node = Node::Map(vec![("References".to_string(), Node::Arr(vec![r("Obj_Heart_Container_4f3d", 7), r("Area_1", 8), r("Obj_Heart_Container_4f3d", 7)]))]);
        let y: Byml = Byml { root: Node::Map(vec![("AiGroups".to_string(), Node::Arr(vec![g]))]), version: 2, big: false };
        assert_eq!(heart_ids(&y, 7), vec!["Obj_Heart_Container_4f3d".to_string()]);
        assert!(heart_ids(&y, 9).is_empty());
    }

    #[test]
    fn the_chest_node_gets_one_unlock_input() {
        use crate::ainb::tests::{node, synth};
        let mut a: Ainb = synth();
        let t: usize = node(&mut a, "ActorLogicSimpleActor", 0, &[(1, "Logic_Create", 1u32.to_le_bytes().to_vec())]);
        let u: usize = node(&mut a, "ActorLogicSimpleActor", 0, &[(1, "Logic_Create", 1u32.to_le_bytes().to_vec())]);
        let nm: u32 = a.pool.add(NODE);
        let v: u32 = a.pool.add("Obj_Heart_Container_4f3d");
        let w: u32 = a.pool.add("Other_1234");
        for (i, val) in [(t, v), (u, w)] {
            let at: u32 = (a.props[3].len() / 12) as u32;
            a.props[3].extend_from_slice(&nm.to_le_bytes());
            a.props[3].extend_from_slice(&0u32.to_le_bytes());
            a.props[3].extend_from_slice(&val.to_le_bytes());
            a.params[i].pe[3] = [at, 1];
            for j in i + 1..a.params.len() {
                a.params[j].pe[3][0] += 1;
            }
        }
        assert_eq!(unlock(&mut a, "Obj_Heart_Container_4f3d").unwrap(), 1);
        assert!(a.find_in(t, 1, UNLOCK).is_some());
        assert!(a.find_in(u, 1, UNLOCK).is_none());
        assert_eq!(unlock(&mut a, "Obj_Heart_Container_4f3d").unwrap(), 0);
        assert!(crate::ainb::audit(&a).is_empty());
    }

    #[test]
    fn only_heart_containers_are_swapped() {
        let mut y: Byml = scene(vec![actor("Obj_TBox_A_01", 5, None)]);
        assert_eq!(swap(&mut y, 5, "Item_Hammer", CHEST), 0);
    }
}
