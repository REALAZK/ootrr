use std::collections::BTreeMap;
use std::collections::HashMap as Map;
use std::collections::HashSet as Set;

use crate::byml::{Byml, Node};
use crate::difficulty::extras::{files, gyaml, own_body, pos, raw, room, tally, tied, EXT, KEYS, OWN, ROOM, SKIP_DIR};
use crate::difficulty::horde::{self, lift_of, Class, Kind};
use crate::error::Res;
use crate::placement::hash_of;
use crate::qol::Have;
use crate::rng::Rng;
use crate::romfs::Romfs;
use crate::staging::{Out, BANC_DICT};
use crate::zs;

pub const SALT: &str = "enemy-randomizer";
pub const SCENES: &str = "Banc/";
pub const CHILD_SUFFIX: &str = "_Child";
pub const CHILD_MAPS: &[&str] = &["InsideTheDekuTree", "DodongoCavern", "InsideJabuJabuBelly", "BottomOfTheWell", "SpiritTemple"];
pub const HARD: &[&str] = &["Enemy_Amos", "Enemy_Wallmaster", "Enemy_Octorok", "Enemy_Freezard", "Enemy_Beamos", "Enemy_ShellBlade", "Enemy_Spike", "Enemy_Anubis"];
pub const KEEP: &[(&str, &str)] = &[
    ("DodongoCavern", "Enemy_Lizalfos"),
    ("SacredForestMeadow", "Enemy_Wolfos"),
    ("ForestTemple", "Enemy_Stalfos"),
    ("ShadowTemple", "Enemy_Stalfos"),
    ("WaterTemple", "Enemy_ShellBlade"),
    ("WaterTemple", "Enemy_Spike"),
    ("SpiritTemple", "Enemy_IronKnuckle"),
    ("LostWood", "Enemy_SkullKid"),
];
pub const FIXED: &[&str] = &["Enemy_Skullwalltula", "Enemy_Tektite_Blue", "Enemy_Amos"];
pub const EXTRA_KEYS: &[&str] = &["Presence", "IsPhysicsStable"];
pub const COUNTER: &str = "Obj_EnemyCounter";
pub const COUNTED_REACH: f32 = 30.0;
pub const BOSS: &str = "Enemy_DungeonBoss";
pub const MINIBOSS: &str = "Enemy_MiddleBoss";
pub const BOSS_REACH: f32 = 40.0;
pub const MINIBOSS_REACH: f32 = 25.0;
pub const SHARE: u64 = 80;
pub const CELL: f32 = 40.0;
pub const SPREAD: u32 = 12;
pub const GUAY: &str = "Enemy_Guay_Fixed";
pub const BABY: &str = "Enemy_BabyDodongo";
pub const SOURCES: &[&str] = &["Enemy_Dekubaba"];
pub const PLACE_KEYS:&[&str] = &["BottomOfTheWell", "CustomPerceptionRange", "SpJumpOut", "ApperMinHeightLimit"];
pub const NOTE: &str = "Enemy randomizer: about four in five ordinary enemies that no door, counter, group, link or event is tied to become another kind from anywhere in the game that moves on the same kind of ground (walkers on flat ground, walkers on slopes, hoppers, fliers, ceiling enemies; Octoroks and Leevers keep their kind), spread so a room gets a mix. Fire Keese only appear where the game already has them, ceiling enemies only indoors and Guays only outdoors. Child areas and rooms with an enemy counter only get kinds the Kokiri Sword alone can beat (a Deku Stick breaks after one hit, so sticks are not counted on). Wall Skulltulas, blue Tektites and Armos statues keep their kind. Bosses, minibosses, Gold Skulltulas, boss and miniboss rooms, Dodongo's Cavern's Dodongos and the enemies the game uses to open a way are left alone. The same seed gives the same enemies. Experimental, untested in the game.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Beat {
    Blade,
    More,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Swap {
    pub map: String,
    pub file: String,
    pub hash: u64,
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, Default)]
pub struct Guard {
    pub dungeon: bool,
    pub child: bool,
    pub banned: Set<i32>,
    pub counted: Set<i32>,
    pub counters: Vec<[f32; 3]>,
    pub bosses: Vec<([f32; 3], f32)>,
    pub blastable: bool,
    pub rising: bool,
    pub local: Set<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Pool {
    pub here: BTreeMap<&'static str, Vec<(String, Node)>>,
    pub all: BTreeMap<&'static str, Vec<(String, Node)>>,
}

impl Pool {
    pub fn shape(&self, g: &str) -> Option<&Vec<(String, Node)>> {
        self.here.get(g).or_else(|| self.all.get(g))
    }

    pub fn kinds(&self) -> Vec<&'static str> {
        let mut v: Vec<&'static str> = self.all.keys().chain(self.here.keys()).copied().collect();
        v.sort();
        v.dedup();
        v
    }
}

pub fn beat(g: &str) -> Beat {
    if HARD.contains(&g) { Beat::More } else { Beat::Blade }
}

pub fn class(g: &str) -> Option<&'static Kind> {
    horde::kind(g).filter(|k| k.class != Class::Mini && !FIXED.contains(&k.gyaml))
}

pub fn child(map: &str) -> bool {
    map.ends_with(CHILD_SUFFIX) || CHILD_MAPS.contains(&map)
}

pub fn kept(map: &str, g: &str) -> bool {
    let blaster: bool = horde::NO_BLASTERS.iter().any(|m| map.starts_with(m)) && horde::BLASTERS.contains(&g);
    blaster || KEEP.iter().any(|(m, k)| map.starts_with(m) && g.starts_with(k))
}

pub fn free(a: &Node, seen: &Map<u64, u32>) -> bool {
    let Some(h) = hash_of(a) else {
        return false;
    };
    let Node::Map(m) = a else {
        return false;
    };
    m.iter().all(|e| KEYS.contains(&e.0.as_str()) || EXTRA_KEYS.contains(&e.0.as_str())) && own_body(a, h) && !tied(a) && seen.get(&h) == Some(&OWN) && pos(a).is_some()
}

