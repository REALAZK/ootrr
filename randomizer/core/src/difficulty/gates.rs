use crate::ainb::build::{Spec, QUERY};
use crate::ainb::{audit, Ainb};
use crate::byml::{Byml, Node};
use crate::difficulty::encounters::{dead_hand_nodes, knight_node, HAND, KNIGHT};
use crate::difficulty::wake;
use crate::difficulty::ambush::{actor, euler, floats, fresh, link, list, text, vec3};
use crate::error::{Error as E, Res};
use crate::flow_locations::dict_id;
use crate::gerudo_card::{bool_node, multi_node};
use crate::murmur::murmur3_32;
use crate::placement::hash_of;
use crate::qol::Have;
use crate::romfs::Romfs;
use crate::staging::{Out, BANC_DICT};
use crate::zs;

pub const PREFIX: &str = "Gimmick_EnemyGate_";
pub const GRAPH: &str = "Logic/Gimmick_AllDay_Skulltula_115c.logic.module.ainb";
pub const NODE: &str = "ActorLogicSimpleActor";
pub const COUNTER_NODE: &str = "ActorLogicEnemyCounter";
pub const NAME: &str = "InstanceName";
pub const CREATE: &str = "Logic_Create";
pub const DELETE: &str = "Logic_Delete";
pub const RESPAWN: &str = "RespawnTrigger";
pub const FINISH: &str = "Finish";
pub const INT: usize = 0;
pub const BOOL: usize = 1;
pub const STR: usize = 3;
pub const SEED: &str = "ootrr enemy gates";
pub const BARRIER: &str = "Obj_EnemyCounter_A_08";
pub const WIDTH: f32 = 4.0;
pub const MARGIN: f32 = 1.5;
pub const REACH: f32 = 8.0;
pub const RADIUS: f32 = 10.0;
pub const HEIGHT: f32 = 8.0;
pub const AREA: &str = "Area";
pub const AREA_NODE: &str = "ActorLogicAreaActor";
pub const HIT_NODE: &str = "HoldActorObserverBySensorHitInfo";
pub const AND_NODE: &str = "OperateANDFixInputTrue";
pub const SWORD_AND: &str = crate::gerudo_card::AND;
pub const SWORD: &str = "IsGet.Weapon_Sword_Kokiri";
pub const POINTER: &str = "const void";
pub const IGNORE: [&str; 9] = [
    "Enemy_Scrub_Mad_Leaf",
    "Enemy_Scrub_Deku_Leaf",
    "Enemy_Scrub_Business",
    "Enemy_Scrub_Business_Leaf",
    "Enemy_Scrub_ForestStage",
    "Enemy_Scrub_ForestStage_Leaf",
    "Enemy_Scrub_ForestStage_Elder",
    "Enemy_Scrub_ForestStage_Elder_Leaf",
    "Enemy_Skulltula_Gold",
];
pub const SHARED: [&str; 2] = ["FarDeleteGroups", "SimultaneousGroups"];
pub const ZORA_GUARD: &str = "Enemy_Lizalfos";
pub const WOODS_GUARD: &str = "Enemy_Wolfos";
pub const SKELETON_GUARD: &str = "Enemy_Stalfos";
pub const SCRUB_GUARD: &str = "Enemy_Scrub_Mad";
pub const DODONGO_GUARD: &str = "Enemy_Dodongo";
pub const POE_GUARD: &str = "Enemy_Poe";
pub const LIKELIKE_GUARD: &str = "Enemy_LikeLike";
pub const GIANT: f32 = 2.5;
pub const LEEVER_GUARD: &str = "Enemy_Leever_Gate";
pub const LEEVER_GIANT: &str = "Enemy_Leever_Gate_Giant";
pub const KEESE_GUARD: &str = "Enemy_Keese";
pub const DARKLINK_GUARD: &str = "Enemy_MiddleBoss_DarkLink";
pub const BARRIER_HALF: f32 = 1.0;
pub const INSET: f32 = 2.5 + BARRIER_HALF;
pub const LEAST: f32 = 2.0;
pub const SMALLEST: f32 = 0.5;
pub const MARKET_GUARD: &str = HAND;
pub const FIELD_GUARD: &str = KNIGHT;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gate {
    pub id: u8,
    pub name: &'static str,
    pub map: &'static str,
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub spot: [f32; 3],
    pub more: &'static [[f32; 3]],
    pub also: &'static [([f32; 3], [f32; 3])],
    pub sink: f32,
    pub radius: f32,
    pub new_kind: &'static str,
    pub more_kind: &'static str,
    pub loose: &'static [[f32; 3]],
    pub scale: f32,
    pub closed_forest_only: bool,
}

pub const HELD: [u8; 1] = [7];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shuffle {
    Off,
    Dungeons,
    Wide,
}

impl Shuffle {
    pub fn from_setting(v: &str) -> Shuffle {
        match v {
            "off" => Shuffle::Off,
            "dungeons" => Shuffle::Dungeons,
            _ => Shuffle::Wide,
        }
    }
}

pub const BEYOND_DUNGEON_DOOR: [u8; 12] = [1, 2, 6, 14, 15, 16, 17, 18, 19, 20, 21, 22];

pub const GATES: [Gate; 21] = [
    Gate { id: 1, name: "the passage out of the Sacred Forest Meadow's scrub lane", map: "SacredForestMeadow_Child", a: [-8.13, 114.46, -82.57], b: [-5.03, 114.47, -82.57], spot: [-6.4, 104.81, -53.4], more: &[], also: &[], sink: 0.0, radius: RADIUS, new_kind: SCRUB_GUARD, more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false },
    Gate { id: 2, name: "the Death Mountain Trail below the Dodongo's Cavern path", map: "DeathMountainTrail_Child", a: [-52.85, 106.91, -2.46], b: [-47.86, 106.93, -0.56], spot: [-54.67, 105.8, 6.39], more: &[], also: &[], sink: 0.0, radius: RADIUS, new_kind: DODONGO_GUARD, more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false },
    Gate { id: 3, name: "Zora's River before the Bean Seller", map: "ZoraRiver_Child", a: [-60.62, 100.11, -8.67], b: [-64.0, 99.85, -7.09], spot: [-64.05, 99.71, -14.09], more: &[], also: &[], sink: 0.0, radius: RADIUS, new_kind: ZORA_GUARD, more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false },
    Gate { id: 4, name: "Zora's River, the second narrows", map: "ZoraRiver_Child", a: [-25.79, 107.46, -23.17], b: [-23.86, 107.72, -26.27], spot: [-18.84, 106.01, -18.5], more: &[], also: &[], sink: 0.0, radius: RADIUS, new_kind: ZORA_GUARD, more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false },
    Gate { id: 5, name: "Zora's River, the upper narrows", map: "ZoraRiver_Child", a: [61.43, 108.87, -21.08], b: [60.85, 108.64, -14.65], spot: [34.73, 106.31, -24.81], more: &[[29.84, 106.68, -20.6]], also: &[], sink: 0.0, radius: RADIUS, new_kind: ZORA_GUARD, more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false },
    Gate { id: 6, name: "the path from Kokiri Forest to the Deku Tree", map: "KokiriForest_Child", a: [45.09, 113.77, -26.27], b: [45.43, 112.83, -19.5], spot: [38.26, 113.25, -17.34], more: &[], also: &[], sink: 0.0, radius: RADIUS, new_kind: "", more_kind: "", loose: &[], scale: 1.0, closed_forest_only: true },
    Gate { id: 7, name: "the Lost Woods, east passage", map: "LostWood_Child", a: [28.81, 100.18, -50.21], b: [28.86, 100.23, -45.78], spot: [19.39, 99.67, -48.13], more: &[], also: &[], sink: 0.0, radius: RADIUS, new_kind: WOODS_GUARD, more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false },
    Gate { id: 9, name: "the Lost Woods, both doors of the small room", map: "LostWood_Child", a: [-4.49, 100.3, 2.16], b: [-4.45, 100.19, -2.56], spot: [0.0, 99.87, -0.5], more: &[], also: &[([4.48, 100.26, -1.96], [4.52, 100.26, 2.08])], sink: 0.0, radius: RADIUS, new_kind: SKELETON_GUARD, more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false },
    Gate { id: 11, name: "the ruined Market, south entrance", map: "Market_Adult", a: [0.65, 102.04, 52.21], b: [-4.51, 102.07, 52.27], spot: [-1.67, 100.07, 32.58], more: &[], also: &[], sink: 0.0, radius: RADIUS, new_kind: MARKET_GUARD, more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false },
    Gate { id: 12, name: "Gerudo Valley, the way on to the fortress", map: "GerudoValley_Adult", a: [10.82, 98.0, 0.76], b: [11.23, 98.0, -6.52], spot: [21.14, 98.18, -3.23], more: &[[21.14, 98.18, -0.23]], also: &[], sink: 0.0, radius: RADIUS, new_kind: SKELETON_GUARD, more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false },
    Gate { id: 13, name: "Hyrule Field, the way to Gerudo Valley", map: "HyruleField_Adult", a: [-251.35, 136.64, 206.06], b: [-246.42, 135.53, 187.44], spot: [-214.24, 129.62, 194.89], more: &[], also: &[], sink: 0.5, radius: RADIUS, new_kind: FIELD_GUARD, more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false },
    Gate { id: 14, name: "Death Mountain Crater, the west ledge by the Deku Scrub", map: "DeathMountainCrater_Adult", a: [-28.0, 119.35, 33.0], b: [-16.5, 119.4, 33.0], spot: [-21.5, 119.39, 36.5], more: &[], also: &[], sink: 0.0, radius: 5.0, new_kind: DODONGO_GUARD, more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false },
    Gate { id: 15, name: "Death Mountain Trail, the way up past the junction", map: "DeathMountainTrail_Child", a: [-16.99, 122.43, -29.2], b: [-12.0, 122.58, -29.2], spot: [-14.0, 123.61, -24.0], more: &[], also: &[], sink: 0.0, radius: RADIUS, new_kind: DODONGO_GUARD, more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false },
    Gate { id: 16, name: "Death Mountain Trail, the way up past the junction", map: "DeathMountainTrail_Adult", a: [-16.82, 122.28, -29.2], b: [-12.0, 122.54, -29.2], spot: [-14.0, 123.13, -24.0], more: &[[-12.5, 123.05, -26.3], [-16.5, 123.6, -20.5], [-17.5, 123.69, -17.0]], also: &[], sink: 0.0, radius: RADIUS, new_kind: DODONGO_GUARD, more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false },
    Gate { id: 17, name: "the Graveyard, the way to the Shadow Temple", map: "Graveyard_Child", a: [72.35, 108.56, 0.62], b: [72.35, 108.46, 4.34], spot: [67.6, 108.64, 2.5], more: &[[69.0, 108.62, 0.9], [69.0, 108.54, 4.3]], also: &[], sink: 0.0, radius: RADIUS, new_kind: POE_GUARD, more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false },
    Gate { id: 18, name: "the Graveyard, the way to the Shadow Temple", map: "Graveyard_Adult", a: [72.35, 108.56, 0.62], b: [72.35, 108.46, 4.34], spot: [67.6, 109.03, 2.5], more: &[[69.0, 109.27, 0.9], [69.0, 108.22, 4.3]], also: &[], sink: 0.0, radius: RADIUS, new_kind: POE_GUARD, more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false },
    Gate { id: 19, name: "Hyrule Field, the way down to Lake Hylia", map: "HyruleField_Child", a: [-104.13, 139.6, 371.34], b: [-61.78, 139.6, 401.59], spot: [-80.6, 141.11, 374.0], more: &[[-88.8, 141.48, 371.7], [-75.1, 140.86, 379.9]], also: &[], sink: 0.0, radius: RADIUS, new_kind: ZORA_GUARD, more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false },
    Gate { id: 20, name: "Lon Lon Ranch, the north gate", map: "HyruleField_Child", a: [-65.5, 150.0, 183.5], b: [-56.5, 150.0, 184.5], spot: [-60.5, 150.02, 168.0], more: &[[-64.0, 150.2, 166.0], [-57.0, 149.91, 166.0]], also: &[], sink: 0.0, radius: RADIUS, new_kind: SKELETON_GUARD, more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false },
    Gate { id: 21, name: "Lake Hylia, the bridge by the laboratory", map: "HyruleField_Child", a: [-168.3, 137.6, 428.0], b: [-158.6, 137.3, 428.0], spot: [-161.0, 137.05, 419.5], more: &[], also: &[], sink: 0.0, radius: RADIUS, new_kind: LIKELIKE_GUARD, more_kind: "", loose: &[[-157.0, 139.0, 419.0], [-157.0, 139.0, 423.5], [-164.0, 139.5, 418.0], [-160.0, 140.5, 424.0]], scale: 1.0, closed_forest_only: false },
    Gate { id: 22, name: "Lake Hylia, the bridge by the laboratory", map: "HyruleField_Adult", a: [-168.3, 137.6, 428.0], b: [-158.6, 137.3, 428.0], spot: [-161.0, 137.02, 419.5], more: &[], also: &[], sink: 0.0, radius: RADIUS, new_kind: LIKELIKE_GUARD, more_kind: "", loose: &[[-157.0, 139.0, 419.0], [-157.0, 139.0, 423.5], [-164.0, 139.5, 418.0], [-160.0, 140.5, 424.0]], scale: 1.0, closed_forest_only: false },
    Gate { id: 23, name: "Ganon's Castle, the north exit of the first round room", map: "InsideGanonCastle", a: [-4.0, 110.3, 60.0], b: [4.0, 110.3, 60.0], spot: [0.0, 110.3, 76.0], more: &[], also: &[], sink: 0.0, radius: RADIUS, new_kind: DARKLINK_GUARD, more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false },
];

