use std::collections::HashMap as Map;

use super::extras::{files, gyaml, own_body, raw, room, tally, EXT, ROOM, SKIP_DIR};
use crate::byml::{Byml, Node};
use crate::error::Res;
use crate::placement::hash_of;
use crate::qol::Have;
use crate::rng::Rng;
use crate::romfs::Romfs;
use crate::sarc::{self, Entry};
use crate::staging::{Out, BANC_DICT};
use crate::zs;

pub const SALT: &str = "fake-pots";
pub const ONE_IN: u64 = 10;
pub const CHANCE: i32 = 6;
pub const NAMED: u32 = 126134492;
pub const POT_TABLE: &str = "DropTable_ResourceSupply_00";
pub const FOLIAGE_TABLE: &str = "DropTable_Foliage_00";
pub const TABLE_DIR: &str = "Drop/DropSubTable/";
pub const TABLE_EXT: &str = ".game__drop__DropSubTable.bgyml";
pub const FOLIAGE_PACK: &str = "Pack/FoliageDropParam.pack.zs";
pub const POT_PACKS: &str = "Obj_Pot_";
pub const GRASS_PACKS: &str = "Obj_Plant_Grass_A_01";
pub const BUG_GRASS: &str = "Land_kokiri_Plant_A_02_DropFixed_03";
pub const BUG: &str = "Animal_Insect";
pub const DREAM: &str = "_Dream";
pub const POE: &str = "Enemy_Poe";
pub const SKULLTULA: &str = "Enemy_Skullwalltula";
pub const LITTLE_HANDS: &str = "Enemy_Floormaster_Little";
pub const CHANCE_VAR: &str = "OOTRR_SURPRISE_CHANCE";
pub const POT_SURPRISES: [&str; 2] = [POE, LITTLE_HANDS];
pub const GRASS_SURPRISES: [&str; 2] = [SKULLTULA, LITTLE_HANDS];
pub const OWN: u32 = 2;
pub const SHOPS: [&str; 6] = ["Shop", "Bazaar", "Shooting", "Bowling", "Mask", "Treasure"];
pub const PLAIN_POT: &str = "Enemy_FlyingPot";
pub const POTS: [(&str, &str); 8] = [
    ("Obj_Pot_A_01", PLAIN_POT),
    ("Obj_Pot_A_02", PLAIN_POT),
    ("Obj_Pot_A_03", PLAIN_POT),
    ("Obj_Pot_A_04", "Enemy_FlyingPot_IceCavern"),
    ("Obj_Pot_A_05", "Enemy_FlyingPot_SpiritTemple"),
    ("Obj_Pot_A_06", PLAIN_POT),
    ("Obj_Pot_A_07", "Enemy_FlyingPot_ShadowTemple"),
    ("Obj_Pot_A_08", PLAIN_POT),
];
pub const POT_KEYS: [&str; 8] = ["Dynamic", "Gyaml", "Hash", "IsPhysicsStable", "Phive", "Rotate", "SRTHash", "Translate"];
pub const RESETTER: &str = "SupplyItemResetSystemActor";
pub const GROUPS: [&str; 2] = ["FarDeleteGroups", "SimultaneousGroups"];
pub const NOTE: &str = "Now and then, about one time in 8, cutting grass or breaking an ordinary pot lets out an enemy instead of the usual drop: a Poe or a small crawling hand from a pot, a Skulltula or a small hand from grass. About 1 in 10 plain pots are flying pots in disguise (never pots that hold a key, a fairy, an item or a set drop, nor any in the first room of a dungeon). Experimental, untested in the game.";

pub fn chance() -> i32 {
    std::env::var(CHANCE_VAR).ok().and_then(|v| v.parse::<i32>().ok()).filter(|v| (1..=20).contains(v)).unwrap_or(CHANCE)
}