pub fn shape(a: &Node) -> Option<Vec<(String, Node)>> {
    let Node::Map(m) = a else {
        return None;
    };
    if !m.iter().all(|e| horde::TEMPLATE_KEYS.contains(&e.0.as_str())) {
        return None;
    }
    match a.get("Dynamic") {
        None => Some(Vec::new()),
        Some(Node::Map(d)) => Some(d.iter().filter(|e| !horde::DROP_KEYS.iter().any(|x| e.0.contains(x))).cloned().collect()),
        Some(_) => None,
    }
}

pub fn donors(acts: &[Node], seen: &Map<u64, u32>) -> BTreeMap<&'static str, Vec<(String, Node)>> {
    let mut best: BTreeMap<&'static str, ((u8, usize), Vec<(String, Node)>)> = BTreeMap::new();
    for a in acts {
        let Some(k) = class(gyaml(a)) else {
            continue;
        };
        let Some(s) = shape(a) else {
            continue;
        };
        let rank: (u8, usize) = (if free(a, seen) { 0 } else { 1 }, s.len());
        if best.get(k.gyaml).is_none_or(|b| rank < b.0) {
            best.insert(k.gyaml, (rank, s));
        }
    }
    best.into_iter().map(|(k, v)| (k, v.1)).collect()
}

pub fn fits(map: &str, o: &Kind, g: &Guard) -> bool {
    let home: bool = g.local.contains(o.gyaml);
    let blaster: bool = horde::BLASTERS.contains(&o.gyaml) && g.blastable && (!home || horde::NO_BLASTERS.iter().any(|m| map.starts_with(m)));
    let away: bool = (horde::HOME_ONLY.contains(&o.gyaml) && !home) || (o.gyaml == GUAY && g.dungeon) || (o.class == Class::Ceiling && !g.dungeon);
    !blaster && !away && !(g.rising && o.class != Class::Fly && !home)
}

pub fn loose(pool: BTreeMap<&'static str, Vec<(String, Node)>>) -> BTreeMap<&'static str, Vec<(String, Node)>> {
    pool.into_iter().map(|(k, s)| (k, s.into_iter().filter(|e| !PLACE_KEYS.iter().any(|x| e.0.contains(x))).collect())).collect()
}

pub fn options(map: &str, k: &Kind, pool: &Pool, g: &Guard, blade: bool) -> Vec<&'static Kind> {
    pool.kinds().into_iter().filter_map(class).filter(|o| o.class == k.class && o.gyaml != k.gyaml && !kept(map, o.gyaml) && fits(map, o, g) && (!horde::percher(o.gyaml) || horde::percher(k.gyaml)) && (!blade || beat(o.gyaml) == Beat::Blade)).collect()
}

pub fn spot(a: &Node, dungeon: bool) -> i64 {
    match (room(a), pos(a)) {
        (Some(r), _) if dungeon => r as i64,
        (_, Some(p)) => horde::OUTDOOR + (((p[0] / CELL).floor() as i64) << 16) + (p[2] / CELL).floor() as i64,
        _ => -1,
    }
}

pub fn mix(acts: &[Node], dungeon: bool) -> Map<i64, Map<String, u32>> {
    let mut m: Map<i64, Map<String, u32>> = Map::new();
    for a in acts.iter().filter(|a| horde::fighting(gyaml(a))) {
        *m.entry(spot(a, dungeon)).or_default().entry(gyaml(a).to_string()).or_insert(0) += 1;
    }
    m
}