pub fn dir_of(g: &Gate) -> &'static str {
    g.map.split('_').next().unwrap_or(g.map)
}

pub fn file(g: &Gate, kind: &str) -> String {
    format!("Banc/{}/{}_{kind}.bcett.byml.zs", dir_of(g), g.map)
}

pub fn makes(g: &Gate) -> bool {
    !g.new_kind.is_empty()
}

pub fn turn(g: &Gate) -> f32 {
    turn_from(g, g.spot)
}

pub fn turn_from(g: &Gate, at: [f32; 3]) -> f32 {
    let m: [f32; 3] = mid(g);
    (m[0] - at[0]).atan2(m[2] - at[2])
}

pub fn mid(g: &Gate) -> [f32; 3] {
    [(g.a[0] + g.b[0]) / 2.0, g.a[1].min(g.b[1]), (g.a[2] + g.b[2]) / 2.0]
}

pub fn span(g: &Gate) -> f32 {
    (g.b[0] - g.a[0]).hypot(g.b[2] - g.a[2])
}

pub fn yaw(g: &Gate) -> f32 {
    wall_yaw(g.a, g.b, g.spot)
}

pub fn faces_guards(g: &Gate) -> bool {
    let m: [f32; 3] = mid(g);
    let y: f32 = yaw(g);
    (g.spot[0] - m[0]) * y.sin() + (g.spot[2] - m[2]) * y.cos() > 0.0
}

pub fn walls(g: &Gate) -> Vec<([f32; 3], [f32; 3])> {
    std::iter::once((g.a, g.b)).chain(g.also.iter().copied()).collect()
}

pub fn wall_yaw(a: [f32; 3], b: [f32; 3], spot: [f32; 3]) -> f32 {
    let t: f32 = (-(b[2] - a[2])).atan2(b[0] - a[0]);
    let m: [f32; 3] = [(a[0] + b[0]) / 2.0, 0.0, (a[2] + b[2]) / 2.0];
    let toward: f32 = (spot[0] - m[0]) * t.sin() + (spot[2] - m[2]) * t.cos();
    if toward < 0.0 {
        t + std::f32::consts::PI
    } else {
        t
    }
}

pub fn wall_count(a: [f32; 3], b: [f32; 3]) -> usize {
    (((b[0] - a[0]).hypot(b[2] - a[2]) + 2.0 * MARGIN) / WIDTH).ceil() as usize
}

pub fn wall_pieces(a: [f32; 3], b: [f32; 3], sink: f32) -> Vec<[f32; 3]> {
    let m: [f32; 3] = [(a[0] + b[0]) / 2.0, a[1].min(b[1]) - sink, (a[2] + b[2]) / 2.0];
    let len: f32 = (b[0] - a[0]).hypot(b[2] - a[2]);
    let d: [f32; 2] = [(b[0] - a[0]) / len, (b[2] - a[2]) / len];
    let n: usize = wall_count(a, b);
    (0..n)
        .map(|i| {
            let off: f32 = (i as f32 - (n as f32 - 1.0) / 2.0) * WIDTH;
            [m[0] + d[0] * off, m[1], m[2] + d[1] * off]
        })
        .collect()
}

pub fn count(g: &Gate) -> usize {
    walls(g).iter().map(|(a, b)| wall_count(*a, *b)).sum()
}

pub fn pieces(g: &Gate) -> Vec<([f32; 3], f32)> {
    let mut out: Vec<([f32; 3], f32)> = Vec::new();
    for (a, b) in walls(g) {
        let y: f32 = wall_yaw(a, b, g.spot);
        out.extend(wall_pieces(a, b, g.sink).into_iter().map(|p| (p, y)));
    }
    out
}

pub fn wall_side(a: [f32; 3], b: [f32; 3], spot: [f32; 3]) -> ([f32; 2], f32) {
    let len: f32 = (b[0] - a[0]).hypot(b[2] - a[2]);
    let mut n: [f32; 2] = [-(b[2] - a[2]) / len, (b[0] - a[0]) / len];
    let mut d: f32 = n[0] * (spot[0] - a[0]) + n[1] * (spot[2] - a[2]);
    if d < 0.0 {
        n = [-n[0], -n[1]];
        d = -d;
    }
    (n, d)
}

pub fn clear(g: &Gate, c: [f32; 2], h: [f32; 2]) -> bool {
    walls(g).iter().all(|(a, b)| {
        let (n, _): ([f32; 2], f32) = wall_side(*a, *b, g.spot);
        n[0] * (c[0] - a[0]) + n[1] * (c[1] - a[2]) - n[0].abs() * h[0] - n[1].abs() * h[1] >= INSET - 1e-3
    })
}

pub fn area_box(g: &Gate) -> ([f32; 2], [f32; 2]) {
    let (c, h): ([f32; 2], [f32; 2]) = area_fit(g, LEAST);
    if clear(g, c, h) {
        (c, h)
    } else {
        area_fit(g, SMALLEST)
    }
}

pub fn area_fit(g: &Gate, least: f32) -> ([f32; 2], [f32; 2]) {
    let mut c: [f32; 2] = [g.spot[0], g.spot[2]];
    let mut h: [f32; 2] = [g.radius, g.radius];
    for _ in 0..3 {
        for (a, b) in walls(g) {
            let (n, _): ([f32; 2], f32) = wall_side(a, b, g.spot);
            let (first, second): (usize, usize) = if n[0].abs() >= n[1].abs() { (0, 1) } else { (1, 0) };
            let room: f32 = n[0] * (c[0] - a[0]) + n[1] * (c[1] - a[2]) - INSET;
            if n[first].abs() * h[first] + n[second].abs() * h[second] > room {
                h[first] = ((room - n[second].abs() * h[second]) / n[first].abs()).clamp(least, h[first]);
            }
            if n[first].abs() * h[first] + n[second].abs() * h[second] > room {
                h[second] = ((room - n[first].abs() * h[first]) / n[second].abs().max(1e-3)).clamp(least, h[second]);
            }
            let short: f32 = n[0].abs() * h[0] + n[1].abs() * h[1] - room;
            if short > 0.0 {
                c = [c[0] + n[0] * short, c[1] + n[1] * short];
            }
        }
    }
    (c, h)
}

pub fn area_half(g: &Gate) -> [f32; 2] {
    area_box(g).1
}

pub fn area_at(g: &Gate) -> [f32; 3] {
    let c: [f32; 2] = area_box(g).0;
    [c[0], g.spot[1] - 1.0, c[1]]
}

pub fn area_size(g: &Gate) -> [f32; 3] {
    let h: [f32; 2] = area_half(g);
    [h[0], HEIGHT, h[1]]
}

pub fn approach(g: &Gate, back: f32) -> [f32; 3] {
    let m: [f32; 3] = mid(g);
    let d: f32 = (g.spot[0] - m[0]).hypot(g.spot[2] - m[2]);
    [g.spot[0] + (g.spot[0] - m[0]) / d * back, g.spot[1], g.spot[2] + (g.spot[2] - m[2]) / d * back]
}

pub fn active(g: &Gate, forest_open: bool, mido_gone: bool, entrances: Shuffle) -> bool {
    let shuffle_ok: bool = match entrances {
        Shuffle::Off => true,
        Shuffle::Dungeons => !BEYOND_DUNGEON_DOOR.contains(&g.id),
        Shuffle::Wide => false,
    };
    shuffle_ok && !(g.closed_forest_only && (forest_open || mido_gone))
}

