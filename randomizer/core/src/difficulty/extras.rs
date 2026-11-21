use std::collections::HashMap as Map;

use crate::byml::{Byml, Node};
use crate::error::Res;
use crate::flow_locations::dict_id;
use crate::placement::hash_of;
use crate::qol::Have;
use crate::rng::Rng;
use crate::romfs::Romfs;
use crate::staging::{Out, BANC_DICT};
use crate::zs;

pub const SALT: &str = "more-enemies";
pub const SCENES: &str = "Banc";
pub const MORE_SCENES: &str = "AoC1";
pub const ANY_EXT: &str = ".byml.zs";
pub const EXT: &str = ".bcett.byml.zs";
pub const SKIP_DIR: &str = "Banc/DreamWorld/";
pub const ROOM: &str = "DefaultDungeonRoomIndex";
pub const KEYS: &[&str] = &["Dynamic", "Gyaml", "Hash", "Phive", "Rotate", "SRTHash", "Translate"];
pub const TIED: &[&str] = &["Event", "GameData", "Drop__"];
pub const OWN: u32 = 2;
pub const FULL: i64 = 100;
pub const NEAR: f32 = 1.0;
pub const FAR: f32 = 1.5;
pub const MID: f32 = 0.5;
pub const SWAY: f32 = std::f32::consts::FRAC_PI_2;
pub const GRAIN: u64 = 1 << 20;

pub const KINDS: &[(&str, f32)] = &[
    ("Enemy_BabyDodongo", 0.0),
    ("Enemy_Biri", 0.0),
    ("Enemy_Bubble_Green", 0.0),
    ("Enemy_Bubble_Red", 0.0),
    ("Enemy_Dekubaba", 0.0),
    ("Enemy_Dekubaba_Withered", 0.0),
    ("Enemy_Dodongo", 0.0),
    ("Enemy_Gibdo", 0.0),
    ("Enemy_Guay_Fixed", 0.0),
    ("Enemy_Keese", 0.0),
    ("Enemy_Keese_Fire", 0.0),
    ("Enemy_Keese_Ice", 0.0),
    ("Enemy_LikeLike", 0.0),
    ("Enemy_Octorok", 0.0),
    ("Enemy_Poe", 0.0),
    ("Enemy_ReDead", 0.0),
    ("Enemy_Stinger", 0.0),
    ("Enemy_Tailpasaran", 0.0),
    ("Enemy_Tektite_Blue", 0.3),
    ("Enemy_Tektite_Red", 0.3),
    ("Enemy_TorchSlug", 0.0),
];

#[derive(Clone, Debug, PartialEq)]
pub struct Made {
    pub gyaml: String,
    pub from: u64,
    pub hash: u64,
    pub pos: [f32; 3],
}

pub fn lift(gyaml: &str) -> Option<f32> {
    KINDS.iter().find(|k| k.0 == gyaml).map(|k| k.1)
}

pub fn tally(n: &Node, out: &mut Map<u64, u32>) {
    match n {
        Node::Map(m) => m.iter().for_each(|e| tally(&e.1, out)),
        Node::Arr(a) => a.iter().for_each(|v| tally(v, out)),
        Node::U64(x) => *out.entry(*x).or_insert(0) += 1,
        Node::I64(x) => *out.entry(*x as u64).or_insert(0) += 1,
        _ => {}
    }
}

pub fn gyaml(a: &Node) -> &str {
    a.get("Gyaml").and_then(|v| v.as_str()).unwrap_or("")
}

pub fn pos(a: &Node) -> Option<[f32; 3]> {
    match a.get("Translate")?.as_arr()? {
        [Node::F32(x), Node::F32(y), Node::F32(z)] => Some([*x, *y, *z]),
        _ => None,
    }
}

pub fn room(a: &Node) -> Option<i32> {
    match a.get("Dynamic")?.get(ROOM)? {
        Node::I32(r) => Some(*r),
        _ => None,
    }
}

pub fn own_body(a: &Node, h: u64) -> bool {
    let want: Node = Node::Map(vec![("Placement".to_string(), Node::Map(vec![("ID".to_string(), Node::U64(h))]))]);
    a.get("Phive") == Some(&want)
}

pub fn tied(a: &Node) -> bool {
    match a.get("Dynamic") {
        Some(Node::Map(m)) => m.iter().any(|e| TIED.iter().any(|t| e.0.contains(t))),
        Some(_) => true,
        None => false,
    }
}