pub fn named(actor: &str) -> Node {
    Node::Map(vec![
        ("BadLuckValueRate".to_string(), Node::F32(1.0)),
        ("DropActorName".to_string(), Node::Str(format!("Work/Actor/{actor}.engine__actor__ActorParam.gyml"))),
        ("DropCreateCount".to_string(), Node::U32(1)),
        ("DropProbability".to_string(), Node::I32(chance())),
        ("IsReplace".to_string(), Node::Bool(true)),
        ("MostDepletedResource".to_string(), Node::Arr(Vec::new())),
        ("Type".to_string(), Node::U32(NAMED)),
    ])
}

fn odds(n: &Node) -> i32 {
    match n.get("DropProbability") {
        Some(Node::I32(v)) => *v,
        _ => 0,
    }
}

pub fn surprise(raw: &[u8], actor: &str) -> Res<Option<Vec<u8>>> {
    let mut y: Byml = Byml::parse(raw)?;
    let Some(items) = y.root.get_mut("Items").and_then(|x| x.as_arr_mut()) else {
        return Ok(None);
    };
    let want: String = format!("Work/Actor/{actor}.engine__actor__ActorParam.gyml");
    if items.iter().any(|i| i.get("DropActorName").and_then(|x| x.as_str()) == Some(want.as_str())) {
        return Ok(None);
    }
    let c: i32 = chance();
    let total: i32 = items.iter().map(odds).sum();
    if total >= 100 {
        let mut best: Option<usize> = None;
        for (i, it) in items.iter().enumerate() {
            if odds(it) >= c.max(2) && best.is_none_or(|b| odds(it) > odds(&items[b])) {
                best = Some(i);
            }
        }
        if let Some(b) = best {
            let v: i32 = odds(&items[b]) - c;
            items[b].set("DropProbability", Node::I32(v));
        }
    }
    items.insert(0, named(actor));
    Ok(Some(y.write()?))
}

pub fn target(path: &str) -> Option<(&'static str, &'static [&'static str])> {
    if path == FOLIAGE_PACK {
        return Some((FOLIAGE_TABLE, &GRASS_SURPRISES));
    }
    let name: &str = path.strip_prefix("Pack/Actor/")?.strip_suffix(".pack.zs")?;
    if name.contains(DREAM) {
        return None;
    }
    if name == BUG_GRASS {
        Some((BUG_GRASS, &GRASS_SURPRISES))
    } else if name.starts_with(POT_PACKS) {
        Some((POT_TABLE, &POT_SURPRISES))
    } else if name.starts_with(GRASS_PACKS) {
        Some((POT_TABLE, &GRASS_SURPRISES))
    } else {
        None
    }
}

pub fn edit_pack(es: &mut [Entry], table: &str, actors: &[&str]) -> Res<bool> {
    let file: String = format!("{TABLE_DIR}{table}{TABLE_EXT}");
    let Some(e) = es.iter_mut().find(|e| e.name == file) else {
        return Ok(false);
    };
    let mut changed: bool = false;
    for actor in actors {
        if let Some(d) = surprise(&e.data, actor)? {
            e.data = d;
            changed = true;
        }
    }
    Ok(changed)
}

pub fn drop_packs(rf: &Romfs) -> Res<Vec<String>> {
    let mut out: Vec<String> = rf.list("Pack/Actor")?.into_iter().filter(|p| target(p).is_some()).collect();
    out.push(FOLIAGE_PACK.to_string());
    out.sort();
    Ok(out)
}

pub fn drops(rf: &mut Romfs, have: Have) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    for f in drop_packs(rf)? {
        let Some((table, actors)) = target(&f) else {
            continue;
        };
        let z: Vec<u8> = match have(&f) {
            Some(b) => b,
            None => rf.read(&f)?,
        };
        let id: Option<u32> = zs::dict_of(&z);
        let b: Vec<u8> = rf.zs.decompress(&z)?;
        let mut es: Vec<Entry> = sarc::parse(&b)?;
        if edit_pack(&mut es, table, actors)? {
            let out: Vec<u8> = sarc::write(&es)?;
            outs.push(Out { path: f, old: b.len(), new: out.len(), z: zs::compress_raw(&out, id)? });
        }
    }
    Ok(outs)
}

pub fn flying(g: &str) -> Option<&'static str> {
    POTS.iter().find(|p| p.0 == g).map(|p| p.1)
}

