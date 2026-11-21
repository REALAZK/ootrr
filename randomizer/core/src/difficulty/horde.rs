use std::collections::BTreeMap;
use std::collections::HashMap as Map;

use crate::byml::{Byml, Node};
use crate::difficulty::ambush::{actor, euler};
use crate::difficulty::extras::{fresh, gyaml, pos, raw, room, tally, ANY_EXT, EXT, ROOM, SKIP_DIR};
use crate::difficulty::ground::{self, Ground};
use crate::difficulty::wake;
use crate::error::Res;
use crate::qol::Have;
use crate::rng::Rng;
use crate::romfs::Romfs;
use crate::staging::{Out, BANC_DICT};
use crate::zs;

pub const SALT: &str = "horde";
pub const FROM: i64 = 100;
pub const MAP_CAP: usize = 240;
pub const ACTIVE_CAP: usize = 120;
pub const ACTIVE_RADIUS: f32 = 60.0;
pub const CLUSTER_CAP: usize = 12;
pub const GROUND_AREA: f32 = 30.0;
pub const AIR_AREA: f32 = 60.0;
pub const WATER_PER_ANCHOR: usize = 6;
pub const WATER_KEY: i64 = -1;
pub const OUTDOOR: i64 = 1 << 32;
pub const SAFE_CLEAR: f32 = 15.0;
pub const SAFE_WIDE: f32 = 30.0;
pub const SAFE_BAND: f32 = 12.0;
pub const SAFE_FEW: usize = 2;
pub const SAFE_ARC: f32 = 120.0;
pub const ARRIVAL_ANCHOR: &str = "DestinationAnchor";
pub const ARRIVAL_NAMES: &[&str] = &["FieldDungeonEntranceGo", "SwapRestartPosAnchor", "HidingHole_HoleExit", "HidingHole_HoleEntrance", "HidingHole_FieldExitGo", "HidingHole_FieldEntranceGo", "PlayerPos", "ReturnHome", "HidingHoleCoverDisappearStrom_SafePosition"];
pub const ARRIVALS: &[&str] = &["StartPos", "AbilityChangeAreaForSceneChange", "ChangeBancBeforehandAnchor", "Obj_WarpPoint_A_01", "FieldAndDungeonEntranceEventRequestActor", "MapIconEntranceAnchor"];
pub const CALM: &[&str] = &["Enemy_LikeLike", "Enemy_Dekubaba", "Enemy_Amos", "Enemy_TorchSlug", "Enemy_Skullwalltula", "Enemy_Floormaster"];
pub const CLUSTER_RADIUS: f32 = 15.0;
pub const NEW_KINDS_CAP: usize = 8;
pub const SPACING: f32 = 2.5;
pub const GROUP_SPACING: f32 = 1.3;
pub const GROUP_REACH: (f32, f32) = (1.6, 3.6);
pub const NEAR_HOME: (f32, f32) = (4.0, 14.0);
pub const WATER_REACH: (f32, f32) = (3.0, 15.0);
pub const WATER_DRY: f32 = 3.0;
pub const WATER_SHORE_ROOFED: f32 = 4.5;
pub const WATER_SHORE_OPEN: f32 = 9.0;
pub const WATER_ROOF: f32 = 10.0;
pub const TRIES: usize = 40;
pub const MAX_FACE: f32 = 20000.0;
pub const MIN_PATCH: f32 = 150.0;
pub const LIVED: f32 = 1.0;
pub const LIVED_MIN: u32 = 3;
pub const PROP_REACH: f32 = 30.0;
pub const SCENERY: &[&str] = &["Land_", "Tree_", "EftLoc", "SoundLocator", "Foliage", "Env", "Animal", "EnvironmentalAnimal"];
pub const PROP_BAND: f32 = 12.0;
pub const AREA_PER_FOE: f32 = 150.0;
pub const ROOM_REACH: f32 = 25.0;
pub const ROOM_BAND: f32 = 6.0;
pub const DUNGEON_LIFT: f32 = 2.5;
pub const GRAIN: u64 = 1 << 20;
pub const LOCAL_WEIGHT: u32 = 3;
pub const NEAR_SHARE: u64 = 60;
pub const SAND_MAPS: &[&str] = &["HountedWasteland_Child", "HountedWasteland_Adult", "DesertColossus_Child", "DesertColossus_Adult"];
pub const SKIP_MAPS: &[&str] = &[
    "Market_Child",
    "HyruleCastle",
    "GerudoFortress_Adult",
    "ZoraDomain_Child",
    "ZoraDomain_Adult",
    "ZoraDomain_Adult_Alternative",
    "GoronCity_Child",
    "GoronCity_Adult",
    "CrumblingGanonCastle",
    "SageRoom",
    "GerudoTrainingGround",
    "FinalBattleField",
    "HountedWasteland_Child",
];
pub const PEACEFUL: &[&str] = &["KokiriForest_Child", "KakarikoVillage_Child", "KakarikoVillage_Adult", "KakarikoVillage_Adult_Alternative"];
pub const BIG: &[&str] = &["Enemy_LikeLike", "Enemy_Wolfos", "Enemy_Stalfos", "Enemy_Dinolfos", "Enemy_Dodongo", "Enemy_ReDead", "Enemy_IronKnuckle", "Enemy_Moblin", "Enemy_Floormaster", "Enemy_Amos", "Enemy_Gibdo", "Enemy_DeadHand", "Enemy_Lizalfos"];
pub const BIG_GAP: f32 = 25.0;
pub const KNIGHT_PREFIX: &str = "Enemy_IronKnuckle";
pub const KNIGHTS_FIELD: usize = 2;
pub const KNIGHTS_ELSEWHERE: usize = 1;
pub const PERCHERS: &[&str] = &["Enemy_Keese", "Enemy_Keese_Ice", "Enemy_Keese_Fire"];
pub const PERCH_TREE: &str = "Tree";
pub const PERCH_LIFT: f32 = 4.0;
pub const PERCH_WALL_LIFT: f32 = 3.0;
pub const PERCH_NUDGE: f32 = 0.8;
pub const HOME_ONLY: &[&str] = &["Enemy_Keese_Fire"];
pub const BLASTERS: &[&str] = &["Enemy_Dodongo", "Enemy_BabyDodongo"];
pub const NO_BLASTERS: &[&str] = &["DodongoCavern"];
pub const BLASTABLE: &[&str] = &["WallCrack", "CrackWall", "HitOnlyBomb", "HitBomb", "Obj_InsideTheDekuTree_SpiderWeb", "InteractiveSpiderWeb", "BreakableRock", "BombRock"];
pub const SIGHTS: &[&str] = &["Tree", "LookAttentionActor", "InterestSpotActor"];
pub const GUARD_SHARE: u64 = 50;
pub const GUARD_REACH: (f32, f32) = (4.0, 10.0);
pub const RISING_WATER: &[&str] = &["WaterTemple"];
pub const NO_MINIBOSS: &[&str] = &["KokiriForest_Child"];
pub const SKIP_PREFIX: &[&str] = &["Demo_"];
pub const FIGHT_SKIP: &[&str] = &["Gold", "Scrub", "DungeonBoss", "_Dictionary", "Haribote", "Attention", "Manager", "Performer"];
pub const BOSSY: &[&str] = &["Enemy_DungeonBoss", "Enemy_MiddleBoss"];
pub const TEMPLATE_KEYS: &[&str] = &["Dynamic", "Gyaml", "Hash", "Phive", "Rotate", "SRTHash", "Translate"];
pub const DROP_KEYS: &[&str] = &["Event", "GameData", "Drop__DropActor", ROOM];
pub const AVOID: &[(&str, f32)] = &[
    ("DestinationAnchor", 8.0),
    ("StartPos", 8.0),
    ("ChangeBancBeforehandAnchor", 8.0),
    ("AbilityChangeAreaForSceneChange", 10.0),
    ("FieldAndDungeonEntranceEventRequestActor", 10.0),
    ("MapIconEntranceAnchor", 8.0),
    ("EventRequestActor", 6.0),
    ("EventProxyRequestActor", 6.0),
    ("Npc_", 10.0),
    ("Shop_", 8.0),
    ("Obj_TBox", 4.0),
    ("SignBoard", 3.0),
    ("Signboard", 3.0),
    ("GossipStone", 4.0),
    ("Door", 5.0),
    ("Warp", 6.0),
    ("_Inside_", 6.0),
    ("_Interior_", 6.0),
    ("Fishing", 15.0),
    ("Horse", 10.0),
    ("Obj_FairySpirit", 5.0),
    ("Obj_EnemyCounter", 4.0),
    ("Enemy_DungeonBoss", 40.0),
    ("Enemy_MiddleBoss", 25.0),
    ("Enemy_Skulltula_Gold", 3.0),
    ("LonLon", 45.0),
    ("TempleOfTime", 30.0),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    Flat,
    Gentle,
    Any,
    Fly,
    Water,
    Ceiling,
    Sand,
    Mini,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kind {
    pub gyaml: &'static str,
    pub class: Class,
    pub fun: u32,
    pub lift: f32,
    pub group: u32,
    pub away: bool,
}

const fn k(gyaml: &'static str, class: Class, fun: u32, lift: f32, group: u32, away: bool) -> Kind {
    Kind { gyaml, class, fun, lift, group, away }
}

pub const KINDS: &[Kind] = &[
    k("Enemy_Stalfos", Class::Mini, 3, 0.0, 1, false),
    k("Enemy_Dinolfos", Class::Mini, 3, 0.0, 2, false),
    k("Enemy_IronKnuckle_Silver", Class::Mini, 2, 0.0, 1, true),
    k("Enemy_Wolfos_White", Class::Mini, 2, 0.0, 1, false),
    k("Enemy_Wolfos", Class::Flat, 3, 0.0, 3, false),
    k("Enemy_Floormaster", Class::Flat, 2, 0.0, 1, false),
    k("Enemy_Dodongo", Class::Flat, 2, 0.0, 2, false),
    k("Enemy_Amos", Class::Flat, 1, 0.0, 2, false),
    k("Enemy_LikeLike", Class::Gentle, 3, 0.0, 2, true),
    k("Enemy_ReDead", Class::Gentle, 1, 0.0, 3, false),
    k("Enemy_BabyDodongo", Class::Gentle, 2, 0.0, 4, true),
    k("Enemy_TorchSlug", Class::Gentle, 1, 0.0, 2, false),
    k("Enemy_Dekubaba", Class::Gentle, 2, 0.0, 3, false),
    k("Enemy_Tektite_Red", Class::Any, 3, 0.0, 4, false),
    k("Enemy_Tektite_Blue", Class::Any, 2, 0.0, 3, false),
    k("Enemy_Skullwalltula", Class::Any, 1, 0.0, 3, true),
    k("Enemy_Keese", Class::Fly, 3, 3.5, 5, true),
    k("Enemy_Keese_Ice", Class::Fly, 1, 3.5, 4, true),
    k("Enemy_Keese_Fire", Class::Fly, 3, 3.0, 1, false),
    k("Enemy_Guay_Fixed", Class::Fly, 2, 6.0, 4, true),
    k("Enemy_Poe", Class::Fly, 2, 0.0, 2, false),
    k("Enemy_Bubble_Green", Class::Fly, 1, 1.5, 2, false),
    k("Enemy_Bubble_Red", Class::Fly, 1, 1.5, 2, false),
    k("Enemy_Shabom_AlwaysRespawn", Class::Fly, 1, 2.0, 3, true),
    k("Enemy_Biri", Class::Fly, 1, 2.0, 3, false),
    k("Enemy_Octorok", Class::Water, 2, 0.0, 3, false),
    k("Enemy_Skulltula", Class::Ceiling, 2, 4.5, 1, false),
    k("Enemy_Wallmaster", Class::Ceiling, 2, 4.0, 1, false),
    k("Enemy_Leever_Fixed", Class::Sand, 3, 0.0, 4, true),
];

pub const NEVER: &[&str] = &[
    "Enemy_DungeonBoss",
    "Enemy_MiddleBoss",
    "Enemy_DeadHand",
    "Enemy_Gibdo",
    "Enemy_Bubble_Blue",
    "Enemy_Poe_Big",
    "Enemy_Poe_Hyrule",
    "Enemy_Poe_Amy",
    "Enemy_Poe_Beth",
    "Enemy_Poe_Joelle",
    "Enemy_Moblin",
    "Enemy_Scrub",
    "Enemy_Skulltula_Gold",
    "Enemy_Peahat",
    "Enemy_Gohma",
    "Enemy_Tentacle",
    "Enemy_Anubis",
    "Enemy_Beamos",
    "Enemy_CrazyFloorTile",
    "Enemy_TrickyDoor",
    "Enemy_FlyingPot",
    "Enemy_Lizalfos",
    "Enemy_IronKnuckle_White",
    "Enemy_IronKnuckle_Black",
    "Enemy_Spike",
    "Enemy_Stinger",
    "Enemy_ShellBlade",
    "Enemy_Bari",
    "Enemy_Tailpasaran",
    "Enemy_Stalchild",
    "Enemy_Floormaster_Little",
    "Enemy_SkullKid",
    "Enemy_Leever_Big",
    "Enemy_Shabom_BigOctoChild",
    "Enemy_Freezard",
];

pub fn limits(c: Class) -> (f32, f32, f32) {
    match c {
        Class::Flat => (10.0, 2.0, 2.5),
        Class::Mini => (8.0, 3.0, 4.0),
        Class::Gentle => (20.0, 1.5, 2.0),
        Class::Any | Class::Sand => (32.0, 1.0, 1.0),
        Class::Fly => (45.0, 0.0, 3.0),
        Class::Ceiling => (20.0, 1.0, 1.5),
        Class::Water => (90.0, 0.0, 0.0),
    }
}

pub fn big(g: &str) -> bool {
    BIG.iter().any(|b| g.starts_with(b))
}

pub fn percher(g: &str) -> bool {
    PERCHERS.contains(&g)
}

pub fn kind(g: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|k| k.gyaml == g)
}