pub fn plain(a: &Node, seen: &Map<u64, u32>) -> Option<f32> {
    let up: f32 = lift(gyaml(a))?;
    let h: u64 = hash_of(a)?;
    let Node::Map(m) = a else {
        return None;
    };
    if !m.iter().all(|e| KEYS.contains(&e.0.as_str())) || !own_body(a, h) || tied(a) || seen.get(&h) != Some(&OWN) {
        return None;
    }
    pos(a).map(|_| up)
}

fn unit(rng: &mut Rng) -> f32 {
    rng.below(GRAIN) as f32 / GRAIN as f32
}

fn centre(list: &[Node], r: i32) -> Option<(f32, f32)> {
    let ps: Vec<[f32; 3]> = list.iter().filter(|a| room(a) == Some(r)).filter_map(pos).collect();
    if ps.is_empty() {
        return None;
    }
    let n: f32 = ps.len() as f32;
    Some((ps.iter().map(|p| p[0]).sum::<f32>() / n, ps.iter().map(|p| p[2]).sum::<f32>() / n))
}

pub fn spot(at: [f32; 3], up: f32, mid: Option<(f32, f32)>, rng: &mut Rng) -> [f32; 3] {
    let turn: f32 = unit(rng);
    let far: f32 = NEAR + unit(rng) * (FAR - NEAR);
    let aim: Option<f32> = mid.map(|c| (c.0 - at[0], c.1 - at[2])).filter(|d| d.0.hypot(d.1) > MID).map(|d| d.1.atan2(d.0));
    let ang: f32 = match aim {
        Some(a) => a + (turn - 0.5) * SWAY,
        None => turn * std::f32::consts::TAU,
    };
    [at[0] + far * ang.cos(), at[1] + up, at[2] + far * ang.sin()]
}

pub fn fresh(seen: &mut Map<u64, u32>, rng: &mut Rng) -> u64 {
    loop {
        let h: u64 = rng.next_u64();
        if h > u32::MAX as u64 && !seen.contains_key(&h) {
            seen.insert(h, OWN);
            return h;
        }
    }
}

fn copy(a: &Node, h: u64, srt: u32, p: [f32; 3]) -> Node {
    let mut out: Node = a.clone();
    out.set("Hash", Node::U64(h));
    out.set("Phive", Node::Map(vec![("Placement".to_string(), Node::Map(vec![("ID".to_string(), Node::U64(h))]))]));
    out.set("SRTHash", Node::U32(srt));
    out.set("Translate", Node::Arr(p.iter().map(|v| Node::F32(*v)).collect()));
    out
}

pub fn add(y: &mut Byml, seen: &mut Map<u64, u32>, rng: &mut Rng, pct: i64) -> Vec<Made> {
    add_in(y, seen, rng, pct, &[])
}

pub fn add_in(y: &mut Byml, seen: &mut Map<u64, u32>, rng: &mut Rng, pct: i64, skip: &[&str]) -> Vec<Made> {
    let mut out: Vec<Made> = Vec::new();
    let Some(list) = y.root.get_mut("Actors").and_then(|l| l.as_arr_mut()) else {
        return out;
    };
    let mut more: Vec<Node> = Vec::new();
    for a in list.iter() {
        let Some(up) = plain(a, seen) else {
            continue;
        };
        if skip.contains(&gyaml(a)) {
            continue;
        }
        if rng.below(100) as i64 >= pct {
            continue;
        }
        let (Some(from), Some(at)) = (hash_of(a), pos(a)) else {
            continue;
        };
        let mid: Option<(f32, f32)> = room(a).and_then(|r| centre(list, r));
        let p: [f32; 3] = spot(at, up, mid, rng);
        let hash: u64 = fresh(seen, rng);
        let srt: u32 = rng.next_u64() as u32;
        more.push(copy(a, hash, srt, p));
        out.push(Made { gyaml: gyaml(a).to_string(), from, hash, pos: p });
    }
    list.extend(more);
    out
}

pub fn raw(rf: &mut Romfs, have: Have, path: &str) -> Res<(Vec<u8>, Option<u32>)> {
    let z: Vec<u8> = match have(path) {
        Some(b) => b,
        None => rf.read(path)?,
    };
    Ok((rf.zs.decompress(&z)?, dict_id(&z)))
}

pub fn files(rf: &Romfs) -> Res<Vec<String>> {
    let mut out: Vec<String> = rf.list(SCENES)?;
    out.extend(rf.list(MORE_SCENES)?);
    out.retain(|f| f.ends_with(ANY_EXT));
    out.sort();
    Ok(out)
}

