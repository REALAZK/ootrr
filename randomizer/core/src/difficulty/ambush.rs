use std::f32::consts::{FRAC_PI_2, PI, TAU};

use crate::ainb::Ainb;
use crate::byml::{Byml, Node};
use crate::difficulty::gates::{id_for, wire, Mode, Plan, GRAPH as SOURCE};
use crate::error::{Error as E, Res};
use crate::flow_locations::dict_id;
use crate::murmur::murmur3_32;
use crate::placement::hash_of;
use crate::qol::Have;
use crate::romfs::Romfs;
use crate::staging::{Out, BANC_DICT};
use crate::zs;

pub const MAPS: [&str; 3] = ["ZoraDomain_Child", "ZoraDomain_Adult", "ZoraDomain_Adult_Alternative"];
pub const BASE: &str = "Land_HidingHole_Base_D_01";
pub const WALL: &str = "Enemy_Skullwalltula";
pub const HANG: &str = "Enemy_Skulltula";
pub const AREA: &str = "Area";
pub const FREE: &str = "Enemy_Skulltula_IsFreeRotationLimit";
pub const GROUPS: [&str; 2] = ["FarDeleteGroups", "SimultaneousGroups"];
pub const SEED: &str = "ootrr zora ambush";
pub const SLACK: f32 = 0.01;
pub const PREFIX: &str = "Ambush_Nook_";
pub const FLOOR: f32 = -16.0;
pub const BACK: f32 = 1.5;
pub const DEEP: f32 = 2.0;
pub const HIGH: f32 = 6.0;
pub const SWORD_KEY: &str = "sword";
pub const NOTE: &str = "Once you have a sword, the empty hole in Zora's Domain hides a wall Skulltula in each side cave, and a Skulltula drops behind you as you reach the back. They come back whenever the area reloads.";

pub fn needs_sword(map: &str) -> bool {
    map.ends_with("_Child")
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nook {
    pub name: &'static str,
    pub wall: [f32; 3],
    pub face: f32,
    pub hang: [f32; 3],
    pub out: f32,
    pub wide: f32,
}

pub const NOOKS: [Nook; 3] = [
    Nook { name: "a", wall: [-8.484, -12.167, -6.613], face: 1.402, hang: [-3.515, -11.7, -3.587], out: -2.2864, wide: 6.0 },
    Nook { name: "b", wall: [-1.255, -11.485, 11.855], face: 3.1875, hang: [-0.493, -11.7, 5.755], out: 0.0, wide: 6.0 },
    Nook { name: "c", wall: [14.29, -12.36, -6.18], face: -1.125, hang: [4.56, -11.7, -2.66], out: 2.0071, wide: 9.0 },
];

impl Nook {
    pub fn area(&self) -> [f32; 3] {
        let (s, c): (f32, f32) = self.out.sin_cos();
        [self.wall[0] - s * BACK, FLOOR, self.wall[2] - c * BACK]
    }

    pub fn size(&self) -> [f32; 3] {
        [self.wide, HIGH, DEEP]
    }

    pub fn plan(&self, sword: bool) -> Plan {
        let key: String = if sword { format!("{} {SWORD_KEY}", self.name) } else { self.name.to_string() };
        let hang: String = id_for(HANG, &format!("{SEED} {key} hang"), &[]);
        let mut guards: Vec<String> = vec![hang];
        let mut modes: Vec<Mode> = vec![Mode::Trigger];
        if sword {
            guards.push(id_for(WALL, &format!("{SEED} {key} wall"), &guards));
            modes.push(Mode::Always);
        }
        let area: String = id_for(AREA, &format!("{SEED} {key} area"), &guards);
        let tag: u32 = murmur3_32(format!("{SEED} {key} module").as_bytes(), 0) & 0xffff;
        let stem: String = if sword { format!("{}_{SWORD_KEY}", self.name) } else { self.name.to_string() };
        let name: String = format!("{PREFIX}{stem}_{tag:04x}");
        Plan { module: format!("Logic/Module/{PREFIX}{stem}/{name}.logic.module.ain"), graph: format!("Logic/{name}.logic.module.ainb"), guards, modes, hands: Vec::new(), area, counters: Vec::new(), sword }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub at: [f32; 3],
    pub yaw: f32,
}

pub fn wrap(a: f32) -> f32 {
    let r: f32 = a.rem_euclid(TAU);
    if r > PI { r - TAU } else { r }
}

pub fn euler(pitch: f32, yaw: f32) -> [f32; 3] {
    let y: f32 = wrap(yaw);
    if y.abs() <= FRAC_PI_2 {
        return [pitch, y, 0.0];
    }
    [wrap(pitch + PI), wrap(PI - y), PI]
}

impl Frame {
    pub fn point(&self, p: [f32; 3]) -> [f32; 3] {
        let (s, c): (f32, f32) = self.yaw.sin_cos();
        [self.at[0] + p[0] * c + p[2] * s, self.at[1] + p[1], self.at[2] - p[0] * s + p[2] * c]
    }

    pub fn turn(&self, a: f32) -> f32 {
        wrap(a + self.yaw)
    }
}

pub fn text<'a>(n: &'a Node, k: &str) -> &'a str {
    n.get(k).and_then(|v| v.as_str()).unwrap_or("")
}