pub fn fighting(g: &str) -> bool {
    g.starts_with("Enemy_") && !FIGHT_SKIP.iter().any(|s| g.contains(s))
}

pub fn map_of(file: &str) -> Option<String> {
    let n: &str = file.rsplit('/').next()?;
    let n: &str = n.strip_suffix(EXT)?;
    n.strip_suffix("_Static").or_else(|| n.strip_suffix("_Dynamic")).map(|s| s.to_string())
}

pub fn wanted(map: &str) -> bool {
    !SKIP_MAPS.contains(&map) && !PEACEFUL.contains(&map) && !SKIP_PREFIX.iter().any(|p| map.starts_with(p))
}

pub fn target(vanilla: usize, extra: i64, open: f32) -> usize {
    let m: f32 = (extra + 100) as f32 / 100.0;
    let by_count: f32 = vanilla as f32 * m;
    let by_room: f32 = if m > 2.0 { open / AREA_PER_FOE * (m - 2.0) / 8.0 } else { 0.0 };
    (by_count.max(by_room).round() as usize).min(MAP_CAP)
}

pub fn group_max(extra: i64) -> u32 {
    let m: i64 = (extra + 100) / 100;
    if m < 5 { 1 } else { (1 + (m - 3) / 2) as u32 }
}

pub fn mini_cap(extra: i64) -> usize {
    let m: i64 = (extra + 100) / 100;
    if m < 5 { 0 } else { (m - 4) as usize }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Spot {
    pub map: String,
    pub file: String,
    pub gyaml: String,
    pub class: Class,
    pub at: [f32; 3],
    pub yaw: f32,
    pub room: Option<i32>,
    pub floor: f32,
    pub safe: f32,
    pub area: i64,
    pub why: &'static str,
    pub hash: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub map: String,
    pub vanilla: usize,
    pub before: usize,
    pub target: usize,
    pub open: f32,
    pub spots: Vec<Spot>,
    pub areas: Vec<Area>,
    pub arrivals: Vec<[f32; 3]>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Area {
    pub key: i64,
    pub vanilla: usize,
    pub ground_cap: usize,
    pub air_cap: usize,
    pub limit: usize,
    pub ground: usize,
    pub air: usize,
    pub total: usize,
}

impl Area {
    pub fn room_for(&self, c: Class) -> bool {
        self.total < self.limit && if c == Class::Fly { self.air < self.air_cap } else { c == Class::Water || self.ground < self.ground_cap }
    }

    pub fn take(&mut self, c: Class) {
        self.total += 1;
        if c == Class::Fly {
            self.air += 1;
        } else if c != Class::Water {
            self.ground += 1;
        }
    }
}

pub fn area_target(vanilla: usize, extra: i64, usable: f32) -> usize {
    let m: f32 = (extra + 100) as f32 / 100.0;
    if vanilla > 0 {
        return (vanilla as f32 * m).round() as usize;
    }
    if m > 2.0 { (usable / AREA_PER_FOE * (m - 2.0) / 8.0).floor() as usize } else { 0 }
}

struct Scene {
    map: String,
    files: Vec<String>,
    acts: Vec<(usize, Node)>,
    vanilla: usize,
}

struct Ctx<'a> {
    g: &'a Ground,
    lived: Vec<bool>,
    avoid: Vec<([f32; 3], f32)>,
    foes: Vec<[f32; 3]>,
    rooms: Vec<([f32; 3], i32)>,
    banned: Vec<i32>,
    dungeon: bool,
    water: Vec<[f32; 3]>,
    cum: Vec<(f32, u32)>,
    props: Map<(i32, i32), Vec<[f32; 3]>>,
    reach: Vec<bool>,
}

pub fn cell(p: [f32; 3]) -> (i32, i32) {
    ((p[0] / PROP_REACH).floor() as i32, (p[2] / PROP_REACH).floor() as i32)
}

pub fn near_prop(props: &Map<(i32, i32), Vec<[f32; 3]>>, p: [f32; 3]) -> bool {
    let c: (i32, i32) = cell(p);
    for x in c.0 - 1..=c.0 + 1 {
        for z in c.1 - 1..=c.1 + 1 {
            if props.get(&(x, z)).is_some_and(|v| v.iter().any(|q| (q[0] - p[0]).hypot(q[2] - p[2]) < PROP_REACH && (q[1] - p[1]).abs() < PROP_BAND)) {
                return true;
            }
        }
    }
    false
}

pub fn centre(p: &[[f32; 3]]) -> [f32; 3] {
    let n: f32 = p.len().max(1) as f32;
    [p.iter().map(|v| v[0]).sum::<f32>() / n, p.iter().map(|v| v[1]).sum::<f32>() / n, p.iter().map(|v| v[2]).sum::<f32>() / n]
}

pub struct Crowd {
    pub pts: Vec<[f32; 3]>,
    pub n: Vec<usize>,
    pub far: Vec<usize>,
    pub arrivals: Vec<[f32; 3]>,
    pub zone: Vec<Vec<f32>>,
}

pub fn calm(k: &Kind) -> bool {
    CALM.contains(&k.gyaml)
}

pub fn flat_gap(a: [f32; 3], p: [f32; 3]) -> Option<f32> {
    if (a[1] - p[1]).abs() >= SAFE_BAND {
        return None;
    }
    Some((a[0] - p[0]).hypot(a[2] - p[2]))
}

pub fn open_arc(angles: &[f32]) -> f32 {
    if angles.is_empty() {
        return 360.0;
    }
    let mut v: Vec<f32> = angles.iter().map(|a| a.rem_euclid(360.0)).collect();
    v.sort_by(|a, b| a.total_cmp(b));
    let mut best: f32 = v[0] + 360.0 - v[v.len() - 1];
    for w in v.windows(2) {
        best = best.max(w[1] - w[0]);
    }
    best
}

pub fn bearing(a: [f32; 3], p: [f32; 3]) -> f32 {
    (p[2] - a[2]).atan2(p[0] - a[0]).to_degrees()
}

pub fn nearest_arrival(arrivals: &[[f32; 3]], p: [f32; 3]) -> f32 {
    arrivals.iter().filter_map(|a| flat_gap(*a, p)).fold(f32::MAX, f32::min)
}

impl Crowd {
    pub fn new(pts: &[[f32; 3]]) -> Crowd {
        let n: Vec<usize> = pts.iter().map(|p| pts.iter().filter(|q| near(*p, **q) < CLUSTER_RADIUS).count()).collect();
        let far: Vec<usize> = pts.iter().map(|p| pts.iter().filter(|q| near(*p, **q) < ACTIVE_RADIUS).count()).collect();
        Crowd { pts: pts.to_vec(), n, far, arrivals: Vec::new(), zone: Vec::new() }
    }

    pub fn with_arrivals(mut self, a: Vec<[f32; 3]>) -> Crowd {
        self.zone = vec![Vec::new(); a.len()];
        self.arrivals = a;
        self
    }

    pub fn safe(&self, k: &Kind, p: [f32; 3]) -> bool {
        for (i, a) in self.arrivals.iter().enumerate() {
            let Some(d) = flat_gap(*a, p) else {
                continue;
            };
            if d < SAFE_CLEAR {
                return false;
            }
            if d < SAFE_WIDE {
                if !calm(k) || self.zone[i].len() >= SAFE_FEW {
                    return false;
                }
                let mut v: Vec<f32> = self.zone[i].clone();
                v.push(bearing(*a, p));
                if open_arc(&v) < SAFE_ARC {
                    return false;
                }
            }
        }
        true
    }

    pub fn fits(&self, p: [f32; 3], g0: usize, gap: f32) -> bool {
        let mut close: usize = 0;
        let mut wide: usize = 0;
        for (i, q) in self.pts.iter().enumerate() {
            let d: f32 = near(*q, p);
            if d < if i >= g0 { gap } else { SPACING } {
                return false;
            }
            if d < CLUSTER_RADIUS {
                if self.n[i] >= CLUSTER_CAP {
                    return false;
                }
                close += 1;
            }
            if d < ACTIVE_RADIUS {
                if self.far[i] >= ACTIVE_CAP {
                    return false;
                }
                wide += 1;
            }
        }
        close < CLUSTER_CAP && wide < ACTIVE_CAP
    }

    pub fn add(&mut self, p: [f32; 3]) {
        let mut close: usize = 1;
        let mut wide: usize = 1;
        for (i, q) in self.pts.iter().enumerate() {
            let d: f32 = near(*q, p);
            if d < CLUSTER_RADIUS {
                self.n[i] += 1;
                close += 1;
            }
            if d < ACTIVE_RADIUS {
                self.far[i] += 1;
                wide += 1;
            }
        }
        self.pts.push(p);
        self.n.push(close);
        self.far.push(wide);
        for (i, a) in self.arrivals.iter().enumerate() {
            if flat_gap(*a, p).is_some_and(|d| d < SAFE_WIDE) {
                self.zone[i].push(bearing(*a, p));
            }
        }
    }
}

pub fn lift_of(k: &Kind, dungeon: bool) -> f32 {
    if dungeon && k.class == Class::Fly { k.lift.min(DUNGEON_LIFT) } else { k.lift }
}

fn unit(rng: &mut Rng) -> f32 {
    rng.below(GRAIN) as f32 / GRAIN as f32
}

fn near(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).hypot(a[2] - b[2]).hypot(a[1] - b[1])
}

