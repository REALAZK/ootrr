use std::collections::BTreeMap;
use std::collections::BTreeSet as Set;
use std::collections::BTreeSet;
use std::collections::HashMap as Map;
use std::fs;
use std::path::{Path, PathBuf as PB};

use crate::byml::{Byml, Node};
use crate::cow_clone::rows::heap;
use crate::difficulty::ambush::floats;
use crate::difficulty::extras::{files, gyaml, pos, raw, room, EXT, OWN, SKIP_DIR};
use crate::difficulty::horde;
use crate::difficulty::leevers::{self, Made};
use crate::difficulty::swap::{self, Guard};
use crate::error::{Error as E, Res};
use crate::flow_locations::dict_id;
use crate::modpack::Pack;
use crate::pickup_pack::rows::{self, ACTOR_INFO, GAME_ACTOR_INFO, HEAP, ROW_ID};
use crate::pickup_pack::sizes::{file_size, pack_size};
use crate::pickup_pack::tag::{Table, TAG};
use crate::pickup_pack::PACK_DICT;
use crate::placement::hash_of;
use crate::qol::Have;
use crate::rng::Rng;
use crate::romfs::Romfs;
use crate::rsizetable::{self, Rst};
use crate::sarc::{self, Entry};
use crate::staging::{Out, BANC_DICT};
use crate::zs;

pub const SALT: &str = "enemy-sizes";
pub const TAIL: &str = "_Size";
pub const GRAIN: u64 = 1 << 20;
pub const SAME: i64 = 4;
pub const SCRIPTED_LIFE: i32 = 100;
pub const SKIP_MAPS: &[&str] = &["SageRoom", "FinalBattleField"];
pub const KEEP_KINDS: &[&str] = &[
    "Enemy_MiddleBoss",
    "Enemy_DungeonBoss",
    "Enemy_Tentacle",
    "Enemy_TrickyDoor",
    "Enemy_Gohma",
    "Enemy_CrazyFloorTile",
    "Enemy_DeadHand",
    "Enemy_Beamos",
    "Enemy_Anubis",
    "Enemy_Amos",
    "Enemy_Skullwalltula",
    "Enemy_Leever",
    "Enemy_Poe_Amy",
    "Enemy_Poe_Beth",
    "Enemy_Poe_Joelle",
    "Enemy_Poe_Flat",
    "Enemy_Poe_Sharp",
    "Enemy_Poe_Big",
];
pub const TIED_KEYS: &[&str] = &["Event", "RecordingDeath", "FenceID", "IsCleared"];
pub const LANES: &[&str] = &["LostWood", "ZoraRiver", "DeathMountainTrail", "DeathMountainCrater", "GerudoValley", "Graveyard", "KakarikoVillage", "Market", "SacredForestMeadow", "KokiriForest"];
pub const INDOORS_MAX: i64 = 150;
pub const LANE_MAX: i64 = 150;
pub const BIG_MAX: i64 = 200;
pub const BIG_INDOORS_MAX: i64 = 125;
pub const MILD: (i64, i64) = (75, 150);
pub const WILD: (i64, i64) = (50, 250);
pub const MILD_STEPS: &[i64] = &[75, 100, 125, 150];
pub const WILD_STEPS: &[i64] = &[50, 75, 100, 150, 200, 250];
pub const NOTE: &str = "Random enemy sizes: every ordinary enemy that no door, counter or event depends on gets its own size, rolled from the seed (so the same seed gives the same sizes): mild is 0.75 to 1.5 times, wild is 0.5 to 2.5 times, with smaller sizes more common than the biggest. Bosses, minibosses, enemies the game counts, the Poe sisters and Big Poes, wall and door enemies, Armos statues and the guards of the gates are left alone. Indoors, in narrow lanes and for the already big kinds the size is capped (1.5 and 1.25 indoors) so nothing is wedged into a wall. Not working yet: the game ignores the placement size, so enemies keep their usual size.";
pub const NOTE_HEALTH: &str = "With health following size, the sizes snap to a few steps and each step is its own copy of the enemy (named like Enemy_Stalfos_Size150) with its health multiplied by the size, rounded, at least 1 and at most 99. It is multiplied after the enemy health setting, so the two stack. Kinds whose health the game keeps in a shared or scripted file are not resized.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Range {
    #[default]
    Off,
    Mild,
    Wild,
}

impl Range {
    pub fn from_setting(v: &str) -> Range {
        match v {
            "mild" => Range::Mild,
            "wild" => Range::Wild,
            _ => Range::Off,
        }
    }

    pub fn bounds(self) -> (i64, i64) {
        match self {
            Range::Wild => WILD,
            _ => MILD,
        }
    }