fn only_room(a: &Node) -> bool {
    match a.get("Dynamic") {
        None => true,
        Some(Node::Map(m)) => m.iter().all(|e| e.0 == ROOM),
        Some(_) => false,
    }
}

pub fn dst(l: &Node) -> Option<u64> {
    match l.get("Dst") {
        Some(Node::U64(x)) => Some(*x),
        Some(Node::I64(x)) => Some(*x as u64),
        _ => None,
    }
}

pub fn groups(root: &Node) -> Map<u64, u32> {
    let mut out: Map<u64, u32> = Map::new();
    for k in GROUPS {
        if let Some(n) = root.get(k) {
            tally(n, &mut out);
        }
    }
    let list: &[Node] = root.get("Actors").and_then(|l| l.as_arr()).unwrap_or(&[]);
    for a in list.iter().filter(|a| gyaml(a) == RESETTER) {
        for h in a.get("Links").and_then(|l| l.as_arr()).unwrap_or(&[]).iter().filter_map(dst) {
            *out.entry(h).or_insert(0) += 1;
        }
    }
    out
}

pub fn fake_ok(a: &Node, seen: &Map<u64, u32>, grouped: &Map<u64, u32>) -> bool {
    let Some(h) = hash_of(a) else {
        return false;
    };
    let Node::Map(m) = a else {
        return false;
    };
    flying(gyaml(a)).is_some()
        && m.iter().all(|e| POT_KEYS.contains(&e.0.as_str()))
        && only_room(a)
        && own_body(a, h)
        && seen.get(&h).copied().unwrap_or(0).saturating_sub(grouped.get(&h).copied().unwrap_or(0)) == OWN
}

pub fn first_room(list: &[Node]) -> Option<i32> {
    list.iter().filter_map(room).min()
}

pub fn fake(y: &mut Byml, rng: &mut Rng, one_in: u64) -> usize {
    let mut seen: Map<u64, u32> = Map::new();
    tally(&y.root, &mut seen);
    let grouped: Map<u64, u32> = groups(&y.root);
    let Some(list) = y.root.get_mut("Actors").and_then(|l| l.as_arr_mut()) else {
        return 0;
    };
    let first: Option<i32> = first_room(list);
    let mut gone: Vec<u64> = Vec::new();
    for a in list.iter_mut() {
        if !fake_ok(a, &seen, &grouped) || (first.is_some() && room(a) == first) {
            continue;
        }
        if rng.below(one_in) != 0 {
            continue;
        }
        let Some(to) = flying(gyaml(a)) else {
            continue;
        };
        a.set("Gyaml", Node::Str(to.to_string()));
        a.remove("IsPhysicsStable");
        gone.extend(hash_of(a));
    }
    for a in list.iter_mut().filter(|a| gyaml(a) == RESETTER) {
        if let Some(links) = a.get_mut("Links").and_then(|l| l.as_arr_mut()) {
            links.retain(|l| !dst(l).is_some_and(|h| gone.contains(&h)));
        }
    }
    gone.len()
}

pub fn unfix(y: &mut Byml) -> usize {
    let Some(list) = y.root.get_mut("Actors").and_then(|l| l.as_arr_mut()) else {
        return 0;
    };
    let mut n: usize = 0;
    for a in list.iter_mut().filter(|a| gyaml(a) == BUG_GRASS) {
        let Some(Node::Map(d)) = a.get_mut("Dynamic") else {
            continue;
        };
        let before: usize = d.len();
        d.retain(|e| !(e.0 == "Drop__DropActor" && e.1.as_str() == Some(BUG)));
        if d.len() < before {
            n += 1;
        }
        if d.is_empty() {
            a.remove("Dynamic");
        }
    }
    n
}

pub fn skipped(f: &str) -> bool {
    f.starts_with(SKIP_DIR) || !f.ends_with(EXT) || SHOPS.iter().any(|s| f.contains(s))
}