pub fn vec3(n: &Node, k: &str) -> Option<[f32; 3]> {
    let a: &[Node] = n.get(k)?.as_arr()?;
    match a {
        [Node::F32(x), Node::F32(y), Node::F32(z)] => Some([*x, *y, *z]),
        _ => None,
    }
}

fn flat(a: f32) -> bool {
    wrap(a).abs() < SLACK
}

fn over(a: f32) -> bool {
    (wrap(a).abs() - PI).abs() < SLACK
}

pub fn yaw_of(r: [f32; 3]) -> Res<f32> {
    if flat(r[0]) && flat(r[2]) {
        return Ok(wrap(r[1]));
    }
    if over(r[0]) && over(r[2]) {
        return Ok(wrap(PI - r[1]));
    }
    Err(E::Msg(format!("zora ambush: the hole is tilted ({}, {}, {}), the game files differ", r[0], r[1], r[2])))
}

pub fn frame(y: &Byml) -> Res<Frame> {
    let list: &[Node] = y.root.get("Actors").and_then(|a| a.as_arr()).ok_or(E::Format("zora ambush: the map has no actor list"))?;
    let hits: Vec<&Node> = list.iter().filter(|a| text(a, "Gyaml") == BASE).collect();
    let [b] = hits.as_slice() else {
        return Err(E::Msg(format!("zora ambush: the map has {} {BASE} pieces and not one, the game files differ", hits.len())));
    };
    let at: [f32; 3] = vec3(b, "Translate").ok_or(E::Format("zora ambush: the hole has no position"))?;
    let yaw: f32 = yaw_of(vec3(b, "Rotate").unwrap_or([0.0; 3]))?;
    Ok(Frame { at, yaw })
}

fn used(raws: &[&[u8]], hash: u64) -> bool {
    let b: [u8; 8] = hash.to_le_bytes();
    raws.iter().any(|raw| raw.windows(8).any(|w| w == b))
}

pub fn fresh(raws: &[&[u8]], seed: &str) -> u64 {
    let b: &[u8] = seed.as_bytes();
    let mut h: u64 = (u64::from(murmur3_32(b, 1)) << 32) | u64::from(murmur3_32(b, 2));
    while h == 0 || used(raws, h) {
        h = h.wrapping_add(1);
    }
    h
}

pub fn floats(v: [f32; 3]) -> Node {
    Node::Arr(v.iter().map(|x| Node::F32(*x)).collect())
}

pub fn actor(gyaml: &str, hash: u64, at: [f32; 3], rot: [f32; 3]) -> Node {
    let mut a: Node = Node::Map(Vec::new());
    a.set("Gyaml", Node::Str(gyaml.to_string()));
    a.set("Hash", Node::U64(hash));
    a.set("Phive", Node::Map(vec![("Placement".to_string(), Node::Map(vec![("ID".to_string(), Node::U64(hash))]))]));
    a.set("Rotate", floats(rot));
    a.set("SRTHash", Node::U32(murmur3_32(&hash.to_le_bytes(), 0)));
    a.set("Translate", floats(at));
    a
}