    pub fn steps(self) -> &'static [i64] {
        match self {
            Range::Wild => WILD_STEPS,
            _ => MILD_STEPS,
        }
    }

    pub fn on(self) -> bool {
        self != Range::Off
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Resized {
    pub map: String,
    pub file: String,
    pub hash: u64,
    pub kind: String,
    pub pct: i64,
    pub at: [f32; 3],
    pub class: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub life_ref: String,
    pub life: String,
    pub max: i32,
}

pub fn class_of(base: &str, pct: i64) -> String {
    format!("{base}{TAIL}{pct}")
}

pub fn split(class: &str) -> Option<(&str, i64)> {
    let at: usize = class.rfind(TAIL)?;
    let pct: i64 = class[at + TAIL.len()..].parse().ok()?;
    let base: &str = &class[..at];
    if base.is_empty() { None } else { Some((base, pct)) }
}

pub fn life_of(max: i32, pct: i64) -> i32 {
    let n: i64 = (max as i64 * pct + 50) / 100;
    n.clamp(1, (SCRIPTED_LIFE - 1) as i64) as i32
}

pub fn hashes(n: &Node, out: &mut Set<u64>) {
    match n {
        Node::Map(m) => m.iter().for_each(|e| hashes(&e.1, out)),
        Node::Arr(a) => a.iter().for_each(|v| hashes(v, out)),
        Node::U64(x) => {
            out.insert(*x);
        }
        Node::I64(x) => {
            out.insert(*x as u64);
        }
        _ => {}
    }
}

pub fn linked(list: &[Node], out: &mut Set<u64>) {
    for a in list {
        if let Some(l) = a.get("Links") {
            hashes(l, out);
        }
    }
}

pub fn held(list: &[Node], links: &Set<u64>, seen: &mut Map<u64, u32>) {
    for h in list.iter().filter_map(hash_of) {
        seen.insert(h, if links.contains(&h) { 1 } else { OWN });
    }
}

pub fn kept(g: &str) -> bool {
    !horde::fighting(g) || KEEP_KINDS.iter().any(|k| g.starts_with(k))
}

pub fn actor_file(name: &str) -> String {
    format!("Actor/{name}.engine__actor__ActorParam.bgyml")
}

pub fn life_ref_file(name: &str) -> String {
    format!("Component/LifeParam/{name}.game__component__LifeParam.bgyml")
}

pub fn life_file(name: &str) -> String {
    format!("Life/LifeParameters/{name}.game__life__LifeParameters.bgyml")
}

pub fn pack_of(name: &str) -> String {
    format!("Pack/Actor/{name}.pack.zs")
}

fn work(p: &str) -> String {
    match p.strip_suffix(".bgyml") {
        Some(s) => format!("Work/{s}.gyml"),
        None => format!("Work/{p}"),
    }
}

fn unwork(p: &str) -> Option<String> {
    let s: &str = p.strip_prefix("Work/")?.strip_suffix(".gyml")?;
    Some(format!("{s}.bgyml"))
}

fn entry<'a>(es: &'a [Entry], p: &str) -> Option<&'a Entry> {
    es.iter().find(|e| e.name == p)
}

pub fn plan(es: &[Entry], base: &str) -> Option<Plan> {
    let mut at: String = actor_file(base);
    let mut life_ref: Option<String> = None;
    for _ in 0..4 {
        let y: Byml = Byml::parse(&entry(es, &at)?.data).ok()?;
        if let Some(r) = y.root.get("Components").and_then(|c| c.get("LifeRef")).and_then(|v| v.as_str()) {
            life_ref = Some(r.strip_prefix('?')?.to_string());
            break;
        }
        at = unwork(y.root.get("$parent")?.as_str()?)?;
    }
    let life_ref: String = life_ref?;
    let c: Byml = Byml::parse(&entry(es, &life_ref)?.data).ok()?;
    let life: String = unwork(c.root.get("LifeParameters")?.as_str()?)?;
    let p: Byml = Byml::parse(&entry(es, &life)?.data).ok()?;
    let Some(Node::I32(max)) = p.root.get("MaxLife") else {
        return None;
    };
    if !(1..SCRIPTED_LIFE).contains(max) {
        return None;
    }
    Some(Plan { life_ref, life, max: *max })
}

pub fn clone_entries(base: &[Entry], name: &str, p: &Plan, pct: i64) -> Res<Vec<Entry>> {
    let class: String = class_of(name, pct);
    if base.iter().any(|e| e.name.contains(&class)) {
        return Err(E::Msg(format!("enemy sizes: the {name} pack already holds files named {class}")));
    }
    let own: &Entry = entry(base, &actor_file(name)).ok_or_else(|| E::Msg(format!("enemy sizes: the {name} pack has no actor file")))?;
    let version: u16 = Byml::parse(&own.data)?.version;
    let ref_name: String = life_ref_file(&class);
    let life_name: String = life_file(&class);
    let mut comp: Byml = Byml::parse(&entry(base, &p.life_ref).ok_or(E::Format("enemy sizes: the life component is gone"))?.data)?;
    comp.root.set("LifeParameters", Node::Str(work(&life_name)));
    let mut life: Byml = Byml::parse(&entry(base, &p.life).ok_or(E::Format("enemy sizes: the life file is gone"))?.data)?;
    life.root.set("MaxLife", Node::I32(life_of(p.max, pct)));
    let actor: Byml = Byml { root: Node::Map(vec![("$parent".to_string(), Node::Str(work(&actor_file(name)))), ("Components".to_string(), Node::Map(vec![("LifeRef".to_string(), Node::Str(format!("?{ref_name}")))]))]), version, big: false };
    let mut out: Vec<Entry> = base.to_vec();
    out.push(Entry { name: actor_file(&class), data: actor.write()? });
    out.push(Entry { name: ref_name, data: comp.write()? });
    out.push(Entry { name: life_name, data: life.write()? });
    Ok(out)
}

pub fn own(class: &str) -> [String; 3] {
    [actor_file(class), life_ref_file(class), life_file(class)]
}

pub fn lane(map: &str) -> bool {
    LANES.iter().any(|l| map.starts_with(l))
}