pub fn made(rf: &mut Romfs, have: Have, seed: &str, pct: i64) -> Res<Vec<(Out, Vec<Made>)>> {
    let mut seen: Map<u64, u32> = Map::new();
    let mut todo: Vec<String> = Vec::new();
    for f in files(rf)? {
        let y: Byml = Byml::parse(&raw(rf, have, &f)?.0)?;
        tally(&y.root, &mut seen);
        let held: bool = y.root.get("Actors").and_then(|l| l.as_arr()).unwrap_or(&[]).iter().any(|a| lift(gyaml(a)).is_some());
        if held && f.ends_with(EXT) && !f.starts_with(SKIP_DIR) {
            todo.push(f);
        }
    }
    let mut out: Vec<(Out, Vec<Made>)> = Vec::new();
    for f in todo {
        let (b, id): (Vec<u8>, Option<u32>) = raw(rf, have, &f)?;
        let mut y: Byml = Byml::parse(&b)?;
        let mut rng: Rng = Rng::from_text(&format!("{seed}|{SALT}|{f}"));
        let skip: &[&str] = if crate::difficulty::horde::NO_BLASTERS.iter().any(|m| f.contains(m)) { crate::difficulty::horde::BLASTERS } else { &[] };
        let ms: Vec<Made> = add_in(&mut y, &mut seen, &mut rng, pct, skip);
        if ms.is_empty() {
            continue;
        }
        let new: Vec<u8> = y.write()?;
        out.push((Out { path: f, old: b.len(), new: new.len(), z: zs::compress_raw(&new, Some(id.unwrap_or(BANC_DICT)))? }, ms));
    }
    Ok(out)
}

