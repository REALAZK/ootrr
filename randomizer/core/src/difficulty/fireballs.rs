use crate::ainb::build::{Spec, QUERY};
use crate::ainb::{audit, Ainb};
use crate::byml::{Byml, Node};
use crate::difficulty::ambush::{actor, euler, floats, fresh, link, list, text, vec3};
use crate::difficulty::gates::{id_for, AND_NODE, AREA, AREA_NODE, BOOL, CREATE, DELETE, GRAPH as SOURCE, HIT_NODE, INT, NAME, POINTER, RESPAWN, SHARED, STR};
use crate::error::{Error as E, Res};
use crate::flow_locations::dict_id;
use crate::murmur::murmur3_32;
use crate::placement::hash_of;
use crate::qol::Have;
use crate::romfs::Romfs;
use crate::staging::{Out, BANC_DICT};
use crate::zs;

pub const MANAGER: &str = "Obj_FallingRockManager_A_04";
pub const NODE: &str = "ActorLogicFallingRockManager";
pub const ANCHOR: &str = "DestinationAnchor";
pub const PREFIX: &str = "Hazard_FireballStorm_";
pub const SEED: &str = "ootrr fireball storms";
pub const LIFT: f32 = 0.5;
pub const SLUG: &str = "Enemy_TorchSlug";

pub const NOTE: &str = "Burning rocks rain down, as on the Death Mountain Trail, while you stand in quiet stretches of Hyrule Field and the Haunted Wasteland (child and adult) or the Hyrule Castle garden (child), and stop when you leave. The Death Mountain Trail storms run on every visit, child and adult. A rock hits as hard as on the mountain.";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub name: &'static str,
    pub dir: &'static str,
    pub maps: &'static [&'static str],
    pub area: [f32; 3],
    pub size: [f32; 3],
    pub manager: [f32; 3],
    pub volcano: [f32; 2],
    pub anchors: &'static [[f32; 3]],
    pub slug: Option<[f32; 3]>,
    pub interval: f32,
    pub fall: f32,
}

pub const FIELD: &[&str] = &["HyruleField_Child", "HyruleField_Adult"];
pub const CASTLE: &[&str] = &["HyruleCastle"];
pub const WASTELAND: &[&str] = &["HountedWasteland_Child", "HountedWasteland_Adult"];

pub const FIELD_VOLCANO: [f32; 2] = [801.0, -370.0];
pub const CASTLE_VOLCANO: [f32; 2] = [1468.0, 478.0];
pub const WASTELAND_VOLCANO: [f32; 2] = [182.0, 87.0];

pub const TRAIL: [&str; 8] = [
    "Logic/DeathMountainTrail_Child_Prefab_0656.logic.module.ainb",
    "Logic/DeathMountainTrail_Child_Prefab_2eb6.logic.module.ainb",
    "Logic/DeathMountainTrail_Child_Prefab_7fa0.logic.module.ainb",
    "Logic/DeathMountainTrail_Child_Prefab_90f5.logic.module.ainb",
    "Logic/DeathMountainTrail_Adult_Prefab_3212.logic.module.ainb",
    "Logic/DeathMountainTrail_Adult_Prefab_f25f.logic.module.ainb",
    "Logic/DeathMountainTrail_Adult_Prefab_f378.logic.module.ainb",
    "Logic/DeathMountainTrail_Adult_Prefab_fb7b.logic.module.ainb",
];