pub fn cap(map: &str, a: &Node, g: &str) -> i64 {
    let indoors: bool = room(a).is_some();
    let mut c: i64 = i64::MAX;
    if indoors {
        c = c.min(INDOORS_MAX);
    }
    if lane(map) {
        c = c.min(LANE_MAX);
    }
    if horde::big(g) {
        c = c.min(if indoors { BIG_INDOORS_MAX } else { BIG_MAX });
    }
    c
}

pub fn roll(range: Range, top: i64, key: &str) -> i64 {
    let (lo, hi): (i64, i64) = range.bounds();
    let hi: i64 = hi.min(top).max(lo);
    let u: f64 = Rng::from_text(key).below(GRAIN) as f64 / GRAIN as f64;
    let size: f64 = lo as f64 * (hi as f64 / lo as f64).powf(u);
    size.round() as i64
}

pub fn snap(range: Range, pct: i64, top: i64) -> i64 {
    range.steps().iter().copied().filter(|s| *s <= top).min_by_key(|s| (s - pct).abs()).unwrap_or(100)
}

pub fn eligible(a: &Node, seen: &Map<u64, u32>, g: &Guard) -> bool {
    let n: &str = gyaml(a);
    let Some(h) = hash_of(a) else {
        return false;
    };
    let Node::Map(m) = a else {
        return false;
    };
    if kept(n) || pos(a).is_none() || seen.get(&h) != Some(&OWN) || g.barred(a) {
        return false;
    }
    if m.iter().any(|e| matches!(e.0.as_str(), "Scale" | "Links" | "Rails")) {
        return false;
    }
    if let Some(Node::Map(d)) = a.get("Dynamic")
        && d.iter().any(|e| TIED_KEYS.iter().any(|t| e.0.contains(t)) || (e.0.contains("GameData") && !e.0.contains("GameDataAttention")))
    {
        return false;
    }
    match (room(a), pos(a)) {
        (Some(r), _) if g.dungeon => !g.counted.contains(&r),
        (_, Some(p)) => !g.counters.iter().any(|c| (c[0] - p[0]).hypot(c[2] - p[2]) < swap::COUNTED_REACH),
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn size_list(map: &str, file: &str, list: &mut [Node], seen: &Map<u64, u32>, g: &Guard, range: Range, clones: Option<&BTreeSet<String>>, seed: &str) -> Vec<Resized> {
    let mut out: Vec<Resized> = Vec::new();
    for a in list.iter_mut() {
        if !eligible(a, seen, g) {
            continue;
        }
        let Some(h) = hash_of(a) else {
            continue;
        };
        let n: String = gyaml(a).to_string();
        if clones.is_some_and(|c| !c.contains(&n)) {
            continue;
        }
        let top: i64 = cap(map, a, &n);
        let rolled: i64 = roll(range, top, &format!("{seed}|{SALT}|{}", h));
        let pct: i64 = if clones.is_some() { snap(range, rolled, top) } else { rolled };
        if (pct - 100).abs() <= SAME {
            continue;
        }
        let class: Option<String> = clones.map(|_| class_of(&n, pct));
        match &class {
            Some(c) => a.set("Gyaml", Node::Str(c.clone())),
            None => a.set("Scale", floats([pct as f32 / 100.0; 3])),
        };
        out.push(Resized { map: map.to_string(), file: file.to_string(), hash: h, kind: n, pct, at: pos(a).unwrap_or_default(), class });
    }
    out
}

fn pack_entries(rf: &mut Romfs, have: Have, name: &str) -> Option<Vec<Entry>> {
    let p: String = pack_of(name);
    let z: Vec<u8> = match have(&p) {
        Some(b) => b,
        None => rf.read(&p).ok()?,
    };
    sarc::parse(&rf.zs.decompress(&z).ok()?).ok()
}

pub fn clonable(rf: &mut Romfs, have: Have, kinds: &BTreeSet<String>) -> BTreeSet<String> {
    kinds.iter().filter(|k| pack_entries(rf, have, k).is_some_and(|es| plan(&es, k).is_some())).cloned().collect()
}

pub fn made(rf: &mut Romfs, have: Have, seed: &str, range: Range, health: bool) -> Res<Vec<(Out, Vec<Resized>)>> {
    let mut links: Set<u64> = Set::new();
    let mut by: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut acts: BTreeMap<String, Vec<Node>> = BTreeMap::new();
    let mut kinds: BTreeSet<String> = BTreeSet::new();
    for f in files(rf)? {
        let y: Byml = Byml::parse(&raw(rf, have, &f)?.0)?;
        linked(y.root.get("Actors").and_then(|l| l.as_arr()).unwrap_or(&[]), &mut links);
        if !f.ends_with(EXT) || f.starts_with(SKIP_DIR) {
            continue;
        }
        let Some(m) = horde::map_of(&f).filter(|m| !horde::SKIP_PREFIX.iter().any(|p| m.starts_with(p)) && !SKIP_MAPS.contains(&m.as_str())) else {
            continue;
        };
        let list: &[Node] = y.root.get("Actors").and_then(|l| l.as_arr()).unwrap_or(&[]);
        kinds.extend(list.iter().map(|a| gyaml(a).to_string()).filter(|g| !kept(g)));
        acts.entry(m.clone()).or_default().extend(list.iter().cloned());
        by.entry(m).or_default().push(f);
    }
    let mut seen: Map<u64, u32> = Map::new();
    for list in acts.values() {
        held(list, &links, &mut seen);
    }
    let clones: Option<BTreeSet<String>> = if health { Some(clonable(rf, have, &kinds)) } else { None };
    let mut out: Vec<(Out, Vec<Resized>)> = Vec::new();
    for (map, fs) in &by {
        let all: &[Node] = acts.get(map).map(|v| v.as_slice()).unwrap_or(&[]);
        let g: Guard = swap::guard(map, all, &seen);
        for f in fs {
            let (b, id): (Vec<u8>, Option<u32>) = raw(rf, have, f)?;
            let mut y: Byml = Byml::parse(&b)?;
            let Some(list) = y.root.get_mut("Actors").and_then(|l| l.as_arr_mut()) else {
                continue;
            };
            let done: Vec<Resized> = size_list(map, f, list, &seen, &g, range, clones.as_ref(), seed);
            if done.is_empty() {
                continue;
            }
            let new: Vec<u8> = y.write()?;
            out.push((Out { path: f.clone(), old: b.len(), new: new.len(), z: zs::compress_raw(&new, Some(id.unwrap_or(BANC_DICT)))? }, done));
        }
    }
    Ok(out)
}

pub fn summary(all: &[(Out, Vec<Resized>)], health: bool) -> String {
    let total: usize = all.iter().map(|x| x.1.len()).sum();
    let maps: BTreeSet<&str> = all.iter().flat_map(|x| x.1.iter().map(|s| s.map.as_str())).collect();
    let big: usize = all.iter().flat_map(|x| x.1.iter()).filter(|s| s.pct > 100).count();
    let tail: &str = if health { " Health follows size." } else { "" };
    format!("Enemy sizes: {total} enemies resized in {} maps ({big} bigger, {} smaller).{tail}", maps.len(), total - big)
}

pub fn stage(rf: &mut Romfs, have: Have, seed: &str, range: Range, health: bool, notes: &mut Vec<String>) -> Res<Vec<Out>> {
    let all: Vec<(Out, Vec<Resized>)> = made(rf, have, seed, range, health)?;
    notes.push(summary(&all, health));
    Ok(all.into_iter().map(|x| x.0).collect())
}

pub fn used(bytes: &[u8]) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = BTreeSet::new();
    let tail: &[u8] = TAIL.as_bytes();
    let mut i: usize = 0;
    while i + tail.len() < bytes.len() {
        if &bytes[i..i + tail.len()] != tail {
            i += 1;
            continue;
        }
        let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
        let mut s: usize = i;
        while s > 0 && word(bytes[s - 1]) {
            s -= 1;
        }
        let mut e: usize = i + tail.len();
        while e < bytes.len() && bytes[e].is_ascii_digit() {
            e += 1;
        }
        if let Ok(name) = std::str::from_utf8(&bytes[s..e])
            && name.starts_with("Enemy_")
            && split(name).is_some()
        {
            out.insert(name.to_string());
        }
        i = e.max(i + 1);
    }
    out
}

fn walk(dir: &Path, out: &mut Vec<PB>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p: PB = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.to_string_lossy().ends_with(EXT) {
            out.push(p);
        }
    }
}