pub fn stage(rf: &mut Romfs, have: Have, seed: &str, pct: i64) -> Res<Vec<Out>> {
    Ok(made(rf, have, seed, pct)?.into_iter().map(|x| x.0).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn foe(g: &str, h: u64, x: f32, z: f32, room: Option<i32>) -> Node {
        let mut m: Vec<(String, Node)> = Vec::new();
        if let Some(r) = room {
            m.push(("Dynamic".to_string(), Node::Map(vec![(ROOM.to_string(), Node::I32(r))])));
        }
        m.push(("Gyaml".to_string(), Node::Str(g.to_string())));
        m.push(("Hash".to_string(), Node::U64(h)));
        m.push(("Phive".to_string(), Node::Map(vec![("Placement".to_string(), Node::Map(vec![("ID".to_string(), Node::U64(h))]))])));
        m.push(("SRTHash".to_string(), Node::U32(h as u32)));
        m.push(("Translate".to_string(), Node::Arr(vec![Node::F32(x), Node::F32(100.0), Node::F32(z)])));
        Node::Map(m)
    }

    fn with(mut a: Node, k: &str, v: Node) -> Node {
        a.set(k, v);
        a
    }

    fn link(src: u64, dst: u64) -> Node {
        Node::Arr(vec![Node::Map(vec![("Dst".to_string(), Node::U64(dst)), ("Name".to_string(), Node::Str("Reference".to_string())), ("Src".to_string(), Node::U64(src))])])
    }

    fn scene(actors: Vec<Node>, groups: Vec<u64>) -> Byml {
        let refs: Vec<Node> = groups.into_iter().map(|h| Node::Map(vec![("Reference".to_string(), Node::U64(h))])).collect();
        let g: Node = Node::Arr(vec![Node::Map(vec![("References".to_string(), Node::Arr(refs))])]);
        Byml { root: Node::Map(vec![("Actors".to_string(), Node::Arr(actors)), ("AiGroups".to_string(), g)]), version: 7, big: false }
    }

    const A: u64 = 0x1_0000_0001;
    const B: u64 = 0x1_0000_0002;
    const C: u64 = 0x1_0000_0003;
    const D: u64 = 0x1_0000_0004;
    const E: u64 = 0x1_0000_0005;
    const F: u64 = 0x1_0000_0006;
    const G: u64 = 0x1_0000_0007;
    const H: u64 = 0x1_0000_0008;
    const I: u64 = 0x1_0000_0009;

    fn sample() -> Byml {
        let drop: Node = Node::Map(vec![("Drop__DropActor".to_string(), Node::Str("Item_Dungeon_SmallKey".to_string()))]);
        let talk: Node = Node::Map(vec![("AttackedEventName".to_string(), Node::Str("x".to_string()))]);
        scene(
            vec![
                foe("Enemy_Keese", A, 10.0, 10.0, Some(3)),
                foe("Enemy_Keese", B, 0.0, 0.0, Some(3)),
                foe("Enemy_Skulltula_Gold", C, 1.0, 1.0, Some(3)),
                foe("Enemy_Stalfos", D, 2.0, 2.0, Some(3)),
                with(foe("Enemy_Keese", E, 3.0, 3.0, None), "Links", link(E, D)),
                foe("Enemy_ReDead", F, 4.0, 4.0, None),
                with(foe("Enemy_ReDead", G, 5.0, 5.0, None), "Dynamic", drop),
                with(foe("Enemy_ReDead", H, 6.0, 6.0, None), "Dynamic", talk),
                with(foe("Obj_EnemyCounter_A_09", 0x2_0000_0000, 7.0, 7.0, None), "Links", link(0x2_0000_0000, F)),
                with(foe("Enemy_Tektite_Blue", I, 8.0, 8.0, None), "Presence", Node::Map(Vec::new())),
            ],
            vec![B],
        )
    }

    fn seen(y: &Byml) -> Map<u64, u32> {
        let mut out: Map<u64, u32> = Map::new();
        tally(&y.root, &mut out);
        out
    }

    #[test]
    fn only_an_enemy_that_nothing_points_at_is_copied() {
        let mut y: Byml = sample();
        let mut s: Map<u64, u32> = seen(&y);
        let was: Vec<Node> = y.root.get("Actors").unwrap().as_arr().unwrap().to_vec();
        let ms: Vec<Made> = add(&mut y, &mut s, &mut Rng::from_text("t"), 100);
        assert_eq!(ms.iter().map(|m| m.from).collect::<Vec<u64>>(), vec![A]);
        let now: &[Node] = y.root.get("Actors").unwrap().as_arr().unwrap();
        assert_eq!(&now[..was.len()], &was[..]);
        assert_eq!(now.len(), was.len() + 1);
        let c: &Node = &now[was.len()];
        assert_eq!(gyaml(c), "Enemy_Keese");
        assert_eq!(hash_of(c), Some(ms[0].hash));
        assert!(own_body(c, ms[0].hash));
        assert_eq!(room(c), Some(3));
        assert!(ms[0].hash > u32::MAX as u64 && ![A, B, C, D, E, F, G, H, I].contains(&ms[0].hash));
        assert_ne!(c.get("SRTHash"), was[0].get("SRTHash"));
        assert_eq!(seen(&y).get(&ms[0].hash), Some(&OWN));
    }

    #[test]
    fn a_copy_stands_close_by_and_leans_into_its_room() {
        let mut y: Byml = sample();
        let mut s: Map<u64, u32> = seen(&y);
        let ms: Vec<Made> = add(&mut y, &mut s, &mut Rng::from_text("lean"), 100);
        let p: [f32; 3] = ms[0].pos;
        let d: f32 = (p[0] - 10.0).hypot(p[2] - 10.0);
        assert!((NEAR - 0.001..=FAR + 0.001).contains(&d), "{d}");
        assert_eq!(p[1], 100.0);
        assert!(p[0] < 10.0 && p[2] < 10.0, "the room centre is toward the origin, got {p:?}");
        let mut rng: Rng = Rng::from_text("free");
        for _ in 0..50 {
            let q: [f32; 3] = spot([1.0, 2.0, 3.0], 0.3, None, &mut rng);
            let e: f32 = (q[0] - 1.0).hypot(q[2] - 3.0);
            assert!((NEAR - 0.001..=FAR + 0.001).contains(&e));
            assert!((q[1] - 2.3).abs() < 0.0001);
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_copies_and_the_share_is_honoured() {
        let many = || -> Byml { scene((0..200).map(|i| foe("Enemy_Keese", 0x5_0000_0000 + i, i as f32 * 4.0, 0.0, None)).collect(), Vec::new()) };
        let run = |seed: &str, pct: i64| -> (Byml, Vec<Made>) {
            let mut y: Byml = many();
            let mut s: Map<u64, u32> = seen(&y);
            let ms: Vec<Made> = add(&mut y, &mut s, &mut Rng::from_text(seed), pct);
            (y, ms)
        };
        assert_eq!(run("a", 50), run("a", 50));
        assert_ne!(run("a", 50).1, run("b", 50).1);
        assert_eq!(run("a", 100).1.len(), 200);
        assert!(run("a", 0).1.is_empty());
        let half: usize = run("a", 50).1.len();
        assert!((70..=130).contains(&half), "{half}");
        let (y, ms): (Byml, Vec<Made>) = run("a", 100);
        let all: Map<u64, u32> = seen(&y);
        assert_eq!(all.len(), 400);
        assert!(all.values().all(|n| *n == OWN));
        assert_eq!(Byml::parse(&y.write().unwrap()).unwrap(), y);
        assert_eq!(ms.len(), 200);
    }

    #[test]
    fn helper_and_boss_actors_are_never_on_the_list() {
        for (k, up) in KINDS {
            assert!(k.starts_with("Enemy_") && (0.0..=0.5).contains(up));
            for bad in ["_Dictionary", "_Attention", "_Haribote", "DungeonBoss", "MiddleBoss", "Skulltula", "Scrub"] {
                assert!(!k.contains(bad), "{k}");
            }
        }
        assert_eq!(lift("Enemy_Skulltula_Gold"), None);
        assert_eq!(lift("Enemy_Keese"), Some(0.0));
    }

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        if p.is_dir() { Some(p) } else { None }
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn the_game_maps_take_extra_enemies_that_nothing_counts() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let all: Vec<(Out, Vec<Made>)> = made(&mut rf, &none, "game", 100).unwrap();
        let n: usize = all.iter().map(|x| x.1.len()).sum();
        eprintln!("{n} copies in {} files", all.len());
        assert!((150..=400).contains(&n), "{n}");
        let mut seen: Map<u64, u32> = Map::new();
        for f in files(&rf).unwrap() {
            let y: Byml = match all.iter().find(|x| x.0.path == f) {
                Some(x) => Byml::parse(&rf.zs.decompress(&x.0.z).unwrap()).unwrap(),
                None => rf.read_byml(&f).unwrap(),
            };
            tally(&y.root, &mut seen);
        }
        for (o, ms) in &all {
            assert!(o.path.ends_with(EXT) && !o.path.starts_with(SKIP_DIR), "{}", o.path);
            assert!(o.new > o.old, "{} did not grow", o.path);
            let was: Byml = rf.read_byml(&o.path).unwrap();
            let now: Byml = Byml::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap();
            let a: &[Node] = was.root.get("Actors").unwrap().as_arr().unwrap();
            let b: &[Node] = now.root.get("Actors").unwrap().as_arr().unwrap();
            assert_eq!(b.len(), a.len() + ms.len());
            assert_eq!(&b[..a.len()], a);
            let Node::Map(x) = &was.root else { panic!() };
            let Node::Map(y) = &now.root else { panic!() };
            assert!(x.iter().zip(y.iter()).all(|(p, q)| p.0 == q.0 && (p.0 == "Actors" || p.1 == q.1)), "{} changed outside its actors", o.path);
            for (m, c) in ms.iter().zip(&b[a.len()..]) {
                let from: &Node = a.iter().find(|n| hash_of(n) == Some(m.from)).unwrap();
                assert_eq!(gyaml(c), gyaml(from));
                assert!(lift(gyaml(c)).is_some() && !gyaml(c).contains("Gold"));
                assert_eq!(c.get("Dynamic"), from.get("Dynamic"));
                assert_eq!(c.get("Rotate"), from.get("Rotate"));
                assert!(c.get("Links").is_none() && from.get("Links").is_none());
                assert_eq!(seen.get(&m.hash), Some(&OWN), "a copy is referenced or not unique");
                assert_eq!(seen.get(&m.from), Some(&OWN), "a copied enemy is referenced");
                let (p, q): ([f32; 3], [f32; 3]) = (pos(c).unwrap(), pos(from).unwrap());
                assert!((p[0] - q[0]).hypot(p[2] - q[2]) <= FAR + 0.001 && (p[1] - q[1]).abs() <= 0.5);
            }
        }
        let again: Vec<(Out, Vec<Made>)> = made(&mut rf, &none, "game", 100).unwrap();
        assert!(all.iter().zip(again.iter()).all(|(p, q)| p.0.z == q.0.z && p.1 == q.1) && all.len() == again.len());
        let half: usize = made(&mut rf, &none, "game", 50).unwrap().iter().map(|x| x.1.len()).sum();
        assert!(half > n / 4 && half < n * 3 / 4, "{half} of {n}");
        assert!(stage(&mut rf, &none, "game", 0).unwrap().is_empty());
    }
}