pub const SPOTS: [Spot; 7] = [
    Spot {
        name: "field_middle_road",
        dir: "HyruleField",
        maps: FIELD,
        area: [75.0, 152.9, 95.0],
        size: [64.0, 12.0, 60.0],
        manager: [157.0, 216.0, 36.0],
        volcano: FIELD_VOLCANO,
        anchors: &[[95.0, 150.4, 105.0], [55.0, 150.0, 105.0], [65.0, 150.7, 115.0]],
        slug: Some([75.0, 149.2, 95.0]),
        interval: 45.0,
        fall: 3.5,
    },
    Spot {
        name: "field_south_central",
        dir: "HyruleField",
        maps: FIELD,
        area: [55.0, 163.0, 190.0],
        size: [64.0, 12.0, 60.0],
        manager: [136.0, 226.0, 132.0],
        volcano: FIELD_VOLCANO,
        anchors: &[[75.0, 158.1, 200.0], [35.0, 161.5, 200.0], [65.0, 157.1, 170.0], [45.0, 160.4, 210.0], [75.0, 156.9, 180.0]],
        slug: Some([55.0, 159.3, 190.0]),
        interval: 45.0,
        fall: 3.5,
    },
    Spot {
        name: "field_west_road",
        dir: "HyruleField",
        maps: FIELD,
        area: [-170.0, 137.1, 170.0],
        size: [64.0, 12.0, 60.0],
        manager: [-80.0, 200.0, 123.0],
        volcano: FIELD_VOLCANO,
        anchors: &[[-150.0, 132.2, 180.0], [-190.0, 132.5, 180.0], [-160.0, 135.4, 150.0], [-180.0, 131.9, 190.0], [-150.0, 134.6, 160.0]],
        slug: Some([-170.0, 133.4, 170.0]),
        interval: 45.0,
        fall: 3.5,
    },
    Spot {
        name: "castle_front_garden",
        dir: "HyruleCastle",
        maps: CASTLE,
        area: [14.0, 118.0, -76.0],
        size: [88.0, 10.0, 52.0],
        manager: [100.0, 186.0, -55.0],
        volcano: CASTLE_VOLCANO,
        anchors: &[[36.0, 114.7, -72.0], [42.0, 114.2, -84.0], [4.0, 114.5, -71.0], [25.0, 114.5, -89.0], [-14.0, 114.5, -86.0]],
        slug: None,
        interval: 60.0,
        fall: 3.5,
    },
    Spot {
        name: "field_south_east",
        dir: "HyruleField",
        maps: FIELD,
        area: [133.0, 162.8, 214.0],
        size: [64.0, 12.0, 60.0],
        manager: [192.0, 225.0, 167.0],
        volcano: FIELD_VOLCANO,
        anchors: &[[157.0, 157.6, 222.0], [157.0, 159.9, 202.0], [137.0, 161.2, 196.0], [107.0, 161.5, 202.0], [107.0, 158.8, 222.0]],
        slug: None,
        interval: 45.0,
        fall: 3.5,
    },
    Spot {
        name: "field_near_market",
        dir: "HyruleField",
        maps: FIELD,
        area: [-33.0, 152.8, 92.0],
        size: [64.0, 12.0, 60.0],
        manager: [35.0, 212.0, 49.0],
        volcano: FIELD_VOLCANO,
        anchors: &[[-19.0, 149.2, 92.0], [-49.0, 148.9, 104.0], [-29.0, 149.0, 108.0], [-19.0, 149.1, 120.0], [-39.0, 148.4, 68.0]],
        slug: Some([-33.0, 149.3, 92.0]),
        interval: 45.0,
        fall: 3.5,
    },
    Spot {
        name: "wasteland_dunes",
        dir: "HountedWasteland",
        maps: WASTELAND,
        area: [-56.0, 103.4, -55.0],
        size: [64.0, 12.0, 60.0],
        manager: [18.0, 165.0, -19.0],
        volcano: WASTELAND_VOLCANO,
        anchors: &[[-48.0, 99.2, -39.0], [-36.0, 100.5, -49.0], [-32.0, 100.3, -65.0]],
        slug: Some([-56.0, 99.7, -55.0]),
        interval: 45.0,
        fall: 3.5,
    },
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub module: String,
    pub graph: String,
    pub manager: String,
    pub area: String,
}

impl Spot {
    pub fn file(&self, map: &str, kind: &str) -> String {
        format!("Banc/{}/{map}_{kind}.bcett.byml.zs", self.dir)
    }

    pub fn added(&self) -> usize {
        self.anchors.len() + 2 + usize::from(self.slug.is_some())
    }

    pub fn plan(&self) -> Plan {
        let manager: String = id_for(MANAGER, &format!("{SEED} {} manager", self.name), &[]);
        let area: String = id_for(AREA, &format!("{SEED} {} area", self.name), &[manager.clone()]);
        let tag: u32 = murmur3_32(format!("{SEED} {} module", self.name).as_bytes(), 0) & 0xffff;
        let name: String = format!("{PREFIX}{}_{tag:04x}", self.name);
        Plan { module: format!("Logic/Module/{PREFIX}{}/{name}.logic.module.ain", self.name), graph: format!("Logic/{name}.logic.module.ainb"), manager, area }
    }
}

pub fn files() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in SPOTS.iter() {
        for m in s.maps {
            let f: String = s.file(m, "Static");
            if !out.contains(&f) {
                out.push(f);
            }
        }
    }
    out
}

fn reference(src: u64, dst: u64) -> Node {
    let mut l: Node = Node::Map(Vec::new());
    l.set("Dst", Node::U64(dst));
    l.set("Gyaml", Node::Str("Reference".to_string()));
    l.set("Name", Node::Str("Reference".to_string()));
    l.set("Src", Node::U64(src));
    l
}