impl Ctx<'_> {
    fn room_at(&self, p: [f32; 3]) -> Option<i32> {
        self.rooms.iter().filter(|r| (r.0[1] - p[1]).abs() < ROOM_BAND).map(|r| ((r.0[0] - p[0]).hypot(r.0[2] - p[2]), r.1)).filter(|r| r.0 < ROOM_REACH).min_by(|a, b| a.0.total_cmp(&b.0)).map(|r| r.1)
    }

    fn free(&self, crowd: &Crowd, k: &Kind, p: [f32; 3], g0: usize, gap: f32) -> bool {
        if !crowd.safe(k, p) {
            return false;
        }
        if self.avoid.iter().any(|(a, r)| near(*a, p) < *r) {
            return false;
        }
        crowd.fits(p, g0, gap)
    }

    fn ground(&self, p: [f32; 3], face: u32, c: Class) -> bool {
        let f: &ground::Face = &self.g.faces[face as usize];
        let (slope, ring, clear): (f32, f32, f32) = limits(c);
        if !self.lived[f.patch as usize] || f.area > MAX_FACE || self.g.patch_area[f.patch as usize] < MIN_PATCH || f.slope > slope || self.g.clear(p) < clear {
            return false;
        }
        if !matches!(c, Class::Fly | Class::Water) && !self.g.tiles && !self.reach[f.patch as usize] {
            return false;
        }
        self.g.steep(p, ring).is_some_and(|s| s <= slope)
    }

    fn key(&self, r: Option<i32>, face: u32) -> i64 {
        match r {
            Some(r) if self.dungeon => r as i64,
            _ => OUTDOOR + self.g.faces[face as usize].patch as i64,
        }
    }

    fn settle(&self, k: &Kind, p: [f32; 3], face: u32) -> Option<([f32; 3], Option<i32>, i64)> {
        if !self.ground(p, face, k.class) || !near_prop(&self.props, p) {
            return None;
        }
        let r: Option<i32> = self.room_at(p);
        if self.dungeon && (r.is_none() || r.is_some_and(|r| self.banned.contains(&r))) {
            return None;
        }
        Some(([p[0], p[1] + lift_of(k, self.dungeon), p[2]], r, self.key(r, face)))
    }

    fn perch(&self, p: [f32; 3], lift: f32) -> Option<([f32; 3], Option<i32>, i64)> {
        let (y, face): (f32, u32) = self.g.floor_near(p[0], p[1] + 0.5, p[2], 1.5, 6.0)?;
        let f: &ground::Face = &self.g.faces[face as usize];
        if !self.lived[f.patch as usize] || self.g.patch_area[f.patch as usize] < MIN_PATCH || !near_prop(&self.props, p) {
            return None;
        }
        let r: Option<i32> = self.room_at(p);
        if self.dungeon && (r.is_none() || r.is_some_and(|r| self.banned.contains(&r))) {
            return None;
        }
        Some(([p[0], y + lift, p[2]], r, self.key(r, face)))
    }

    fn pick(&self, rng: &mut Rng) -> Option<([f32; 3], u32)> {
        let total: f32 = self.cum.last()?.0;
        let t: f32 = unit(rng) * total;
        let i: usize = self.cum.partition_point(|c| c.0 < t).min(self.cum.len() - 1);
        let f: u32 = self.cum[i].1;
        Some((self.g.point_in(f as usize, rng), f))
    }

    fn wet(&self, rng: &mut Rng) -> Option<[f32; 3]> {
        if self.water.is_empty() {
            return None;
        }
        let a: [f32; 3] = self.water[rng.below(self.water.len() as u64) as usize];
        let d: f32 = WATER_REACH.0 + unit(rng) * (WATER_REACH.1 - WATER_REACH.0);
        let t: f32 = unit(rng) * std::f32::consts::TAU;
        let p: [f32; 3] = [a[0] + d * t.cos(), a[1], a[2] + d * t.sin()];
        if self.g.floor_near(p[0], p[1], p[2], WATER_DRY, WATER_DRY).is_some() || !self.open_water(p) {
            return None;
        }
        Some(p)
    }

    fn open_water(&self, p: [f32; 3]) -> bool {
        let reach: f32 = if self.dungeon { WATER_SHORE_ROOFED } else { WATER_SHORE_OPEN };
        self.g.shore(p, reach) <= reach && !self.g.roof(p, WATER_ROOF)
    }
}