pub fn shuffle_note(entrances: Shuffle) -> Option<String> {
    let off: Vec<String> = GATES.iter().filter(|g| !HELD.contains(&g.id) && !active(g, false, false, entrances)).map(|g| format!("{} (gate {})", g.name, g.id)).collect();
    match entrances {
        Shuffle::Off => None,
        Shuffle::Dungeons => Some(format!("Because dungeon entrances are shuffled, these gates are left out, as a dungeon door lies beyond them: {}.", off.join(", "))),
        Shuffle::Wide => Some(format!("Because overworld entrances are shuffled, every gate is left out: {}.", off.join(", "))),
    }
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn is_guard(a: &Node, spot: [f32; 3]) -> bool {
    let k: &str = text(a, "Gyaml");
    k.starts_with("Enemy_") && !IGNORE.contains(&k) && vec3(a, "Translate").is_some_and(|t| dist(t, spot) < REACH)
}

fn actors(y: &Byml) -> &[Node] {
    y.root.get("Actors").and_then(|a| a.as_arr()).unwrap_or(&[])
}

pub fn near_guards(y: &Byml, spot: [f32; 3]) -> Vec<Node> {
    actors(y).iter().filter(|a| is_guard(a, spot)).cloned().collect()
}

pub fn take(y: &mut Byml, spot: [f32; 3]) -> Res<Vec<Node>> {
    let acts: &mut Vec<Node> = list(y, "Actors")?;
    let found: Vec<Node> = acts.iter().filter(|a| is_guard(a, spot)).cloned().collect();
    let hs: Vec<u64> = found.iter().filter_map(hash_of).collect();
    acts.retain(|a| !hash_of(a).is_some_and(|h| hs.contains(&h)));
    Ok(found)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Trigger,
    Always,
    Never,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub guards: Vec<Node>,
    pub modes: Vec<Mode>,
    pub add: Vec<Node>,
    pub ring: Vec<Node>,
}

pub fn claimed(y: &Byml, h: u64) -> bool {
    let groups: &[Node] = y.root.get("AiGroups").and_then(|a| a.as_arr()).unwrap_or(&[]);
    groups.iter().any(|grp| grp.get("References").and_then(|r| r.as_arr()).unwrap_or(&[]).iter().any(|r| r.get("Reference") == Some(&Node::U64(h))))
}

pub fn resolve(ys: &Byml, yd: &mut Byml, raws: &[&[u8]], g: &Gate) -> Res<Option<Found>> {
    let mut guards: Vec<Node> = Vec::new();
    let mut modes: Vec<Mode> = Vec::new();
    let mut add: Vec<Node> = Vec::new();
    let mut ring: Vec<Node> = Vec::new();
    if !makes(g) {
        for n in near_guards(ys, g.spot) {
            let owned: bool = hash_of(&n).is_some_and(|h| claimed(ys, h));
            modes.push(if owned { Mode::Never } else { Mode::Always });
            guards.push(n);
        }
        let moved: Vec<Node> = take(yd, g.spot)?;
        add = moved.clone();
        for n in moved {
            modes.push(Mode::Always);
            guards.push(n);
        }
        if guards.is_empty() {
            return Ok(None);
        }
        return Ok(Some(Found { guards, modes, add, ring }));
    }
    let seed: String = format!("{SEED} gate {} guard", g.id);
    match g.new_kind {
        HAND => {
            let (body, hands): (Node, Vec<Node>) = dead_hand_nodes(g.spot, turn(g), raws, &seed);
            add.push(body.clone());
            add.extend(hands.iter().cloned());
            ring = hands;
            guards.push(body);
            modes.push(Mode::Trigger);
        }
        KNIGHT => {
            let n: Node = knight_node(g.spot, turn(g), raws, &seed);
            add.push(n.clone());
            guards.push(n);
            modes.push(Mode::Trigger);
        }
        kind => {
            let spots: Vec<[f32; 3]> = std::iter::once(g.spot).chain(g.more.iter().copied()).collect();
            for (j, at) in spots.iter().enumerate() {
                let s: String = if j == 0 { seed.clone() } else { format!("{seed} {j}") };
                let h: u64 = fresh(raws, &s);
                let k: &str = if j == 0 || g.more_kind.is_empty() { kind } else { g.more_kind };
                let mut n: Node = actor(k, h, *at, euler(0.0, turn_from(g, *at)));
                if j == 0 && g.scale != 1.0 {
                    n.set("Scale", floats([g.scale; 3]));
                }
                add.push(n.clone());
                guards.push(n);
                modes.push(Mode::Trigger);
            }
            for (j, at) in g.loose.iter().enumerate() {
                let h: u64 = fresh(raws, &format!("{seed} loose {j}"));
                add.push(actor(KEESE_GUARD, h, *at, euler(0.0, turn_from(g, *at))));
            }
        }
    }
    Ok(Some(Found { guards, modes, add, ring }))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub module: String,
    pub graph: String,
    pub guards: Vec<String>,
    pub modes: Vec<Mode>,
    pub hands: Vec<String>,
    pub area: String,
    pub counters: Vec<(usize, String)>,
    pub sword: bool,
}

pub fn id_for(gyaml: &str, seed: &str, taken: &[String]) -> String {
    let mut n: u32 = murmur3_32(seed.as_bytes(), 0) & 0xffff;
    let mut id: String = format!("{gyaml}_{n:04x}");
    while taken.contains(&id) {
        n = (n + 1) & 0xffff;
        id = format!("{gyaml}_{n:04x}");
    }
    id
}

pub fn names(g: &Gate, kinds: &[String], modes: &[Mode], hand_kinds: &[String]) -> Plan {
    let mut taken: Vec<String> = Vec::new();
    let mut guards: Vec<String> = Vec::new();
    for (j, k) in kinds.iter().enumerate() {
        let id: String = id_for(k, &format!("{SEED} gate {} guard {j}", g.id), &taken);
        taken.push(id.clone());
        guards.push(id);
    }
    let mut hands: Vec<String> = Vec::new();
    for (j, k) in hand_kinds.iter().enumerate() {
        let id: String = id_for(k, &format!("{SEED} gate {} hand {j}", g.id), &taken);
        taken.push(id.clone());
        hands.push(id);
    }
    let area: String = id_for(AREA, &format!("{SEED} gate {} area", g.id), &taken);
    taken.push(area.clone());
    let mut counters: Vec<(usize, String)> = Vec::new();
    for j in 0..kinds.len() {
        for i in 0..count(g) {
            let id: String = id_for(BARRIER, &format!("{SEED} gate {} barrier {j} {i}", g.id), &taken);
            taken.push(id.clone());
            counters.push((j, id));
        }
    }
    let tag: u32 = murmur3_32(format!("{SEED} gate {} module", g.id).as_bytes(), 0) & 0xffff;
    let name: String = format!("{PREFIX}{:02}_{tag:04x}", g.id);
    Plan { module: format!("Logic/Module/{PREFIX}{:02}/{name}.logic.module.ain", g.id), graph: format!("Logic/{name}.logic.module.ainb"), guards, modes: modes.to_vec(), hands, area, counters, sword: g.map.ends_with("_Child") }
}

fn pair(a: u64, b: u64) -> Node {
    Node::Arr(vec![Node::U64(a.min(b)), Node::U64(a.max(b))])
}

pub fn place(y: &mut Byml, raws: &[&[u8]], g: &Gate, found: &Found) -> Res<Plan> {
    let kinds: Vec<String> = found.guards.iter().map(|n| text(n, "Gyaml").to_string()).collect();
    let hand_kinds: Vec<String> = found.ring.iter().map(|n| text(n, "Gyaml").to_string()).collect();
    let plan: Plan = names(g, &kinds, &found.modes, &hand_kinds);
    let ghs: Vec<u64> = found.guards.iter().map(|n| hash_of(n).ok_or(E::Format("enemy gates: a guard has no hash"))).collect::<Res<Vec<u64>>>()?;
    let hhs: Vec<u64> = found.ring.iter().map(|n| hash_of(n).ok_or(E::Format("enemy gates: a guard has no hash"))).collect::<Res<Vec<u64>>>()?;
    for n in &found.add {
        let h: u64 = hash_of(n).ok_or(E::Format("enemy gates: a guard has no hash"))?;
        if list(y, "Actors")?.iter().any(|a| hash_of(a) == Some(h)) {
            return Err(E::Msg(format!("enemy gates: the {} is already in the static map of {}", text(n, "Gyaml"), g.map)));
        }
        list(y, "Actors")?.push(n.clone());
    }
    if !found.ring.is_empty() {
        let mut all: Vec<u64> = found.add.iter().filter_map(hash_of).collect();
        all.sort();
        for name in SHARED {
            if y.root.get(name).is_some() {
                list(y, name)?.push(Node::Arr(all.iter().map(|h| Node::U64(*h)).collect()));
            }
        }
    }
    let spots: Vec<([f32; 3], f32)> = pieces(g);
    let mut refs: Vec<Node> = Vec::new();
    for (j, id) in plan.guards.iter().enumerate() {
        refs.push(link(id, ghs[j]));
    }
    for (j, id) in plan.hands.iter().enumerate() {
        refs.push(link(id, hhs[j]));
    }
    let mut used: Vec<u64> = ghs.clone();
    used.extend(hhs.iter().copied());
    let mut area_hash: u64 = fresh(raws, &format!("{SEED} gate {} area hash", g.id));
    while used.contains(&area_hash) {
        area_hash = area_hash.wrapping_add(1);
    }
    used.push(area_hash);
    let mut area: Node = actor(AREA, area_hash, area_at(g), euler(0.0, 0.0));
    area.set("Scale", floats(area_size(g)));
    list(y, "Actors")?.push(area);
    refs.push(link(&plan.area, area_hash));
    for gh in &ghs {
        for name in SHARED {
            if y.root.get(name).is_some() {
                list(y, name)?.push(pair(*gh, area_hash));
            }
        }
    }
    for (n, (j, id)) in plan.counters.iter().enumerate() {
        let mut h: u64 = fresh(raws, &format!("{SEED} gate {} barrier hash {n}", g.id));
        while used.contains(&h) {
            h = h.wrapping_add(1);
        }
        used.push(h);
        let (at, turned): ([f32; 3], f32) = spots[n % spots.len()];
        let mut c: Node = actor(BARRIER, h, at, euler(0.0, turned));
        let mut l: Node = Node::Map(Vec::new());
        l.set("Dst", Node::U64(ghs[*j]));
        l.set("Gyaml", Node::Str("Reference".to_string()));
        l.set("Name", Node::Str("Reference".to_string()));
        l.set("Src", Node::U64(h));
        c.set("Links", Node::Arr(vec![l]));
        list(y, "Actors")?.push(c);
        refs.push(link(id, h));
        for name in SHARED {
            if y.root.get(name).is_some() {
                list(y, name)?.push(pair(ghs[*j], h));
            }
        }
    }
    let group: u64 = fresh(raws, &format!("{SEED} gate {} group", g.id));
    let mut n: Node = Node::Map(Vec::new());
    n.set("Hash", Node::U64(group));
    n.set("Logic", Node::Str(plan.module.clone()));
    n.set("References", Node::Arr(refs));
    let gs: &mut Vec<Node> = list(y, "AiGroups")?;
    let i: usize = gs.iter().position(|x| hash_of(x).is_some_and(|v| v > group)).unwrap_or(gs.len());
    gs.insert(i, n);
    Ok(plan)
}

fn named(a: &Ainb, id: &str) -> Vec<usize> {
    (0..a.els.len()).filter(|&i| a.prop_str(i, NAME) == Some(id)).collect()
}

fn actor_node(a: &mut Ainb, id: &str, mode: Mode, inside_at: usize, armed: Option<usize>, dead: bool) -> Res<usize> {
    let n: Vec<u8> = a.pool.add(id).to_le_bytes().to_vec();
    let one: Vec<u8> = 1u32.to_le_bytes().to_vec();
    let zero: Vec<u8> = 0u32.to_le_bytes().to_vec();
    let seed: String = format!("{SEED} {id}");
    let mut s: Spec = Spec::node(NODE, &seed).prop(STR, NAME, n);
    s = match mode {
        Mode::Trigger => s.linked(BOOL, CREATE, inside_at, 1),
        Mode::Always => match armed {
            Some(at) => s.linked(BOOL, CREATE, at, 0),
            None => s.idle(BOOL, CREATE, one),
        },
        Mode::Never => s.idle(BOOL, CREATE, zero.clone()),
    };
    s = s.idle(BOOL, DELETE, zero.clone()).idle(BOOL, RESPAWN, zero);
    s = if dead { s.out(BOOL, "Dead") } else { s.spare(BOOL, "Dead") };
    s = s.spare(BOOL, "Logic_Alive");
    s.role = QUERY;
    a.add_node(&s)
}

pub fn wire(a: &mut Ainb, p: &Plan) -> Res<usize> {
    for id in p.guards.iter().chain(p.hands.iter()).chain(std::iter::once(&p.area)).chain(p.counters.iter().map(|(_, c)| c)) {
        if !named(a, id).is_empty() {
            return Err(E::Msg(format!("enemy gates: the graph already has a node named {id}")));
        }
    }
    let zero: Vec<u8> = 0u32.to_le_bytes().to_vec();
    let one: Vec<u8> = 1u32.to_le_bytes().to_vec();
    let area_name: Vec<u8> = a.pool.add(&p.area).to_le_bytes().to_vec();
    let empty: Vec<u8> = a.pool.add("").to_le_bytes().to_vec();
    let player: Vec<u8> = a.pool.add("Player").to_le_bytes().to_vec();
    let seed_a: String = format!("{SEED} {} node", p.area);
    let seed_h: String = format!("{SEED} {} hit", p.area);
    let seed_i: String = format!("{SEED} {} inside", p.area);
    let mut area: Spec = Spec::node(AREA_NODE, &seed_a)
        .prop(STR, "AddLayer", empty)
        .prop(STR, NAME, area_name)
        .prop(STR, "Layer", player.clone())
        .idle(BOOL, CREATE, one.clone())
        .idle(BOOL, DELETE, zero.clone())
        .idle(BOOL, RESPAWN, zero.clone())
        .spare(BOOL, "Dead")
        .spare(BOOL, "Logic_Alive")
        .out_ptr("SensorHitInfo", POINTER);
    area.role = QUERY;
    let area_at: usize = a.add_node(&area)?;
    let mut hit: Spec = Spec::node(HIT_NODE, &seed_h)
        .prop(BOOL, "NoEventPause", zero.clone())
        .idle(INT, "ActorNum", one.clone())
        .idle(BOOL, "IsRecordContactActorLink", zero.clone())
        .idle(BOOL, "SendMessage", zero.clone())
        .constant(STR, "ActorName", player)
        .linked_ptr("SensorHitInfoMulti", POINTER, area_at)
        .out(BOOL, "IsHit")
        .spare(BOOL, "IsNotHit")
        .spare_ptr("ContactActorLinkPtr", POINTER);
    hit.role = QUERY;
    let hit_at: usize = a.add_node(&hit)?;
    let armed: Option<usize> = if p.sword { Some(bool_node(a, SWORD, &format!("{SEED} {} sword", p.area))?) } else { None };
    let mut inside: Spec = Spec::node(AND_NODE, &seed_i)
        .linked(BOOL, "BoolMulti", hit_at, 0)
        .idle(BOOL, "UpdateOnce", zero.clone())
        .spare(BOOL, "False")
        .out(BOOL, "True");
    inside.role = QUERY;
    let held_at: usize = a.add_node(&inside)?;
    let inside_at: usize = match armed {
        Some(sword_at) => multi_node(a, SWORD_AND, &format!("{seed_i} sword"), &[(held_at, 1), (sword_at, 0)])?,
        None => held_at,
    };
    let mut at: Vec<usize> = Vec::new();
    for (j, id) in p.guards.iter().enumerate() {
        at.push(actor_node(a, id, p.modes[j], inside_at, armed, true)?);
    }
    for id in &p.hands {
        actor_node(a, id, Mode::Trigger, inside_at, armed, false)?;
    }
    for (j, id) in &p.counters {
        let n: Vec<u8> = a.pool.add(id).to_le_bytes().to_vec();
        let seed: String = format!("{SEED} {id}");
        let mut s: Spec = Spec::node(COUNTER_NODE, &seed)
            .prop(STR, NAME, n)
            .constant(INT, "MaxNum", zero.clone())
            .linked(BOOL, FINISH, at[*j], 0)
            .linked(BOOL, CREATE, inside_at, 1)
            .idle(BOOL, DELETE, zero.clone())
            .idle(BOOL, RESPAWN, zero.clone())
            .spare(INT, "EnemyRemain")
            .spare(BOOL, "AllDestroyed")
            .spare(BOOL, "Dead")
            .spare(BOOL, "Logic_Alive");
        s.role = QUERY;
        a.add_node(&s)?;
    }
    let bad: Vec<String> = audit(a);
    if !bad.is_empty() {
        return Err(E::Msg(format!("the gate graph no longer checks out: {}", bad.join(", "))));
    }
    Ok(p.guards.len() + p.hands.len() + p.counters.len() + 3)
}

fn source(rf: &mut Romfs, have: Have, p: &str) -> Res<Vec<u8>> {
    match have(p) {
        Some(b) => Ok(b),
        None => rf.read(p),
    }
}

struct Loaded {
    map: &'static str,
    path_s: String,
    path_d: String,
    dict_s: u32,
    dict_d: u32,
    raw_s: Vec<u8>,
    raw_d: Vec<u8>,
    ys: Byml,
    yd: Byml,
    moved: bool,
}

fn load(rf: &mut Romfs, have: Have, g: &Gate) -> Res<Loaded> {
    let (ps, pd): (String, String) = (file(g, "Static"), file(g, "Dynamic"));
    let zs_s: Vec<u8> = source(rf, have, &ps)?;
    let zs_d: Vec<u8> = source(rf, have, &pd)?;
    let raw_s: Vec<u8> = rf.zs.decompress(&zs_s)?;
    let raw_d: Vec<u8> = rf.zs.decompress(&zs_d)?;
    Ok(Loaded { map: g.map, path_s: ps, path_d: pd, dict_s: dict_id(&zs_s).unwrap_or(BANC_DICT), dict_d: dict_id(&zs_d).unwrap_or(BANC_DICT), ys: Byml::parse(&raw_s)?, yd: Byml::parse(&raw_d)?, raw_s, raw_d, moved: false })
}

pub fn stage(rf: &mut Romfs, have: Have, gates: &[Gate]) -> Res<Vec<Out>> {
    let mut maps: Vec<Loaded> = Vec::new();
    let mut plans: Vec<Plan> = Vec::new();
    for g in gates {
        let at: usize = match maps.iter().position(|m| m.map == g.map) {
            Some(i) => i,
            None => {
                maps.push(load(rf, have, g)?);
                maps.len() - 1
            }
        };
        let m: &mut Loaded = &mut maps[at];
        let mut yd: Byml = m.yd.clone();
        let raws: [&[u8]; 2] = [&m.raw_s, &m.raw_d];
        let Some(found) = resolve(&m.ys, &mut yd, &raws, g)? else {
            continue;
        };
        if found.add.iter().any(|n| actors(&m.yd).iter().any(|a| hash_of(a) == hash_of(n))) {
            m.moved = true;
        }
        m.yd = yd;
        plans.push(place(&mut m.ys, &raws, g, &found)?);
    }
    let mut outs: Vec<Out> = Vec::new();
    for m in &maps {
        let bs: Vec<u8> = m.ys.write()?;
        outs.push(Out { path: m.path_s.clone(), old: m.raw_s.len(), new: bs.len(), z: zs::compress_raw(&bs, Some(m.dict_s))? });
        if m.moved {
            let bd: Vec<u8> = m.yd.write()?;
            outs.push(Out { path: m.path_d.clone(), old: m.raw_d.len(), new: bd.len(), z: zs::compress_raw(&bd, Some(m.dict_d))? });
        }
    }
    if plans.is_empty() {
        return Ok(Vec::new());
    }
    let was: Vec<u8> = source(rf, have, GRAPH)?;
    for p in &plans {
        let mut a: Ainb = Ainb::parse(&was)?;
        wire(&mut a, p)?;
        let g: Vec<u8> = a.write()?;
        outs.push(Out { path: p.graph.clone(), old: 0, new: g.len(), z: g });
    }
    if gates.iter().any(|x| x.new_kind == KNIGHT) && have(wake::PACK).is_none() {
        outs.push(wake::stage(rf, have)?);
    }
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::difficulty::encounters::RING;

    const G: Gate = Gate { id: 9, name: "t", map: "T_Child", a: [0.0, 5.0, 0.0], b: [6.0, 6.0, 0.0], spot: [3.0, 5.0, 10.0], more: &[], also: &[], sink: 0.0, radius: RADIUS, new_kind: "Enemy_New", more_kind: "", loose: &[], scale: 1.0, closed_forest_only: false };

    fn foe(kind: &str, h: u64, at: [f32; 3], flag: bool) -> Node {
        let mut a: Node = actor(kind, h, at, [0.0; 3]);
        if flag {
            a.set("Presence", Node::Map(vec![("FlagName".to_string(), Node::Str("F".to_string()))]));
        }
        a
    }

    fn map(acts: Vec<Node>, groups: bool) -> Byml {
        let mut root: Node = Node::Map(Vec::new());
        root.set("Actors", Node::Arr(acts));
        if groups {
            root.set("AiGroups", Node::Arr(vec![Node::Map(vec![("Hash".to_string(), Node::U64(u64::MAX))])]));
            root.set("FarDeleteGroups", Node::Arr(Vec::new()));
            root.set("SimultaneousGroups", Node::Arr(Vec::new()));
        }
        Byml { root, version: 7, big: false }
    }

    fn found_two() -> Found {
        let a: Node = foe("Enemy_X", 10, [3.0, 5.0, 9.0], false);
        let b: Node = foe("Enemy_X", 11, [4.0, 5.0, 9.0], false);
        Found { guards: vec![a, b], modes: vec![Mode::Trigger, Mode::Never], add: Vec::new(), ring: Vec::new() }
    }

    #[test]
    fn the_barrier_pieces_cover_every_wall_and_a_margin_on_each_side() {
        for g in GATES.iter() {
            let mut total: usize = 0;
            for (a, b) in walls(g) {
                let ps: Vec<[f32; 3]> = wall_pieces(a, b, g.sink);
                let len: f32 = (b[0] - a[0]).hypot(b[2] - a[2]);
                assert_eq!(ps.len(), wall_count(a, b));
                total += ps.len();
                let d: [f32; 2] = [(b[0] - a[0]) / len, (b[2] - a[2]) / len];
                let along = |p: [f32; 3]| (p[0] - a[0]) * d[0] + (p[2] - a[2]) * d[1];
                let lo: f32 = ps.iter().map(|p| along(*p)).fold(f32::MAX, f32::min) - WIDTH / 2.0;
                let hi: f32 = ps.iter().map(|p| along(*p)).fold(f32::MIN, f32::max) + WIDTH / 2.0;
                assert!(lo <= -MARGIN + 1e-3 && hi >= len + MARGIN - 1e-3, "gate {}", g.id);
                assert!(ps.windows(2).all(|w| (dist(w[0], w[1]) - WIDTH).abs() < 1e-3));
                assert!(ps.iter().all(|p| (p[1] - (a[1].min(b[1]) - g.sink)).abs() < 1e-4));
            }
            assert_eq!(pieces(g).len(), total);
            assert_eq!(count(g), total);
        }
        assert_eq!(GATES.iter().map(count).collect::<Vec<usize>>(), vec![2, 3, 2, 2, 3, 3, 2, 4, 3, 3, 6, 4, 2, 2, 2, 2, 14, 4, 4, 4, 3]);
    }

    #[test]
    fn the_field_barrier_sits_half_a_unit_lower_and_the_rest_do_not_move() {
        for g in GATES.iter() {
            assert_eq!(g.sink, if g.id == 13 { 0.5 } else { 0.0 }, "gate {}", g.id);
        }
        let g: &Gate = GATES.iter().find(|g| g.id == 13).unwrap();
        assert!(pieces(g).iter().all(|(p, _)| (p[1] - 135.03).abs() < 1e-3));
    }

    #[test]
    fn the_trigger_box_lies_wholly_on_the_guard_side_of_every_wall() {
        for g in GATES.iter() {
            let h: [f32; 2] = area_half(g);
            let ctr: [f32; 2] = area_box(g).0;
            assert!(h[0] >= SMALLEST && h[1] >= SMALLEST && h[0] <= g.radius && h[1] <= g.radius, "gate {}", g.id);
            for (a, b) in walls(g) {
                let (n, d): ([f32; 2], f32) = wall_side(a, b, g.spot);
                assert!(d > 0.0, "gate {}", g.id);
                for sx in [-1.0f32, 1.0] {
                    for sz in [-1.0f32, 1.0] {
                        let c: [f32; 2] = [ctr[0] + sx * h[0], ctr[1] + sz * h[1]];
                        let side: f32 = n[0] * (c[0] - a[0]) + n[1] * (c[1] - a[2]);
                        assert!(side >= 2.5 + BARRIER_HALF - 1e-3, "gate {} corner {sx} {sz}", g.id);
                    }
                }
            }
        }
        let far: Gate = Gate { a: [0.0, 5.0, 0.0], b: [6.0, 5.0, 0.0], spot: [3.0, 5.0, 4.0], ..G };
        assert!(area_half(&far)[1] <= 3.0 + 1e-3);
        assert_eq!(area_half(&G), [RADIUS, 6.5]);
    }

    #[test]
    fn the_room_gate_shuts_both_doors_on_one_guard_and_the_gerudo_gate_has_two_skeletons() {
        let room: &Gate = GATES.iter().find(|g| g.id == 9).unwrap();
        assert_eq!(room.also.len(), 1);
        assert_eq!(room.new_kind, SKELETON_GUARD);
        assert!(room.more.is_empty());
        assert!(GATES.iter().all(|g| g.id != 10));
        let ys: Byml = map(Vec::new(), true);
        let mut yd: Byml = map(Vec::new(), false);
        let f: Found = resolve(&ys, &mut yd, &[], room).unwrap().unwrap();
        assert_eq!(f.guards.len(), 1);
        let plan: Plan = names(room, &[SKELETON_GUARD.to_string()], &f.modes, &[]);
        assert_eq!(plan.counters.len(), count(room));
        let valley: &Gate = GATES.iter().find(|g| g.id == 12).unwrap();
        assert_eq!(valley.new_kind, SKELETON_GUARD);
        let two: Found = resolve(&ys, &mut yd, &[], valley).unwrap().unwrap();
        assert_eq!(two.guards.len(), 2);
        assert!((dist(valley.spot, valley.more[0]) - 3.0).abs() < 0.1);
    }

    #[test]
    fn the_scrub_and_dodongo_gates_make_their_own_guard_and_leave_the_vanilla_ones_alone() {
        for id in [1u8, 2] {
            let g: &Gate = GATES.iter().find(|g| g.id == id).unwrap();
            assert!(makes(g));
            let ys: Byml = map(vec![foe(g.new_kind, 1, g.spot, false)], true);
            let mut yd: Byml = map(vec![foe(g.new_kind, 2, g.spot, false)], false);
            let f: Found = resolve(&ys, &mut yd, &[], g).unwrap().unwrap();
            assert_eq!(f.modes, vec![Mode::Trigger]);
            assert_eq!(actors(&yd).len(), 1);
            assert!(f.guards.iter().all(|n| hash_of(n) != Some(1) && hash_of(n) != Some(2)));
        }
        assert_eq!(GATES.iter().filter(|g| !makes(g)).map(|g| g.id).collect::<Vec<u8>>(), vec![6]);
        assert_eq!(GATES[0].new_kind, SCRUB_GUARD);
        assert_eq!(GATES[1].new_kind, DODONGO_GUARD);
    }

    #[test]
    fn the_barrier_runs_along_the_marked_wall() {
        let g: &Gate = &GATES[0];
        assert!(yaw(g).abs() < 1e-3);
        let (s, c): (f32, f32) = yaw(&G).sin_cos();
        assert!((c - 1.0).abs() < 1e-3 && s.abs() < 1e-3);
        let turned: Gate = Gate { a: [0.0, 0.0, 0.0], b: [0.0, 0.0, 4.0], ..G };
        let (s, c): (f32, f32) = yaw(&turned).sin_cos();
        assert!(c.abs() < 1e-3 && (s - 1.0).abs() < 1e-3);
    }

    #[test]
    fn the_gates_follow_the_forest_and_entrance_settings() {
        assert!(GATES.iter().all(|g| active(g, false, false, Shuffle::Off)));
        assert!(GATES.iter().all(|g| !active(g, false, false, Shuffle::Wide)));
        let kept: Vec<u8> = GATES.iter().filter(|g| active(g, false, false, Shuffle::Dungeons)).map(|g| g.id).collect();
        assert_eq!(kept, vec![3, 4, 5, 7, 9, 11, 12, 13, 23]);
        assert!(GATES.iter().filter(|g| (14..23).contains(&g.id)).all(|g| active(g, false, false, Shuffle::Off) && !active(g, false, false, Shuffle::Dungeons)));
        let closed: Vec<u8> = GATES.iter().filter(|g| g.closed_forest_only).map(|g| g.id).collect();
        assert_eq!(closed, vec![6]);
        assert_eq!(GATES.iter().filter(|g| active(g, true, false, Shuffle::Off)).count(), GATES.len() - 1);
        assert_eq!(GATES.iter().filter(|g| active(g, false, true, Shuffle::Off)).count(), GATES.len() - 1);
        assert_eq!(Shuffle::from_setting("off"), Shuffle::Off);
        assert_eq!(Shuffle::from_setting("dungeons"), Shuffle::Dungeons);
        assert_eq!(Shuffle::from_setting("everything"), Shuffle::Wide);
        assert_eq!(Shuffle::from_setting("nonsense"), Shuffle::Wide);
        assert!(shuffle_note(Shuffle::Off).is_none());
        let n: String = shuffle_note(Shuffle::Dungeons).unwrap();
        assert!(n.contains("gate 6") && n.contains("gate 1)") && n.contains("gate 2)") && !n.contains("gate 9"), "{n}");
        assert!(shuffle_note(Shuffle::Wide).unwrap().contains("gate 9"));
    }

    fn side(a: [f32; 3], b: [f32; 3], p: [f64; 3]) -> bool {
        let dx: f64 = (b[0] - a[0]) as f64;
        let dz: f64 = (b[2] - a[2]) as f64;
        dx * (p[2] - a[2] as f64) - dz * (p[0] - a[0] as f64) > 0.0
    }

    #[test]
    fn a_gate_is_left_out_exactly_when_a_dungeon_door_lies_beyond_its_wall() {
        let j: crate::json::Json = crate::json::parse(crate::entr::data::CATALOG).unwrap();
        let mut beyond: Vec<u8> = Vec::new();
        for e in j.get("entrances").and_then(|v| v.as_arr()).unwrap() {
            if e.get("cat").and_then(|v| v.as_str()) != Some("dungeon") {
                continue;
            }
            let map: &str = e.get("map").and_then(|v| v.as_str()).unwrap();
            let pos: Vec<f64> = e.get("pos").and_then(|v| v.as_arr()).unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
            let p: [f64; 3] = [pos[0], pos[1], pos[2]];
            for g in GATES.iter().filter(|g| g.map == map) {
                let guard: [f64; 3] = [g.spot[0] as f64, g.spot[1] as f64, g.spot[2] as f64];
                let mut walls: Vec<([f32; 3], [f32; 3])> = vec![(g.a, g.b)];
                walls.extend_from_slice(g.also);
                if walls.iter().any(|(a, b)| side(*a, *b, p) != side(*a, *b, guard)) {
                    beyond.push(g.id);
                }
            }
        }
        beyond.sort();
        beyond.dedup();
        assert_eq!(beyond, BEYOND_DUNGEON_DOOR.to_vec());
    }

    #[test]
    fn the_barrier_faces_the_guard_side_whatever_order_the_ends_were_marked_in() {
        for g in GATES.iter() {
            assert!(faces_guards(g), "gate {}", g.id);
            let flipped: Gate = Gate { a: g.b, b: g.a, ..*g };
            assert!(faces_guards(&flipped), "gate {} flipped", g.id);
        }
    }

    #[test]
    fn the_trigger_area_is_a_box_round_the_guard_spot_and_the_arrival_point_is_outside_it() {
        for g in GATES.iter() {
            let c: [f32; 3] = area_at(g);
            let s: [f32; 3] = area_size(g);
            assert!(c[0] - s[0] / 2.0 < g.spot[0] && g.spot[0] < c[0] + s[0] / 2.0, "gate {}", g.id);
            assert!(c[1] < g.spot[1] && g.spot[1] < c[1] + s[1], "gate {}", g.id);
            let m: [f32; 3] = mid(g);
            let far: [f32; 3] = approach(g, 13.0);
            assert!(dist([far[0], 0.0, far[2]], [m[0], 0.0, m[2]]) > dist([g.spot[0], 0.0, g.spot[2]], [m[0], 0.0, m[2]]), "gate {}", g.id);
            assert!(dist([far[0], 0.0, far[2]], [g.spot[0], 0.0, g.spot[2]]) > g.radius, "gate {}", g.id);
        }
    }

    #[test]
    fn guards_are_the_enemies_near_the_spot_flagged_ones_included_and_the_harmless_scrubs_left_out() {
        let acts: Vec<Node> = vec![
            foe("Enemy_X", 1, [3.0, 5.0, 12.0], false),
            foe("Enemy_X", 2, [3.0, 5.0, 30.0], false),
            foe("Enemy_X", 3, [3.0, 5.0, 11.0], true),
            foe("Enemy_Scrub_Mad_Leaf", 4, [3.0, 5.0, 10.0], false),
            foe("Enemy_Scrub_Business", 6, [3.0, 5.0, 10.0], false),
            foe("Rock", 5, [3.0, 5.0, 10.0], false),
        ];
        let y: Byml = map(acts, true);
        let got: Vec<u64> = near_guards(&y, G.spot).iter().filter_map(hash_of).collect();
        assert_eq!(got, vec![1, 3]);
    }

    #[test]
    fn guards_in_the_dynamic_map_move_to_the_static_one() {
        let ys: Byml = map(vec![foe("Enemy_X", 1, [3.0, 5.0, 12.0], false)], true);
        let mut yd: Byml = map(vec![foe("Enemy_X", 2, [3.0, 5.0, 8.0], false), foe("Enemy_X", 3, [60.0, 5.0, 8.0], false)], false);
        let old: Gate = Gate { new_kind: "", ..G };
        let f: Found = resolve(&ys, &mut yd, &[], &old).unwrap().unwrap();
        assert_eq!(f.modes, vec![Mode::Always, Mode::Always]);
        assert_eq!(f.guards.iter().filter_map(hash_of).collect::<Vec<u64>>(), vec![1, 2]);
        assert_eq!(f.add.iter().filter_map(hash_of).collect::<Vec<u64>>(), vec![2]);
        assert_eq!(actors(&yd).iter().filter_map(hash_of).collect::<Vec<u64>>(), vec![3]);
    }

    #[test]
    fn a_new_guard_is_added_where_none_stands_and_only_if_the_gate_names_a_kind() {
        let ys: Byml = map(vec![foe("Enemy_X", 1, [40.0, 5.0, 12.0], false)], true);
        let mut yd: Byml = map(Vec::new(), false);
        let f: Found = resolve(&ys, &mut yd, &[], &G).unwrap().unwrap();
        assert_eq!(f.guards.len(), 1);
        assert_eq!(f.modes, vec![Mode::Trigger]);
        assert_eq!(f.add, f.guards);
        assert_eq!(text(&f.guards[0], "Gyaml"), "Enemy_New");
        assert_eq!(vec3(&f.guards[0], "Translate"), Some(G.spot));
        let none: Gate = Gate { new_kind: "", ..G };
        assert_eq!(resolve(&ys, &mut yd, &[], &none).unwrap(), None);
    }

    #[test]
    fn an_existing_guard_that_a_vanilla_group_owns_is_never_created_a_second_time() {
        let mut ys: Byml = map(vec![foe("Enemy_X", 1, [3.0, 5.0, 12.0], false), foe("Enemy_X", 2, [4.0, 5.0, 12.0], false)], true);
        let mut r: Node = Node::Map(Vec::new());
        r.set("Reference", Node::U64(2));
        let mut grp: Node = Node::Map(Vec::new());
        grp.set("Hash", Node::U64(5));
        grp.set("References", Node::Arr(vec![r]));
        ys.root.get_mut("AiGroups").unwrap().as_arr_mut().unwrap().push(grp);
        let mut yd: Byml = map(Vec::new(), false);
        let old: Gate = Gate { new_kind: "", ..G };
        let f: Found = resolve(&ys, &mut yd, &[], &old).unwrap().unwrap();
        assert_eq!(f.guards.iter().filter_map(hash_of).collect::<Vec<u64>>(), vec![1, 2]);
        assert_eq!(f.modes, vec![Mode::Always, Mode::Never]);
        assert!(f.add.is_empty());
    }

    #[test]
    fn a_gate_that_makes_its_guards_makes_every_marked_one_and_leaves_vanilla_enemies_alone() {
        let ys: Byml = map(vec![foe("Enemy_Other", 1, [3.0, 5.0, 9.0], false)], true);
        let mut yd: Byml = map(Vec::new(), false);
        let two: Gate = Gate { more: &[[8.0, 5.0, 14.0]], ..G };
        let f: Found = resolve(&ys, &mut yd, &[], &two).unwrap().unwrap();
        assert_eq!(f.guards.len(), 2);
        assert_eq!(f.modes, vec![Mode::Trigger, Mode::Trigger]);
        assert_eq!(vec3(&f.guards[0], "Translate"), Some(two.spot));
        assert_eq!(vec3(&f.guards[1], "Translate"), Some([8.0, 5.0, 14.0]));
        assert!(f.guards.iter().all(|n| text(n, "Gyaml") == "Enemy_New"));
        assert_ne!(hash_of(&f.guards[0]), hash_of(&f.guards[1]));
        assert_eq!(f.add.len(), 2);
        let gate5: &Gate = GATES.iter().find(|g| g.id == 5).unwrap();
        assert_eq!(gate5.more.len(), 1);
    }

    #[test]
    fn each_guard_gets_its_own_stack_of_barrier_pieces_and_one_graph_group() {
        let mut y: Byml = map(vec![actor("Other", 1, [0.0; 3], [0.0; 3])], true);
        let f: Found = found_two();
        let plan: Plan = place(&mut y, &[], &G, &f).unwrap();
        assert_eq!(plan, names(&G, &["Enemy_X".to_string(), "Enemy_X".to_string()], &f.modes, &[]));
        assert!(plan.module.starts_with("Logic/Module/Gimmick_EnemyGate_09/") && plan.graph.starts_with("Logic/Gimmick_EnemyGate_09_"));
        assert_eq!(plan.guards.len(), 2);
        assert_eq!(plan.counters.len(), 2 * count(&G));
        let acts: &[Node] = actors(&y);
        let bars: Vec<&Node> = acts.iter().filter(|a| text(a, "Gyaml") == BARRIER).collect();
        assert_eq!(bars.len(), plan.counters.len());
        assert_eq!(acts.len(), 2 + bars.len());
        let areas: Vec<&Node> = acts.iter().filter(|a| text(a, "Gyaml") == AREA).collect();
        assert_eq!(areas.len(), 1);
        assert_eq!(vec3(areas[0], "Translate"), Some(area_at(&G)));
        assert_eq!(vec3(areas[0], "Scale"), Some(area_size(&G)));
        for (b, (j, _)) in bars.iter().zip(plan.counters.iter()) {
            let l: &Node = &b.get("Links").unwrap().as_arr().unwrap()[0];
            assert_eq!(l.get("Src"), b.get("Hash"));
            assert_eq!(l.get("Dst"), f.guards[*j].get("Hash"));
            assert_eq!(text(l, "Name"), "Reference");
            assert!(b.get("Presence").is_none());
        }
        let ps: Vec<([f32; 3], f32)> = pieces(&G);
        for (n, b) in bars.iter().enumerate() {
            assert_eq!(vec3(b, "Translate"), Some(ps[n % ps.len()].0));
        }
        let mut hs: Vec<u64> = acts.iter().filter_map(hash_of).collect();
        hs.extend(f.guards.iter().filter_map(hash_of));
        let all: usize = hs.len();
        hs.sort();
        hs.dedup();
        assert_eq!(hs.len(), all);
        let gs: &[Node] = y.root.get("AiGroups").unwrap().as_arr().unwrap();
        assert_eq!(gs.len(), 2);
        assert!(gs.windows(2).all(|w| hash_of(&w[0]) < hash_of(&w[1])));
        let g: &Node = gs.iter().find(|g| text(g, "Logic") == plan.module).unwrap();
        let rs: &[Node] = g.get("References").unwrap().as_arr().unwrap();
        let ids: Vec<String> = rs.iter().map(|r| text(r, "Id").to_string()).collect();
        let want: Vec<String> = plan.guards.iter().cloned().chain(std::iter::once(plan.area.clone())).chain(plan.counters.iter().map(|(_, c)| c.clone())).collect();
        assert_eq!(ids, want);
        for name in SHARED {
            assert_eq!(y.root.get(name).unwrap().as_arr().unwrap().len(), plan.counters.len() + plan.guards.len());
        }
        let back: Byml = Byml::parse(&y.write().unwrap()).unwrap();
        assert_eq!(back.root, y.root);
    }

    #[test]
    fn a_new_guard_is_appended_once_and_one_already_in_the_map_is_refused() {
        let mut y: Byml = map(vec![actor("Other", 1, [0.0; 3], [0.0; 3])], true);
        let n: Node = foe("Enemy_New", 77, G.spot, false);
        let f: Found = Found { guards: vec![n.clone()], modes: vec![Mode::Trigger], add: vec![n], ring: Vec::new() };
        place(&mut y, &[], &G, &f).unwrap();
        assert_eq!(actors(&y).iter().filter(|a| text(a, "Gyaml") == "Enemy_New").count(), 1);
        assert!(place(&mut y, &[], &Gate { id: 8, ..G }, &f).is_err());
    }

    #[test]
    fn names_stay_distinct_and_stable() {
        let a: String = id_for("Enemy_X", "s", &[]);
        assert_eq!(a, id_for("Enemy_X", "s", &[]));
        assert_ne!(a, id_for("Enemy_X", "s", &[a.clone()]));
        assert!(a.starts_with("Enemy_X_") && a.len() == "Enemy_X_".len() + 4);
        let p: Plan = names(&G, &["Enemy_X".to_string()], &[Mode::Trigger], &["Hand_X".to_string()]);
        let mut all: Vec<String> = p.guards.clone();
        all.extend(p.counters.iter().map(|(_, c)| c.clone()));
        all.push(p.area.clone());
        all.extend(p.hands.iter().cloned());
        let n: usize = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), n);
    }

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        if p.is_dir() { Some(p) } else { None }
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    fn out_map(outs: &[Out], rf: &mut Romfs, path: &str) -> Byml {
        let o: &Out = outs.iter().find(|o| o.path == path).unwrap();
        Byml::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap()
    }

    #[test]
    fn the_game_files_take_every_gate_and_the_graph_grows_to_match() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let outs: Vec<Out> = stage(&mut rf, &none, &GATES).unwrap();
        let mut maps: Vec<&str> = GATES.iter().map(|g| g.map).collect();
        maps.dedup();
        for map in maps {
            let mine: Vec<&Gate> = GATES.iter().filter(|g| g.map == map).collect();
            let path: String = file(mine[0], "Static");
            let was: Byml = rf.read_byml(&path).unwrap();
            let now: Byml = out_map(&outs, &mut rf, &path);
            let count_of = |y: &Byml, k: &str| actors(y).iter().filter(|a| text(a, "Gyaml") == k).count();
            let old_groups: Vec<Option<u64>> = was.root.get("AiGroups").unwrap().as_arr().unwrap().iter().map(hash_of).collect();
            let gs: &[Node] = now.root.get("AiGroups").unwrap().as_arr().unwrap();
            assert!(gs.windows(2).all(|w| hash_of(&w[0]) < hash_of(&w[1])), "{map}");
            let added: Vec<&Node> = gs.iter().filter(|x| text(x, "Logic").starts_with("Logic/Module/Gimmick_EnemyGate_") && !old_groups.contains(&hash_of(x))).collect();
            assert_eq!(added.len(), mine.len(), "{map}");
            let mut bars: usize = 0;
            for grp in added {
                let ids: Vec<&str> = grp.get("References").unwrap().as_arr().unwrap().iter().map(|r| text(r, "Id")).collect();
                assert_eq!(ids.iter().filter(|i| i.starts_with("Area_")).count(), 1, "{map}");
                let guards: usize = ids.iter().filter(|i| i.starts_with("Enemy_")).count();
                let counters: usize = ids.iter().filter(|i| i.starts_with(BARRIER)).count();
                assert!(guards >= 1 && counters >= 1, "{map}");
                bars += counters;
            }
            assert_eq!(count_of(&now, BARRIER), count_of(&was, BARRIER) + bars, "{map}");
            assert_eq!(count_of(&now, AREA), count_of(&was, AREA) + mine.len(), "{map}");
            assert_eq!(&actors(&now)[..actors(&was).len()], actors(&was), "{map}: old actors untouched");
            let hs: Vec<u64> = actors(&now).iter().filter_map(hash_of).collect();
            let mut u: Vec<u64> = hs.clone();
            u.sort();
            u.dedup();
            assert_eq!(u.len(), hs.len(), "{map}");
            let back: Byml = Byml::parse(&now.write().unwrap()).unwrap();
            assert_eq!(back.root, now.root, "{map}");
        }
        assert!(outs.iter().any(|o| o.path == wake::PACK));
        let raw: Vec<u8> = rf.read(GRAPH).unwrap();
        let old: Ainb = Ainb::parse(&raw).unwrap();
        let graphs: Vec<&Out> = outs.iter().filter(|o| o.path.starts_with("Logic/Gimmick_EnemyGate_")).collect();
        assert_eq!(graphs.len(), GATES.len());
        assert!(graphs.iter().all(|o| o.old == 0));
        assert!(!outs.iter().any(|o| o.path == GRAPH), "the shared vanilla graph is left alone");
        let mut paths: Vec<&str> = graphs.iter().map(|o| o.path.as_str()).collect();
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), GATES.len());
        for o in graphs {
            let a: Ainb = Ainb::parse(&o.z).unwrap();
            assert!(a.els.len() > old.els.len());
            assert!(audit(&a).is_empty());
            let and_create = |i: usize| -> bool {
                let c: crate::ainb::Input = a.input(BOOL, a.find_in(i, BOOL, CREATE).unwrap()).unwrap();
                c.src >= 0 && c.plug == 1 && (a.el_name(c.src as usize) == AND_NODE || a.el_name(c.src as usize) == SWORD_AND)
            };
            let counters: Vec<usize> = (0..a.els.len()).filter(|&i| a.prop_str(i, NAME).is_some_and(|n| n.starts_with(BARRIER))).collect();
            assert!(!counters.is_empty());
            for i in counters {
                let x: crate::ainb::Input = a.input(BOOL, a.find_in(i, BOOL, FINISH).unwrap()).unwrap();
                assert_eq!(x.plug, 0);
                assert!(a.prop_str(x.src as usize, NAME).is_some_and(|n| n.starts_with("Enemy_")));
                assert!(and_create(i));
            }
            let id: u8 = o.path["Logic/Gimmick_EnemyGate_".len()..][..2].parse().unwrap();
            let gate: &Gate = GATES.iter().find(|g| g.id == id).unwrap();
            let swords: usize = (0..a.els.len()).filter(|&i| a.el_name(i) == SWORD_AND).count();
            assert_eq!(swords > 0, gate.map.ends_with("_Child"), "gate {id}: a child gate waits for the Kokiri Sword, an adult one does not");
            let foes: Vec<usize> = (0..a.els.len()).filter(|&i| a.el_name(i) == NODE && a.prop_str(i, NAME).is_some_and(|n| n.starts_with("Enemy_"))).collect();
            assert!(!foes.is_empty());
            for i in foes {
                let c: crate::ainb::Input = a.input(BOOL, a.find_in(i, BOOL, CREATE).unwrap()).unwrap();
                if makes(gate) {
                    assert!(and_create(i), "gate {id}: a new guard is created by the trigger that raises the barrier");
                } else if gate.map.ends_with("_Child") {
                    assert!(c.src >= 0 && !and_create(i), "gate {id}: an existing guard is created by the sword flag alone");
                } else {
                    assert!(c.src < 0, "gate {id}: an existing guard is not created by the trigger");
                }
            }
        }
        let mut table: crate::rsizetable::Rst = crate::rsizetable::load(&mut rf).unwrap();
        for o in outs.iter().filter(|o| o.path.starts_with("Logic/Gimmick_EnemyGate_")) {
            assert!(table.get(&o.path).is_none());
            assert_eq!(crate::staging::grow_out(&mut rf, &mut table, o).unwrap(), 1);
            assert!(table.get(&o.path).is_some_and(|n| n as usize > o.new), "{}", o.path);
        }
        let again: Vec<Out> = stage(&mut rf, &none, &GATES).unwrap();
        assert!(outs.iter().zip(again.iter()).all(|(x, y)| x.path == y.path && x.z == y.z));
    }

    #[test]
    fn the_barrier_class_and_the_guard_kind_exist_in_the_game() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        assert!(rf.read("Pack/Actor/Obj_EnemyCounter_A_08.pack.zs").is_ok());
        assert!(rf.read(&format!("Pack/Actor/{ZORA_GUARD}.pack.zs")).is_ok());
        for k in GATES.iter().flat_map(|g| [g.new_kind, g.more_kind]).filter(|k| !k.is_empty() && *k != LEEVER_GUARD && *k != LEEVER_GIANT) {
            assert!(rf.read(&format!("Pack/Actor/{k}.pack.zs")).is_ok(), "{k}");
        }
        let meadow: Byml = rf.read_byml(&file(&GATES[0], "Static")).unwrap();
        assert_eq!(actors(&meadow).iter().filter(|a| text(a, "Gyaml") == BARRIER).count(), 1);
    }

    #[test]
    fn a_dead_hand_gate_keeps_every_hand_well_inside_the_trigger_area() {
        for g in GATES.iter().filter(|g| g.new_kind == HAND) {
            assert!(g.radius - RING >= 6.0, "gate {}", g.id);
        }
    }

    #[test]
    fn the_special_guards_are_always_new_and_come_with_their_pieces() {
        let ys: Byml = map(vec![foe("Enemy_ReDead", 1, [3.0, 5.0, 12.0], false)], true);
        let mut yd: Byml = map(Vec::new(), false);
        let hand: Gate = Gate { new_kind: HAND, ..G };
        let f: Found = resolve(&ys, &mut yd, &[], &hand).unwrap().unwrap();
        assert_eq!(f.guards.len(), 1);
        assert_eq!(text(&f.guards[0], "Gyaml"), HAND);
        assert_eq!(f.ring.len(), 4);
        assert_eq!(f.add.len(), 5);
        let knight: Gate = Gate { new_kind: KNIGHT, ..G };
        let k: Found = resolve(&ys, &mut yd, &[], &knight).unwrap().unwrap();
        assert_eq!(text(&k.guards[0], "Gyaml"), KNIGHT);
        assert!(k.guards[0].get("Dynamic").unwrap().get(wake::KEY).is_some());
        let mut y: Byml = map(Vec::new(), true);
        let plan: Plan = place(&mut y, &[], &hand, &f).unwrap();
        assert_eq!(plan.hands.len(), 4);
        assert_eq!(y.root.get("SimultaneousGroups").unwrap().as_arr().unwrap().len(), 1 + count(&G) + 1);
    }

    #[test]
    fn the_twentieth_of_november_gates_make_the_marked_guards_for_the_ages_asked() {
        let want: [(u8, &str, &str, usize); 9] = [
            (14, "DeathMountainCrater_Adult", DODONGO_GUARD, 1),
            (15, "DeathMountainTrail_Child", DODONGO_GUARD, 1),
            (16, "DeathMountainTrail_Adult", DODONGO_GUARD, 4),
            (17, "Graveyard_Child", POE_GUARD, 3),
            (18, "Graveyard_Adult", POE_GUARD, 3),
            (19, "HyruleField_Child", ZORA_GUARD, 3),
            (20, "HyruleField_Child", SKELETON_GUARD, 3),
            (21, "HyruleField_Child", LIKELIKE_GUARD, 1),
            (22, "HyruleField_Adult", LIKELIKE_GUARD, 1),
        ];
        let ys: Byml = map(Vec::new(), true);
        let mut yd: Byml = map(Vec::new(), false);
        for (id, m, kind, n) in want {
            let g: &Gate = GATES.iter().find(|g| g.id == id).unwrap();
            assert_eq!(g.map, m, "gate {id}");
            assert!(!g.closed_forest_only && !HELD.contains(&id), "gate {id}");
            let f: Found = resolve(&ys, &mut yd, &[], g).unwrap().unwrap();
            assert_eq!(f.guards.len(), n, "gate {id}");
            assert_eq!(text(&f.guards[0], "Gyaml"), kind, "gate {id}");
            let rest: &str = if g.more_kind.is_empty() { kind } else { g.more_kind };
            assert!(f.guards[1..].iter().all(|x| text(x, "Gyaml") == rest), "gate {id}");
            assert!(f.modes.iter().all(|x| *x == Mode::Trigger), "gate {id}");
            assert_eq!(f.add.len(), n + g.loose.len(), "gate {id}");
            assert_eq!(g.loose.len(), if id >= 21 { 4 } else { 0 }, "gate {id}");
            assert!(f.guards[0].get("Scale").is_none(), "gate {id}");
            assert!(f.guards[1..].iter().all(|x| x.get("Scale").is_none()), "gate {id}");
        }
        assert!(GATES.iter().all(|g| g.scale == 1.0 && g.more_kind.is_empty()));
        for (c, a) in [(15u8, 16u8), (17, 18), (21, 22)] {
            let x: &Gate = GATES.iter().find(|g| g.id == c).unwrap();
            let y: &Gate = GATES.iter().find(|g| g.id == a).unwrap();
            assert!(x.map.ends_with("_Child") && y.map.ends_with("_Adult"));
            assert!(dist(x.a, y.a) < 0.5 && dist(x.b, y.b) < 0.5 && dist(x.spot, y.spot) < 1.0, "gates {c} and {a}");
            assert_eq!(x.name, y.name);
        }
    }

    #[test]
    fn the_ganon_castle_gate_puts_dark_link_in_the_first_round_room_and_the_lake_and_ranch_gates_use_their_new_kinds() {
        let g: &Gate = GATES.iter().find(|g| g.id == 23).unwrap();
        assert_eq!(g.map, "InsideGanonCastle");
        assert_eq!(g.new_kind, DARKLINK_GUARD);
        assert!(g.more.is_empty() && g.also.is_empty() && !g.closed_forest_only);
        assert!(file(g, "Static").ends_with("InsideGanonCastle/InsideGanonCastle_Static.bcett.byml.zs"));
        assert!(faces_guards(g));
        assert!(clear(g, [area_box(g).0[0], area_box(g).0[1]], area_box(g).1));
        let ys: Byml = map(Vec::new(), true);
        let mut yd: Byml = map(Vec::new(), false);
        let f: Found = resolve(&ys, &mut yd, &[], g).unwrap().unwrap();
        assert_eq!(f.guards.len(), 1);
        assert_eq!(text(&f.guards[0], "Gyaml"), "Enemy_MiddleBoss_DarkLink");
        let ranch: &Gate = GATES.iter().find(|g| g.id == 20).unwrap();
        assert_eq!((ranch.new_kind, ranch.more.len()), (SKELETON_GUARD, 2));
        for id in [21u8, 22] {
            let lake: &Gate = GATES.iter().find(|g| g.id == id).unwrap();
            assert_eq!((lake.new_kind, lake.more.len(), lake.loose.len()), (LIKELIKE_GUARD, 0, 4));
        }
        assert_eq!(GATES.iter().find(|g| g.id == 9).unwrap().new_kind, SKELETON_GUARD);
    }

    #[test]
    fn every_gate_guard_stands_on_the_navmesh_floor_with_clearance() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut bad: Vec<String> = Vec::new();
        for g in GATES.iter().filter(|g| makes(g)) {
            let Some(ground) = crate::difficulty::ground::load(&mut rf, g.map).unwrap() else {
                continue;
            };
            for (j, p) in std::iter::once(g.spot).chain(g.more.iter().copied()).enumerate() {
                let kind: &str = if j == 0 || g.more_kind.is_empty() { g.new_kind } else { g.more_kind };
                let flies: bool = kind == KEESE_GUARD || kind == POE_GUARD;
                let (up, down): (f32, f32) = if flies { (1.0, 10.0) } else { (0.6, 0.6) };
                match ground.floor_near(p[0], p[1], p[2], up, down) {
                    None => bad.push(format!("gate {} {:?} {kind}: no navmesh floor, floors here {:?}", g.id, p, ground.floors(p[0], p[2]).iter().map(|f| f.0).collect::<Vec<f32>>())),
                    Some((_, face)) => {
                        let c: f32 = ground.clear(p);
                        if !flies && c < 1.0 {
                            bad.push(format!("gate {} {:?} {kind}: {c:.2} from a wall (face {face})", g.id, p));
                        }
                    }
                }
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    const NEAR: f32 = 12.0;
    const DROP: f32 = 3.6;
    const JUMP: f32 = 4.1;
    const STEP: f32 = 0.8;
    const CELL: f32 = 0.5;
    const REACH: f32 = 140.0;
    const CLOSED: [u8; 1] = [14];
    const WEST_ROCKS: (f32, f32) = (-19.9, -33.0);

    fn seg_dist(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
        let d: [f32; 2] = [b[0] - a[0], b[1] - a[1]];
        let l: f32 = (d[0] * d[0] + d[1] * d[1]).max(1e-9);
        let t: f32 = (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / l).clamp(0.0, 1.0);
        (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1])
    }

    fn blocked(x: f32, y: f32, z: f32, g: &Gate) -> bool {
        (matches!(g.id, 15 | 16) && x < WEST_ROCKS.0 && z > WEST_ROCKS.1)
            || walls(g).into_iter().any(|(a, b)| {
                let len: f32 = (b[0] - a[0]).hypot(b[2] - a[2]);
                let d: [f32; 2] = [(b[0] - a[0]) / len, (b[2] - a[2]) / len];
                let (p, q): ([f32; 2], [f32; 2]) = ([a[0] - d[0] * MARGIN, a[2] - d[1] * MARGIN], [b[0] + d[0] * MARGIN, b[2] + d[1] * MARGIN]);
                (y - a[1].min(b[1])).abs() < 3.5 && seg_dist([x, z], p, q) < BARRIER_HALF
            })
    }

    fn lower(ground: &crate::difficulty::ground::Ground, x: f32, z: f32, from: f32, down: f32) -> Option<f32> {
        ground.floors(x, z).into_iter().map(|f| f.0).filter(|y| *y <= from && *y >= from - down).fold(None, |m: Option<f32>, y| Some(m.map_or(y, |m| m.max(y))))
    }

    fn leaks(rf: &mut Romfs, g: &Gate) -> Option<Vec<[f32; 3]>> {
        let ground = crate::difficulty::ground::load(rf, g.map).unwrap()?;
        if ground.tiles {
            return None;
        }
        let start: (f32, u32) = ground.floor_near(g.spot[0], g.spot[1], g.spot[2], 0.6, 0.6)?;
        let c: [f32; 3] = mid(g);
        let key = |x: f32, z: f32, y: f32| ((x / CELL).round() as i32, (z / CELL).round() as i32, (y * 2.0).round() as i32);
        let mut seen: std::collections::BTreeSet<(i32, i32, i32)> = std::collections::BTreeSet::new();
        let mut todo: Vec<[f32; 3]> = vec![[g.spot[0], start.0, g.spot[2]]];
        seen.insert(key(g.spot[0], g.spot[2], start.0));
        let mut out: Vec<[f32; 3]> = Vec::new();
        while let Some(p) = todo.pop() {
            for (dx, dz) in [(1.0f32, 0.0f32), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0), (1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                let mut found: Option<(f32, f32, f32)> = None;
                for k in 1..=(JUMP * 2.0 / dx.hypot(dz)) as usize {
                    let (x, z): (f32, f32) = (p[0] + dx * CELL * k as f32, p[2] + dz * CELL * k as f32);
                    if blocked(x, p[1], z, g) {
                        break;
                    }
                    let hit: Option<f32> = if k == 1 { lower(&ground, x, z, p[1] + STEP, DROP + STEP) } else { lower(&ground, x, z, p[1] - 0.3, DROP) };
                    if let Some(y) = hit {
                        found = Some((x, y, z));
                        break;
                    }
                    if k == 1 && ground.floors(x, z).iter().any(|f| f.0 > p[1] + STEP && f.0 < p[1] + 8.0) {
                        break;
                    }
                }
                let Some((x, y, z)) = found else {
                    continue;
                };
                if (x - c[0]).hypot(z - c[2]) > REACH || blocked(x, y, z, g) || !seen.insert(key(x, z, y)) {
                    continue;
                }
                todo.push([x, y, z]);
                for (a, b) in walls(g) {
                    let (n, _): ([f32; 2], f32) = wall_side(a, b, g.spot);
                    let side: f32 = n[0] * (x - a[0]) + n[1] * (z - a[2]);
                    if side < -2.0 && (y - a[1].min(b[1])).abs() < 6.0 && seg_dist([x, z], [a[0], a[2]], [b[0], b[2]]) < NEAR {
                        out.push([x, y, z]);
                    }
                }
            }
        }
        Some(out)
    }

    #[test]
    fn the_barrier_cuts_the_floor_so_a_walker_on_the_guard_side_cannot_reach_the_far_side() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut bad: Vec<String> = Vec::new();
        let mut checked: Vec<u8> = Vec::new();
        for g in GATES.iter().filter(|g| makes(g)) {
            let Some(l) = leaks(&mut rf, g) else {
                continue;
            };
            checked.push(g.id);
            if !l.is_empty() && CLOSED.contains(&g.id) {
                bad.push(format!("gate {}: {} floor points past the wall are reachable, first at {:?}", g.id, l.len(), l[0]));
            }
        }
        assert!(CLOSED.iter().all(|id| checked.contains(id)), "{checked:?}");
        assert!(bad.is_empty(), "{bad:#?}");
    }

    #[test]
    fn the_trail_guards_and_walls_keep_well_clear_of_the_goron_city_loading_zone() {
        let zone: [f32; 3] = [-1.65, 120.17, -26.12];
        for g in GATES.iter().filter(|g| matches!(g.id, 15 | 16)) {
            for p in std::iter::once(g.spot).chain(g.more.iter().copied()) {
                assert!(dist(p, zone) >= 8.0, "gate {} guard {:?}", g.id, p);
            }
            for (a, b) in walls(g) {
                let reach: f32 = MARGIN + BARRIER_HALF;
                let d: f32 = seg_dist([zone[0], zone[2]], [a[0], a[2]], [b[0], b[2]]);
                assert!(d - reach >= 8.0, "gate {} wall {d}", g.id);
            }
        }
    }
}