pub fn graph(raw: &[u8], s: &Spot) -> Res<Out> {
    let p: Plan = s.plan();
    let mut a: Ainb = Ainb::parse(raw)?;
    for id in [&p.manager, &p.area] {
        if (0..a.els.len()).any(|i| a.prop_str(i, NAME) == Some(id.as_str())) {
            return Err(E::Msg(format!("fireball storms: the graph already has a node named {id}")));
        }
    }
    let zero: Vec<u8> = 0u32.to_le_bytes().to_vec();
    let one: Vec<u8> = 1u32.to_le_bytes().to_vec();
    let area_name: Vec<u8> = a.pool.add(&p.area).to_le_bytes().to_vec();
    let manager_name: Vec<u8> = a.pool.add(&p.manager).to_le_bytes().to_vec();
    let empty: Vec<u8> = a.pool.add("").to_le_bytes().to_vec();
    let player: Vec<u8> = a.pool.add("Player").to_le_bytes().to_vec();
    let seed_a: String = format!("{SEED} {} area node", s.name);
    let seed_h: String = format!("{SEED} {} hit", s.name);
    let seed_i: String = format!("{SEED} {} inside", s.name);
    let seed_m: String = format!("{SEED} {} manager node", s.name);
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
    let mut inside: Spec = Spec::node(AND_NODE, &seed_i).linked(BOOL, "BoolMulti", hit_at, 0).idle(BOOL, "UpdateOnce", zero.clone()).spare(BOOL, "False").out(BOOL, "True");
    inside.role = QUERY;
    let inside_at: usize = a.add_node(&inside)?;
    let mut man: Spec = Spec::node(NODE, &seed_m)
        .prop(STR, NAME, manager_name)
        .linked(BOOL, "IsActive", inside_at, 1)
        .idle(BOOL, "IsDisableRare", zero.clone())
        .idle(BOOL, CREATE, one)
        .idle(BOOL, DELETE, zero.clone())
        .idle(BOOL, RESPAWN, zero)
        .spare(BOOL, "Dead")
        .spare(BOOL, "isFalling")
        .spare(BOOL, "Logic_Alive");
    man.role = QUERY;
    a.add_node(&man)?;
    let bad: Vec<String> = audit(&a);
    if !bad.is_empty() {
        return Err(E::Msg(format!("the fireball storm graph no longer checks out: {}", bad.join(", "))));
    }
    let g: Vec<u8> = a.write()?;
    Ok(Out { path: p.graph, old: 0, new: g.len(), z: g })
}

pub fn free(raw: &[u8]) -> Res<Out> {
    let mut a: Ainb = Ainb::parse(raw)?;
    let managers: Vec<usize> = a.find_els(NODE);
    if managers.len() != 1 {
        return Err(E::Msg(format!("fireball storms: a Trail module holds {} falling rock managers, not 1", managers.len())));
    }
    let linked: Vec<usize> = a.find_els(AREA_NODE).into_iter().filter(|i| a.find_in(*i, BOOL, DELETE).and_then(|g| a.input(BOOL, g)).is_some_and(|x| x.src != -1)).collect();
    if linked.len() != 1 {
        return Err(E::Msg(format!("fireball storms: a Trail module has {} areas that can be deleted by a flag, not 1", linked.len())));
    }
    a.force_bool(linked[0], DELETE, false)?;
    let bad: Vec<String> = audit(&a);
    if !bad.is_empty() {
        return Err(E::Msg(format!("the freed Trail module no longer checks out: {}", bad.join(", "))));
    }
    let g: Vec<u8> = a.write()?;
    Ok(Out { path: String::new(), old: raw.len(), new: g.len(), z: g })
}

fn next(raws: &[&[u8]], taken: &mut Vec<u64>, seed: &str) -> u64 {
    let mut n: u32 = 0;
    loop {
        let h: u64 = fresh(raws, &format!("{seed} {n}"));
        if !taken.contains(&h) {
            taken.push(h);
            return h;
        }
        n += 1;
    }
}