fn template(all: &[Node], g: &str) -> Option<Node> {
    for a in all.iter().filter(|a| gyaml(a) == g) {
        let Node::Map(m) = a else {
            continue;
        };
        if !m.iter().all(|e| TEMPLATE_KEYS.contains(&e.0.as_str())) {
            continue;
        }
        let mut t: Node = a.clone();
        if let Some(Node::Map(d)) = t.get("Dynamic") {
            let keep: Vec<(String, Node)> = d.iter().filter(|e| !DROP_KEYS.iter().any(|x| e.0.contains(x))).cloned().collect();
            t.set("Dynamic", Node::Map(keep));
        }
        return Some(t);
    }
    None
}

pub fn node(s: &Spot, temps: &Map<String, Node>, srt: u32) -> Node {
    let mut n: Node = match temps.get(&s.gyaml) {
        Some(t) if s.gyaml != wake_kind() => {
            let mut c: Node = t.clone();
            c.set("Hash", Node::U64(s.hash));
            c.set("Phive", Node::Map(vec![("Placement".to_string(), Node::Map(vec![("ID".to_string(), Node::U64(s.hash))]))]));
            c.set("Rotate", Node::Arr(euler(0.0, s.yaw).iter().map(|v| Node::F32(*v)).collect()));
            c.set("SRTHash", Node::U32(srt));
            c.set("Translate", Node::Arr(s.at.iter().map(|v| Node::F32(*v)).collect()));
            c
        }
        _ => {
            let mut c: Node = actor(&s.gyaml, s.hash, s.at, euler(0.0, s.yaw));
            c.set("SRTHash", Node::U32(srt));
            let mut d: Vec<(String, Node)> = vec![("Drop__DropTable".to_string(), Node::Str("None".to_string()))];
            if s.gyaml == wake_kind() {
                d.push((wake::KEY.to_string(), Node::Bool(true)));
            }
            c.set("Dynamic", Node::Map(d));
            c
        }
    };
    let mut d: Vec<(String, Node)> = match n.get("Dynamic") {
        Some(Node::Map(m)) => m.clone(),
        _ => Vec::new(),
    };
    d.retain(|e| e.0 != ROOM);
    if let Some(r) = s.room {
        d.push((ROOM.to_string(), Node::I32(r)));
        d.sort_by(|a, b| a.0.cmp(&b.0));
    }
    if d.is_empty() {
        if let Node::Map(m) = &mut n {
            m.retain(|e| e.0 != "Dynamic");
        }
    } else {
        n.set("Dynamic", Node::Map(d));
    }
    n
}

pub fn wake_kind() -> &'static str {
    wake::PACK.trim_start_matches("Pack/Actor/").trim_end_matches(".pack.zs")
}