pub fn classes_in(rf: &mut Romfs, root: &Path) -> Res<BTreeSet<String>> {
    let mut found: Vec<PB> = Vec::new();
    walk(&root.join("Banc"), &mut found);
    walk(&root.join("AoC1"), &mut found);
    let mut out: BTreeSet<String> = BTreeSet::new();
    for p in found {
        out.extend(used(&rf.zs.decompress(&fs::read(&p)?)?));
    }
    Ok(out)
}

fn current(rf: &mut Romfs, have: &dyn Fn(&str) -> Option<Vec<u8>>, p: &str) -> Res<(Vec<u8>, u32)> {
    let z: Vec<u8> = match have(p) {
        Some(z) => z,
        None => rf.read(p)?,
    };
    let id: u32 = dict_id(&z).or_else(|| dict_id(&rf.read(p).unwrap_or_default())).unwrap_or(BANC_DICT);
    Ok((rf.zs.decompress(&z)?, id))
}

pub fn rows(info: &mut Byml, game: &mut Byml, base: &str, class: &str, base_len: usize, len: usize) -> Res<usize> {
    let mut a: Node = rows::need(info, base, ACTOR_INFO)?.clone();
    let h: i32 = heap(rows::own_heap(info, base)?, base_len, len)?;
    let v: Node = match a.get(HEAP) {
        Some(Node::U32(_)) => Node::U32(h.max(0) as u32),
        _ => Node::I32(h),
    };
    a.set(HEAP, v);
    a.set(ROW_ID, Node::Str(class.to_string()));
    let mut g: Node = rows::need(game, base, GAME_ACTOR_INFO)?.clone();
    g.set(ROW_ID, Node::Str(class.to_string()));
    if rows::find(info, class).is_some() || rows::find(game, class).is_some() {
        return Err(E::Msg(format!("enemy sizes: the actor tables already hold {class}")));
    }
    let i: usize = rows::insert(info, a)?;
    let j: usize = rows::insert(game, g)?;
    if i != j {
        return Err(E::Msg(format!("enemy sizes: {class} sits at {i} in the actor table and at {j} in the game actor table")));
    }
    Ok(i)
}