pub fn pots(rf: &mut Romfs, have: Have, seed: &str) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    for f in files(rf)? {
        if skipped(&f) {
            continue;
        }
        let (b, id): (Vec<u8>, Option<u32>) = raw(rf, have, &f)?;
        let mut y: Byml = Byml::parse(&b)?;
        let mut rng: Rng = Rng::from_text(&format!("{seed}|{SALT}|{f}"));
        if fake(&mut y, &mut rng, ONE_IN) + unfix(&mut y) == 0 {
            continue;
        }
        let new: Vec<u8> = y.write()?;
        outs.push(Out { path: f, old: b.len(), new: new.len(), z: zs::compress_raw(&new, Some(id.unwrap_or(BANC_DICT)))? });
    }
    Ok(outs)
}

pub fn stage(rf: &mut Romfs, have: Have, seed: &str) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = drops(rf, have)?;
    outs.extend(pots(rf, have, seed)?);
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(actor: &str, t: u32, p: i32) -> Node {
        let a: String = if actor.is_empty() { String::new() } else { format!("Work/Actor/{actor}.engine__actor__ActorParam.gyml") };
        Node::Map(vec![
            ("BadLuckValueRate".to_string(), Node::F32(1.0)),
            ("DropActorName".to_string(), Node::Str(a)),
            ("DropCreateCount".to_string(), Node::U32(1)),
            ("DropProbability".to_string(), Node::I32(p)),
            ("IsReplace".to_string(), Node::Bool(true)),
            ("MostDepletedResource".to_string(), Node::Arr(Vec::new())),
            ("Type".to_string(), Node::U32(t)),
        ])
    }

    fn table(items: Vec<Node>) -> Vec<u8> {
        Byml { root: Node::Map(vec![("Items".to_string(), Node::Arr(items))]), version: 7, big: false }.write().unwrap()
    }

    fn read(raw: &[u8]) -> Vec<(String, i32)> {
        let y: Byml = Byml::parse(raw).unwrap();
        y.root.get("Items").unwrap().as_arr().unwrap().iter().map(|i| (i.get("DropActorName").and_then(|x| x.as_str()).unwrap_or("").to_string(), odds(i))).collect()
    }

    const HEART: u32 = 1872319531;
    const MAGIC: u32 = 832898412;
    const SHOTS: u32 = 1113099761;

    fn pot_table() -> Vec<u8> {
        table(vec![item("", HEART, 20), item("", MAGIC, 20), item("", SHOTS, 20), item("", 2211636230, 20), item("", 3237216321, 20), item("", 4192551701, 0)])
    }

    fn both(raw: &[u8]) -> Vec<u8> {
        let mut out: Vec<u8> = raw.to_vec();
        for a in POT_SURPRISES {
            if let Some(d) = surprise(&out, a).unwrap() {
                out = d;
            }
        }
        out
    }

    #[test]
    fn two_surprises_take_one_point_each_and_the_total_stays_one_hundred() {
        let out: Vec<u8> = both(&pot_table());
        let rows: Vec<(String, i32)> = read(&out);
        assert_eq!(rows.len(), 8);
        assert_eq!(rows.iter().map(|r| r.1).sum::<i32>(), 100);
        assert_eq!(rows.iter().filter(|r| r.1 == CHANCE).count(), 2);
        assert_eq!(both(&out), out, "a second pass changes nothing");
    }

    #[test]
    fn a_full_table_gives_one_point_to_the_enemy_and_stays_at_one_hundred() {
        let raw: Vec<u8> = pot_table();
        let out: Vec<u8> = surprise(&raw, POE).unwrap().unwrap();
        let rows: Vec<(String, i32)> = read(&out);
        assert_eq!(rows.len(), 7);
        assert_eq!(rows[0], ("Work/Actor/Enemy_Poe.engine__actor__ActorParam.gyml".to_string(), CHANCE));
        assert_eq!(rows.iter().map(|r| r.1).sum::<i32>(), 100);
        assert_eq!(rows[1].1, 20 - CHANCE);
        assert!(surprise(&out, POE).unwrap().is_none(), "a second pass changes nothing");
        let y: Byml = Byml::parse(&out).unwrap();
        let first: &Node = &y.root.get("Items").unwrap().as_arr().unwrap()[0];
        assert_eq!(first.get("Type"), Some(&Node::U32(NAMED)));
        assert_eq!(first, &named(POE));
    }

    #[test]
    fn a_table_short_of_one_hundred_only_gains_the_entry() {
        let raw: Vec<u8> = table(vec![item("", HEART, 0), item("", MAGIC, 20), item("", SHOTS, 0)]);
        let rows: Vec<(String, i32)> = read(&surprise(&raw, SKULLTULA).unwrap().unwrap());
        assert_eq!(rows.iter().map(|r| r.1).collect::<Vec<i32>>(), vec![CHANCE, 0, 20, 0]);
    }

    #[test]
    fn it_composes_with_the_heart_and_ammo_cuts_in_either_order() {
        let cut: super::super::drops::Cut = super::super::drops::Cut { hearts: true, ammo: true };
        let raw: Vec<u8> = pot_table();
        let a: Vec<u8> = both(&super::super::drops::edit(&raw, cut).unwrap().unwrap());
        let b: Vec<u8> = super::super::drops::edit(&both(&raw), cut).unwrap().unwrap();
        for out in [&a, &b] {
            let rows: Vec<(String, i32)> = read(out);
            assert_eq!((rows[0].1, rows[1].1), (CHANCE, CHANCE));
            assert!(rows[0].0.contains(LITTLE_HANDS) && rows[1].0.contains("Enemy_Poe"));
            assert!(surprise(out, POE).unwrap().is_none() && surprise(out, LITTLE_HANDS).unwrap().is_none());
            assert_eq!(rows.iter().filter(|r| r.0.is_empty() && r.1 > 0).count(), 1, "only magic is left");
        }
    }

    #[test]
    fn each_source_names_its_own_enemies() {
        assert_eq!(target("Pack/Actor/Obj_Pot_A_01.pack.zs"), Some((POT_TABLE, &POT_SURPRISES[..])));
        assert_eq!(target("Pack/Actor/Obj_Pot_A_08_Respawn.pack.zs"), Some((POT_TABLE, &POT_SURPRISES[..])));
        assert_eq!(target("Pack/Actor/Obj_Plant_Grass_A_01.pack.zs"), Some((POT_TABLE, &GRASS_SURPRISES[..])));
        assert_eq!(target(FOLIAGE_PACK), Some((FOLIAGE_TABLE, &GRASS_SURPRISES[..])));
        assert_eq!(target("Pack/Actor/Land_kokiri_Plant_A_02_DropFixed_03.pack.zs"), Some((BUG_GRASS, &GRASS_SURPRISES[..])));
        for p in ["Pack/Actor/Obj_Pot_A_03_Dream.pack.zs", "Pack/Actor/Obj_BrokenBox_A_01.pack.zs", "Pack/Actor/Obj_Plant_Grass_A_03.pack.zs", "Pack/Actor/Enemy_Poe.pack.zs", "Pack/Actor/Obj_Pot_A_01.txt"] {
            assert_eq!(target(p), None, "{p}");
        }
    }

    #[test]
    fn only_the_named_table_of_a_pack_is_edited() {
        let mut es: Vec<Entry> = vec![
            Entry { name: format!("{TABLE_DIR}{POT_TABLE}{TABLE_EXT}"), data: pot_table() },
            Entry { name: format!("{TABLE_DIR}DropTable_Obj_Heart{TABLE_EXT}"), data: table(vec![item("", HEART, 100)]) },
        ];
        let heart: Vec<u8> = es[1].data.clone();
        assert!(edit_pack(&mut es, POT_TABLE, &POT_SURPRISES).unwrap());
        assert_eq!(es[1].data, heart);
        assert!(!edit_pack(&mut es, POT_TABLE, &POT_SURPRISES).unwrap());
        assert!(!edit_pack(&mut es, FOLIAGE_TABLE, &POT_SURPRISES).unwrap());
    }

    fn actor(g: &str, h: u64, dynamic: Option<Node>, extra: Vec<(&str, Node)>) -> Node {
        let mut m: Vec<(String, Node)> = Vec::new();
        if let Some(d) = dynamic {
            m.push(("Dynamic".to_string(), d));
        }
        m.push(("Gyaml".to_string(), Node::Str(g.to_string())));
        m.push(("Hash".to_string(), Node::U64(h)));
        m.push(("IsPhysicsStable".to_string(), Node::Bool(true)));
        for (k, v) in extra {
            m.push((k.to_string(), v));
        }
        m.push(("Phive".to_string(), Node::Map(vec![("Placement".to_string(), Node::Map(vec![("ID".to_string(), Node::U64(h))]))])));
        m.push(("SRTHash".to_string(), Node::U32(h as u32)));
        m.push(("Translate".to_string(), Node::Arr(vec![Node::F32(1.0), Node::F32(2.0), Node::F32(3.0)])));
        Node::Map(m)
    }

    fn rooms(r: i32) -> Node {
        Node::Map(vec![(ROOM.to_string(), Node::I32(r))])
    }

    fn scene(actors: Vec<Node>, groups: Vec<u64>) -> Byml {
        let g: Node = Node::Arr(vec![Node::Map(vec![("References".to_string(), Node::Arr(groups.into_iter().map(|h| Node::Map(vec![("Reference".to_string(), Node::U64(h))])).collect()))])]);
        Byml { root: Node::Map(vec![("Actors".to_string(), Node::Arr(actors)), ("FarDeleteGroups".to_string(), g)]), version: 7, big: false }
    }

    fn kinds(y: &Byml) -> Vec<String> {
        y.root.get("Actors").unwrap().as_arr().unwrap().iter().map(|a| gyaml(a).to_string()).collect()
    }

    const A: u64 = 0x1_0000_0001;
    const B: u64 = 0x1_0000_0002;
    const C: u64 = 0x1_0000_0003;
    const D: u64 = 0x1_0000_0004;
    const E: u64 = 0x1_0000_0005;
    const F: u64 = 0x1_0000_0006;
    const G: u64 = 0x1_0000_0007;
    const R: u64 = 0x1_0000_0008;

    fn link(d: u64) -> Node {
        Node::Map(vec![("Dst".to_string(), Node::U64(d)), ("Gyaml".to_string(), Node::Str("Reference".to_string())), ("Name".to_string(), Node::Str("Reference".to_string())), ("Src".to_string(), Node::U64(R))])
    }

    fn sample() -> Byml {
        let table: Node = Node::Map(vec![("Drop__DropTable".to_string(), Node::Str("DropTable_Obj_Heart".to_string()))]);
        let item: Node = Node::Map(vec![("Drop__DropActor".to_string(), Node::Str("Item_Dungeon_SmallKey".to_string()))]);
        let reset: Node = Node::Map(vec![("Gyaml".to_string(), Node::Str(RESETTER.to_string())), ("Hash".to_string(), Node::U64(R)), ("Links".to_string(), Node::Arr(vec![link(G), link(F)])), ("Phive".to_string(), Node::Map(vec![("Placement".to_string(), Node::Map(vec![("ID".to_string(), Node::U64(R))]))])), ("SRTHash".to_string(), Node::U32(1))]);
        scene(
            vec![
                actor("Obj_Pot_A_01", A, None, vec![]),
                actor("Obj_Pot_A_04", B, Some(rooms(3)), vec![]),
                actor("Obj_Pot_A_05", C, Some(rooms(0)), vec![]),
                actor("Obj_Pot_A_01", D, Some(item), vec![]),
                actor("Obj_Pot_A_01", E, Some(table), vec![]),
                actor("Obj_Pot_A_08", F, None, vec![("Links", Node::Arr(vec![link(A)]))]),
                actor("Obj_Pot_A_07", G, Some(rooms(2)), vec![]),
                actor("Obj_Gerudo_HorsebackArcheryPot_A_01", 0x1_0000_0009, None, vec![]),
                reset,
            ],
            vec![B, G],
        )
    }

    #[test]
    fn only_plain_pots_are_candidates() {
        let y: Byml = sample();
        let mut seen: Map<u64, u32> = Map::new();
        tally(&y.root, &mut seen);
        let g: Map<u64, u32> = groups(&y.root);
        let list: &[Node] = y.root.get("Actors").unwrap().as_arr().unwrap();
        let ok: Vec<bool> = list.iter().map(|a| fake_ok(a, &seen, &g)).collect();
        assert_eq!(ok, vec![false, true, true, false, false, false, true, false, false]);
        assert_eq!(first_room(list), Some(0));
    }

    #[test]
    fn plain_pots_outside_the_first_room_become_their_matching_flying_pot() {
        let mut y: Byml = sample();
        let mut rng: Rng = Rng::from_text("seed");
        assert_eq!(fake(&mut y, &mut rng, 1), 2);
        assert_eq!(kinds(&y), vec!["Obj_Pot_A_01", "Enemy_FlyingPot_IceCavern", "Obj_Pot_A_05", "Obj_Pot_A_01", "Obj_Pot_A_01", "Obj_Pot_A_08", "Enemy_FlyingPot_ShadowTemple", "Obj_Gerudo_HorsebackArcheryPot_A_01", RESETTER]);
        let list: &[Node] = y.root.get("Actors").unwrap().as_arr().unwrap();
        assert!(list[1].get("IsPhysicsStable").is_none() && list[6].get("IsPhysicsStable").is_none());
        assert_eq!(list[3].get("IsPhysicsStable"), Some(&Node::Bool(true)));
        let links: Vec<u64> = list[8].get("Links").unwrap().as_arr().unwrap().iter().filter_map(dst).collect();
        assert_eq!(links, vec![F], "the reset actor forgets the pot that left");
        assert_eq!(Byml::parse(&y.write().unwrap()).unwrap(), y);
    }

    #[test]
    fn a_pot_with_a_link_of_its_own_or_one_pointing_at_it_is_not_fake() {
        let mut rng: Rng = Rng::from_text("x");
        let mut z: Byml = scene(vec![actor("Obj_Pot_A_03", A, None, vec![]), actor("Obj_Pot_A_03", B, None, vec![("Links", Node::Arr(vec![link(0x9_0000_0000)]))]), actor("Obj_Pot_A_03", C, None, vec![]), actor("Obj_Pot_A_03", D, None, vec![("Links", Node::Arr(vec![link(C)]))])], vec![]);
        assert_eq!(fake(&mut z, &mut rng, 1), 1);
        assert_eq!(kinds(&z), vec![PLAIN_POT, "Obj_Pot_A_03", "Obj_Pot_A_03", "Obj_Pot_A_03"]);
    }

    #[test]
    fn the_dice_are_seeded_and_about_one_in_ten() {
        let make = || -> Byml {
            let acts: Vec<Node> = (0..400u64).map(|i| actor("Obj_Pot_A_01", 0x2_0000_0000 + i, None, vec![])).collect();
            scene(acts, Vec::new())
        };
        let run = |s: &str| -> Vec<String> {
            let mut y: Byml = make();
            let mut rng: Rng = Rng::from_text(s);
            fake(&mut y, &mut rng, ONE_IN);
            kinds(&y)
        };
        let a: Vec<String> = run("one");
        assert_eq!(a, run("one"));
        assert_ne!(a, run("two"));
        let n: usize = a.iter().filter(|k| *k == PLAIN_POT).count();
        assert!((20..=60).contains(&n), "{n} of 400");
        assert_eq!(ONE_IN, 10);
    }

    #[test]
    fn shops_minigames_and_the_dream_world_are_skipped() {
        assert!(skipped("Banc/DreamWorld/X_Static.bcett.byml.zs"));
        assert!(skipped("Banc/Market/Market_Child_Static.bcett.byml.zs.bak"));
        assert!(skipped("Banc/ShootingGallery/ShootingGallery_Static.bcett.byml.zs"));
        assert!(!skipped("Banc/HyruleField/HyruleField_Child_Dynamic.bcett.byml.zs"));
    }

    #[test]
    fn the_bug_grass_loses_its_fixed_insect_key_and_only_that() {
        let bugs: Node = Node::Map(vec![("Drop__DropActor".to_string(), Node::Str(BUG.to_string()))]);
        let kept: Node = Node::Map(vec![("Drop__DropActor".to_string(), Node::Str("Obj_Rupee_Green".to_string()))]);
        let mut y: Byml = scene(vec![actor(BUG_GRASS, 1, Some(bugs.clone()), vec![]), actor(BUG_GRASS, 2, Some(kept.clone()), vec![]), actor("Obj_Pot_A_01", 3, Some(bugs), vec![])], vec![]);
        assert_eq!(unfix(&mut y), 1);
        let list: &[Node] = y.root.get("Actors").unwrap().as_arr().unwrap();
        assert!(list[0].get("Dynamic").is_none());
        assert_eq!(list[1].get("Dynamic"), Some(&kept));
        assert!(list[2].get("Dynamic").is_some());
        assert_eq!(unfix(&mut y), 0);
    }

    #[test]
    fn the_surprise_chance_is_six_percent_by_default() {
        assert_eq!(CHANCE, 6);
    }

    fn game() -> Option<Romfs> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        Romfs::open(&p).ok()
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn the_game_tables_gain_the_enemy_and_the_maps_gain_flying_pots() {
        let Some(mut rf) = game() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let outs: Vec<Out> = drops(&mut rf, &none).unwrap();
        let packs: Vec<String> = drop_packs(&rf).unwrap();
        assert!(packs.iter().filter(|p| p.contains("Obj_Pot_")).count() >= 10 && packs.contains(&FOLIAGE_PACK.to_string()));
        assert_eq!(outs.len(), packs.len(), "every listed pack carries its table");
        for x in &outs {
            assert!(x.new > x.old, "{} grew", x.path);
            let es: Vec<Entry> = sarc::parse(&rf.zs.decompress(&x.z).unwrap()).unwrap();
            let (table, actors): (&str, &[&str]) = target(&x.path).unwrap();
            let t: &Entry = sarc::get(&es, &format!("{TABLE_DIR}{table}{TABLE_EXT}")).unwrap();
            let rows: Vec<(String, i32)> = read(&t.data);
            assert!(actors.iter().all(|a| rows.iter().any(|r| r.0.contains(a) && r.1 == CHANCE)));
            assert_eq!(rows.iter().map(|r| r.1).sum::<i32>(), 100, "{}", x.path);
            let old: Vec<Entry> = sarc::parse(&rf.read_zs(&x.path).unwrap()).unwrap();
            let same: usize = es.iter().filter(|e| sarc::get(&old, &e.name).is_some_and(|o| o.data == e.data)).count();
            assert_eq!(same, es.len() - 1, "{} changes one file", x.path);
        }
        let maps: Vec<Out> = pots(&mut rf, &none, "seed").unwrap();
        assert!(maps.len() >= 8, "{} maps carry fake pots", maps.len());
        let again: Vec<Out> = pots(&mut rf, &none, "seed").unwrap();
        assert_eq!(maps.iter().map(|m| (m.path.clone(), m.z.clone())).collect::<Vec<_>>(), again.iter().map(|m| (m.path.clone(), m.z.clone())).collect::<Vec<_>>());
        let mut total: usize = 0;
        for m in &maps {
            let y: Byml = Byml::parse(&rf.zs.decompress(&m.z).unwrap()).unwrap();
            let was: Byml = Byml::parse(&rf.zs.decompress(&rf.read(&m.path).unwrap()).unwrap()).unwrap();
            let (a, b): (&[Node], &[Node]) = (y.root.get("Actors").unwrap().as_arr().unwrap(), was.root.get("Actors").unwrap().as_arr().unwrap());
            assert_eq!(a.len(), b.len());
            for (n, o) in a.iter().zip(b) {
                if gyaml(n) != gyaml(o) {
                    total += 1;
                    assert_eq!(flying(gyaml(o)), Some(gyaml(n)));
                    assert_eq!(hash_of(n), hash_of(o));
                    assert!(o.get("Links").is_none());
                }
            }
            assert!(!m.path.contains("Shop") && !m.path.starts_with(SKIP_DIR));
        }
        eprintln!("fake pots {total} in {} maps", maps.len());
        assert!(total >= 3, "{total} fake pots in all maps");
    }
}