pub fn spread(opts: &[&'static Kind], here: Option<&Map<String, u32>>, rng: &mut Rng) -> &'static Kind {
    let w: Vec<u32> = opts.iter().map(|o| (SPREAD / (1 + 2 * here.and_then(|h| h.get(o.gyaml)).copied().unwrap_or(0))).max(1)).collect();
    let mut t: u64 = rng.below(w.iter().map(|x| *x as u64).sum());
    for (o, x) in opts.iter().zip(w.iter()) {
        if t < *x as u64 {
            return o;
        }
        t -= *x as u64;
    }
    opts[opts.len() - 1]
}

pub fn swapped(a: &Node, from: &Kind, to: &Kind, shape: &[(String, Node)], dungeon: bool) -> Node {
    let mut n: Node = a.clone();
    n.set("Gyaml", Node::Str(to.gyaml.to_string()));
    let mut d: Vec<(String, Node)> = shape.iter().filter(|e| e.0 != ROOM).cloned().collect();
    if let Some(r) = room(a) {
        d.push((ROOM.to_string(), Node::I32(r)));
        d.sort_by(|x, y| x.0.cmp(&y.0));
    }
    if d.is_empty() {
        n.remove("Dynamic");
    } else {
        n.set("Dynamic", Node::Map(d));
    }
    let up: f32 = lift_of(to, dungeon) - lift_of(from, dungeon);
    if let Some(p) = pos(a)
        && up != 0.0
    {
        n.set("Translate", Node::Arr(vec![Node::F32(p[0]), Node::F32(p[1] + up), Node::F32(p[2])]));
    }
    n
}

fn flat(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).hypot(a[2] - b[2])
}

pub fn guard(map: &str, acts: &[Node], seen: &Map<u64, u32>) -> Guard {
    let mut g: Guard = Guard { child: child(map), rising: horde::RISING_WATER.contains(&map), blastable: horde::NO_BLASTERS.iter().any(|m| map.starts_with(m)), ..Guard::default() };
    for a in acts {
        let n: &str = gyaml(a);
        let r: Option<i32> = room(a);
        g.dungeon |= r.is_some();
        g.blastable |= horde::BLASTABLE.iter().any(|b| n.contains(b));
        if class(n).is_some() || horde::kind(n).is_some() {
            g.local.insert(n.to_string());
        }
        let Some(p) = pos(a) else {
            continue;
        };
        if n.starts_with(BOSS) || n.starts_with(MINIBOSS) {
            if let Some(r) = r {
                g.banned.insert(r);
            }
            g.bosses.push((p, if n.starts_with(BOSS) { BOSS_REACH } else { MINIBOSS_REACH }));
        }
        let held: bool = horde::fighting(n) && hash_of(a).is_some_and(|h| seen.get(&h) != Some(&OWN));
        if n.starts_with(COUNTER) || held {
            if let Some(r) = r {
                g.counted.insert(r);
            }
            g.counters.push(p);
        }
    }
    g
}

impl Guard {
    pub fn barred(&self, a: &Node) -> bool {
        let Some(p) = pos(a) else {
            return true;
        };
        room(a).is_some_and(|r| self.banned.contains(&r)) || self.bosses.iter().any(|(b, d)| flat(*b, p) < *d)
    }

    pub fn blade(&self, a: &Node) -> bool {
        if self.child {
            return true;
        }
        match (room(a), pos(a)) {
            (Some(r), _) if self.dungeon => self.counted.contains(&r),
            (_, Some(p)) => self.counters.iter().any(|c| flat(*c, p) < COUNTED_REACH),
            _ => true,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn swap_list(map: &str, list: &mut [Node], vanilla: &Set<u64>, pool: &Pool, g: &Guard, seen: &Map<u64, u32>, mixed: &mut Map<i64, Map<String, u32>>, share: u64, rng: &mut Rng) -> Vec<(u64, String, String)> {
    let mut out: Vec<(u64, String, String)> = Vec::new();
    for a in list.iter_mut() {
        let Some(h) = hash_of(a) else {
            continue;
        };
        let n: String = gyaml(a).to_string();
        let Some(k) = class(&n) else {
            continue;
        };
        if !vanilla.contains(&h) || !free(a, seen) || (kept(map, &n) && n != BABY) || g.barred(a) || (g.child && SOURCES.contains(&n.as_str())) {
            continue;
        }
        let opts: Vec<&'static Kind> = options(map, k, pool, g, g.blade(a));
        if opts.is_empty() || rng.below(100) >= share {
            continue;
        }
        let key: i64 = spot(a, g.dungeon);
        let to: &Kind = spread(&opts, mixed.get(&key), rng);
        let Some(s) = pool.shape(to.gyaml) else {
            continue;
        };
        let here: &mut Map<String, u32> = mixed.entry(key).or_default();
        if let Some(c) = here.get_mut(&n) {
            *c = c.saturating_sub(1);
        }
        *here.entry(to.gyaml.to_string()).or_insert(0) += 1;
        *a = swapped(a, k, to, s, g.dungeon);
        out.push((h, n, to.gyaml.to_string()));
    }
    out
}

pub fn made(rf: &mut Romfs, have: Have, seed: &str) -> Res<Vec<(Out, Vec<Swap>)>> {
    made_with(rf, have, seed, SHARE)
}

pub fn made_with(rf: &mut Romfs, have: Have, seed: &str, share: u64) -> Res<Vec<(Out, Vec<Swap>)>> {
    let mut seen: Map<u64, u32> = Map::new();
    let mut by: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut vans: BTreeMap<String, Vec<Node>> = BTreeMap::new();
    for f in files(rf)? {
        let y: Byml = Byml::parse(&raw(rf, have, &f)?.0)?;
        tally(&y.root, &mut seen);
        if !f.ends_with(EXT) || !f.starts_with(SCENES) || f.starts_with(SKIP_DIR) {
            continue;
        }
        let Some(m) = horde::map_of(&f).filter(|m| !horde::SKIP_PREFIX.iter().any(|p| m.starts_with(p))) else {
            continue;
        };
        let v: Byml = rf.read_byml(&f)?;
        vans.entry(m.clone()).or_default().extend(v.root.get("Actors").and_then(|l| l.as_arr()).unwrap_or(&[]).iter().cloned());
        if horde::wanted(&m) {
            by.entry(m).or_default().push(f);
        }
    }
    let every: Vec<Node> = vans.values().flatten().cloned().collect();
    let all: BTreeMap<&'static str, Vec<(String, Node)>> = loose(donors(&every, &seen));
    let mut out: Vec<(Out, Vec<Swap>)> = Vec::new();
    for (map, fs) in &by {
        let van: &[Node] = vans.get(map).map(|v| v.as_slice()).unwrap_or(&[]);
        let vanilla: Set<u64> = van.iter().filter_map(hash_of).collect();
        let pool: Pool = Pool { here: donors(van, &seen), all: all.clone() };
        let g: Guard = guard(map, van, &seen);
        let mut mixed: Map<i64, Map<String, u32>> = mix(van, g.dungeon);
        let mut rng: Rng = Rng::from_text(&format!("{seed}|{SALT}|{map}"));
        for f in fs {
            let (b, id): (Vec<u8>, Option<u32>) = raw(rf, have, f)?;
            let mut y: Byml = Byml::parse(&b)?;
            let Some(list) = y.root.get_mut("Actors").and_then(|l| l.as_arr_mut()) else {
                continue;
            };
            let sw: Vec<(u64, String, String)> = swap_list(map, list, &vanilla, &pool, &g, &seen, &mut mixed, share, &mut rng);
            if sw.is_empty() {
                continue;
            }
            let new: Vec<u8> = y.write()?;
            let ss: Vec<Swap> = sw.into_iter().map(|(hash, from, to)| Swap { map: map.clone(), file: f.clone(), hash, from, to }).collect();
            out.push((Out { path: f.clone(), old: b.len(), new: new.len(), z: zs::compress_raw(&new, Some(id.unwrap_or(BANC_DICT)))? }, ss));
        }
    }
    Ok(out)
}

pub fn per_map(all: &[(Out, Vec<Swap>)]) -> BTreeMap<String, usize> {
    let mut m: BTreeMap<String, usize> = BTreeMap::new();
    for s in all.iter().flat_map(|x| x.1.iter()) {
        *m.entry(s.map.clone()).or_insert(0) += 1;
    }
    m
}

pub fn summary(all: &[(Out, Vec<Swap>)]) -> String {
    let m: BTreeMap<String, usize> = per_map(all);
    let total: usize = m.values().sum();
    let list: Vec<String> = m.iter().map(|(k, v)| format!("{k} {v}")).collect();
    format!("Enemy randomizer swapped {total} enemies in {} maps: {}.", m.len(), list.join(", "))
}

pub fn stage(rf: &mut Romfs, have: Have, seed: &str, notes: &mut Vec<String>) -> Res<Vec<Out>> {
    let all: Vec<(Out, Vec<Swap>)> = made(rf, have, seed)?;
    notes.push(summary(&all));
    Ok(all.into_iter().map(|x| x.0).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn foe(g: &str, h: u64, p: [f32; 3], room: Option<i32>, dynamic: Vec<(&str, Node)>) -> Node {
        let mut d: Vec<(String, Node)> = dynamic.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        if let Some(r) = room {
            d.push((ROOM.to_string(), Node::I32(r)));
        }
        d.sort_by(|a, b| a.0.cmp(&b.0));
        let mut m: Vec<(String, Node)> = Vec::new();
        if !d.is_empty() {
            m.push(("Dynamic".to_string(), Node::Map(d)));
        }
        m.push(("Gyaml".to_string(), Node::Str(g.to_string())));
        m.push(("Hash".to_string(), Node::U64(h)));
        m.push(("Phive".to_string(), Node::Map(vec![("Placement".to_string(), Node::Map(vec![("ID".to_string(), Node::U64(h))]))])));
        m.push(("SRTHash".to_string(), Node::U32(h as u32)));
        m.push(("Translate".to_string(), Node::Arr(p.iter().map(|v| Node::F32(*v)).collect())));
        Node::Map(m)
    }

    fn seen(list: &[Node]) -> Map<u64, u32> {
        let mut out: Map<u64, u32> = Map::new();
        for a in list {
            tally(a, &mut out);
        }
        out
    }

    fn run_with(map: &str, list: &[Node], elsewhere: &[Node], share: u64, seed: &str) -> (Vec<Node>, Vec<(u64, String, String)>) {
        let both: Vec<Node> = list.iter().chain(elsewhere.iter()).cloned().collect();
        let s: Map<u64, u32> = seen(&both);
        let pool: Pool = Pool { here: donors(list, &s), all: donors(&both, &s) };
        let g: Guard = guard(map, list, &s);
        let vanilla: Set<u64> = list.iter().filter_map(hash_of).collect();
        let mut out: Vec<Node> = list.to_vec();
        let mut mixed: Map<i64, Map<String, u32>> = mix(list, g.dungeon);
        let sw: Vec<(u64, String, String)> = swap_list(map, &mut out, &vanilla, &pool, &g, &s, &mut mixed, share, &mut Rng::from_text(seed));
        (out, sw)
    }

    fn run(map: &str, list: &[Node], seed: &str) -> (Vec<Node>, Vec<(u64, String, String)>) {
        run_with(map, list, &[], 100, seed)
    }

    fn many(n: u64, g: &str, room: Option<i32>, y: f32) -> Vec<Node> {
        (0..n).map(|i| foe(g, 0x9_0000_0000 + (i << 8) + y as u64, [i as f32 * 3.0, y, 0.0], room, Vec::new())).collect()
    }

    fn others() -> Vec<Node> {
        let mut v: Vec<Node> = Vec::new();
        for (i, g) in ["Enemy_Keese_Ice", "Enemy_Poe", "Enemy_Bubble_Green", "Enemy_Biri", "Enemy_Guay_Fixed", "Enemy_Keese_Fire", "Enemy_Wallmaster", "Enemy_Skulltula", "Enemy_ReDead", "Enemy_LikeLike", "Enemy_Dodongo", "Enemy_BabyDodongo", "Enemy_Dekubaba"].iter().enumerate() {
            v.push(foe(g, 0x8_0000_0000 + i as u64, [900.0, 50.0, 900.0], Some(99), vec![("Far", Node::Bool(true))]));
        }
        v
    }

    #[test]
    fn kinds_from_other_maps_are_offered_and_mixed_in_a_room() {
        let keese: Vec<Node> = many(12, "Enemy_Keese", Some(3), 3.5);
        let (out, sw): (Vec<Node>, Vec<(u64, String, String)>) = run_with("ForestTemple", &keese, &others(), 100, "a");
        assert_eq!(sw.len(), 12);
        let kinds: Set<String> = sw.iter().map(|s| s.2.clone()).collect();
        assert!(kinds.len() >= 3, "{kinds:?}");
        assert!(!kinds.contains("Enemy_Keese_Fire") && !kinds.contains(GUAY), "{kinds:?}");
        for a in &out {
            if let Some(Node::Map(d)) = a.get("Dynamic") {
                assert!(d.iter().any(|e| e.0 == ROOM && e.1 == Node::I32(3)));
            }
        }
        let (_, sw): (Vec<Node>, Vec<(u64, String, String)>) = run_with("HyruleField_Adult", &many(12, "Enemy_Keese", None, 3.5), &others(), 100, "a");
        assert!(sw.iter().all(|s| s.2 != "Enemy_Keese_Fire"), "{sw:?}");
        let (_, sw): (Vec<Node>, Vec<(u64, String, String)>) = run_with("ForestTemple", &many(12, "Enemy_Poe", Some(3), 0.0), &others(), 100, "a");
        assert!(sw.len() == 12 && sw.iter().all(|s| !horde::percher(&s.2)), "{sw:?}");
        let mut home: Vec<Node> = many(12, "Enemy_Keese", Some(3), 3.5);
        home.push(foe("Enemy_Keese_Fire", 0x7_0000_0001, [0.0, 3.0, 50.0], Some(3), Vec::new()));
        let got: usize = ["a", "b", "c", "d"].iter().map(|s| run_with("FireTemple", &home, &others(), 100, s).1.iter().filter(|x| x.2 == "Enemy_Keese_Fire").count()).sum();
        assert!(got > 0);
    }

    #[test]
    fn ceiling_kinds_stay_indoors_guays_outdoors_and_blasters_away_from_bomb_walls() {
        let g: Guard = guard("HyruleField_Adult", &many(2, "Enemy_Keese", None, 3.5), &Map::new());
        assert!(!fits("HyruleField_Adult", horde::kind("Enemy_Wallmaster").unwrap(), &g) && fits("HyruleField_Adult", horde::kind(GUAY).unwrap(), &g));
        let d: Guard = guard("ForestTemple", &many(2, "Enemy_Keese", Some(1), 3.5), &Map::new());
        assert!(fits("ForestTemple", horde::kind("Enemy_Wallmaster").unwrap(), &d) && !fits("ForestTemple", horde::kind(GUAY).unwrap(), &d));
        let mut walls: Vec<Node> = many(2, "Enemy_Wolfos", None, 0.0);
        walls.push(foe("Obj_CrackWall_A_01", 0x7_0000_0002, P, None, Vec::new()));
        let b: Guard = guard("DeathMountainTrail_Adult", &walls, &Map::new());
        assert!(!fits("DeathMountainTrail_Adult", horde::kind("Enemy_Dodongo").unwrap(), &b) && fits("DeathMountainTrail_Adult", horde::kind("Enemy_Floormaster").unwrap(), &b));
        let w: Guard = guard("WaterTemple", &many(2, "Enemy_Keese", Some(1), 3.5), &Map::new());
        assert!(!fits("WaterTemple", horde::kind("Enemy_ReDead").unwrap(), &w) && fits("WaterTemple", horde::kind("Enemy_Poe").unwrap(), &w));
    }

    #[test]
    fn parameters_from_another_map_lose_their_place_keys() {
        let mut p: BTreeMap<&'static str, Vec<(String, Node)>> = BTreeMap::new();
        p.insert("Enemy_ReDead", vec![("Enemy_Redead_IsBottomOfTheWell".to_string(), Node::Bool(true)), ("Fierce".to_string(), Node::Bool(true))]);
        p.insert("Enemy_Bubble_Red", vec![("ApperMinHeightLimit".to_string(), Node::F32(2.0)), ("BubbleBase_UseCustomPerceptionRangeName".to_string(), Node::Str("x".to_string()))]);
        let l: BTreeMap<&'static str, Vec<(String, Node)>> = loose(p);
        assert_eq!(l["Enemy_ReDead"], vec![("Fierce".to_string(), Node::Bool(true))]);
        assert!(l["Enemy_Bubble_Red"].is_empty());
    }

    #[test]
    fn about_four_in_five_free_enemies_change() {
        let list: Vec<Node> = many(400, "Enemy_Keese", Some(2), 3.5);
        let (_, sw): (Vec<Node>, Vec<(u64, String, String)>) = run_with("ForestTemple", &list, &others(), SHARE, "a");
        assert!((280..=360).contains(&sw.len()), "{}", sw.len());
        let (out, none): (Vec<Node>, Vec<(u64, String, String)>) = run_with("ForestTemple", &list, &others(), 0, "a");
        assert!(none.is_empty() && out == list);
    }

    const P: [f32; 3] = [100.0, 10.0, 100.0];

    fn field() -> Vec<Node> {
        vec![
            foe("Enemy_Wolfos", 0x1_0000_0001, P, None, vec![("Fierce", Node::Bool(true))]),
            foe("Enemy_Dodongo", 0x1_0000_0002, [120.0, 10.0, 100.0], None, Vec::new()),
            foe("Enemy_Floormaster", 0x1_0000_0003, [140.0, 10.0, 100.0], None, vec![("Wake", Node::I32(2))]),
            foe("Enemy_Keese", 0x1_0000_0004, [100.0, 13.5, 120.0], None, Vec::new()),
            foe("Enemy_Bubble_Red", 0x1_0000_0005, [100.0, 11.5, 140.0], None, Vec::new()),
            foe("Enemy_Octorok", 0x1_0000_0006, [100.0, 0.0, 160.0], None, Vec::new()),
            foe("Enemy_Skulltula_Gold", 0x1_0000_0007, [100.0, 10.0, 180.0], None, Vec::new()),
        ]
    }

    #[test]
    fn a_swap_keeps_the_ground_class_and_takes_the_new_kinds_parameters() {
        let (out, sw): (Vec<Node>, Vec<(u64, String, String)>) = run("HyruleField", &field(), "a");
        assert!(!sw.is_empty());
        for (h, from, to) in &sw {
            assert_eq!(class(from).unwrap().class, class(to).unwrap().class, "{from} -> {to}");
            assert_ne!(from, to);
            let n: &Node = out.iter().find(|a| hash_of(a) == Some(*h)).unwrap();
            assert_eq!(gyaml(n), to);
            assert!(own_body(n, *h));
        }
        assert!(out.iter().all(|a| gyaml(a) != "Enemy_Octorok" || hash_of(a) == Some(0x1_0000_0006)));
        assert_eq!(gyaml(&out[5]), "Enemy_Octorok");
        assert_eq!(gyaml(&out[6]), "Enemy_Skulltula_Gold");
        assert_eq!(out[6], field()[6]);
        let flier: &Node = &out[3];
        assert_eq!(gyaml(flier), "Enemy_Bubble_Red");
        assert!((pos(flier).unwrap()[1] - (13.5 - 3.5 + 1.5)).abs() < 0.001);
        for a in &out[..3] {
            match gyaml(a) {
                "Enemy_Wolfos" => assert_eq!(a.get("Dynamic").and_then(|d| d.get("Fierce")), Some(&Node::Bool(true))),
                "Enemy_Floormaster" => assert_eq!(a.get("Dynamic").and_then(|d| d.get("Wake")), Some(&Node::I32(2))),
                _ => assert!(a.get("Dynamic").is_none()),
            }
        }
    }

    #[test]
    fn linked_counted_evented_and_kept_enemies_are_never_swapped() {
        let mut list: Vec<Node> = field();
        let mut linked: Node = foe("Enemy_Dodongo", 0x2_0000_0001, P, None, Vec::new());
        linked.set("Links", Node::Arr(Vec::new()));
        let counted: Node = foe("Enemy_Wolfos", 0x2_0000_0002, P, None, Vec::new());
        let mut counter: Node = foe("Obj_EnemyCounter_A_09", 0x2_0000_0003, [900.0, 0.0, 900.0], None, Vec::new());
        counter.set("Links", Node::Arr(vec![Node::Map(vec![("Dst".to_string(), Node::U64(0x2_0000_0002))])]));
        let evented: Node = foe("Enemy_Wolfos", 0x2_0000_0004, P, None, vec![("AttackedEventName", Node::Str("x".to_string()))]);
        let dropping: Node = foe("Enemy_Wolfos", 0x2_0000_0005, P, None, vec![("Drop__DropActor", Node::Str("key".to_string()))]);
        list.extend([linked.clone(), counted.clone(), counter, evented.clone(), dropping.clone()]);
        for seed in ["a", "b", "c", "d", "e", "f"] {
            let (out, sw): (Vec<Node>, Vec<(u64, String, String)>) = run("HyruleField", &list, seed);
            for keep in [&linked, &counted, &evented, &dropping] {
                assert!(out.contains(keep));
                assert!(!sw.iter().any(|s| Some(s.0) == hash_of(keep)));
            }
        }
        let dodongos: Vec<Node> = vec![foe("Enemy_Dodongo", 0x3_0000_0001, P, Some(1), Vec::new()), foe("Enemy_Wolfos", 0x3_0000_0002, P, Some(1), Vec::new()), foe("Enemy_Dodongo", 0x3_0000_0003, P, Some(1), Vec::new()), foe("Enemy_LikeLike", 0x3_0000_0004, P, Some(1), Vec::new())];
        let (out, sw): (Vec<Node>, Vec<(u64, String, String)>) = run("DodongoCavern", &dodongos, "a");
        assert_eq!(out, dodongos);
        assert!(sw.is_empty());
        assert!(kept("SacredForestMeadow_Child", "Enemy_Wolfos") && kept("WaterTemple", "Enemy_ShellBlade") && kept("SpiritTemple", "Enemy_IronKnuckle_Black"));
        assert!(!kept("HyruleField", "Enemy_Wolfos"));
    }

    #[test]
    fn boss_rooms_and_the_ground_near_a_miniboss_are_left_alone() {
        let list: Vec<Node> = vec![
            foe("Enemy_DungeonBoss_Goma", 0x4_0000_0001, [0.0, 0.0, 0.0], Some(7), Vec::new()),
            foe("Enemy_Keese", 0x4_0000_0002, [50.0, 3.5, 0.0], Some(7), Vec::new()),
            foe("Enemy_Bubble_Green", 0x4_0000_0003, [50.0, 1.5, 0.0], Some(7), Vec::new()),
            foe("Enemy_Keese", 0x4_0000_0004, [500.0, 3.5, 0.0], Some(2), Vec::new()),
        ];
        let (out, sw): (Vec<Node>, Vec<(u64, String, String)>) = run("InsideTheDekuTree", &list, "a");
        assert_eq!(&out[..3], &list[..3]);
        assert_eq!(sw.iter().map(|s| s.0).collect::<Vec<u64>>(), vec![0x4_0000_0004]);
        let outdoors: Vec<Node> = vec![foe("Enemy_MiddleBoss_Lizalfos", 0x5_0000_0001, P, None, Vec::new()), foe("Enemy_Poe", 0x5_0000_0002, [110.0, 0.0, 100.0], None, Vec::new()), foe("Enemy_Bubble_Green", 0x5_0000_0003, [400.0, 1.5, 100.0], None, Vec::new())];
        let (_, sw): (Vec<Node>, Vec<(u64, String, String)>) = run("HyruleField", &outdoors, "a");
        assert_eq!(sw.iter().map(|s| s.0).collect::<Vec<u64>>(), vec![0x5_0000_0003]);
    }

    #[test]
    fn child_maps_and_counted_rooms_only_get_kinds_a_sword_can_beat() {
        assert!(child("HyruleField_Child") && child("InsideTheDekuTree") && child("SpiritTemple") && !child("ForestTemple") && !child("HyruleField_Adult"));
        assert_eq!(beat("Enemy_Amos"), Beat::More);
        assert_eq!(beat("Enemy_Wallmaster"), Beat::More);
        assert_eq!(beat("Enemy_Wolfos"), Beat::Blade);
        for seed in ["a", "b", "c", "d", "e", "f", "g", "h"] {
            let (_, sw): (Vec<Node>, Vec<(u64, String, String)>) = run("HyruleField_Child", &field(), seed);
            assert!(sw.iter().all(|s| beat(&s.2) == Beat::Blade), "{sw:?}");
        }
        let babas: Vec<Node> = vec![foe("Enemy_Dekubaba", 0x6_0000_0011, P, Some(1), Vec::new()), foe("Enemy_ReDead", 0x6_0000_0012, P, Some(1), Vec::new())];
        let (_, sw): (Vec<Node>, Vec<(u64, String, String)>) = run("InsideTheDekuTree", &babas, "a");
        assert_eq!(sw, vec![(0x6_0000_0012, "Enemy_ReDead".to_string(), "Enemy_Dekubaba".to_string())]);
        let (_, sw): (Vec<Node>, Vec<(u64, String, String)>) = run("ForestTemple", &babas, "a");
        assert_eq!(sw.len(), 2);
        let mut room: Vec<Node> = vec![foe("Enemy_Skulltula", 0x6_0000_0001, P, Some(4), Vec::new()), foe("Enemy_Wallmaster", 0x6_0000_0002, P, Some(4), Vec::new())];
        let (_, open): (Vec<Node>, Vec<(u64, String, String)>) = run("ForestTemple", &room, "a");
        assert_eq!(open.len(), 2);
        room.push(foe("Obj_EnemyCounter_A_09", 0x6_0000_0003, P, Some(4), Vec::new()));
        let (_, shut): (Vec<Node>, Vec<(u64, String, String)>) = run("ForestTemple", &room, "a");
        assert!(shut.iter().all(|s| beat(&s.2) == Beat::Blade) && shut.len() == 1, "{shut:?}");
    }

    #[test]
    fn the_same_seed_gives_the_same_swaps() {
        let many: Vec<Node> = (0..60).flat_map(|i| field().into_iter().map(move |mut a| {
            let h: u64 = hash_of(&a).unwrap() + (i << 40);
            a.set("Hash", Node::U64(h));
            a.set("Phive", Node::Map(vec![("Placement".to_string(), Node::Map(vec![("ID".to_string(), Node::U64(h))]))]));
            a
        })).collect();
        assert_eq!(run("HyruleField", &many, "x"), run("HyruleField", &many, "x"));
        assert_ne!(run("HyruleField", &many, "x").1, run("HyruleField", &many, "y").1);
    }

    #[test]
    fn the_pool_has_no_boss_miniboss_or_never_placed_kind() {
        for k in horde::KINDS.iter().filter(|k| class(k.gyaml).is_some()) {
            assert!(k.class != Class::Mini && !horde::NEVER.contains(&k.gyaml), "{}", k.gyaml);
            for bad in ["Boss", "Gold", "Lizalfos", "IronKnuckle", "Stalfos", "Dinolfos", "DeadHand"] {
                assert!(!k.gyaml.contains(bad), "{}", k.gyaml);
            }
        }
        assert!(class("Enemy_Amos").is_none() && class("Enemy_Skullwalltula").is_none() && class("Enemy_Tektite_Blue").is_none() && class("Enemy_Tektite_Red").is_some());
        assert!(class("Enemy_Stalfos").is_none() && class("Enemy_Skulltula_Gold").is_none() && class("Enemy_Keese").is_some());
    }

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        if p.is_dir() { Some(p) } else { None }
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn the_game_maps_swap_only_free_enemies_within_their_class() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let all: Vec<(Out, Vec<Swap>)> = made(&mut rf, &none, "game").unwrap();
        eprintln!("{}", summary(&all));
        let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
        for s in all.iter().flat_map(|x| x.1.iter()) {
            *kinds.entry(format!("{} -> {}", s.from, s.to)).or_insert(0) += 1;
        }
        eprintln!("{kinds:?}");
        let n: usize = all.iter().map(|x| x.1.len()).sum();
        let full: Vec<(Out, Vec<Swap>)> = made_with(&mut rf, &none, "game", 100).unwrap();
        let (can, got): (BTreeMap<String, usize>, BTreeMap<String, usize>) = (per_map(&full), per_map(&all));
        for (m, c) in &can {
            eprintln!("count|{m}|{}|{c}", got.get(m).copied().unwrap_or(0));
        }
        let mut to: BTreeMap<String, usize> = BTreeMap::new();
        for s in all.iter().flat_map(|x| x.1.iter()) {
            *to.entry(s.to.clone()).or_insert(0) += 1;
        }
        eprintln!("now used: {to:?}");
        let c: usize = can.values().sum();
        assert!(n * 100 >= c * 70 && n * 100 <= c * 90, "{n} of {c}");
        assert!((80..=200).contains(&n), "{n}");
        assert!(to.len() >= 12, "{to:?}");
        let mut keys: BTreeMap<String, std::collections::BTreeSet<String>> = BTreeMap::new();
        assert!(crate::difficulty::stage(&mut rf, &crate::difficulty::Opts::default(), &none, "game").unwrap().is_empty());
        let on: crate::difficulty::Opts = crate::difficulty::Opts { randomizer: true, ..crate::difficulty::Opts::default() };
        let mut notes: Vec<String> = Vec::new();
        let staged: Vec<Out> = crate::difficulty::stage_noted(&mut rf, &on, &none, "game", &mut notes).unwrap();
        assert_eq!(staged.len(), all.len());
        assert_eq!(notes, vec![summary(&all)]);
        let mut seen: Map<u64, u32> = Map::new();
        for f in files(&rf).unwrap() {
            tally(&rf.read_byml(&f).unwrap().root, &mut seen);
        }
        for (o, ss) in &all {
            let map: String = horde::map_of(&o.path).unwrap();
            assert!(horde::wanted(&map) && !o.path.starts_with(SKIP_DIR));
            let was: Byml = rf.read_byml(&o.path).unwrap();
            let now: Byml = Byml::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap();
            let a: &[Node] = was.root.get("Actors").unwrap().as_arr().unwrap();
            let b: &[Node] = now.root.get("Actors").unwrap().as_arr().unwrap();
            assert_eq!(a.len(), b.len());
            let Node::Map(x) = &was.root else { panic!() };
            let Node::Map(y) = &now.root else { panic!() };
            assert!(x.iter().zip(y.iter()).all(|(p, q)| p.0 == q.0 && (p.0 == "Actors" || p.1 == q.1)), "{} changed outside its actors", o.path);
            let mut van: Vec<Node> = Vec::new();
            for f in files(&rf).unwrap().iter().filter(|f| horde::map_of(f).as_deref() == Some(map.as_str())) {
                van.extend(rf.read_byml(f).unwrap().root.get("Actors").unwrap().as_arr().unwrap().iter().cloned());
            }
            let here: Set<String> = van.iter().map(|v| gyaml(v).to_string()).collect();
            let mut changed: usize = 0;
            for (p, q) in a.iter().zip(b.iter()) {
                if p == q {
                    continue;
                }
                changed += 1;
                let h: u64 = hash_of(p).unwrap();
                assert_eq!(hash_of(q), Some(h));
                assert_eq!(seen.get(&h), Some(&OWN), "{} {h:x} is referenced", o.path);
                assert!(free(p, &seen) && (!kept(&map, gyaml(p)) || gyaml(p) == BABY));
                assert!(p.get("Links").is_none() && p.get("Rails").is_none());
                let (kp, kq): (&Kind, &Kind) = (class(gyaml(p)).unwrap(), class(gyaml(q)).unwrap());
                assert_eq!(kp.class, kq.class, "{} -> {}", kp.gyaml, kq.gyaml);
                assert!(!matches!(kp.class, Class::Water | Class::Sand | Class::Mini));
                let g: Guard = guard(&map, &van, &seen);
                assert!(fits(&map, kq, &g), "{} does not fit {map}", kq.gyaml);
                assert!(here.contains(kq.gyaml) || !horde::HOME_ONLY.contains(&kq.gyaml), "{} is new to {map}", kq.gyaml);
                assert!(!horde::NEVER.iter().any(|x| kq.gyaml.starts_with(x)) && class(kq.gyaml).is_some());
                if let Some(Node::Map(d)) = q.get("Dynamic") {
                    keys.entry(kq.gyaml.to_string()).or_default().extend(d.iter().map(|e| e.0.clone()));
                    assert!(here.contains(kq.gyaml) || d.iter().all(|e| !PLACE_KEYS.iter().any(|x| e.0.contains(x))), "{map} {} carries a place key", kq.gyaml);
                }
                if child(&map) {
                    assert_eq!(beat(kq.gyaml), Beat::Blade, "{map} got {}", kq.gyaml);
                    assert!(!SOURCES.contains(&kp.gyaml), "{map} lost a {}", kp.gyaml);
                }
                assert!(!horde::percher(kq.gyaml) || horde::percher(kp.gyaml), "{map}: a {} became a {}", kp.gyaml, kq.gyaml);
                assert_eq!(room(p), room(q));
                assert_eq!(p.get("Rotate"), q.get("Rotate"));
                let (pp, pq): ([f32; 3], [f32; 3]) = (pos(p).unwrap(), pos(q).unwrap());
                assert!(pp[0] == pq[0] && pp[2] == pq[2] && (pp[1] - pq[1]).abs() <= 7.0);
                if let Some(Node::Map(d)) = q.get("Dynamic") {
                    assert!(d.iter().all(|e| !e.0.contains("Event") && !e.0.contains("GameData") && !e.0.contains("Drop__DropActor")));
                }
                assert!(ss.iter().any(|s| s.hash == h && s.from == gyaml(p) && s.to == gyaml(q)));
                eprintln!("spot|{map}|{}|{}|{:?}|{:.1}, {:.1}, {:.1}", gyaml(p), gyaml(q), room(q), pq[0], pq[1], pq[2]);
            }
            assert_eq!(changed, ss.len(), "{}", o.path);
        }
        eprintln!("keys: {keys:?}");
        let again: Vec<(Out, Vec<Swap>)> = made(&mut rf, &none, "game").unwrap();
        assert!(all.len() == again.len() && all.iter().zip(again.iter()).all(|(p, q)| p.0.z == q.0.z && p.1 == q.1));
        let other: Vec<(Out, Vec<Swap>)> = made(&mut rf, &none, "other").unwrap();
        assert_ne!(all.iter().flat_map(|x| x.1.iter()).collect::<Vec<&Swap>>(), other.iter().flat_map(|x| x.1.iter()).collect::<Vec<&Swap>>());
    }

    #[test]
    fn the_child_dungeons_keep_every_enemy_a_counter_group_link_or_event_needs() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let all: Vec<(Out, Vec<Swap>)> = made(&mut rf, &none, "game").unwrap();
        let mut seen: Map<u64, u32> = Map::new();
        let fs: Vec<String> = files(&rf).unwrap();
        for f in &fs {
            tally(&rf.read_byml(f).unwrap().root, &mut seen);
        }
        let mut held_total: usize = 0;
        for map in ["InsideTheDekuTree", "DodongoCavern", "InsideJabuJabuBelly", "BottomOfTheWell"] {
            let mut counters: BTreeMap<String, usize> = BTreeMap::new();
            let mut held: BTreeMap<String, usize> = BTreeMap::new();
            let mut groups: usize = 0;
            for f in fs.iter().filter(|f| horde::map_of(f).as_deref() == Some(map)) {
                let was: Byml = rf.read_byml(f).unwrap();
                groups += was.root.get("AiGroups").and_then(|g| g.as_arr()).map(|g| g.len()).unwrap_or(0);
                let now: Byml = match all.iter().find(|x| &x.0.path == f) {
                    Some(x) => Byml::parse(&rf.zs.decompress(&x.0.z).unwrap()).unwrap(),
                    None => was.clone(),
                };
                let a: &[Node] = was.root.get("Actors").unwrap().as_arr().unwrap();
                let b: &[Node] = now.root.get("Actors").unwrap().as_arr().unwrap();
                for (p, q) in a.iter().zip(b.iter()) {
                    let n: &str = gyaml(p);
                    if n.contains("Counter") || n.contains("Group") {
                        *counters.entry(n.to_string()).or_insert(0) += 1;
                    }
                    if horde::fighting(n) && !free(p, &seen) {
                        *held.entry(n.to_string()).or_insert(0) += 1;
                        assert_eq!(p, q, "{map}: a needed {n} changed");
                    }
                    if p != q {
                        assert_eq!(beat(gyaml(q)), Beat::Blade, "{map} got {}", gyaml(q));
                    }
                }
            }
            held_total += held.values().sum::<usize>();
            eprintln!("{map}: {groups} AI groups, counters {counters:?}, enemies kept because something needs them {held:?}");
        }
        assert!(held_total > 0);
    }
}