fn pick_kind<'a>(pool: &'a [(&'static Kind, u32)], rng: &mut Rng) -> Option<&'a (&'static Kind, u32)> {
    let total: u64 = pool.iter().map(|p| p.1 as u64).sum();
    if total == 0 {
        return None;
    }
    let mut t: u64 = rng.below(total);
    for p in pool {
        if t < p.1 as u64 {
            return Some(p);
        }
        t -= p.1 as u64;
    }
    None
}

fn plan_map(sc: &Scene, g: &Ground, extra: i64, seed: &str, seen: &mut Map<u64, u32>) -> Plan {
    let m: i64 = (extra + 100) / 100;
    let mut rng: Rng = Rng::from_text(&format!("{seed}|{SALT}|{}|{extra}", sc.map));
    let mut foes: Vec<[f32; 3]> = Vec::new();
    let mut avoid: Vec<([f32; 3], f32)> = Vec::new();
    let mut rooms: Vec<([f32; 3], i32)> = Vec::new();
    let mut banned: Vec<i32> = Vec::new();
    let mut water: Vec<[f32; 3]> = Vec::new();
    let mut homes: BTreeMap<&str, Vec<[f32; 3]>> = BTreeMap::new();
    let mut lives: Vec<u32> = vec![0; g.patch_area.len()];
    let mut arrivals: Vec<[f32; 3]> = Vec::new();
    let mut props: Map<(i32, i32), Vec<[f32; 3]>> = Map::new();
    let mut trees: Vec<[f32; 3]> = Vec::new();
    let mut sights: Vec<[f32; 3]> = Vec::new();
    let mut bigs: Vec<[f32; 3]> = Vec::new();
    let mut knights: usize = 0;
    let mut blastable: bool = NO_BLASTERS.contains(&sc.map.as_str());
    let mut walkers: Vec<[f32; 3]> = Vec::new();
    for (_, a) in &sc.acts {
        let Some(p) = pos(a) else {
            continue;
        };
        let origin: bool = p[0].abs() < 0.01 && p[2].abs() < 0.01;
        let n: &str = gyaml(a);
        if fighting(n) {
            foes.push(p);
            if !kind(n).is_some_and(|k| matches!(k.class, Class::Fly | Class::Water | Class::Ceiling)) {
                walkers.push(p);
            }
        }
        if big(n) {
            bigs.push(p);
        }
        if n.starts_with(KNIGHT_PREFIX) {
            knights += 1;
        }
        if BLASTABLE.iter().any(|b| n.contains(b)) {
            blastable = true;
        }
        if !origin && n.contains(PERCH_TREE) {
            trees.push(p);
        }
        if !origin && SIGHTS.iter().any(|x| n.contains(x)) {
            sights.push(p);
        }
        if let Some(k) = kind(n) {
            homes.entry(k.gyaml).or_default().push(p);
        }
        if n == "Enemy_Octorok" {
            water.push(p);
        }
        let named: bool = n == ARRIVAL_ANCHOR && a.get("Name").and_then(|v| v.as_str()).is_some_and(|t| ARRIVAL_NAMES.iter().any(|x| t.starts_with(x)));
        if (ARRIVALS.contains(&n) || named) && !origin {
            arrivals.push(p);
        }
        if let Some((_, r)) = AVOID.iter().find(|(s, _)| n.contains(s)) {
            avoid.push((p, *r));
        }
        if let Some(r) = room(a) {
            rooms.push((p, r));
            if BOSSY.iter().any(|b| n.starts_with(b)) {
                banned.push(r);
            }
        }
        if origin || n.starts_with("Area") {
            continue;
        }
        if !SCENERY.iter().any(|x| n.starts_with(x)) {
            props.entry(cell(p)).or_default().push(p);
        }
        if !n.starts_with("Land_") {
            if let Some((_, f)) = g.floor_near(p[0], p[1], p[2], LIVED, LIVED) {
                lives[g.faces[f as usize].patch as usize] += 1;
            }
        }
    }
    let lived: Vec<bool> = lives.iter().map(|n| *n >= LIVED_MIN).collect();
    let propped = |p: [f32; 3]| -> bool { near_prop(&props, p) };
    let dungeon: bool = !rooms.is_empty();
    let mut cum: Vec<(f32, u32)> = Vec::new();
    let mut run: f32 = 0.0;
    for (i, f) in g.faces.iter().enumerate() {
        if lived[f.patch as usize] && f.area <= MAX_FACE && g.patch_area[f.patch as usize] >= MIN_PATCH && f.slope <= limits(Class::Fly).0 && propped(centre(&f.pts)) {
            run += f.area;
            cum.push((run, i as u32));
        }
    }
    let open: f32 = cum.iter().map(|c| &g.faces[c.1 as usize]).filter(|f| f.slope < 10.0).map(|f| f.area).sum();
    let mut reach: Vec<bool> = vec![false; g.patch_area.len()];
    for p in arrivals.iter().chain(walkers.iter()) {
        if let Some((_, f)) = g.floor_near(p[0], p[1], p[2], 1.0, 3.0) {
            reach[g.faces[f as usize].patch as usize] = true;
        }
    }
    let perches: Vec<([f32; 3], f32)> = if dungeon {
        g.walls.iter().map(|w| ([(w[0][0] + w[1][0]) / 2.0, (w[0][1] + w[1][1]) / 2.0, (w[0][2] + w[1][2]) / 2.0], PERCH_WALL_LIFT)).collect()
    } else {
        trees.iter().map(|t| (*t, PERCH_LIFT)).collect()
    };
    let ctx: Ctx = Ctx { g, lived, avoid, foes, rooms, banned, dungeon, water, cum, props, reach };
    let before: usize = ctx.foes.len();
    let goal: usize = target(sc.vanilla, extra, open);
    let mut plan: Plan = Plan { map: sc.map.clone(), vanilla: sc.vanilla, before, target: goal, open, spots: Vec::new(), areas: Vec::new(), arrivals: arrivals.clone() };
    let mut areas: BTreeMap<i64, Area> = BTreeMap::new();
    let mut usable: BTreeMap<i64, (f32, f32)> = BTreeMap::new();
    for (_, f) in &ctx.cum {
        let face: &ground::Face = &g.faces[*f as usize];
        let c: [f32; 3] = centre(&face.pts);
        if nearest_arrival(&arrivals, c) < SAFE_CLEAR {
            continue;
        }
        let r: Option<i32> = ctx.room_at(c);
        if dungeon && (r.is_none() || r.is_some_and(|r| ctx.banned.contains(&r))) {
            continue;
        }
        let u: &mut (f32, f32) = usable.entry(ctx.key(r, *f)).or_insert((0.0, 0.0));
        u.1 += face.area;
        if face.slope <= limits(Class::Gentle).0 {
            u.0 += face.area;
        }
    }
    for (key, (ug, ua)) in &usable {
        areas.insert(*key, Area { key: *key, ground_cap: (ug / GROUND_AREA) as usize, air_cap: (ua / AIR_AREA) as usize, ..Area::default() });
    }
    areas.insert(WATER_KEY, Area { key: WATER_KEY, ..Area::default() });
    for p in &ctx.foes {
        let hit: Option<(f32, u32)> = g.floor_near(p[0], p[1], p[2], 1.0, 30.0);
        let r: Option<i32> = ctx.room_at(*p);
        let key: i64 = match (hit, r) {
            (Some((_, f)), _) => ctx.key(r, f),
            (None, Some(r)) if dungeon => r as i64,
            _ => WATER_KEY,
        };
        let a: &mut Area = areas.entry(key).or_insert(Area { key, ..Area::default() });
        a.vanilla += 1;
        a.total += 1;
        match hit {
            Some((y, _)) if (p[1] - y).abs() <= 1.0 => a.ground += 1,
            Some(_) => a.air += 1,
            None => {}
        }
    }
    for a in areas.values_mut() {
        let ug: f32 = usable.get(&a.key).map(|u| u.0).unwrap_or(0.0);
        a.limit = if a.key == WATER_KEY { a.vanilla + WATER_PER_ANCHOR * ctx.water.len() } else { area_target(a.vanilla, extra, ug).min(a.ground_cap + a.air_cap).max(a.vanilla) };
    }
    let room: usize = areas.values().map(|a| a.limit.saturating_sub(a.total)).sum();
    let want: usize = goal.saturating_sub(before).min(room);
    if want == 0 {
        plan.areas = areas.into_values().filter(|a| a.total > 0 || a.limit > 0).collect();
        return plan;
    }
    let local = |k: &Kind| -> bool { homes.contains_key(k.gyaml) };
    let sand: bool = SAND_MAPS.contains(&sc.map.as_str());
    let rising: bool = RISING_WATER.contains(&sc.map.as_str());
    let mut pool: Vec<(&'static Kind, u32)> = Vec::new();
    for k in KINDS {
        let home: bool = local(k);
        let fits: bool = match k.class {
            Class::Sand => sand,
            Class::Water => !ctx.water.is_empty(),
            Class::Ceiling => dungeon,
            Class::Fly => !(k.gyaml == "Enemy_Guay_Fixed" && dungeon),
            Class::Mini => m >= 5 && !NO_MINIBOSS.contains(&sc.map.as_str()) && !rising,
            _ => !rising || home,
        };
        if !fits || (HOME_ONLY.contains(&k.gyaml) && !home) || (BLASTERS.contains(&k.gyaml) && blastable && (!home || NO_BLASTERS.contains(&sc.map.as_str()))) || (percher(k.gyaml) && perches.is_empty()) {
            continue;
        }
        let w: u32 = if home {
            k.fun * LOCAL_WEIGHT
        } else if m >= 5 || (m >= 3 && matches!(k.gyaml, "Enemy_Keese" | "Enemy_Guay_Fixed" | "Enemy_Octorok")) {
            k.fun
        } else {
            0
        };
        if w > 0 {
            pool.push((k, w));
        }
    }
    let mut crowd: Crowd = Crowd::new(&ctx.foes).with_arrivals(arrivals.clone());
    let gmax: u32 = group_max(extra);
    let mini: usize = mini_cap(extra);
    let mut minis: usize = 0;
    let mut news: Vec<&str> = Vec::new();
    let mut tries: usize = 0;
    while plan.spots.len() < want && tries < want * TRIES {
        tries += 1;
        let knight_cap: usize = if sc.map.starts_with("HyruleField") { KNIGHTS_FIELD } else { KNIGHTS_ELSEWHERE };
        let live: Vec<(&'static Kind, u32)> = pool.iter().filter(|(k, _)| (k.class != Class::Mini || minis < mini) && (!k.gyaml.starts_with(KNIGHT_PREFIX) || knights < knight_cap) && (local(k) || news.contains(&k.gyaml) || news.len() < NEW_KINDS_CAP)).copied().collect();
        let Some(&(k, _)) = pick_kind(&live, &mut rng) else {
            break;
        };
        let home: bool = local(k);
        let tough: bool = big(k.gyaml) || k.class == Class::Mini;
        let (anchor, why): (Option<([f32; 3], Option<i32>, i64)>, &'static str) = if percher(k.gyaml) {
            let (b, _): ([f32; 3], f32) = perches[rng.below(perches.len() as u64) as usize];
            let lift: f32 = lift_of(k, dungeon);
            let t: f32 = unit(&mut rng) * std::f32::consts::TAU;
            (ctx.perch([b[0] + PERCH_NUDGE * t.cos(), b[1], b[2] + PERCH_NUDGE * t.sin()], lift), if dungeon { "perched by a wall" } else { "perched in a tree" })
        } else if tough && !sights.is_empty() && rng.below(100) < GUARD_SHARE {
            let h: [f32; 3] = sights[rng.below(sights.len() as u64) as usize];
            let d: f32 = GUARD_REACH.0 + unit(&mut rng) * (GUARD_REACH.1 - GUARD_REACH.0);
            let t: f32 = unit(&mut rng) * std::f32::consts::TAU;
            let hit: Option<(f32, u32)> = g.floor_near(h[0] + d * t.cos(), h[1], h[2] + d * t.sin(), 4.0, 4.0);
            (hit.and_then(|(y, f)| ctx.settle(k, [h[0] + d * t.cos(), y, h[2] + d * t.sin()], f)), "guarding a landmark")
        } else if k.class == Class::Water {
            (ctx.wet(&mut rng).filter(|p| ctx.free(&crowd, k, *p, crowd.pts.len(), SPACING)).map(|p| (p, ctx.room_at(p), WATER_KEY)), "water near a vanilla Octorok")
        } else if home && rng.below(100) < NEAR_SHARE {
            let hs: &Vec<[f32; 3]> = &homes[k.gyaml];
            let h: [f32; 3] = hs[rng.below(hs.len() as u64) as usize];
            let d: f32 = NEAR_HOME.0 + unit(&mut rng) * (NEAR_HOME.1 - NEAR_HOME.0);
            let t: f32 = unit(&mut rng) * std::f32::consts::TAU;
            let base: f32 = h[1] - k.lift;
            let hit: Option<(f32, u32)> = g.floor_near(h[0] + d * t.cos(), base, h[2] + d * t.sin(), 2.0, 4.0);
            (hit.and_then(|(y, f)| ctx.settle(k, [h[0] + d * t.cos(), y, h[2] + d * t.sin()], f)), "near one of its own kind")
        } else {
            (ctx.pick(&mut rng).and_then(|(p, f)| ctx.settle(k, p, f)), if home { "open ground (own kind)" } else { "open ground (new kind)" })
        };
        let Some((at, r, key)) = anchor else {
            continue;
        };
        if !areas.get(&key).is_some_and(|a| a.room_for(k.class)) || !ctx.free(&crowd, k, at, crowd.pts.len(), SPACING) {
            continue;
        }
        if big(k.gyaml) && bigs.iter().any(|b| near(*b, at) < BIG_GAP) {
            continue;
        }
        if big(k.gyaml) {
            bigs.push(at);
        }
        if k.gyaml.starts_with(KNIGHT_PREFIX) {
            knights += 1;
        }
        if let Some(a) = areas.get_mut(&key) {
            a.take(k.class);
        }
        let g0: usize = crowd.pts.len();
        crowd.add(at);
        let size: u32 = if big(k.gyaml) || percher(k.gyaml) {
            1
        } else if gmax > 1 { 1 + rng.below(k.group.min(gmax) as u64) as u32 } else { 1 };
        let mut mine: Vec<[f32; 3]> = vec![at];
        let mut rs: Vec<Option<i32>> = vec![r];
        for _ in 0..size.saturating_sub(1) * 4 {
            if mine.len() as u32 >= size || plan.spots.len() + mine.len() >= want {
                break;
            }
            let d: f32 = GROUP_REACH.0 + unit(&mut rng) * (GROUP_REACH.1 - GROUP_REACH.0);
            let t: f32 = unit(&mut rng) * std::f32::consts::TAU;
            let q: [f32; 3] = [at[0] + d * t.cos(), at[1] - k.lift, at[2] + d * t.sin()];
            let got: Option<([f32; 3], Option<i32>, i64)> = if k.class == Class::Water {
                if g.floor_near(q[0], at[1], q[2], WATER_DRY, WATER_DRY).is_none() && ctx.open_water([q[0], at[1], q[2]]) { Some(([q[0], at[1], q[2]], r, WATER_KEY)) } else { None }
            } else {
                g.floor_near(q[0], q[1], q[2], 1.5, 1.5).and_then(|(y, f)| ctx.settle(k, [q[0], y, q[2]], f))
            };
            let Some((p, pr, pk)) = got else {
                continue;
            };
            if pr != r || pk != key || !areas.get(&key).is_some_and(|a| a.room_for(k.class)) || !ctx.free(&crowd, k, p, g0, GROUP_SPACING) {
                continue;
            }
            if let Some(a) = areas.get_mut(&key) {
                a.take(k.class);
            }
            crowd.add(p);
            mine.push(p);
            rs.push(pr);
        }
        if !home && !news.contains(&k.gyaml) {
            news.push(k.gyaml);
        }
        if k.class == Class::Mini {
            minis += 1;
        }
        let grouped: bool = mine.len() > 1;
        for (p, pr) in mine.into_iter().zip(rs) {
            let hash: u64 = fresh(seen, &mut rng);
            let yaw: f32 = unit(&mut rng) * std::f32::consts::TAU;
            plan.spots.push(Spot { map: sc.map.clone(), file: String::new(), gyaml: k.gyaml.to_string(), class: k.class, at: p, yaw, room: pr, floor: if k.class == Class::Water { p[1] } else { p[1] - lift_of(k, dungeon) }, safe: nearest_arrival(&crowd.arrivals, p), area: key, why: if grouped { "group" } else { why }, hash });
        }
    }
    plan.areas = areas.into_values().filter(|a| a.total > 0 || a.limit > 0).collect();
    plan
}

fn scenes(rf: &mut Romfs, have: Have, seen: &mut Map<u64, u32>, all: &mut Vec<Node>) -> Res<Vec<Scene>> {
    let mut by: BTreeMap<String, Scene> = BTreeMap::new();
    for f in crate::difficulty::extras::files(rf)? {
        let y: Byml = Byml::parse(&raw(rf, have, &f)?.0)?;
        tally(&y.root, seen);
        if !f.ends_with(EXT) || !f.ends_with(ANY_EXT) || !f.starts_with("Banc/") {
            continue;
        }
        let van: Byml = rf.read_byml(&f)?;
        let vl: &[Node] = van.root.get("Actors").and_then(|l| l.as_arr()).unwrap_or(&[]);
        all.extend(vl.iter().filter(|a| kind(gyaml(a)).is_some()).cloned());
        if f.starts_with(SKIP_DIR) {
            continue;
        }
        let Some(map) = map_of(&f) else {
            continue;
        };
        let sc: &mut Scene = by.entry(map.clone()).or_insert(Scene { map, files: Vec::new(), acts: Vec::new(), vanilla: 0 });
        sc.vanilla += vl.iter().filter(|a| fighting(gyaml(a))).count();
        let i: usize = sc.files.len();
        sc.files.push(f.clone());
        for a in y.root.get("Actors").and_then(|l| l.as_arr()).unwrap_or(&[]) {
            sc.acts.push((i, a.clone()));
        }
    }
    Ok(by.into_values().collect())
}

pub fn plan(rf: &mut Romfs, have: Have, seed: &str, extra: i64) -> Res<(Vec<Plan>, Map<String, Node>)> {
    let mut seen: Map<u64, u32> = Map::new();
    let mut all: Vec<Node> = Vec::new();
    let scs: Vec<Scene> = scenes(rf, have, &mut seen, &mut all)?;
    let mut temps: Map<String, Node> = Map::new();
    for k in KINDS {
        if let Some(t) = template(&all, k.gyaml) {
            temps.insert(k.gyaml.to_string(), t);
        }
    }
    let mut out: Vec<Plan> = Vec::new();
    if extra <= FROM {
        return Ok((out, temps));
    }
    for sc in scs.iter().filter(|s| wanted(&s.map)) {
        let Some(g) = ground::load(rf, &sc.map)? else {
            continue;
        };
        let mut p: Plan = plan_map(sc, &g, extra, seed, &mut seen);
        let mut counts: Vec<usize> = vec![0; sc.files.len()];
        for (i, a) in &sc.acts {
            if fighting(gyaml(a)) {
                counts[*i] += 1;
            }
        }
        let best: usize = (0..sc.files.len()).max_by_key(|i| (counts[*i], sc.files[*i].contains("_Static"))).unwrap_or(0);
        for s in p.spots.iter_mut() {
            s.file = sc.files[best].clone();
        }
        out.push(p);
    }
    Ok((out, temps))
}

pub const NIGHT_FILE: &str = "Banc/HyruleField/HyruleField_Child_Static.bcett.byml.zs";
pub const NIGHT_AREA: &str = "StalchildAutoPlacementArea";
pub const NIGHT_KEYS: [&str; 2] = ["CreateStalchildCountMin", "CreateStalchildCountMax"];

pub fn double_night(y: &mut Byml) -> usize {
    let mut n: usize = 0;
    let Some(list) = y.root.get_mut("Actors").and_then(|l| l.as_arr_mut()) else {
        return 0;
    };
    for a in list.iter_mut().filter(|a| a.get("Gyaml").and_then(|g| g.as_str()) == Some(NIGHT_AREA)) {
        let Some(d) = a.get_mut("Dynamic") else {
            continue;
        };
        for k in NIGHT_KEYS {
            if let Some(Node::I32(v)) = d.get(k) {
                let v2: i32 = *v * 2;
                d.set(k, Node::I32(v2));
                n += 1;
            }
        }
    }
    n
}

pub fn stage(rf: &mut Romfs, have: Have, seed: &str, extra: i64, woken: bool) -> Res<Vec<Out>> {
    let (plans, temps): (Vec<Plan>, Map<String, Node>) = plan(rf, have, seed, extra)?;
    let mut by: BTreeMap<String, Vec<&Spot>> = BTreeMap::new();
    for s in plans.iter().flat_map(|p| p.spots.iter()) {
        by.entry(s.file.clone()).or_default().push(s);
    }
    by.entry(NIGHT_FILE.to_string()).or_default();
    let mut outs: Vec<Out> = Vec::new();
    for (f, ss) in &by {
        let (b, id): (Vec<u8>, Option<u32>) = raw(rf, have, f)?;
        let mut y: Byml = Byml::parse(&b)?;
        if f == NIGHT_FILE {
            double_night(&mut y);
        }
        let mut rng: Rng = Rng::from_text(&format!("{seed}|{SALT}|srt|{f}"));
        let more: Vec<Node> = ss.iter().map(|s| node(s, &temps, rng.next_u64() as u32)).collect();
        let Some(list) = y.root.get_mut("Actors").and_then(|l| l.as_arr_mut()) else {
            continue;
        };
        list.extend(more);
        let new: Vec<u8> = y.write()?;
        outs.push(Out { path: f.clone(), old: b.len(), new: new.len(), z: zs::compress_raw(&new, Some(id.unwrap_or(BANC_DICT)))? });
    }
    if !woken && by.values().flatten().any(|s| s.gyaml == wake_kind()) {
        outs.push(wake::stage(rf, have)?);
    }
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_night_stalchild_areas_double_their_counts() {
        let Some(root) = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from).filter(|p| p.join("Banc").is_dir()) else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut y: Byml = rf.read_byml(NIGHT_FILE).unwrap();
        let counts = |y: &Byml| -> Vec<i32> {
            y.root.get("Actors").and_then(|l| l.as_arr()).unwrap().iter().filter(|a| a.get("Gyaml").and_then(|g| g.as_str()) == Some(NIGHT_AREA)).flat_map(|a| NIGHT_KEYS.iter().filter_map(|k| match a.get("Dynamic").and_then(|d| d.get(k)) { Some(Node::I32(v)) => Some(*v), _ => None })).collect()
        };
        let was: Vec<i32> = counts(&y);
        assert!(!was.is_empty());
        assert_eq!(double_night(&mut y), was.len());
        assert_eq!(counts(&y), was.iter().map(|v| v * 2).collect::<Vec<i32>>());
    }

    #[test]
    fn the_class_table_is_sane() {
        for k in KINDS {
            assert!(k.gyaml.starts_with("Enemy_") && k.fun > 0 && k.group >= 1 && k.lift >= 0.0, "{}", k.gyaml);
            assert!(!NEVER.iter().any(|n| k.gyaml == *n || (k.gyaml.starts_with(n) && !KINDS.iter().any(|o| o.gyaml == *n))), "{} is on the never list", k.gyaml);
            assert!(!k.gyaml.contains("Lizalfos") && !k.gyaml.contains("DeadHand") && !k.gyaml.contains("Gold"), "{}", k.gyaml);
        }
        assert_eq!(KINDS.iter().map(|k| k.gyaml).collect::<std::collections::BTreeSet<&str>>().len(), KINDS.len());
        assert_eq!(wake_kind(), "Enemy_IronKnuckle_Silver");
        let (f, _, _): (f32, f32, f32) = limits(Class::Flat);
        let (gs, _, _): (f32, f32, f32) = limits(Class::Gentle);
        let (a, _, _): (f32, f32, f32) = limits(Class::Any);
        assert!(limits(Class::Mini).0 <= f && f < gs && gs < a);
    }

    #[test]
    fn the_target_grows_with_the_slider_and_stops_at_the_cap() {
        assert_eq!(target(10, 0, 100000.0), 10);
        assert_eq!(target(10, 100, 100000.0), 20);
        assert!(target(10, 400, 0.0) == 50 && target(10, 900, 0.0) == 100);
        assert!(target(0, 900, 15000.0) == 100 && target(0, 200, 15000.0) > 0);
        assert_eq!(target(200, 900, 0.0), MAP_CAP);
        let mut last: usize = 0;
        for e in [0, 50, 100, 200, 300, 400, 600, 900] {
            let t: usize = target(30, e, 20000.0);
            assert!(t >= last);
            last = t;
        }
        assert_eq!((group_max(100), group_max(400), group_max(600), group_max(900)), (1, 2, 3, 4));
        assert_eq!((mini_cap(300), mini_cap(400), mini_cap(900)), (0, 1, 6));
    }

    #[test]
    fn map_names_come_from_file_names() {
        assert_eq!(map_of("Banc/HyruleField/HyruleField_Child_Static.bcett.byml.zs"), Some("HyruleField_Child".to_string()));
        assert_eq!(map_of("Banc/DodongoCavern/DodongoCavern_Dynamic.bcett.byml.zs"), Some("DodongoCavern".to_string()));
        assert!(!wanted("Market_Child") && !wanted("Demo_ShootingLocation_Tapestry_ED") && wanted("HyruleField_Adult"));
    }

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        if p.is_dir() { Some(p) } else { None }
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    const WATCH: [&str; 6] = ["HyruleField_Child", "HyruleField_Adult", "KokiriForest_Child", "KokiriForest_Adult", "DodongoCavern", "FireTemple"];

    fn count(ps: &[Plan], map: &str) -> (usize, usize, usize) {
        ps.iter().find(|p| p.map == map).map(|p| (p.vanilla, p.before, p.before + p.spots.len())).unwrap_or((0, 0, 0))
    }

    #[test]
    fn the_game_maps_fill_up_with_the_slider_and_respect_the_ground() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        assert!(plan(&mut rf, &none, "game", 0).unwrap().0.is_empty());
        assert!(plan(&mut rf, &none, "game", 100).unwrap().0.is_empty());
        let mut last: Map<String, usize> = Map::new();
        let mut all: Vec<(i64, Vec<Plan>)> = Vec::new();
        for e in [200, 400, 900] {
            let (ps, _): (Vec<Plan>, Map<String, Node>) = plan(&mut rf, &none, "game", e).unwrap();
            for m in WATCH {
                let (v, b, n): (usize, usize, usize) = count(&ps, m);
                eprintln!("{}x {m}: vanilla {v}, before {b}, after {n}", (e + 100) / 100);
                assert!(n >= *last.get(m).unwrap_or(&0), "{m} shrank at {e}");
                last.insert(m.to_string(), n);
            }
            all.push((e, ps));
        }
        for m in ["HyruleField_Child", "HyruleField_Adult", "DodongoCavern"] {
            let (v, _, n): (usize, usize, usize) = count(&all[2].1, m);
            assert!(n > v + v / 2 && n <= MAP_CAP, "{m}: {n} of vanilla {v}");
        }
        assert_eq!(count(&all[2].1, "KokiriForest_Child").2, 0);
        let ten: &[Plan] = &all[2].1;
        let busy: Vec<(&Plan, &Area)> = ten.iter().flat_map(|p| p.areas.iter().map(move |a| (p, a))).filter(|(_, a)| a.vanilla >= 2 && a.key != WATER_KEY).collect();
        let full: usize = busy.iter().filter(|(_, a)| a.total >= a.vanilla * 10).count();
        let held: usize = busy.iter().filter(|(_, a)| a.limit < a.vanilla * 10).count();
        eprintln!("10x: {} areas with vanilla enemies, {full} reach 10x, {held} limited by capacity, {} added", busy.len(), ten.iter().map(|p| p.spots.len()).sum::<usize>());
        assert!(busy.iter().any(|(p, a)| a.key < OUTDOOR && p.areas.len() > 1 && a.total < a.vanilla * 5), "a small dungeon room stays well under 10x");
        assert!(busy.iter().any(|(_, a)| a.total >= a.vanilla * 8), "some big area nears 10x");
        for (e, ps) in &all {
            let mut hashes: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
            for p in ps {
                assert!(wanted(&p.map) && p.before + p.spots.len() <= MAP_CAP.max(p.before), "{} over the cap", p.map);
                let g: Ground = ground::load(&mut rf, &p.map).unwrap().unwrap();
                for s in &p.spots {
                    let k: &Kind = kind(&s.gyaml).unwrap();
                    assert!(hashes.insert(s.hash), "a hash repeats");
                    match k.class {
                        Class::Water => {
                            assert!(g.floor_near(s.at[0], s.at[1], s.at[2], WATER_DRY - 0.01, WATER_DRY - 0.01).is_none(), "{} {:?} is on land", s.gyaml, s.at);
                            let reach: f32 = if s.room.is_some() { WATER_SHORE_ROOFED } else { WATER_SHORE_OPEN };
                            assert!(g.shore(s.at, reach) <= reach, "{} {:?} in {} is far from any shore", s.gyaml, s.at, p.map);
                            assert!(!g.roof(s.at, WATER_ROOF), "{} {:?} in {} has a floor just above", s.gyaml, s.at, p.map);
                        }
                        c => {
                            let (_, f): (f32, u32) = g.floor_near(s.at[0], s.floor, s.at[2], 0.05, 0.05).unwrap_or_else(|| panic!("{} {:?} in {} has no floor", s.gyaml, s.at, p.map));
                            if percher(&s.gyaml) {
                                continue;
                            }
                            let (slope, ring, _): (f32, f32, f32) = limits(c);
                            assert!(g.faces[f as usize].slope <= slope + 0.01, "{} on a {} degree face", s.gyaml, g.faces[f as usize].slope);
                            assert!(g.steep([s.at[0], s.floor, s.at[2]], ring).is_some_and(|x| x <= slope + 0.01), "{} {:?} is on a steep spot", s.gyaml, s.at);
                            assert!(s.at[1] >= s.floor && s.at[1] - s.floor <= k.lift + 0.01);
                        }
                    }
                    if k.class == Class::Mini {
                        assert!(*e >= 400 && !NO_MINIBOSS.contains(&p.map.as_str()));
                    }
                    if k.class == Class::Sand {
                        assert!(SAND_MAPS.contains(&p.map.as_str()));
                    }
                }
                let mut per: Map<i64, usize> = Map::new();
                for s in &p.spots {
                    *per.entry(s.area).or_insert(0) += 1;
                }
                for a in &p.areas {
                    let added: usize = per.get(&a.key).copied().unwrap_or(0);
                    assert_eq!(a.total, a.vanilla + added, "{} area {}", p.map, a.key);
                    assert!(added == 0 || a.total <= a.limit, "{} area {} holds {} over its limit {}", p.map, a.key, a.total, a.limit);
                    assert!(a.limit <= a.vanilla.max(area_target(a.vanilla, *e, f32::MAX).min(a.ground_cap + a.air_cap)) || a.key == WATER_KEY);
                }
                assert!(per.keys().all(|k| p.areas.iter().any(|a| a.key == *k)));
                let cap: usize = if p.map.starts_with("HyruleField") { KNIGHTS_FIELD } else { KNIGHTS_ELSEWHERE };
                assert!(p.spots.iter().filter(|s| s.gyaml.starts_with(KNIGHT_PREFIX)).count() <= cap, "{} has too many knights", p.map);
                let bg: Vec<&Spot> = p.spots.iter().filter(|s| big(&s.gyaml)).collect();
                for (i, x) in bg.iter().enumerate() {
                    assert!(bg[i + 1..].iter().all(|y| near(x.at, y.at) >= BIG_GAP), "{}: two big enemies closer than {BIG_GAP}", p.map);
                }
                assert!(p.spots.iter().filter(|s| percher(&s.gyaml)).all(|s| s.why.starts_with("perched")), "{}: a Keese in open air", p.map);
                assert!(!NO_BLASTERS.contains(&p.map.as_str()) || p.spots.iter().all(|s| !BLASTERS.contains(&s.gyaml.as_str())), "Dodongos added in {}", p.map);
                assert!(!PEACEFUL.contains(&p.map.as_str()));
                for (i, a) in p.arrivals.iter().enumerate() {
                    let near: Vec<&Spot> = p.spots.iter().filter(|s| flat_gap(*a, s.at).is_some_and(|d| d < SAFE_WIDE)).collect();
                    assert!(near.iter().all(|s| flat_gap(*a, s.at).unwrap() >= SAFE_CLEAR), "{} arrival {i} has an enemy inside {SAFE_CLEAR}", p.map);
                    assert!(near.len() <= SAFE_FEW && near.iter().all(|s| calm(kind(&s.gyaml).unwrap())), "{} arrival {i}: {:?}", p.map, near.iter().map(|s| &s.gyaml).collect::<Vec<_>>());
                    assert!(open_arc(&near.iter().map(|s| bearing(*a, s.at)).collect::<Vec<f32>>()) >= SAFE_ARC);
                }
                if *e == 900 {
                    let crowd: usize = p.spots.iter().map(|s| p.spots.iter().filter(|o| near(o.at, s.at) < CLUSTER_RADIUS).count()).max().unwrap_or(0);
                    assert!(crowd <= CLUSTER_CAP, "{} has {crowd} added enemies in one cluster", p.map);
                    let busy: usize = p.spots.iter().map(|s| p.spots.iter().filter(|o| near(o.at, s.at) < ACTIVE_RADIUS).count()).max().unwrap_or(0);
                    assert!(busy <= ACTIVE_CAP, "{} has {busy} added enemies within {ACTIVE_RADIUS}", p.map);
                    assert!(p.spots.iter().all(|s| !matches!(s.gyaml.as_str(), "Enemy_Freezard" | "Enemy_Gibdo" | "Enemy_Bubble_Blue" | "Enemy_Poe_Big")));
                    assert!(p.map != "HountedWasteland_Child");
                    for exit in [[71.2, 41.6, 273.7], [76.5, 41.2, 257.9]] {
                        assert!(!p.map.starts_with("GerudoValley") || p.spots.iter().all(|s| near(s.at, exit) >= SAFE_CLEAR), "{} has an enemy on the exit to the lake", p.map);
                    }
                }
            }
        }
        let again: (Vec<Plan>, Map<String, Node>) = plan(&mut rf, &none, "game", 900).unwrap();
        assert_eq!(again.0, all[2].1);
        assert_ne!(plan(&mut rf, &none, "other", 900).unwrap().0, all[2].1);
        if let Some(out) = std::env::var_os("HORDE_DUMP") {
            let copies: Vec<(Out, Vec<crate::difficulty::extras::Made>)> = crate::difficulty::extras::made(&mut rf, &none, "game", crate::difficulty::extras::FULL).unwrap();
            let have = |f: &str| -> Option<Vec<u8>> { copies.iter().find(|c| c.0.path == f).map(|c| c.0.z.clone()) };
            let (real, _): (Vec<Plan>, Map<String, Node>) = plan(&mut rf, &have, "game", 900).unwrap();
            let mut t: String = String::from("map,age,enemy,x,y,z,class,why,file,area,arrival_dist,zone\n");
            for (o, ms) in &copies {
                let map: String = map_of(&o.path).unwrap_or_default();
                let age: &str = if map.contains("_Child") { "child" } else if map.contains("_Adult") { "adult" } else { "both" };
                for c in ms {
                    t.push_str(&format!("{map},{age},{},{:.2},{:.2},{:.2},Copy,2x copy next to the original,{},,,\n", c.gyaml, c.pos[0], c.pos[1], c.pos[2], o.path.rsplit('/').next().unwrap_or("")));
                }
            }
            for p in &real {
                let age: &str = if p.map.contains("_Child") { "child" } else if p.map.contains("_Adult") { "adult" } else { "both" };
                for s in &p.spots {
                    let zone: &str = if s.safe < SAFE_WIDE { "wide zone (calm kinds, at most 2)" } else { "outside" };
                    t.push_str(&format!("{},{age},{},{:.2},{:.2},{:.2},{:?},{},{},{},{:.1},{zone}\n", p.map, s.gyaml, s.at[0], s.at[1], s.at[2], s.class, s.why, s.file.rsplit('/').next().unwrap_or(""), s.area, s.safe.min(999.0)));
                }
            }
            let mut sum: String = String::from("map,area,vanilla,ground_capacity,air_capacity,limit_10x,at_3x,at_5x,at_10x\n");
            for p in &all[2].1 {
                sum.push_str(&format!("{},all,{},,,{},{},{},{}\n", p.map, p.vanilla, p.target, count(&all[0].1, &p.map).2, count(&all[1].1, &p.map).2, p.before + p.spots.len()));
                let at = |ps: &[Plan], k: i64| -> usize { ps.iter().find(|q| q.map == p.map).and_then(|q| q.areas.iter().find(|a| a.key == k)).map(|a| a.total).unwrap_or(0) };
                for a in &p.areas {
                    let name: String = if a.key == WATER_KEY { "water".to_string() } else if a.key >= OUTDOOR { format!("patch {}", a.key - OUTDOOR) } else { format!("room {}", a.key) };
                    sum.push_str(&format!("{},{name},{},{},{},{},{},{},{}\n", p.map, a.vanilla, a.ground_cap, a.air_cap, a.limit, at(&all[0].1, a.key), at(&all[1].1, a.key), a.total));
                }
            }
            std::fs::write(std::path::PathBuf::from(&out).join("placements-10x.csv"), t).unwrap();
            std::fs::write(std::path::PathBuf::from(&out).join("counts.csv"), sum).unwrap();
        }
    }

    #[test]
    fn the_staged_maps_keep_every_old_actor_and_add_unique_ids() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let outs: Vec<Out> = stage(&mut rf, &none, "game", 900, false).unwrap();
        let (ps, _): (Vec<Plan>, Map<String, Node>) = plan(&mut rf, &none, "game", 900).unwrap();
        assert!(outs.iter().any(|o| o.path == wake::PACK), "the knights get their wake edit");
        assert!(stage(&mut rf, &none, "game", 900, true).unwrap().iter().all(|o| o.path != wake::PACK));
        for o in outs.iter().filter(|o| o.path.ends_with(EXT)) {
            let was: Byml = rf.read_byml(&o.path).unwrap();
            let now: Byml = Byml::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap();
            let a: &[Node] = was.root.get("Actors").unwrap().as_arr().unwrap();
            let b: &[Node] = now.root.get("Actors").unwrap().as_arr().unwrap();
            let n: usize = ps.iter().flat_map(|p| p.spots.iter()).filter(|s| s.file == o.path).count();
            assert_eq!(b.len(), a.len() + n, "{}", o.path);
            assert!(o.path == NIGHT_FILE || &b[..a.len()] == a);
            for c in &b[a.len()..] {
                assert!(c.get("Links").is_none() && c.get("Name").is_none());
                assert!(kind(gyaml(c)).is_some());
                assert!(c.get("Dynamic").map_or(true, |d| d.get("Drop__DropActor").is_none()), "an added enemy carries a drop item");
            }
        }
        let mut seen: Map<u64, u32> = Map::new();
        for f in crate::difficulty::extras::files(&rf).unwrap() {
            let y: Byml = match outs.iter().find(|o| o.path == f) {
                Some(o) => Byml::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap(),
                None => rf.read_byml(&f).unwrap(),
            };
            tally(&y.root, &mut seen);
        }
        for s in ps.iter().flat_map(|p| p.spots.iter()) {
            assert_eq!(seen.get(&s.hash), Some(&2), "{} id is not unique", s.gyaml);
        }
    }
}