pub fn tags(t: &mut Table, base: &str, class: &str) -> Res<()> {
    let own: Vec<String> = t.tags_of(base);
    if own.is_empty() {
        return Err(E::Msg(format!("enemy sizes: {base} has no tag row, the game files differ")));
    }
    if t.find(class).is_some() {
        return Err(E::Msg(format!("enemy sizes: the tag table already holds {class}")));
    }
    t.insert(class, &own)?;
    Ok(())
}

pub fn build(rf: &mut Romfs, have: &dyn Fn(&str) -> Option<Vec<u8>>, rst: &Rst, classes: &BTreeSet<String>) -> Res<Made> {
    let mut m: Made = Made::default();
    if classes.is_empty() {
        return Ok(m);
    }
    let (iraw, iid): (Vec<u8>, u32) = current(rf, have, ACTOR_INFO)?;
    let (graw, gid): (Vec<u8>, u32) = current(rf, have, GAME_ACTOR_INFO)?;
    let (traw, tid): (Vec<u8>, u32) = current(rf, have, TAG)?;
    let mut info: Byml = Byml::parse(&iraw)?;
    let mut game: Byml = Byml::parse(&graw)?;
    let mut tag: Table = Table::parse(&traw)?;
    let mut by: BTreeMap<&str, Vec<i64>> = BTreeMap::new();
    for c in classes {
        let (b, pct): (&str, i64) = split(c).ok_or_else(|| E::Msg(format!("enemy sizes: {c} is not a size class")))?;
        by.entry(b).or_default().push(pct);
    }
    for (name, pcts) in by {
        if rf.root.join(pack_of(&class_of(name, pcts[0]))).is_file() {
            return Err(E::Msg(format!("enemy sizes: the game already ships {}", class_of(name, pcts[0]))));
        }
        let bz: Vec<u8> = match have(&pack_of(name)) {
            Some(z) => z,
            None => rf.read(&pack_of(name))?,
        };
        let braw: Vec<u8> = rf.zs.decompress(&bz)?;
        let base: Vec<Entry> = sarc::parse(&braw)?;
        let pid: u32 = dict_id(&bz).unwrap_or(PACK_DICT);
        let p: Plan = plan(&base, name).ok_or_else(|| E::Msg(format!("enemy sizes: {name} has no life file of its own, the game files differ")))?;
        let near: Vec<(String, usize)> = base.iter().map(|e| (e.name.clone(), e.data.len())).collect();
        for pct in pcts {
            let class: String = class_of(name, pct);
            let es: Vec<Entry> = clone_entries(&base, name, &p, pct)?;
            let packed: Vec<u8> = sarc::write(&es)?;
            for f in own(&class) {
                let len: usize = entry(&es, &f).map(|e| e.data.len()).unwrap_or(0);
                m.sizes.push((f.clone(), file_size(rst, &near, &f, len)?));
            }
            m.sizes.push((pack_of(&class), pack_size(packed.len())?));
            m.outs.push(Out { path: pack_of(&class), z: zs::compress_raw(&packed, Some(pid))?, old: 0, new: packed.len() });
            rows(&mut info, &mut game, name, &class, braw.len(), packed.len())?;
            tags(&mut tag, name, &class)?;
        }
    }
    for (p, y, old, id) in [(ACTOR_INFO, &info, iraw.len(), iid), (GAME_ACTOR_INFO, &game, graw.len(), gid)] {
        let out: Vec<u8> = y.write()?;
        m.outs.push(Out { path: p.to_string(), z: zs::compress_raw(&out, Some(id))?, old, new: out.len() });
    }
    let tout: Vec<u8> = tag.write()?;
    m.outs.push(Out { path: TAG.to_string(), z: zs::compress_raw(&tout, Some(tid))?, old: traw.len(), new: tout.len() });
    Ok(m)
}