pub fn link(id: &str, hash: u64) -> Node {
    let mut r: Node = Node::Map(Vec::new());
    r.set("Id", Node::Str(id.to_string()));
    r.set("Path", Node::Str(id.to_string()));
    r.set("Reference", Node::U64(hash));
    r
}

pub fn list<'a>(y: &'a mut Byml, k: &'static str) -> Res<&'a mut Vec<Node>> {
    y.root.get_mut(k).and_then(|a| a.as_arr_mut()).ok_or_else(|| E::Msg(format!("zora ambush: the map has no {k} list, the game files differ")))
}

pub fn place(y: &mut Byml, f: &Frame, raws: &[&[u8]], map: &str) -> Res<usize> {
    let mut n: usize = 0;
    for k in NOOKS.iter() {
        let seed: String = format!("{SEED} {map} {}", k.name);
        let wall: u64 = fresh(raws, &format!("{seed} wall"));
        let hang: u64 = fresh(raws, &format!("{seed} hang"));
        let area: u64 = fresh(raws, &format!("{seed} area"));
        let group: u64 = fresh(raws, &format!("{seed} group"));
        let sword: bool = needs_sword(map);
        let plan: Plan = k.plan(sword);
        let mut h: Node = actor(HANG, hang, f.point(k.hang), euler(0.0, f.turn(k.out)));
        h.set("Dynamic", Node::Map(vec![(FREE.to_string(), Node::Bool(true))]));
        let mut s: Node = actor(AREA, area, f.point(k.area()), euler(0.0, f.turn(k.out)));
        s.set("Scale", floats(k.size()));
        let acts: &mut Vec<Node> = list(y, "Actors")?;
        acts.push(actor(WALL, wall, f.point(k.wall), euler(FRAC_PI_2, f.turn(k.face))));
        acts.push(h);
        acts.push(s);
        n += 3;
        let mut g: Node = Node::Map(Vec::new());
        g.set("Hash", Node::U64(group));
        g.set("Logic", Node::Str(plan.module.clone()));
        let mut refs: Vec<Node> = vec![link(&plan.guards[0], hang)];
        if sword {
            refs.push(link(&plan.guards[1], wall));
        }
        refs.push(link(&plan.area, area));
        g.set("References", Node::Arr(refs));
        let gs: &mut Vec<Node> = list(y, "AiGroups")?;
        let i: usize = gs.iter().position(|x| hash_of(x).is_some_and(|v| v > group)).unwrap_or(gs.len());
        gs.insert(i, g);
        for name in GROUPS {
            list(y, name)?.push(Node::Arr(vec![Node::U64(hang.min(area)), Node::U64(hang.max(area))]));
            if sword {
                list(y, name)?.push(Node::Arr(vec![Node::U64(wall.min(area)), Node::U64(wall.max(area))]));
            }
        }
    }
    Ok(n)
}

pub fn graph(raw: &[u8], k: &Nook, sword: bool) -> Res<Out> {
    let p: Plan = k.plan(sword);
    let mut a: Ainb = Ainb::parse(raw)?;
    wire(&mut a, &p)?;
    let g: Vec<u8> = a.write()?;
    Ok(Out { path: p.graph, old: 0, new: g.len(), z: g })
}

fn source(rf: &Romfs, have: Have, p: &str) -> Res<Vec<u8>> {
    match have(p) {
        Some(b) => Ok(b),
        None => rf.read(p),
    }
}

pub fn file(map: &str, kind: &str) -> String {
    format!("Banc/ZoraDomain/{map}_{kind}.bcett.byml.zs")
}

fn one(rf: &mut Romfs, have: Have, map: &str) -> Res<Out> {
    let p: String = file(map, "Static");
    let z: Vec<u8> = source(rf, have, &p)?;
    let dict: u32 = dict_id(&z).unwrap_or(BANC_DICT);
    let raw: Vec<u8> = rf.zs.decompress(&z)?;
    let land: Vec<u8> = rf.zs.decompress(&source(rf, have, &file(map, "Dynamic"))?)?;
    let f: Frame = frame(&Byml::parse(&land)?)?;
    let mut y: Byml = Byml::parse(&raw)?;
    place(&mut y, &f, &[&raw, &land], map)?;
    let bytes: Vec<u8> = y.write()?;
    Ok(Out { path: p, old: raw.len(), new: bytes.len(), z: zs::compress_raw(&bytes, Some(dict))? })
}