pub fn place(y: &mut Byml, raws: &[&[u8]], s: &Spot, map: &str) -> Res<usize> {
    let p: Plan = s.plan();
    let seed: String = format!("{SEED} {map} {}", s.name);
    let mut taken: Vec<u64> = Vec::new();
    for a in y.root.get("Actors").and_then(|a| a.as_arr()).unwrap_or(&[]) {
        taken.extend(hash_of(a));
    }
    let manager: u64 = next(raws, &mut taken, &format!("{seed} manager"));
    let area: u64 = next(raws, &mut taken, &format!("{seed} area"));
    let group: u64 = next(raws, &mut taken, &format!("{seed} group"));
    let mut anchors: Vec<u64> = Vec::new();
    for i in 0..s.anchors.len() {
        anchors.push(next(raws, &mut taken, &format!("{seed} anchor {i}")));
    }
    let mut m: Node = actor(MANAGER, manager, s.manager, euler(0.0, 0.0));
    m.set("Dynamic", Node::Map(vec![("FallTime".to_string(), Node::F32(s.fall)), ("IntervalFrame".to_string(), Node::F32(s.interval))]));
    m.set("Links", Node::Arr(anchors.iter().map(|a| reference(manager, *a)).collect()));
    let mut b: Node = actor(AREA, area, s.area, euler(0.0, 0.0));
    b.set("Scale", floats(s.size));
    let acts: &mut Vec<Node> = list(y, "Actors")?;
    acts.push(m);
    for (i, at) in s.anchors.iter().enumerate() {
        let mut n: Node = actor(ANCHOR, anchors[i], *at, [0.0; 3]);
        n.set("Name", Node::Str(i.to_string()));
        acts.push(n);
    }
    acts.push(b);
    if let Some(at) = s.slug {
        let slug: u64 = next(raws, &mut taken, &format!("{seed} slug"));
        acts.push(actor(SLUG, slug, at, [0.0; 3]));
    }
    let mut all: Vec<u64> = anchors.clone();
    all.push(manager);
    all.push(area);
    all.sort();
    for name in SHARED {
        if y.root.get(name).is_some() {
            list(y, name)?.push(Node::Arr(all.iter().map(|h| Node::U64(*h)).collect()));
        }
    }
    let mut g: Node = Node::Map(Vec::new());
    g.set("Hash", Node::U64(group));
    g.set("Logic", Node::Str(p.module.clone()));
    g.set("References", Node::Arr(vec![link(&p.manager, manager), link(&p.area, area)]));
    let gs: &mut Vec<Node> = list(y, "AiGroups")?;
    let i: usize = gs.iter().position(|x| hash_of(x).is_some_and(|v| v > group)).unwrap_or(gs.len());
    gs.insert(i, g);
    Ok(s.added())
}

fn source(rf: &Romfs, have: Have, p: &str) -> Res<Vec<u8>> {
    match have(p) {
        Some(b) => Ok(b),
        None => rf.read(p),
    }
}

fn one(rf: &mut Romfs, have: Have, path: &str, spots: &[(&Spot, &str)]) -> Res<Out> {
    let z: Vec<u8> = source(rf, have, path)?;
    let dict: u32 = dict_id(&z).unwrap_or(BANC_DICT);
    let raw: Vec<u8> = rf.zs.decompress(&z)?;
    let dynamic: String = path.replace("_Static", "_Dynamic");
    let land: Vec<u8> = rf.zs.decompress(&source(rf, have, &dynamic)?)?;
    let mut y: Byml = Byml::parse(&raw)?;
    for (s, map) in spots {
        place(&mut y, &[&raw, &land], s, map)?;
    }
    let bytes: Vec<u8> = y.write()?;
    Ok(Out { path: path.to_string(), old: raw.len(), new: bytes.len(), z: zs::compress_raw(&bytes, Some(dict))? })
}

pub fn stage(rf: &mut Romfs, have: Have) -> Res<Vec<Out>> {
    let was: Vec<u8> = source(rf, have, SOURCE)?;
    let mut outs: Vec<Out> = Vec::new();
    for s in SPOTS.iter() {
        outs.push(graph(&was, s)?);
    }
    for path in TRAIL.iter() {
        let mut o: Out = free(&source(rf, have, path)?)?;
        o.path = path.to_string();
        outs.push(o);
    }
    for f in files() {
        let mut here: Vec<(&Spot, &str)> = Vec::new();
        for s in SPOTS.iter() {
            for m in s.maps {
                if s.file(m, "Static") == f {
                    here.push((s, m));
                }
            }
        }
        outs.push(one(rf, have, &f, &here)?);
    }
    Ok(outs)
}