pub fn write(rf: &mut Romfs, pk: &Pack, rst: &mut Option<Rst>) -> Res<(usize, Vec<String>)> {
    let root: PB = pk.work.join("romfs");
    let classes: BTreeSet<String> = classes_in(rf, &root)?;
    if classes.is_empty() {
        return Ok((0, Vec::new()));
    }
    if rst.is_none() {
        *rst = Some(rsizetable::load(rf)?);
    }
    let t: &mut Rst = rst.as_mut().ok_or(E::Format("enemy sizes: no size table"))?;
    let have = |p: &str| -> Option<Vec<u8>> { fs::read(p.split('/').fold(root.clone(), |a, s| a.join(s))).ok() };
    let m: Made = build(rf, &have, t, &classes)?;
    for o in &m.outs {
        pk.write_romfs(&o.path, &o.z)?;
    }
    leevers::sized(rf, t, &m)?;
    Ok((m.outs.len(), vec![format!("Enemy sizes: {} sized copies of enemies added", classes.len())]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: [f32; 3] = [100.0, 10.0, 100.0];

    fn foe(g: &str, h: u64, p: [f32; 3], room: Option<i32>, dynamic: Vec<(&str, Node)>) -> Node {
        let mut d: Vec<(String, Node)> = dynamic.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        if let Some(r) = room {
            d.push((crate::difficulty::extras::ROOM.to_string(), Node::I32(r)));
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

    fn seen_of(list: &[Node]) -> Map<u64, u32> {
        let mut links: Set<u64> = Set::new();
        linked(list, &mut links);
        let mut out: Map<u64, u32> = Map::new();
        held(list, &links, &mut out);
        out
    }

    fn run(map: &str, list: &mut [Node], range: Range, clones: Option<&BTreeSet<String>>, seed: &str) -> Vec<Resized> {
        let s: Map<u64, u32> = seen_of(list);
        let g: Guard = swap::guard(map, list, &s);
        size_list(map, "f", list, &s, &g, range, clones, seed)
    }

    fn many(n: u64, g: &str, room: Option<i32>) -> Vec<Node> {
        (0..n).map(|i| foe(g, 0x9_0000_0000 + (i << 8), [i as f32 * 30.0, 10.0, 0.0], room, Vec::new())).collect()
    }

    fn scale_of(a: &Node) -> Option<f32> {
        match a.get("Scale")?.as_arr()? {
            [Node::F32(x), Node::F32(y), Node::F32(z)] if x == y && y == z => Some(*x),
            _ => None,
        }
    }

    #[test]
    fn mild_and_wild_stay_in_their_ranges_and_the_small_sizes_are_commoner() {
        for (r, lo, hi) in [(Range::Mild, 75, 150), (Range::Wild, 50, 250)] {
            let mut list: Vec<Node> = many(600, "Enemy_Wolfos", None);
            let done: Vec<Resized> = run("HyruleField_Adult", &mut list, r, None, "a");
            assert!(done.len() > 500, "{}", done.len());
            assert!(done.iter().all(|s| s.pct >= lo && s.pct <= hi), "{r:?}");
            let mut mean: f64 = 0.0;
            for s in &done {
                mean += s.pct as f64 / 100.0;
            }
            mean /= done.len() as f64;
            assert!(mean < (lo + hi) as f64 / 200.0, "{r:?} {mean}");
            assert!(done.iter().any(|s| s.pct < 100) && done.iter().any(|s| s.pct > 100));
        }
        let mut keese: Vec<Node> = many(600, "Enemy_Keese", None);
        let done: Vec<Resized> = run("HyruleField_Adult", &mut keese, Range::Wild, None, "a");
        assert!(done.iter().map(|s| s.pct).max().unwrap() >= 240 && done.iter().map(|s| s.pct).min().unwrap() <= 52);
        assert_eq!(keese.iter().filter(|a| scale_of(a).is_some()).count(), done.len());
    }

    #[test]
    fn the_same_seed_gives_the_same_sizes_and_another_seed_gives_others() {
        let base: Vec<Node> = many(50, "Enemy_Keese", None);
        let (mut a, mut b, mut c): (Vec<Node>, Vec<Node>, Vec<Node>) = (base.clone(), base.clone(), base.clone());
        run("HyruleField_Adult", &mut a, Range::Wild, None, "x");
        run("HyruleField_Adult", &mut b, Range::Wild, None, "x");
        run("HyruleField_Adult", &mut c, Range::Wild, None, "y");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn bosses_counted_linked_and_scripted_enemies_are_left_alone() {
        let mut list: Vec<Node> = vec![
            foe("Enemy_MiddleBoss_Lizalfos", 0x1_0000_0001, P, None, Vec::new()),
            foe("Enemy_DungeonBoss_Gohma", 0x1_0000_0002, [500.0, 0.0, 0.0], None, Vec::new()),
            foe("Enemy_Tentacle_Blue", 0x1_0000_0003, [900.0, 0.0, 0.0], None, Vec::new()),
            foe("Enemy_Poe_Big", 0x1_0000_0004, [1000.0, 0.0, 0.0], None, Vec::new()),
            foe("Enemy_Stalfos", 0x1_0000_0005, [1100.0, 0.0, 0.0], None, vec![("TalkEventName", Node::Str("x".to_string()))]),
            foe("Enemy_Stalfos", 0x1_0000_0006, [1200.0, 0.0, 0.0], None, vec![("ScrubBusiness_Ev", Node::Bool(true)), ("Drop__DropTable", Node::Str("None".to_string()))]),
            foe("Enemy_Skullwalltula", 0x1_0000_0007, [1300.0, 0.0, 0.0], None, Vec::new()),
            foe("Obj_EnemyCounter_A", 0x1_0000_0008, [2000.0, 0.0, 0.0], None, Vec::new()),
            foe("Enemy_Keese", 0x1_0000_0009, [2010.0, 0.0, 0.0], None, Vec::new()),
            foe("Enemy_Keese", 0x1_0000_000a, [3000.0, 0.0, 0.0], Some(4), Vec::new()),
        ];
        list.push(foe("Obj_Switch", 0x1_0000_000b, [0.0, 0.0, 0.0], None, Vec::new()));
        let before: Vec<Node> = list.clone();
        for seed in ["a", "b", "c", "d", "e", "f"] {
            let mut l: Vec<Node> = before.clone();
            run("HyruleField_Adult", &mut l, Range::Wild, None, seed);
            for (i, (x, y)) in l.iter().zip(before.iter()).enumerate() {
                if i != 5 && i != 9 {
                    assert_eq!(x, y, "{i} {seed}");
                }
            }
        }
        let mut l: Vec<Node> = before.clone();
        let done: Vec<Resized> = run("HyruleField_Adult", &mut l, Range::Wild, None, "z");
        assert!(done.iter().all(|s| s.hash == 0x1_0000_0006 || s.hash == 0x1_0000_000a), "{done:?}");
    }

    #[test]
    fn a_room_with_a_counted_enemy_keeps_every_enemy_in_it() {
        let mut list: Vec<Node> = many(40, "Enemy_Keese", Some(7));
        let mut linked: Node = foe("Enemy_Stalfos", 0x5_0000_0001, P, Some(7), Vec::new());
        if let Node::Map(m) = &mut linked {
            m.push(("Links".to_string(), Node::Arr(Vec::new())));
        }
        list.push(linked);
        list.push(foe("Obj_EnemyCounter_A", 0x5_0000_0002, P, Some(7), Vec::new()));
        list.extend(many(40, "Enemy_Biri", Some(8)).into_iter().map(|mut a| {
            a.set("Hash", Node::U64(hash_of(&a).unwrap() + 0x7_0000_0000));
            a
        }));
        let before: Vec<Node> = list.clone();
        run("ForestTemple", &mut list, Range::Wild, None, "a");
        assert_eq!(&list[..42], &before[..42]);
        assert!(list[42..].iter().any(|a| a.get("Scale").is_some()));
    }

    #[test]
    fn caps_keep_big_rolls_out_of_dungeons_lanes_and_the_big_kinds() {
        let mut dungeon: Vec<Node> = many(400, "Enemy_Keese", Some(3));
        let done: Vec<Resized> = run("ForestTemple", &mut dungeon, Range::Wild, None, "a");
        assert!(!done.is_empty() && done.iter().all(|s| s.pct <= INDOORS_MAX && s.pct >= 50));
        let mut lanes: Vec<Node> = many(400, "Enemy_Keese", None);
        assert!(run("LostWood_Child", &mut lanes, Range::Wild, None, "a").iter().all(|s| s.pct <= LANE_MAX));
        let mut big: Vec<Node> = many(400, "Enemy_Wolfos", None);
        assert!(run("HyruleField_Adult", &mut big, Range::Wild, None, "a").iter().all(|s| s.pct <= BIG_MAX));
        let mut tight: Vec<Node> = many(400, "Enemy_Stalfos", Some(3));
        assert!(run("ForestTemple", &mut tight, Range::Wild, None, "a").iter().all(|s| s.pct <= BIG_INDOORS_MAX));
        let mut open: Vec<Node> = many(400, "Enemy_Keese", None);
        assert!(run("HyruleField_Adult", &mut open, Range::Wild, None, "a").iter().any(|s| s.pct > 200));
    }

    #[test]
    fn with_health_following_size_the_sizes_snap_to_copies_and_only_known_kinds_change() {
        let known: BTreeSet<String> = ["Enemy_Keese".to_string()].into_iter().collect();
        let mut list: Vec<Node> = many(200, "Enemy_Keese", None);
        list.extend(many(50, "Enemy_Biri", None).into_iter().map(|mut a| {
            a.set("Hash", Node::U64(hash_of(&a).unwrap() + 0x7_0000_0000));
            a
        }));
        let done: Vec<Resized> = run("HyruleField_Adult", &mut list, Range::Wild, Some(&known), "a");
        assert!(!done.is_empty());
        for s in &done {
            assert!(WILD_STEPS.contains(&s.pct) && s.pct != 100 && s.kind == "Enemy_Keese");
            assert_eq!(s.class.as_deref(), Some(class_of("Enemy_Keese", s.pct).as_str()));
            assert_eq!(split(s.class.as_deref().unwrap()), Some(("Enemy_Keese", s.pct)));
        }
        assert!(list[200..].iter().all(|a| gyaml(a) == "Enemy_Biri" && a.get("Scale").is_none()));
        assert!(list[..200].iter().all(|a| a.get("Scale").is_none()));
        let names: BTreeSet<&str> = list.iter().map(gyaml).collect();
        assert!(names.contains("Enemy_Keese") && names.iter().any(|n| n.starts_with("Enemy_Keese_Size")));
    }

    #[test]
    fn life_follows_size_rounded_at_least_one_and_under_the_scripted_value() {
        assert_eq!(life_of(18, 50), 9);
        assert_eq!(life_of(18, 250), 45);
        assert_eq!(life_of(1, 50), 1);
        assert_eq!(life_of(1, 150), 2);
        assert_eq!(life_of(3, 75), 2);
        assert_eq!(life_of(90, 250), 99);
        assert_eq!(life_of(18, 100), 18);
    }

    #[test]
    fn class_names_round_trip_and_the_game_names_are_not_taken_for_them() {
        assert_eq!(split("Enemy_Stalfos_AlwaysRespawn_Size150"), Some(("Enemy_Stalfos_AlwaysRespawn", 150)));
        assert_eq!(split("Enemy_Stalfos"), None);
        assert_eq!(split("Enemy_Size"), None);
        let bytes: Vec<u8> = b"\0Enemy_Keese_Size75\0Enemy_Biri\0Enemy_Stalfos_Size250\0Obj_Size3\0".to_vec();
        let u: BTreeSet<String> = used(&bytes);
        assert_eq!(u.into_iter().collect::<Vec<String>>(), vec!["Enemy_Keese_Size75".to_string(), "Enemy_Stalfos_Size250".to_string()]);
    }

    #[test]
    fn the_settings_map_to_ranges() {
        assert_eq!((Range::from_setting("off"), Range::from_setting("mild"), Range::from_setting("wild")), (Range::Off, Range::Mild, Range::Wild));
        assert!(!Range::Off.on() && Range::Mild.on() && Range::Wild.on());
        assert_eq!((Range::Mild.bounds(), Range::Wild.bounds()), ((75, 150), (50, 250)));
        assert!(Range::Mild.steps().contains(&75) && Range::Wild.steps().contains(&250));
        assert_eq!(snap(Range::Wild, 120, 250), 100);
        assert_eq!(snap(Range::Wild, 240, 250), 250);
        assert_eq!(snap(Range::Wild, 240, 150), 150);
    }

    fn dump() -> Option<PB> {
        let p: PB = std::env::var_os("OOTRR_ROMFS").map(PB::from)?;
        if p.is_dir() { Some(p) } else { None }
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn the_game_maps_get_mixed_sizes_and_nothing_excluded_changes() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        for (range, health) in [(Range::Mild, false), (Range::Wild, false), (Range::Wild, true)] {
            let all: Vec<(Out, Vec<Resized>)> = made(&mut rf, &none, "game", range, health).unwrap();
            eprintln!("{range:?} {health}: {}", summary(&all, health));
            let total: usize = all.iter().map(|x| x.1.len()).sum();
            assert!(total > 150, "{total}");
            let (lo, hi): (i64, i64) = range.bounds();
            let mut kinds: BTreeMap<String, (usize, i64, i64)> = BTreeMap::new();
            for s in all.iter().flat_map(|x| x.1.iter()) {
                assert!(!kept(&s.kind), "{s:?}");
                assert!(s.pct >= lo && s.pct <= hi, "{s:?}");
                let e: &mut (usize, i64, i64) = kinds.entry(s.kind.clone()).or_insert((0, 999, 0));
                e.0 += 1;
                e.1 = e.1.min(s.pct);
                e.2 = e.2.max(s.pct);
            }
            eprintln!("{kinds:?}");
            assert!(kinds.len() >= 15, "{}", kinds.len());
            if health {
                let used: BTreeSet<String> = all.iter().flat_map(|x| x.1.iter().filter_map(|s| s.class.clone())).collect();
                eprintln!("{} classes", used.len());
                assert!(!used.is_empty());
            }
        }
        let off: Vec<(Out, Vec<Resized>)> = made(&mut rf, &none, "game", Range::Mild, false).unwrap();
        let again: Vec<(Out, Vec<Resized>)> = made(&mut rf, &none, "game", Range::Mild, false).unwrap();
        assert_eq!(off.iter().map(|x| x.0.new).collect::<Vec<usize>>(), again.iter().map(|x| x.0.new).collect::<Vec<usize>>());
    }

    #[test]
    fn every_clonable_kind_builds_its_copies_with_scaled_life_and_size_table_entries() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let rst: Rst = rsizetable::load(&mut rf).unwrap();
        let all: Vec<(Out, Vec<Resized>)> = made(&mut rf, &none, "game", Range::Wild, true).unwrap();
        let classes: BTreeSet<String> = all.iter().flat_map(|x| x.1.iter().filter_map(|s| s.class.clone())).collect();
        let m: Made = build(&mut rf, &none, &rst, &classes).unwrap();
        assert_eq!(m.outs.len(), classes.len() + 3);
        assert_eq!(m.sizes.len(), classes.len() * 4);
        let mut kinds: BTreeSet<String> = BTreeSet::new();
        for c in &classes {
            let (b, pct): (&str, i64) = split(c).unwrap();
            kinds.insert(b.to_string());
            let base: Vec<Entry> = sarc::parse(&rf.read_zs(&pack_of(b)).unwrap()).unwrap();
            let p: Plan = plan(&base, b).unwrap();
            let es: Vec<Entry> = clone_entries(&base, b, &p, pct).unwrap();
            assert_eq!(es.len(), base.len() + 3);
            let y: Byml = Byml::parse(&entry(&es, &life_file(c)).unwrap().data).unwrap();
            assert_eq!(y.root.get("MaxLife"), Some(&Node::I32(life_of(p.max, pct))));
            let a: Byml = Byml::parse(&entry(&es, &actor_file(c)).unwrap().data).unwrap();
            assert_eq!(a.root.get("$parent").and_then(|v| v.as_str()), Some(work(&actor_file(b)).as_str()));
            let r: Byml = Byml::parse(&entry(&es, &life_ref_file(c)).unwrap().data).unwrap();
            assert_eq!(r.root.get("LifeParameters").and_then(|v| v.as_str()), Some(work(&life_file(c)).as_str()));
        }
        eprintln!("{} kinds, {} classes, {} bytes of packs", kinds.len(), classes.len(), m.outs.iter().filter(|o| o.path.ends_with(".pack.zs")).map(|o| o.z.len()).sum::<usize>());
    }
}

#[cfg(test)]
mod where_tests {
    use super::*;

    #[test]
    #[ignore]
    fn list() {
        let Some(p) = std::env::var_os("OOTRR_ROMFS").map(PB::from) else { return };
        let mut rf: Romfs = Romfs::open(&p).unwrap();
        let none = |_: &str| -> Option<Vec<u8>> { None };
        let want: Vec<String> = std::env::var("SIZE_MAPS").unwrap_or_default().split(',').map(|x| x.to_string()).collect();
        let health: bool = std::env::var("SIZE_HEALTH").is_ok();
        for (_, v) in made(&mut rf, &none, "sizes-seed", Range::Wild, health).unwrap() {
            for s in v.iter().filter(|s| want.contains(&s.map)) {
                println!("SIZED {} {} {} {:.1} {:.1} {:.1}", s.map, s.kind, s.pct, s.at[0], s.at[1], s.at[2]);
            }
        }
    }
}