pub fn stage(rf: &mut Romfs, have: Have) -> Res<Vec<Out>> {
    let was: Vec<u8> = source(rf, have, SOURCE)?;
    let mut outs: Vec<Out> = Vec::new();
    for sword in [false, true] {
        for k in &NOOKS {
            outs.push(graph(&was, k, sword)?);
        }
    }
    for map in MAPS {
        outs.push(one(rf, have, map)?);
    }
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 3], b: [f32; 3], tol: f32) -> bool {
        a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < tol)
    }

    #[test]
    fn the_frame_puts_the_vanilla_wall_spiders_back_where_the_castle_hole_has_them() {
        let f: Frame = Frame { at: [35.850994, 112.3131, -106.245], yaw: yaw_of([1.5099579e-7, -1.0448445, -1.5099579e-7]).unwrap() };
        assert!(close(f.point(NOOKS[0].wall), [37.31075, 100.14566, -116.90233], 0.01));
        assert!(close(f.point(NOOKS[1].wall), [24.96784, 100.82793, -101.378746], 0.01));
        assert!(close(euler(FRAC_PI_2, f.turn(NOOKS[0].face)), [FRAC_PI_2, 0.35715657, 0.0], 0.01));
        assert!(close(euler(FRAC_PI_2, f.turn(NOOKS[1].face)), [-FRAC_PI_2, 0.99889755, PI], 0.01));
    }

    #[test]
    fn a_flipped_hole_reads_as_a_plain_turn() {
        assert!((yaw_of([PI, -0.7853982, PI]).unwrap() - wrap(PI + 0.7853982)).abs() < 1e-5);
        assert!((yaw_of([0.0, 0.5, 0.0]).unwrap() - 0.5).abs() < 1e-6);
        assert!(yaw_of([0.4, 0.0, 0.0]).is_err());
        assert_eq!(euler(0.0, 0.25), [0.0, 0.25, 0.0]);
        let e: [f32; 3] = euler(0.0, 2.0);
        assert!(close(e, [PI, PI - 2.0, PI], 1e-5));
        let f: Frame = Frame { at: [16.98658, 75.32813, -18.26067], yaw: yaw_of([PI, -0.7853982, PI]).unwrap() };
        assert!(close(f.point([3.98, -5.0, -1.02]), [14.889927, 70.32946, -14.725923], 0.02));
    }

    fn map() -> Byml {
        let mut b: Node = actor(BASE, 7, [10.0, 50.0, 20.0], [0.0; 3]);
        b.remove("Rotate");
        let mut g: Node = Node::Map(Vec::new());
        g.set("Hash", Node::U64(u64::MAX));
        let mut root: Node = Node::Map(Vec::new());
        root.set("Actors", Node::Arr(vec![b]));
        root.set("AiGroups", Node::Arr(vec![g]));
        root.set("FarDeleteGroups", Node::Arr(Vec::new()));
        root.set("SimultaneousGroups", Node::Arr(Vec::new()));
        Byml { root, version: 7, big: false }
    }

    fn hashes(y: &Byml) -> Vec<u64> {
        y.root.get("Actors").unwrap().as_arr().unwrap().iter().filter_map(hash_of).collect()
    }

    #[test]
    fn each_nook_gets_a_wall_spider_and_a_spider_that_waits_for_its_area() {
        let mut y: Byml = map();
        let f: Frame = frame(&y).unwrap();
        assert_eq!(f, Frame { at: [10.0, 50.0, 20.0], yaw: 0.0 });
        assert_eq!(place(&mut y, &f, &[], "Test").unwrap(), 9);
        let acts: &[Node] = y.root.get("Actors").unwrap().as_arr().unwrap();
        for (name, n) in [(WALL, 3), (HANG, 3), (AREA, 3), (BASE, 1)] {
            assert_eq!(acts.iter().filter(|a| text(a, "Gyaml") == name).count(), n, "{name}");
        }
        let mut hs: Vec<u64> = hashes(&y);
        hs.sort();
        hs.dedup();
        assert_eq!(hs.len(), 10);
        for a in acts.iter().skip(1) {
            let h: u64 = hash_of(a).unwrap();
            assert_eq!(a.get("Phive").and_then(|p| p.get("Placement")).and_then(|p| p.get("ID")), Some(&Node::U64(h)));
            assert!(vec3(a, "Translate").is_some() && vec3(a, "Rotate").is_some());
        }
        let wall: &Node = acts.iter().find(|a| text(a, "Gyaml") == WALL).unwrap();
        assert!(close(vec3(wall, "Translate").unwrap(), [1.516, 37.833, 13.387], 0.001));
        assert!(wall.get("Dynamic").is_none() && wall.get("Scale").is_none());
        let hang: &Node = acts.iter().find(|a| text(a, "Gyaml") == HANG).unwrap();
        assert_eq!(hang.get("Dynamic").and_then(|d| d.get(FREE)), Some(&Node::Bool(true)));
        let area: &Node = acts.iter().find(|a| text(a, "Gyaml") == AREA).unwrap();
        assert_eq!(vec3(area, "Scale"), Some(NOOKS[0].size()));
        let gs: &[Node] = y.root.get("AiGroups").unwrap().as_arr().unwrap();
        assert_eq!(gs.len(), 4);
        assert!(gs.windows(2).all(|w| hash_of(&w[0]) < hash_of(&w[1])), "the groups stay sorted");
        for g in &gs[..3] {
            assert!(text(g, "Logic").starts_with(&format!("Logic/Module/{PREFIX}")));
            let rs: &[Node] = g.get("References").unwrap().as_arr().unwrap();
            let ids: Vec<&str> = rs.iter().map(|r| text(r, "Id")).collect();
            assert!(ids[0].starts_with(HANG) && ids[1].starts_with(AREA));
            let plan: Plan = NOOKS.iter().map(|k| k.plan(false)).find(|p| p.module == text(g, "Logic")).unwrap();
            assert_eq!(ids, vec![plan.guards[0].as_str(), plan.area.as_str()]);
            let own = |r: &Node, name: &str| acts.iter().any(|a| text(a, "Gyaml") == name && a.get("Hash") == r.get("Reference"));
            assert!(own(&rs[0], HANG) && own(&rs[1], AREA));
        }
        for name in GROUPS {
            let v: &[Node] = y.root.get(name).unwrap().as_arr().unwrap();
            assert_eq!(v.len(), 3);
            for p in v {
                let [Node::U64(a), Node::U64(b)] = p.as_arr().unwrap() else { panic!() };
                assert!(a < b);
            }
        }
        let back: Byml = Byml::parse(&y.write().unwrap()).unwrap();
        assert_eq!(back.root, y.root);
    }

    #[test]
    fn a_map_without_the_hole_is_refused() {
        let mut y: Byml = map();
        y.root.set("Actors", Node::Arr(Vec::new()));
        assert!(frame(&y).is_err());
        let mut y: Byml = map();
        y.root.remove("AiGroups");
        assert!(place(&mut y, &Frame { at: [0.0; 3], yaw: 0.0 }, &[], "Test").is_err());
    }

    #[test]
    fn new_hashes_step_past_the_ones_in_the_file() {
        let h: u64 = fresh(&[], "x");
        let raw: Vec<u8> = h.to_le_bytes().to_vec();
        assert_eq!(fresh(&[&raw], "x"), h + 1);
        assert_ne!(fresh(&[], "y"), h);
    }

    fn radial(k: &Nook, p: [f32; 3]) -> f32 {
        let (s, c): (f32, f32) = k.out.sin_cos();
        p[0] * s + p[2] * c
    }

    #[test]
    fn the_droppers_sit_a_little_nearer_the_middle_than_they_did() {
        let was: [[f32; 3]; 3] = [[-4.39, -11.7, -4.48], [-0.6, -11.7, 7.0], [5.64, -11.7, -3.29]];
        for (k, old) in NOOKS.iter().zip(was) {
            let from = |p: [f32; 3]| (p[0] * p[0] + p[2] * p[2]).sqrt();
            let moved: f32 = from(old) - from(k.hang);
            assert!((1.0..1.5).contains(&moved), "{} moved {moved}", k.name);
            assert_eq!(k.hang[1], old[1]);
            assert!((k.hang[0] * old[2] - k.hang[2] * old[0]).abs() < 0.05, "{} moved sideways", k.name);
        }
    }

    #[test]
    fn each_trigger_box_lies_at_the_far_end_past_every_dropper() {
        for k in &NOOKS {
            let a: [f32; 3] = k.area();
            let near: f32 = radial(k, a) - k.size()[2];
            for d in &NOOKS {
                assert!(near > radial(k, d.hang) + 1.0, "{} box starts at {near}, dropper {} at {}", k.name, d.name, radial(k, d.hang));
            }
            assert!(radial(k, k.wall) - radial(k, a) < k.size()[2], "{} box does not reach the wall spider", k.name);
            assert!(radial(k, k.wall) - near < 4.0, "{} box is not at the back", k.name);
        }
    }

    #[test]
    fn the_nooks_get_private_graphs_with_the_gate_construct() {
        let names: Vec<Plan> = [false, true].iter().flat_map(|w| NOOKS.iter().map(|k| k.plan(*w))).collect();
        for (i, p) in names.iter().enumerate() {
            assert!(p.graph.starts_with(&format!("Logic/{PREFIX}")) && p.graph.ends_with(".logic.module.ainb"));
            let n: usize = if p.sword { 2 } else { 1 };
            assert_eq!((p.guards.len(), p.modes.len(), p.hands.len(), p.counters.len()), (n, n, 0, 0));
            assert!(names.iter().enumerate().all(|(j, q)| i == j || (q.graph != p.graph && q.module != p.module)));
        }
        assert_eq!(names[0], NOOKS[0].plan(false));
        assert!(names[..3].iter().all(|p| !p.sword) && names[3..].iter().all(|p| p.sword && p.guards[1].starts_with(WALL)));
        assert!(needs_sword("ZoraDomain_Child") && !needs_sword("ZoraDomain_Adult") && !needs_sword("ZoraDomain_Adult_Alternative"));
    }

    #[test]
    fn each_dropper_waits_on_the_way_out_from_its_trigger_box_and_is_found_by_walking_out() {
        for k in &NOOKS {
            let a: [f32; 3] = k.area();
            let len: f32 = (a[0] * a[0] + a[2] * a[2]).sqrt();
            let side: f32 = (k.hang[0] * a[2] - k.hang[2] * a[0]).abs() / len;
            assert!(side < 1.5, "{} dropper lies {side} off the line from its box to the middle of the hole", k.name);
            let along: f32 = (k.hang[0] * a[0] + k.hang[2] * a[2]) / len;
            assert!(along > 0.0 && along < len - 3.0, "{} dropper is not between the box and the middle", k.name);
            let apart: f32 = len - along;
            assert!(apart > 3.0 && apart < 10.0, "{} dropper is {apart} from the box: a script that only stands in the box never sees it fall", k.name);
        }
    }

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        if p.is_dir() { Some(p) } else { None }
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    fn count(y: &Byml, name: &str) -> usize {
        y.root.get("Actors").unwrap().as_arr().unwrap().iter().filter(|a| text(a, "Gyaml") == name).count()
    }

    #[test]
    fn the_game_maps_take_the_ambush() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let all: Vec<Out> = stage(&mut rf, &none).unwrap();
        let (graphs, outs): (Vec<Out>, Vec<Out>) = all.into_iter().partition(|o| o.path.ends_with(".ainb"));
        assert_eq!(outs.iter().map(|o| o.path.clone()).collect::<Vec<String>>(), MAPS.iter().map(|m| file(m, "Static")).collect::<Vec<String>>());
        assert_eq!(graphs.iter().map(|o| o.path.clone()).collect::<Vec<String>>(), [false, true].iter().flat_map(|w| NOOKS.iter().map(|k| k.plan(*w).graph)).collect::<Vec<String>>());
        for o in &outs {
            let was: Byml = rf.read_byml(&o.path).unwrap();
            let now: Byml = Byml::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap();
            assert!(o.new > o.old);
            for (name, n) in [(WALL, 3), (HANG, 3), (AREA, 3), ("Enemy_Skulltula_Gold", 0)] {
                assert_eq!(count(&now, name), count(&was, name) + n, "{name} in {}", o.path);
            }
            let old: &[Node] = was.root.get("Actors").unwrap().as_arr().unwrap();
            let new: &[Node] = now.root.get("Actors").unwrap().as_arr().unwrap();
            assert_eq!(&new[..old.len()], old, "the actors that were there are untouched");
            let mut hs: Vec<u64> = hashes(&now);
            let n: usize = hs.len();
            hs.sort();
            hs.dedup();
            assert_eq!(hs.len(), n, "no hash is used twice in {}", o.path);
            let gs: &[Node] = now.root.get("AiGroups").unwrap().as_arr().unwrap();
            assert!(gs.windows(2).all(|w| hash_of(&w[0]) < hash_of(&w[1])));
            assert_eq!(gs.iter().filter(|g| text(g, "Logic").starts_with(&format!("Logic/Module/{PREFIX}"))).count(), 3);
            let mid: &Node = new.iter().find(|a| text(a, "Gyaml") == "DestinationAnchor" && text(a, "Name") == "HidingHole_FieldExitGo").unwrap();
            let c: [f32; 3] = vec3(mid, "Translate").unwrap();
            for a in &new[old.len()..] {
                let t: [f32; 3] = vec3(a, "Translate").unwrap();
                let far: f32 = ((t[0] - c[0]).powi(2) + (t[2] - c[2]).powi(2)).sqrt();
                assert!((4.5..17.0).contains(&far), "{} sits {far} from the middle of the hole", text(a, "Gyaml"));
                assert!((-1.0..5.0).contains(&(t[1] - c[1])), "{} sits {} above the floor", text(a, "Gyaml"), t[1] - c[1]);
            }
            let dist = |a: &Node| -> f32 {
                let t: [f32; 3] = vec3(a, "Translate").unwrap();
                ((t[0] - c[0]).powi(2) + (t[2] - c[2]).powi(2)).sqrt()
            };
            let drops: f32 = new[old.len()..].iter().filter(|a| text(a, "Gyaml") == HANG).map(dist).fold(0.0, f32::max);
            let boxes: Vec<&Node> = new[old.len()..].iter().filter(|a| text(a, "Gyaml") == AREA).collect();
            assert_eq!(boxes.len(), 3);
            for b in boxes {
                let near: f32 = dist(b) - vec3(b, "Scale").unwrap()[2];
                assert!(near > drops, "a trigger box sits at {} from the entrance", dist(b));
            }
        }
    }

    #[test]
    fn in_the_child_map_the_wall_spiders_join_the_group_that_waits_for_the_sword() {
        let mut y: Byml = map();
        let f: Frame = frame(&y).unwrap();
        assert_eq!(place(&mut y, &f, &[], "Test_Child").unwrap(), 9);
        let acts: &[Node] = y.root.get("Actors").unwrap().as_arr().unwrap();
        let gs: &[Node] = y.root.get("AiGroups").unwrap().as_arr().unwrap();
        for g in &gs[..3] {
            let plan: Plan = NOOKS.iter().map(|k| k.plan(true)).find(|p| p.module == text(g, "Logic")).unwrap();
            let rs: &[Node] = g.get("References").unwrap().as_arr().unwrap();
            let ids: Vec<&str> = rs.iter().map(|r| text(r, "Id")).collect();
            assert_eq!(ids, vec![plan.guards[0].as_str(), plan.guards[1].as_str(), plan.area.as_str()]);
            let own = |r: &Node, name: &str| acts.iter().any(|a| text(a, "Gyaml") == name && a.get("Hash") == r.get("Reference"));
            assert!(own(&rs[0], HANG) && own(&rs[1], WALL) && own(&rs[2], AREA));
        }
        for name in GROUPS {
            assert_eq!(y.root.get(name).unwrap().as_arr().unwrap().len(), 6);
        }
    }

    #[test]
    fn the_game_graphs_create_the_child_ambush_only_with_the_kokiri_sword() {
        use crate::ainb::Input;
        use crate::difficulty::gates::{BOOL, CREATE, HIT_NODE, NAME, SWORD, SWORD_AND};
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let all: Vec<Out> = stage(&mut rf, &none).unwrap();
        for sword in [false, true] {
            for k in &NOOKS {
                let p: Plan = k.plan(sword);
                let o: &Out = all.iter().find(|o| o.path == p.graph).unwrap();
                let a: Ainb = Ainb::parse(&o.z).unwrap();
                assert!(crate::ainb::audit(&a).is_empty());
                let is_sword = |e: usize| -> bool { a.el_name(e) == crate::gerudo_card::GET_BOOL && a.in_str(e, "GameDataNameBool") == Some(SWORD) };
                let flags: usize = (0..a.els.len()).filter(|&e| is_sword(e)).count();
                assert_eq!(flags, usize::from(sword), "{}", p.graph);
                let made = |id: &str| -> Input {
                    let i: usize = (0..a.els.len()).find(|&i| a.prop_str(i, NAME) == Some(id)).unwrap();
                    a.input(BOOL, a.find_in(i, BOOL, CREATE).unwrap()).unwrap()
                };
                let hang: Input = made(&p.guards[0]);
                assert!(hang.src >= 0 && hang.plug == 1);
                if !sword {
                    continue;
                }
                let and: usize = hang.src as usize;
                assert_eq!(a.el_name(and), SWORD_AND, "{}: the dropper waits for the area and the sword", p.graph);
                let g: Input = a.input(BOOL, a.find_in(and, BOOL, "BoolMulti").unwrap()).unwrap();
                let start: usize = (-100 - g.src as i64) as usize;
                let ins: Vec<(u16, u16)> = (start..start + g.plug as usize).map(|j| crate::door_open::multi_entry(&a, j).unwrap()).collect();
                assert_eq!(ins.len(), 2);
                let held: usize = ins[0].0 as usize;
                assert!(a.el_name(held) == crate::difficulty::gates::AND_NODE && ins[0].1 == 1 && is_sword(ins[1].0 as usize), "{}: the area hit is held once seen, then joined with the sword", p.graph);
                let h: Input = a.input(BOOL, a.find_in(held, BOOL, "BoolMulti").unwrap()).unwrap();
                assert!(h.src >= 0 && a.el_name(h.src as usize) == HIT_NODE, "{}", p.graph);
                let wall: Input = made(&p.guards[1]);
                assert!(wall.src >= 0 && is_sword(wall.src as usize) && wall.plug == 0, "{}: the wall spider is created by the sword flag", p.graph);
            }
        }
        let child: &Out = all.iter().find(|o| o.path == file("ZoraDomain_Child", "Static")).unwrap();
        let y: Byml = Byml::parse(&rf.zs.decompress(&child.z).unwrap()).unwrap();
        let mods: Vec<String> = y.root.get("AiGroups").unwrap().as_arr().unwrap().iter().map(|g| text(g, "Logic").to_string()).filter(|m| m.starts_with(&format!("Logic/Module/{PREFIX}"))).collect();
        assert_eq!(mods.len(), 3);
        assert!(NOOKS.iter().all(|k| mods.contains(&k.plan(true).module)), "the child map runs the graphs that wait for the sword");
        for m in ["ZoraDomain_Adult", "ZoraDomain_Adult_Alternative"] {
            let o: &Out = all.iter().find(|o| o.path == file(m, "Static")).unwrap();
            let y: Byml = Byml::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap();
            let n: usize = y.root.get("AiGroups").unwrap().as_arr().unwrap().iter().filter(|g| NOOKS.iter().any(|k| k.plan(false).module == text(g, "Logic"))).count();
            assert_eq!(n, 3, "{m} keeps the ambush without the sword");
        }
    }
}