pub fn describe(y: &Byml) -> Vec<(String, [f32; 3])> {
    let list: &[Node] = y.root.get("Actors").and_then(|a| a.as_arr()).unwrap_or(&[]);
    list.iter().filter(|a| text(a, "Gyaml") == MANAGER).filter_map(|a| vec3(a, "Translate").map(|t| (text(a, "Gyaml").to_string(), t))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dump() -> Option<std::path::PathBuf> {
        std::env::var("OOTRR_ROMFS").ok().map(std::path::PathBuf::from).filter(|p| p.join("Banc").is_dir())
    }

    fn none(_: &str) -> Option<Vec<u8>> {
        None
    }

    fn near(a: [f32; 3], b: [f32; 3]) -> f32 {
        ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    }

    #[test]
    fn spots_are_laid_out_to_be_dodged() {
        assert_eq!(SPOTS.len(), 7);
        for s in SPOTS.iter() {
            assert!(s.anchors.len() >= 3 && s.anchors.len() <= 8, "{}", s.name);
            assert!(s.interval >= 30.0 && s.fall >= 3.0, "{}", s.name);
            let far: f32 = near(s.area, s.manager);
            assert!((60.0..=110.0).contains(&far), "{} manager {far} away", s.name);
            assert!((55.0..=80.0).contains(&(s.manager[1] - s.area[1])), "{} manager height", s.name);
            for a in s.anchors {
                assert!(near(*a, s.area) <= 60.0, "{} anchor too far from the box", s.name);
                assert!((s.area[1] - a[1]).abs() < 9.0, "{} anchor height", s.name);
            }
            let mut names: Vec<String> = Vec::new();
            for i in 0..s.anchors.len() {
                names.push(i.to_string());
            }
            assert_eq!(names.len(), s.anchors.len());
        }
        let mut seen: Vec<String> = SPOTS.iter().map(|s| s.plan().graph).collect();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), SPOTS.len());
    }

    fn bearing(from: [f32; 3], to: [f32; 2], toward: bool) -> f32 {
        let d: [f32; 2] = if toward { [to[0] - from[0], to[1] - from[2]] } else { to };
        d[1].atan2(d[0]).to_degrees()
    }

    #[test]
    fn the_rocks_come_in_from_the_volcano() {
        for s in SPOTS.iter() {
            let far: bool = s.dir == "HyruleField" || s.dir == "HyruleCastle";
            let want: f32 = bearing(s.area, s.volcano, far);
            let got: f32 = bearing(s.area, [s.manager[0] - s.area[0], s.manager[2] - s.area[2]], false);
            let off: f32 = ((got - want + 540.0) % 360.0 - 180.0).abs();
            assert!(off <= 30.0, "{} thrower is {off} degrees off the volcano bearing", s.name);
        }
    }

    #[test]
    fn the_volcano_reference_is_the_maps_own_backdrop_and_the_thrower_clears_the_ground() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        for (dir, map, at) in [("HyruleField", "HyruleField_Child", FIELD_VOLCANO), ("HyruleField", "HyruleField_Adult", FIELD_VOLCANO), ("HyruleCastle", "HyruleCastle", CASTLE_VOLCANO)] {
            let y: Byml = rf.read_byml(&format!("Banc/{dir}/{map}_Static.bcett.byml.zs")).unwrap();
            let far: &Node = y.root.get("Actors").unwrap().as_arr().unwrap().iter().find(|a| text(a, "Gyaml") == "Land_DeathMountainFar_A_01").unwrap();
            let t: [f32; 3] = vec3(far, "Translate").unwrap();
            assert!((t[0] - at[0]).abs() < 1.0 && (t[2] - at[1]).abs() < 1.0, "{map} backdrop is at {t:?}");
        }
        for s in SPOTS.iter() {
            for m in s.maps {
                let g: crate::difficulty::ground::Ground = crate::difficulty::ground::load(&mut rf, m).unwrap().unwrap();
                for (y, _) in g.floors(s.manager[0], s.manager[2]) {
                    assert!(s.manager[1] - y >= 20.0, "{} thrower is {} above the ground in {m}", s.name, s.manager[1] - y);
                }
            }
        }
    }

    #[test]
    fn the_slug_marks_the_centre_and_stands_clear_of_the_anchors() {
        for s in SPOTS.iter() {
            if let Some(at) = s.slug {
                assert!(near(at, s.area) < 1.0, "{}", s.name);
                for a in s.anchors {
                    assert!(near(*a, at) >= 8.0, "{} anchor {a:?} is too close to the slug", s.name);
                }
            }
        }
    }

    #[test]
    fn each_spot_is_linked_only_to_its_own_pieces() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let all: Vec<Out> = stage(&mut rf, &none).unwrap();
        let mut ids: Vec<String> = Vec::new();
        for s in SPOTS.iter() {
            let p: Plan = s.plan();
            ids.push(p.module.clone());
            ids.push(p.manager.clone());
            ids.push(p.area.clone());
            ids.push(p.graph.clone());
        }
        let n: usize = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n, "two spots share a module, graph, manager or area id");
        for o in all.iter().filter(|o| o.path.ends_with(".map") || o.path.starts_with("Banc/")) {
            let now: Byml = Byml::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap();
            let acts: &[Node] = now.root.get("Actors").unwrap().as_arr().unwrap();
            let gs: &[Node] = now.root.get("AiGroups").unwrap().as_arr().unwrap();
            let mine: Vec<&Spot> = SPOTS.iter().filter(|s| s.maps.iter().any(|m| s.file(m, "Static") == o.path)).collect();
            let mut claimed: Vec<u64> = Vec::new();
            for s in mine {
                let p: Plan = s.plan();
                let g: &Node = gs.iter().find(|g| text(g, "Logic") == p.module).unwrap();
                let refs: &[Node] = g.get("References").unwrap().as_arr().unwrap();
                assert_eq!(text(&refs[0], "Id"), p.manager);
                assert_eq!(text(&refs[1], "Id"), p.area);
                let m: &Node = acts.iter().find(|a| a.get("Hash") == refs[0].get("Reference")).unwrap();
                let b: &Node = acts.iter().find(|a| a.get("Hash") == refs[1].get("Reference")).unwrap();
                assert_eq!(text(m, "Gyaml"), MANAGER);
                assert_eq!(vec3(m, "Translate"), Some(s.manager));
                assert_eq!(text(b, "Gyaml"), AREA);
                assert_eq!(vec3(b, "Translate"), Some(s.area));
                assert_eq!(vec3(b, "Scale"), Some(s.size));
                let links: &[Node] = m.get("Links").unwrap().as_arr().unwrap();
                assert_eq!(links.len(), s.anchors.len());
                let mut got: Vec<[f32; 3]> = Vec::new();
                for l in links {
                    assert_eq!(l.get("Src"), m.get("Hash"));
                    let t: &Node = acts.iter().find(|a| a.get("Hash") == l.get("Dst")).unwrap();
                    assert_eq!(text(t, "Gyaml"), ANCHOR);
                    let h: u64 = hash_of(t).unwrap();
                    assert!(!claimed.contains(&h), "{} shares an anchor with another spot", s.name);
                    claimed.push(h);
                    got.push(vec3(t, "Translate").unwrap());
                }
                for a in s.anchors {
                    assert!(got.contains(a), "{} lost anchor {a:?}", s.name);
                }
                let graph: &Out = all.iter().find(|x| x.path == p.graph).unwrap();
                let a: Ainb = Ainb::parse(&graph.z).unwrap();
                assert!((0..a.els.len()).any(|i| a.prop_str(i, NAME) == Some(p.manager.as_str())));
                assert!((0..a.els.len()).any(|i| a.prop_str(i, NAME) == Some(p.area.as_str())));
                for other in SPOTS.iter().filter(|x| x.name != s.name) {
                    let q: Plan = other.plan();
                    assert!(!(0..a.els.len()).any(|i| a.prop_str(i, NAME) == Some(q.manager.as_str()) || a.prop_str(i, NAME) == Some(q.area.as_str())), "{} graph mentions {}", s.name, other.name);
                }
            }
        }
    }

    #[test]
    fn the_storm_graph_wires_the_area_to_the_manager() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read(SOURCE).unwrap();
        let was: Ainb = Ainb::parse(&raw).unwrap();
        for s in SPOTS.iter() {
            let o: Out = graph(&raw, s).unwrap();
            let a: Ainb = Ainb::parse(&o.z).unwrap();
            assert_eq!(a.els.len(), was.els.len() + 4);
            assert!(audit(&a).is_empty());
            let p: Plan = s.plan();
            let m: usize = (0..a.els.len()).find(|&i| a.prop_str(i, NAME) == Some(p.manager.as_str())).unwrap();
            assert_eq!(a.el_name(m), NODE);
            let areas: Vec<usize> = (0..a.els.len()).filter(|&i| a.prop_str(i, NAME) == Some(p.area.as_str())).collect();
            assert_eq!(areas.len(), 1);
            assert_eq!(a.el_name(areas[0]), AREA_NODE);
            assert_eq!(a.write().unwrap(), o.z);
            assert_eq!(graph(&raw, s).unwrap().z, o.z);
        }
    }

    #[test]
    fn the_game_maps_take_the_storms_and_the_eruption_is_untouched() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let all: Vec<Out> = stage(&mut rf, &none).unwrap();
        let (graphs, maps): (Vec<Out>, Vec<Out>) = all.into_iter().partition(|o| o.path.ends_with(".ainb"));
        assert_eq!(graphs.len(), SPOTS.len() + TRAIL.len());
        assert_eq!(maps.iter().map(|o| o.path.clone()).collect::<Vec<String>>(), files());
        assert!(maps.iter().all(|o| !o.path.contains("DeathMountain")));
        for o in &maps {
            let was: Byml = rf.read_byml(&o.path).unwrap();
            let now: Byml = Byml::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap();
            let here: Vec<&Spot> = SPOTS.iter().filter(|s| s.maps.iter().any(|m| s.file(m, "Static") == o.path)).collect();
            let old: &[Node] = was.root.get("Actors").unwrap().as_arr().unwrap();
            let new: &[Node] = now.root.get("Actors").unwrap().as_arr().unwrap();
            let added: usize = here.iter().map(|s| s.added()).sum();
            assert_eq!(new.len(), old.len() + added, "{}", o.path);
            assert_eq!(&new[..old.len()], old, "the actors that were there are untouched in {}", o.path);
            let count = |y: &[Node], g: &str| y.iter().filter(|a| text(a, "Gyaml") == g).count();
            assert_eq!(count(new, MANAGER), count(old, MANAGER) + here.len());
            assert_eq!(count(new, SLUG), count(old, SLUG) + here.iter().filter(|s| s.slug.is_some()).count());
            assert_eq!(count(new, ANCHOR), count(old, ANCHOR) + here.iter().map(|s| s.anchors.len()).sum::<usize>());
            let mut hs: Vec<u64> = new.iter().filter_map(hash_of).collect();
            let n: usize = hs.len();
            hs.sort();
            hs.dedup();
            assert_eq!(hs.len(), n, "no hash is used twice in {}", o.path);
            let gs: &[Node] = now.root.get("AiGroups").unwrap().as_arr().unwrap();
            assert!(gs.windows(2).all(|w| hash_of(&w[0]) < hash_of(&w[1])));
            assert_eq!(gs.len(), was.root.get("AiGroups").unwrap().as_arr().unwrap().len() + here.len());
            for s in here {
                let p: Plan = s.plan();
                let g: &Node = gs.iter().find(|g| text(g, "Logic") == p.module).unwrap();
                let refs: &[Node] = g.get("References").unwrap().as_arr().unwrap();
                assert_eq!(refs.len(), 2);
                let m: &Node = new.iter().find(|a| text(a, "Gyaml") == MANAGER && vec3(a, "Translate") == Some(s.manager)).unwrap();
                assert_eq!(refs[0].get("Reference"), m.get("Hash"));
                assert_eq!(m.get("Links").unwrap().as_arr().unwrap().len(), s.anchors.len());
                assert_eq!(m.get("Dynamic").and_then(|d| d.get("FallTime")), Some(&Node::F32(s.fall)));
                let dynamic: Byml = rf.read_byml(&o.path.replace("_Static", "_Dynamic")).unwrap();
                let every: Vec<&Node> = old.iter().chain(dynamic.root.get("Actors").unwrap().as_arr().unwrap().iter()).collect();
                for n in every.iter().filter(|n| text(n, "Gyaml") == "StartPos" || text(n, "Gyaml").contains("Entrance") || text(n, "Gyaml").contains("HidingHole")) {
                    let t: [f32; 3] = vec3(n, "Translate").unwrap();
                    assert!(!(s.area[0] - s.size[0] / 2.0..=s.area[0] + s.size[0] / 2.0).contains(&t[0]) || !(s.area[2] - s.size[2] / 2.0..=s.area[2] + s.size[2] / 2.0).contains(&t[2]) || (t[1] - s.area[1]).abs() > 12.0, "{} trigger box holds {} at {t:?}", s.name, text(n, "Gyaml"));
                    for at in s.anchors {
                        assert!(near(*at, t) >= 12.0 || (at[1] - t[1]).abs() > 12.0, "{} anchor {at:?} lands within 12 of {} at {t:?}", s.name, text(n, "Gyaml"));
                    }
                }
                for at in s.anchors {
                    for n in old.iter().filter(|n| text(n, "Gyaml").starts_with("Npc_") || text(n, "Gyaml").starts_with("Enemy_")) {
                        let t: [f32; 3] = vec3(n, "Translate").unwrap();
                        assert!(near(*at, t) >= 9.0 || (at[1] - t[1]).abs() > 4.0, "{} anchor {at:?} lands within 9 of {} at {t:?}", s.name, text(n, "Gyaml"));
                    }
                }
                for (i, at) in s.anchors.iter().enumerate() {
                    let a: &Node = new.iter().find(|a| text(a, "Gyaml") == ANCHOR && vec3(a, "Translate") == Some(*at)).unwrap();
                    assert_eq!(text(a, "Name"), i.to_string());
                }
            }
        }
        let trail: Byml = rf.read_byml("Banc/DeathMountainTrail/DeathMountainTrail_Child_Static.bcett.byml.zs").unwrap();
        assert_eq!(describe(&trail).len(), 5);
    }

    #[test]
    fn the_setting_off_writes_nothing_and_on_writes_only_these_files() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let off: Vec<Out> = crate::difficulty::stage(&mut rf, &crate::difficulty::Opts::default(), &none, "seed").unwrap();
        assert!(off.is_empty());
        let on: Vec<Out> = crate::difficulty::stage(&mut rf, &crate::difficulty::Opts { fireballs: true, ..crate::difficulty::Opts::default() }, &none, "seed").unwrap();
        let mut paths: Vec<String> = on.iter().map(|o| o.path.clone()).collect();
        let mut want: Vec<String> = SPOTS.iter().map(|s| s.plan().graph).chain(TRAIL.iter().map(|t| t.to_string())).chain(files()).collect();
        paths.sort();
        want.sort();
        assert_eq!(paths, want);
    }

    #[test]
    fn the_anchors_stand_on_the_ground_in_both_ages() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        for s in SPOTS.iter() {
            for m in s.maps {
                let g: crate::difficulty::ground::Ground = crate::difficulty::ground::load(&mut rf, m).unwrap().unwrap();
                for a in s.anchors {
                    let (y, _): (f32, u32) = g.floor_near(a[0], a[1], a[2], 2.0, 2.0).unwrap_or_else(|| panic!("{} anchor {a:?} is off the ground in {m}", s.name));
                    assert!((a[1] - LIFT - y).abs() < 1.0, "{} anchor {a:?} is {} above the ground in {m}", s.name, a[1] - y);
                }
            }
        }
    }

    #[test]
    fn the_trail_storms_run_on_every_visit_only_with_the_setting_on() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let on: Vec<Out> = stage(&mut rf, &none).unwrap();
        for age in ["Child", "Adult"] {
            let y: Byml = rf.read_byml(&format!("Banc/DeathMountainTrail/DeathMountainTrail_{age}_Static.bcett.byml.zs")).unwrap();
            let acts: &[Node] = y.root.get("Actors").unwrap().as_arr().unwrap();
            let gs: &[Node] = y.root.get("AiGroups").unwrap().as_arr().unwrap();
            let mut found: Vec<String> = Vec::new();
            for m in acts.iter().filter(|a| text(a, "Gyaml") == MANAGER) {
                for g in gs.iter().filter(|g| g.get("References").unwrap().as_arr().unwrap().iter().any(|r| r.get("Reference") == m.get("Hash"))) {
                    let base: &str = text(g, "Logic").rsplit('/').next().unwrap().trim_end_matches(".ain");
                    found.push(format!("Logic/{base}.ainb"));
                }
            }
            assert_eq!(found.len(), 5, "{age}");
            let mut freed: Vec<&String> = found.iter().filter(|f| TRAIL.contains(&f.as_str())).collect();
            freed.sort();
            assert_eq!(freed.len(), 4, "{age}");
            for f in found.iter().filter(|f| !TRAIL.contains(&f.as_str())) {
                let a: Ainb = Ainb::parse(&rf.read(f).unwrap()).unwrap();
                let flags: Vec<&str> = a.find_els("HoldGameDataGetBoolSimple").into_iter().filter_map(|i| a.in_str(i, "GameDataNameBool")).collect();
                assert!(flags.iter().any(|n| n.contains("FallingRockDemoActivated")), "{f} is not tied to the demo: {flags:?}");
                assert!(on.iter().all(|o| &o.path != f), "{f} is the demo-gated storm and must stay vanilla");
            }
        }
        for path in TRAIL.iter() {
            let was: Ainb = Ainb::parse(&rf.read(path).unwrap()).unwrap();
            let o: &Out = on.iter().find(|o| o.path == *path).unwrap();
            let now: Ainb = Ainb::parse(&o.z).unwrap();
            assert_eq!(now.els.len(), was.els.len(), "{path}");
            assert!(audit(&now).is_empty());
            let delete = |a: &Ainb| -> Vec<bool> { a.find_els(AREA_NODE).into_iter().map(|i| a.input(BOOL, a.find_in(i, BOOL, DELETE).unwrap()).unwrap().src != -1).collect() };
            assert_eq!(delete(&was), vec![true], "{path}");
            assert_eq!(delete(&now), vec![false], "{path}");
            let m: usize = now.find_els(NODE)[0];
            assert_eq!(now.prop_str(m, NAME), was.prop_str(was.find_els(NODE)[0], NAME));
            assert_eq!(was.find_els("HoldGameDataGetBoolSimple").len(), now.find_els("HoldGameDataGetBoolSimple").len());
            assert_eq!(free(&rf.read(path).unwrap()).unwrap().z, o.z);
        }
        let off: Vec<Out> = crate::difficulty::stage(&mut rf, &crate::difficulty::Opts::default(), &none, "seed").unwrap();
        assert!(off.iter().all(|o| !o.path.contains("DeathMountainTrail")));
        let again: Vec<Out> = crate::difficulty::stage(&mut rf, &crate::difficulty::Opts { fireballs: true, ..crate::difficulty::Opts::default() }, &none, "seed").unwrap();
        assert_eq!(again.iter().filter(|o| o.path.contains("DeathMountainTrail")).count(), TRAIL.len());
    }

    #[test]
    fn the_new_spots_have_open_ground_and_the_ages_they_claim() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        for s in SPOTS.iter().filter(|s| s.dir != "HyruleCastle") {
            assert_eq!(s.maps.len(), 2, "{}", s.name);
            for m in s.maps {
                let g: crate::difficulty::ground::Ground = crate::difficulty::ground::load(&mut rf, m).unwrap().unwrap();
                for a in s.anchors {
                    assert!(g.clear(*a) >= 4.0, "{} anchor {a:?} is {} from a wall in {m}", s.name, g.clear(*a));
                }
            }
        }
    }
}
