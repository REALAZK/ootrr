use std::f32::consts::{FRAC_PI_2, PI};

use crate::byml::{Byml, Node};
use crate::difficulty::wake;
use crate::difficulty::ambush::{actor, euler, fresh, list, Frame};
use crate::error::Res;
use crate::flow_locations::dict_id;
use crate::placement::hash_of;
use crate::qol::Have;
use crate::romfs::Romfs;
use crate::staging::{Out, BANC_DICT};
use crate::zs;

pub const HAND: &str = "Enemy_DeadHand";
pub const FINGERS: &str = "Enemy_DeadHand_Hand";
pub const KNIGHT: &str = "Enemy_IronKnuckle_Silver";
pub const LIKE: &str = "Enemy_LikeLike";
pub const GUAY: &str = "Enemy_Guay_Fixed";
pub const OCTO: &str = "Enemy_Octorok";
pub const SCRUB: &str = "Enemy_Scrub_Mad";
pub const LEAF: &str = "Enemy_Scrub_Mad_Leaf";
pub const BEAMOS: &str = crate::difficulty::beamos::CLASS;
pub const BEAMOS_SCALE: f32 = 1.6;
pub const FIRST_ADULT: &str = "HavePlayedEvent.Dm_TimeTemple_0065";
pub const LAKE_FULL: &str = "HavePlayedEvent.Dm_LakeHrl_0020";
pub const ADULT: &str = "HyruleField_Adult";
pub const SEED: &str = "ootrr special encounters";
pub const GROUPS: [&str; 2] = ["FarDeleteGroups", "SimultaneousGroups"];
pub const RING: f32 = 3.0;
pub const APPEAR: f32 = 3.5;
pub const NOTE: &str = "Special encounters: a Dead Hand waits in the dark hole in east Hyrule Field (by the ReDead and the chest) and in the foggy hole at the foot of the Death Mountain Trail (with the Baby Dodongos), and an Iron Knuckle stands in the open field between Kakariko, Lon Lon Ranch and Kokiri Forest and gets up when you come close. Nine Like Likes wait around Lake Hylia, child and adult. Child Hyrule Field gets 24 more Guays in six loose flocks over the open grass. Twelve Octoroks swim in the open water of Lake Hylia, along the two rope bridges and off the fishing pond shore, child and adult (once the lake is full again). Ten Mad Scrubs hide in the trees south west of Lon Lon Ranch, in both maps. Ten Beamos stand between the pillars of the adult Temple of Time hall and are only there after the first adult scenes in the temple. They are a new kind with a longer sight, so they see Link anywhere in the aisle; the game's own Beamos are untouched. They come back whenever the area reloads.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum What {
    DeadHand,
    IronKnuckle,
    LikeLike,
    Guay,
    Octorok,
    MadScrub,
    Beamos,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub name: &'static str,
    pub dir: &'static str,
    pub maps: &'static [&'static str],
    pub what: What,
    pub at: [f32; 3],
    pub turn: f32,
}

pub const PLACES: [Place; 68] = [
    Place { name: "dead_hand_hyrule_field_hole", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::DeadHand, at: [148.6, 136.41, 13.7], turn: 0.7887344 },
    Place { name: "dead_hand_death_mountain_hole", dir: "DeathMountainTrail", maps: &["DeathMountainTrail_Child", "DeathMountainTrail_Adult"], what: What::DeadHand, at: [-5.0, 58.17, -38.5], turn: 0.2617994 },
    Place { name: "iron_knuckle_hyrule_field", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::IronKnuckle, at: [134.0, 157.7, 168.5], turn: 0.0 },
    Place { name: "like_like_lake_hylia_north_west_corner", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::LikeLike, at: [-202.4, 133.5, 403.7], turn: 0.0 },
    Place { name: "like_like_lake_hylia_north_east_shore", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::LikeLike, at: [-87.3, 133.5, 416.6], turn: 0.0 },
    Place { name: "like_like_lake_hylia_bridge_island", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::LikeLike, at: [-206.3, 130.9, 452.85], turn: 0.0 },
    Place { name: "like_like_lake_hylia_west_island", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::LikeLike, at: [-294.2, 131.3, 482.7], turn: 0.0 },
    Place { name: "like_like_lake_hylia_central_island", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::LikeLike, at: [-188.5, 131.3, 510.3], turn: 0.0 },
    Place { name: "like_like_lake_hylia_ring_island", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::LikeLike, at: [-155.5, 131.46, 523.0], turn: 0.0 },
    Place { name: "like_like_lake_hylia_islet_south_west", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::LikeLike, at: [-214.3, 131.3, 530.7], turn: 0.0 },
    Place { name: "like_like_lake_hylia_west_shore_strip", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::LikeLike, at: [-292.7, 131.5, 547.7], turn: 0.0 },
    Place { name: "like_like_lake_hylia_south_island", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::LikeLike, at: [-231.7, 131.0, 580.8], turn: 0.0 },
    Place { name: "guay_hyrule_field_west_edge_south_1", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [-104.0, 153.3, 57.0], turn: 0.4 },
    Place { name: "guay_hyrule_field_west_edge_south_2", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [-97.0, 154.1, 56.0], turn: 2.1 },
    Place { name: "guay_hyrule_field_west_edge_south_3", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [-96.0, 152.9, 63.0], turn: 3.7 },
    Place { name: "guay_hyrule_field_west_edge_south_4", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [-103.0, 154.5, 64.0], turn: 5.2 },
    Place { name: "guay_hyrule_field_east_south_1", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [88.0, 153.5, 67.0], turn: 0.4 },
    Place { name: "guay_hyrule_field_east_south_2", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [95.0, 154.3, 66.0], turn: 2.1 },
    Place { name: "guay_hyrule_field_east_south_3", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [96.0, 153.1, 73.0], turn: 3.7 },
    Place { name: "guay_hyrule_field_east_south_4", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [89.0, 154.7, 74.0], turn: 5.2 },
    Place { name: "guay_hyrule_field_ranch_south_1", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [-104.0, 150.6, 312.0], turn: 0.4 },
    Place { name: "guay_hyrule_field_ranch_south_2", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [-97.0, 151.4, 311.0], turn: 2.1 },
    Place { name: "guay_hyrule_field_ranch_south_3", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [-96.0, 150.2, 318.0], turn: 3.7 },
    Place { name: "guay_hyrule_field_ranch_south_4", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [-103.0, 151.8, 319.0], turn: 5.2 },
    Place { name: "guay_hyrule_field_middle_south_1", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [36.0, 150.0, 319.0], turn: 0.4 },
    Place { name: "guay_hyrule_field_middle_south_2", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [43.0, 150.8, 318.0], turn: 2.1 },
    Place { name: "guay_hyrule_field_middle_south_3", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [44.0, 149.6, 325.0], turn: 3.7 },
    Place { name: "guay_hyrule_field_middle_south_4", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [37.0, 151.2, 326.0], turn: 5.2 },
    Place { name: "guay_hyrule_field_far_west_1", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [-244.0, 142.0, 293.0], turn: 0.4 },
    Place { name: "guay_hyrule_field_far_west_2", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [-237.0, 142.8, 292.0], turn: 2.1 },
    Place { name: "guay_hyrule_field_far_west_3", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [-236.0, 141.6, 299.0], turn: 3.7 },
    Place { name: "guay_hyrule_field_far_west_4", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [-243.0, 143.2, 300.0], turn: 5.2 },
    Place { name: "guay_hyrule_field_east_middle_1", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [136.0, 158.5, 132.0], turn: 0.4 },
    Place { name: "guay_hyrule_field_east_middle_2", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [143.0, 159.3, 131.0], turn: 2.1 },
    Place { name: "guay_hyrule_field_east_middle_3", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [144.0, 158.1, 138.0], turn: 3.7 },
    Place { name: "guay_hyrule_field_east_middle_4", dir: "HyruleField", maps: &["HyruleField_Child"], what: What::Guay, at: [137.0, 159.7, 139.0], turn: 5.2 },
    Place { name: "octorok_lake_hylia_bridge_1_north_1", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::Octorok, at: [-179.7, 129.0, 435.1], turn: 0.0 },
    Place { name: "octorok_lake_hylia_bridge_1_north_2", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::Octorok, at: [-187.7, 129.0, 441.7], turn: 0.0 },
    Place { name: "octorok_lake_hylia_bridge_1_north_3", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::Octorok, at: [-194.5, 129.0, 445.7], turn: 0.0 },
    Place { name: "octorok_lake_hylia_bridge_1_south_1", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::Octorok, at: [-174.9, 129.0, 445.5], turn: 0.0 },
    Place { name: "octorok_lake_hylia_bridge_1_south_2", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::Octorok, at: [-185.5, 129.0, 452.4], turn: 0.0 },
    Place { name: "octorok_lake_hylia_bridge_2_west_1", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::Octorok, at: [-206.4, 129.0, 470.8], turn: 0.0 },
    Place { name: "octorok_lake_hylia_bridge_2_east_1", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::Octorok, at: [-192.9, 129.0, 474.6], turn: 0.0 },
    Place { name: "octorok_lake_hylia_bridge_2_west_2", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::Octorok, at: [-205.8, 129.0, 481.1], turn: 0.0 },
    Place { name: "octorok_lake_hylia_bridge_2_east_2", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::Octorok, at: [-193.0, 129.0, 484.0], turn: 0.0 },
    Place { name: "octorok_lake_hylia_bridge_2_west_3", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::Octorok, at: [-206.9, 129.0, 489.1], turn: 0.0 },
    Place { name: "octorok_lake_hylia_fishing_pond_shore_1", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::Octorok, at: [-114.2, 129.0, 472.8], turn: 0.0 },
    Place { name: "octorok_lake_hylia_fishing_pond_shore_2", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::Octorok, at: [-103.5, 129.0, 465.5], turn: 0.0 },
    Place { name: "mad_scrub_ranch_trees_1", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::MadScrub, at: [-154.2, 143.89, 286.3], turn: 0.0 },
    Place { name: "mad_scrub_ranch_trees_2", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::MadScrub, at: [-137.9, 145.36, 289.4], turn: 0.9 },
    Place { name: "mad_scrub_ranch_trees_3", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::MadScrub, at: [-116.3, 148.27, 294.1], turn: 1.8 },
    Place { name: "mad_scrub_ranch_trees_4", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::MadScrub, at: [-98.5, 147.53, 301.1], turn: 2.7 },
    Place { name: "mad_scrub_ranch_trees_5", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::MadScrub, at: [-76.2, 149.12, 310.2], turn: 3.6 },
    Place { name: "mad_scrub_ranch_trees_6", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::MadScrub, at: [-161.7, 142.29, 312.2], turn: 4.5 },
    Place { name: "mad_scrub_ranch_trees_7", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::MadScrub, at: [-150.6, 142.72, 319.7], turn: 5.4 },
    Place { name: "mad_scrub_ranch_trees_8", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::MadScrub, at: [-139.8, 143.56, 323.5], turn: 6.3 },
    Place { name: "mad_scrub_ranch_trees_9", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::MadScrub, at: [-118.4, 144.69, 328.6], turn: 7.2 },
    Place { name: "mad_scrub_ranch_trees_10", dir: "HyruleField", maps: &["HyruleField_Child", "HyruleField_Adult"], what: What::MadScrub, at: [-97.4, 144.54, 330.2], turn: 8.1 },
    Place { name: "beamos_temple_of_time_north_1", dir: "Market", maps: &["Market_Adult"], what: What::Beamos, at: [85.0, 103.94, -43.5], turn: 0.0 },
    Place { name: "beamos_temple_of_time_north_2", dir: "Market", maps: &["Market_Adult"], what: What::Beamos, at: [92.5, 103.94, -43.5], turn: 0.0 },
    Place { name: "beamos_temple_of_time_north_3", dir: "Market", maps: &["Market_Adult"], what: What::Beamos, at: [99.5, 103.94, -43.5], turn: 0.0 },
    Place { name: "beamos_temple_of_time_north_4", dir: "Market", maps: &["Market_Adult"], what: What::Beamos, at: [106.5, 103.94, -43.5], turn: 0.0 },
    Place { name: "beamos_temple_of_time_north_5", dir: "Market", maps: &["Market_Adult"], what: What::Beamos, at: [113.5, 104.01, -43.5], turn: 0.0 },
    Place { name: "beamos_temple_of_time_south_1", dir: "Market", maps: &["Market_Adult"], what: What::Beamos, at: [85.0, 103.94, -24.5], turn: 3.1415927 },
    Place { name: "beamos_temple_of_time_south_2", dir: "Market", maps: &["Market_Adult"], what: What::Beamos, at: [92.5, 103.94, -24.5], turn: 3.1415927 },
    Place { name: "beamos_temple_of_time_south_3", dir: "Market", maps: &["Market_Adult"], what: What::Beamos, at: [99.5, 103.94, -24.5], turn: 3.1415927 },
    Place { name: "beamos_temple_of_time_south_4", dir: "Market", maps: &["Market_Adult"], what: What::Beamos, at: [106.5, 103.94, -24.5], turn: 3.1415927 },
    Place { name: "beamos_temple_of_time_south_5", dir: "Market", maps: &["Market_Adult"], what: What::Beamos, at: [113.5, 103.96, -24.5], turn: 3.1415927 },
];

pub fn file(dir: &str, map: &str, kind: &str) -> String {
    format!("Banc/{dir}/{map}_{kind}.bcett.byml.zs")
}

pub fn files() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for p in PLACES.iter() {
        for m in p.maps {
            let f: String = file(p.dir, m, "Static");
            if !out.contains(&f) {
                out.push(f);
            }
        }
    }
    out
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

fn reference(src: u64, dst: u64) -> Node {
    let mut l: Node = Node::Map(Vec::new());
    l.set("Dst", Node::U64(dst));
    l.set("Gyaml", Node::Str("Reference".to_string()));
    l.set("Name", Node::Str("Reference".to_string()));
    l.set("Src", Node::U64(src));
    l
}

pub fn dead_hand_nodes(at: [f32; 3], turn: f32, raws: &[&[u8]], seed: &str) -> (Node, Vec<Node>) {
    let mut taken: Vec<u64> = Vec::new();
    let body: u64 = next(raws, &mut taken, &format!("{seed} body"));
    let ring: Frame = Frame { at, yaw: turn };
    let spots: [([f32; 3], f32, Option<(&str, Node)>); 4] = [
        ([0.0, 0.0, -RING], 0.0, Some(("DeadHandHand_IsEnableAppearDelay", Node::Bool(false)))),
        ([0.0, 0.0, RING], PI, Some(("DeadHandHand_AppearOrderNum", Node::I32(2)))),
        ([-RING, 0.0, 0.0], FRAC_PI_2, Some(("DeadHandHand_AppearOrderNum", Node::I32(3)))),
        ([RING, 0.0, 0.0], -FRAC_PI_2, Some(("DeadHandHand_AppearOrderNum", Node::I32(3)))),
    ];
    let mut hands: Vec<Node> = Vec::new();
    let mut ids: Vec<u64> = Vec::new();
    for (i, (off, face, dynamic)) in spots.iter().enumerate() {
        let id: u64 = next(raws, &mut taken, &format!("{seed} hand {i}"));
        let mut h: Node = actor(FINGERS, id, ring.point(*off), euler(0.0, ring.turn(*face)));
        if let Some((k, v)) = dynamic {
            h.set("Dynamic", Node::Map(vec![(k.to_string(), v.clone())]));
        }
        hands.push(h);
        ids.push(id);
    }
    let mut b: Node = actor(HAND, body, at, [0.0; 3]);
    b.set(
        "Dynamic",
        Node::Map(vec![
            ("DeadHand_HoldActorCancelSleepFrame".to_string(), Node::F32(1800.0)),
            ("DeadHand_IsManageHandNumByLife".to_string(), Node::Bool(true)),
            ("DeadHand_RangeOfAppearArea".to_string(), Node::F32(APPEAR)),
            ("Drop__DropTable".to_string(), Node::Str("None".to_string())),
        ]),
    );
    b.set("Links", Node::Arr(ids.iter().map(|i| reference(body, *i)).collect()));
    b.set("Name", Node::Str("DeadHand".to_string()));
    (b, hands)
}

pub fn knight_node(at: [f32; 3], turn: f32, raws: &[&[u8]], seed: &str) -> Node {
    let mut taken: Vec<u64> = Vec::new();
    let id: u64 = next(raws, &mut taken, seed);
    let mut k: Node = actor(KNIGHT, id, at, euler(0.0, turn));
    k.set("Dynamic", Node::Map(vec![("Drop__DropTable".to_string(), Node::Str("None".to_string())), (wake::KEY.to_string(), Node::Bool(true))]));
    k
}

pub fn dead_hand(y: &mut Byml, p: &Place, raws: &[&[u8]], map: &str) -> Res<usize> {
    let seed: String = format!("{SEED} {map} {}", p.name);
    let (b, hands): (Node, Vec<Node>) = dead_hand_nodes(p.at, p.turn, raws, &seed);
    let mut all: Vec<u64> = hands.iter().filter_map(hash_of).collect();
    all.extend(hash_of(&b));
    all.sort();
    let acts: &mut Vec<Node> = list(y, "Actors")?;
    acts.push(b);
    acts.extend(hands);
    for g in GROUPS {
        list(y, g)?.push(Node::Arr(all.iter().map(|h| Node::U64(*h)).collect()));
    }
    Ok(5)
}

pub fn iron_knuckle(y: &mut Byml, p: &Place, raws: &[&[u8]], map: &str) -> Res<usize> {
    list(y, "Actors")?.push(knight_node(p.at, p.turn, raws, &format!("{SEED} {map} {}", p.name)));
    Ok(1)
}

pub fn like_like(y: &mut Byml, p: &Place, raws: &[&[u8]], map: &str) -> Res<usize> {
    let mut taken: Vec<u64> = Vec::new();
    let id: u64 = next(raws, &mut taken, &format!("{SEED} {map} {}", p.name));
    let mut l: Node = actor(LIKE, id, p.at, euler(0.0, p.turn));
    l.set("Dynamic", Node::Map(vec![("Drop__DropTable".to_string(), Node::Str("None".to_string()))]));
    list(y, "Actors")?.push(l);
    Ok(1)
}

pub fn guay(y: &mut Byml, p: &Place, raws: &[&[u8]], map: &str) -> Res<usize> {
    let mut taken: Vec<u64> = Vec::new();
    let id: u64 = next(raws, &mut taken, &format!("{SEED} {map} {}", p.name));
    let g: Node = actor(GUAY, id, p.at, euler(0.0, p.turn));
    list(y, "Actors")?.push(g);
    Ok(1)
}

pub fn octorok(y: &mut Byml, p: &Place, raws: &[&[u8]], map: &str) -> Res<usize> {
    let mut taken: Vec<u64> = Vec::new();
    let id: u64 = next(raws, &mut taken, &format!("{SEED} {map} {}", p.name));
    let mut o: Node = actor(OCTO, id, p.at, euler(0.0, p.turn));
    o.set("Dynamic", Node::Map(vec![("Drop__DropTable".to_string(), Node::Str("None".to_string()))]));
    if map == ADULT {
        let flag: Vec<(String, Node)> = vec![("FlagName".to_string(), Node::Str(LAKE_FULL.to_string())), ("IsCheckLazy".to_string(), Node::Bool(false)), ("IsNegation".to_string(), Node::Bool(false))];
        o.set("Presence", Node::Map(flag));
    }
    list(y, "Actors")?.push(o);
    Ok(1)
}

pub fn mad_scrub(y: &mut Byml, p: &Place, raws: &[&[u8]], map: &str) -> Res<usize> {
    let mut taken: Vec<u64> = Vec::new();
    let body: u64 = next(raws, &mut taken, &format!("{SEED} {map} {} body", p.name));
    let leaf: u64 = next(raws, &mut taken, &format!("{SEED} {map} {} leaf", p.name));
    let b: Node = actor(SCRUB, body, p.at, euler(0.0, p.turn));
    let mut l: Node = actor(LEAF, leaf, p.at, euler(0.0, p.turn));
    l.set("Links", Node::Arr(vec![reference(leaf, body)]));
    let acts: &mut Vec<Node> = list(y, "Actors")?;
    acts.push(b);
    acts.push(l);
    Ok(2)
}

pub fn beamos(y: &mut Byml, p: &Place, raws: &[&[u8]], map: &str) -> Res<usize> {
    let mut taken: Vec<u64> = Vec::new();
    let id: u64 = next(raws, &mut taken, &format!("{SEED} {map} {}", p.name));
    let mut b: Node = actor(BEAMOS, id, p.at, euler(0.0, p.turn));
    b.set("Dynamic", Node::Map(vec![("Drop__DropTable".to_string(), Node::Str("None".to_string()))]));
    b.set("Scale", crate::difficulty::ambush::floats([BEAMOS_SCALE; 3]));
    let flag: Vec<(String, Node)> = vec![("FlagName".to_string(), Node::Str(FIRST_ADULT.to_string())), ("IsCheckLazy".to_string(), Node::Bool(false)), ("IsNegation".to_string(), Node::Bool(false))];
    b.set("Presence", Node::Map(flag));
    list(y, "Actors")?.push(b);
    Ok(1)
}

pub fn place(y: &mut Byml, p: &Place, raws: &[&[u8]], map: &str) -> Res<usize> {
    match p.what {
        What::DeadHand => dead_hand(y, p, raws, map),
        What::IronKnuckle => iron_knuckle(y, p, raws, map),
        What::LikeLike => like_like(y, p, raws, map),
        What::Guay => guay(y, p, raws, map),
        What::Octorok => octorok(y, p, raws, map),
        What::MadScrub => mad_scrub(y, p, raws, map),
        What::Beamos => beamos(y, p, raws, map),
    }
}

fn source(rf: &Romfs, have: Have, p: &str) -> Res<Vec<u8>> {
    match have(p) {
        Some(b) => Ok(b),
        None => rf.read(p),
    }
}

fn one(rf: &mut Romfs, have: Have, dir: &str, map: &str, only: &[Place]) -> Res<Out> {
    let p: String = file(dir, map, "Static");
    let z: Vec<u8> = source(rf, have, &p)?;
    let dict: u32 = dict_id(&z).unwrap_or(BANC_DICT);
    let raw: Vec<u8> = rf.zs.decompress(&z)?;
    let land: Vec<u8> = rf.zs.decompress(&source(rf, have, &file(dir, map, "Dynamic"))?)?;
    let mut y: Byml = Byml::parse(&raw)?;
    for pl in only {
        place(&mut y, pl, &[&raw, &land], map)?;
    }
    let bytes: Vec<u8> = y.write()?;
    Ok(Out { path: p, old: raw.len(), new: bytes.len(), z: zs::compress_raw(&bytes, Some(dict))? })
}

pub fn stage_places(rf: &mut Romfs, have: Have, places: &[Place]) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    for f in files_of(places) {
        let (dir, map): (&str, &str) = f;
        let here: Vec<Place> = places.iter().filter(|p| p.dir == dir && p.maps.contains(&map)).copied().collect();
        outs.push(one(rf, have, dir, map, &here)?);
    }
    Ok(outs)
}

fn files_of(places: &[Place]) -> Vec<(&'static str, &'static str)> {
    let mut out: Vec<(&'static str, &'static str)> = Vec::new();
    for p in places {
        for m in p.maps {
            if !out.contains(&(p.dir, *m)) {
                out.push((p.dir, *m));
            }
        }
    }
    out
}

pub fn stage(rf: &mut Romfs, have: Have) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = stage_places(rf, have, &PLACES)?;
    if PLACES.iter().any(|p| p.what == What::IronKnuckle) {
        outs.push(wake::stage(rf, have)?);
    }
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::difficulty::ambush::{text, vec3};

    fn around(y: &Byml, at: [f32; 3]) -> Vec<(String, f32)> {
        let acts: &[Node] = y.root.get("Actors").and_then(|a| a.as_arr()).unwrap_or(&[]);
        acts.iter()
            .filter_map(|a| {
                let t: [f32; 3] = vec3(a, "Translate")?;
                Some((text(a, "Gyaml").to_string(), ((t[0] - at[0]).powi(2) + (t[1] - at[1]).powi(2) + (t[2] - at[2]).powi(2)).sqrt()))
            })
            .collect()
    }

    fn map() -> Byml {
        let mut g: Node = Node::Map(Vec::new());
        g.set("Hash", Node::U64(5));
        let mut root: Node = Node::Map(Vec::new());
        root.set("Actors", Node::Arr(vec![actor("Enemy_Octorok", 5, [0.0; 3], [0.0; 3])]));
        root.set("FarDeleteGroups", Node::Arr(vec![Node::Arr(vec![Node::U64(5), Node::U64(6)])]));
        root.set("SimultaneousGroups", Node::Arr(vec![Node::Arr(vec![Node::U64(5), Node::U64(6)])]));
        Byml { root, version: 7, big: false }
    }

    fn actors(y: &Byml) -> &[Node] {
        y.root.get("Actors").unwrap().as_arr().unwrap()
    }

    fn count(y: &Byml, name: &str) -> usize {
        actors(y).iter().filter(|a| text(a, "Gyaml") == name).count()
    }

    fn close(a: [f32; 3], b: [f32; 3], tol: f32) -> bool {
        a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < tol)
    }

    #[test]
    fn a_dead_hand_comes_with_four_hands_that_it_links_to_and_one_group() {
        let mut y: Byml = map();
        let p: Place = PLACES[0];
        assert_eq!(place(&mut y, &p, &[], "Test").unwrap(), 5);
        assert_eq!((count(&y, HAND), count(&y, FINGERS)), (1, 4));
        let body: &Node = actors(&y).iter().find(|a| text(a, "Gyaml") == HAND).unwrap();
        assert_eq!(vec3(body, "Translate"), Some(p.at));
        assert_eq!(text(body, "Name"), "DeadHand");
        assert_eq!(body.get("Dynamic").and_then(|d| d.get("DeadHand_IsManageHandNumByLife")), Some(&Node::Bool(true)));
        assert_eq!(body.get("Dynamic").and_then(|d| d.get("DeadHand_RangeOfAppearArea")), Some(&Node::F32(APPEAR)));
        let links: &[Node] = body.get("Links").unwrap().as_arr().unwrap();
        assert_eq!(links.len(), 4);
        let me: u64 = hash_of(body).unwrap();
        for l in links {
            assert_eq!(l.get("Src"), Some(&Node::U64(me)));
            assert_eq!(text(l, "Gyaml"), "Reference");
            let Some(Node::U64(dst)) = l.get("Dst") else { panic!() };
            let hand: &Node = actors(&y).iter().find(|a| hash_of(a) == Some(*dst)).unwrap();
            assert_eq!(text(hand, "Gyaml"), FINGERS);
            let t: [f32; 3] = vec3(hand, "Translate").unwrap();
            assert!(((t[0] - p.at[0]).powi(2) + (t[2] - p.at[2]).powi(2)).sqrt() - RING < 0.01 && t[1] == p.at[1]);
        }
        let mut hs: Vec<u64> = actors(&y).iter().filter_map(hash_of).collect();
        hs.sort();
        hs.dedup();
        assert_eq!(hs.len(), 6);
        for g in GROUPS {
            let gs: &[Node] = y.root.get(g).unwrap().as_arr().unwrap();
            assert_eq!(gs.len(), 2);
            let ids: Vec<u64> = gs[1].as_arr().unwrap().iter().map(|n| if let Node::U64(v) = n { *v } else { 0 }).collect();
            assert_eq!(ids.len(), 5);
            assert!(ids.windows(2).all(|w| w[0] < w[1]) && ids.contains(&me));
        }
        let back: Byml = Byml::parse(&y.write().unwrap()).unwrap();
        assert_eq!(back.root, y.root);
    }

    #[test]
    fn the_hands_face_the_middle_like_the_vanilla_ring() {
        let mut y: Byml = map();
        let p: Place = Place { turn: 0.0, ..PLACES[1] };
        dead_hand(&mut y, &p, &[], "Test").unwrap();
        let hands: Vec<&Node> = actors(&y).iter().filter(|a| text(a, "Gyaml") == FINGERS).collect();
        let north: &Node = hands.iter().find(|h| vec3(h, "Translate").unwrap()[2] < p.at[2] - 1.0).unwrap();
        assert!(close(vec3(north, "Translate").unwrap(), [p.at[0], p.at[1], p.at[2] - RING], 0.001));
        assert_eq!(vec3(north, "Rotate"), Some([0.0, 0.0, 0.0]));
        let south: &Node = hands.iter().find(|h| vec3(h, "Translate").unwrap()[2] > p.at[2] + 1.0).unwrap();
        assert!(close(vec3(south, "Rotate").unwrap(), [PI, 0.0, PI], 0.001));
        let west: &Node = hands.iter().find(|h| vec3(h, "Translate").unwrap()[0] < p.at[0] - 1.0).unwrap();
        assert!(close(vec3(west, "Rotate").unwrap(), [0.0, FRAC_PI_2, 0.0], 0.001));
        let order = |h: &Node, k: &str| h.get("Dynamic").and_then(|d| d.get(k)).cloned();
        assert_eq!(order(north, "DeadHandHand_IsEnableAppearDelay"), Some(Node::Bool(false)));
        assert_eq!(order(south, "DeadHandHand_AppearOrderNum"), Some(Node::I32(2)));
        assert_eq!(order(west, "DeadHandHand_AppearOrderNum"), Some(Node::I32(3)));
    }

    #[test]
    fn an_iron_knuckle_is_one_plain_actor_with_no_links_or_groups() {
        let mut y: Byml = map();
        let p: Place = PLACES[2];
        assert_eq!(place(&mut y, &p, &[], "Test").unwrap(), 1);
        assert_eq!(count(&y, KNIGHT), 1);
        let k: &Node = actors(&y).iter().find(|a| text(a, "Gyaml") == KNIGHT).unwrap();
        assert_eq!(vec3(k, "Translate"), Some(p.at));
        assert!(k.get("Links").is_none() && k.get("Name").is_none());
        assert_eq!(k.get("Dynamic").and_then(|d| d.get("Drop__DropTable")), Some(&Node::Str("None".to_string())));
        assert_eq!(k.get("Dynamic").and_then(|d| d.get(wake::KEY)), Some(&Node::Bool(true)));
        for g in GROUPS {
            assert_eq!(y.root.get(g).unwrap().as_arr().unwrap().len(), 1);
        }
    }

    #[test]
    fn a_like_like_is_one_plain_actor_with_no_drops_and_no_links() {
        let mut y: Byml = map();
        let likes: Vec<Place> = PLACES.iter().filter(|p| p.what == What::LikeLike).copied().collect();
        assert_eq!(likes.len(), 9);
        for p in &likes {
            assert_eq!(place(&mut y, p, &[], "Test").unwrap(), 1);
        }
        assert_eq!(count(&y, LIKE), 9);
        let mut seen: Vec<[f32; 3]> = Vec::new();
        for a in actors(&y).iter().filter(|a| text(a, "Gyaml") == LIKE) {
            assert!(a.get("Links").is_none() && a.get("Name").is_none());
            assert_eq!(a.get("Dynamic").and_then(|d| d.get("Drop__DropTable")), Some(&Node::Str("None".to_string())));
            let t: [f32; 3] = vec3(a, "Translate").unwrap();
            assert!(!seen.contains(&t));
            seen.push(t);
        }
        for g in GROUPS {
            assert_eq!(y.root.get(g).unwrap().as_arr().unwrap().len(), 1);
        }
    }

    #[test]
    fn the_octoroks_wait_for_the_full_lake_in_the_adult_map_only_and_a_scrub_comes_with_its_leaf() {
        let mut child: Byml = map();
        let mut adult: Byml = map();
        for p in PLACES.iter().filter(|p| p.what == What::Octorok) {
            assert_eq!(place(&mut child, p, &[], "HyruleField_Child").unwrap(), 1);
            assert_eq!(place(&mut adult, p, &[], ADULT).unwrap(), 1);
        }
        assert_eq!(count(&child, OCTO), 13);
        assert!(actors(&child).iter().all(|a| a.get("Presence").is_none()));
        let gated: Vec<&Node> = actors(&adult).iter().filter(|a| a.get("Presence").is_some()).collect();
        assert_eq!(gated.len(), 12);
        for a in gated {
            assert_eq!(a.get("Presence").and_then(|p| p.get("FlagName")), Some(&Node::Str(LAKE_FULL.to_string())));
            assert_eq!(a.get("Presence").and_then(|p| p.get("IsNegation")), Some(&Node::Bool(false)));
        }
        let mut y: Byml = map();
        for p in PLACES.iter().filter(|p| p.what == What::MadScrub) {
            assert_eq!(place(&mut y, p, &[], "Test").unwrap(), 2);
        }
        assert_eq!((count(&y, SCRUB), count(&y, LEAF)), (10, 10));
        for l in actors(&y).iter().filter(|a| text(a, "Gyaml") == LEAF) {
            let Some(Node::U64(dst)) = l.get("Links").and_then(|k| k.as_arr()).and_then(|k| k[0].get("Dst")) else {
                panic!("the leaf links to its scrub");
            };
            let body: &Node = actors(&y).iter().find(|a| hash_of(a) == Some(*dst)).unwrap();
            assert_eq!(text(body, "Gyaml"), SCRUB);
            assert_eq!(vec3(body, "Translate"), vec3(l, "Translate"));
        }
    }

    #[test]
    fn ten_beamos_stand_between_the_pillars_and_wait_for_the_first_adult_scenes() {
        let beamos: Vec<Place> = PLACES.iter().filter(|p| p.what == What::Beamos).copied().collect();
        assert_eq!(beamos.len(), 10);
        assert!(beamos.iter().all(|p| p.maps == ["Market_Adult"] && p.dir == "Market"));
        let mut y: Byml = map();
        for p in &beamos {
            assert_eq!(place(&mut y, p, &[], "Market_Adult").unwrap(), 1);
        }
        assert_eq!(count(&y, BEAMOS), 10);
        let mut ids: Vec<u64> = actors(&y).iter().filter_map(hash_of).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 11);
        for a in actors(&y).iter().filter(|a| text(a, "Gyaml") == BEAMOS) {
            assert!(a.get("Links").is_none() && a.get("Name").is_none());
            assert_eq!(a.get("Dynamic").and_then(|d| d.get("Drop__DropTable")), Some(&Node::Str("None".to_string())));
            assert_eq!(a.get("Presence").and_then(|p| p.get("FlagName")), Some(&Node::Str(FIRST_ADULT.to_string())));
            assert_eq!(a.get("Presence").and_then(|p| p.get("IsNegation")), Some(&Node::Bool(false)));
            assert_eq!(text(a, "Gyaml"), "Enemy_Beamos_Temple");
            assert_eq!(vec3(a, "Scale"), Some([BEAMOS_SCALE; 3]));
            let t: [f32; 3] = vec3(a, "Translate").unwrap();
            assert!(t[0] > 80.0 && t[0] < 116.0 && ((t[2] + 43.5).abs() < 0.01 || (t[2] + 24.5).abs() < 0.01));
        }
        for g in GROUPS {
            assert_eq!(y.root.get(g).unwrap().as_arr().unwrap().len(), 1);
        }
    }

    #[test]
    fn the_guays_are_plain_night_flyers_in_the_child_map_only() {
        let guays: Vec<Place> = PLACES.iter().filter(|p| p.what == What::Guay).copied().collect();
        assert_eq!(guays.len(), 24);
        assert!(guays.iter().all(|p| p.maps == ["HyruleField_Child"] && p.dir == "HyruleField"));
        let mut child: Byml = map();
        let mut adult: Byml = map();
        for p in PLACES.iter().filter(|p| p.dir == "HyruleField") {
            for (m, y) in [("HyruleField_Child", &mut child), ("HyruleField_Adult", &mut adult)] {
                if p.maps.contains(&m) {
                    place(y, p, &[], m).unwrap();
                }
            }
        }
        assert_eq!(count(&child, GUAY), 24);
        assert_eq!(count(&adult, GUAY), 0);
        let mut seen: Vec<[f32; 3]> = Vec::new();
        let mut ids: Vec<u64> = Vec::new();
        for a in actors(&child).iter().filter(|a| text(a, "Gyaml") == GUAY) {
            assert!(a.get("Links").is_none() && a.get("Name").is_none());
            assert!(a.get("Dynamic").is_none());
            let t: [f32; 3] = vec3(a, "Translate").unwrap();
            assert!(!seen.contains(&t));
            seen.push(t);
            ids.extend(hash_of(a));
        }
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 24);
        let knight: [f32; 3] = PLACES[2].at;
        for t in &seen {
            assert!(((t[0] - knight[0]).powi(2) + (t[2] - knight[2]).powi(2)).sqrt() > 20.0);
        }
        for g in GROUPS {
            assert_eq!(child.root.get(g).unwrap().as_arr().unwrap().len(), 2);
        }
    }

    #[test]
    fn two_places_in_one_map_do_not_share_a_hash_and_a_broken_map_is_refused() {
        let mut y: Byml = map();
        for p in PLACES.iter().filter(|p| p.dir == "HyruleField") {
            place(&mut y, p, &[], "Test").unwrap();
        }
        let mut hs: Vec<u64> = actors(&y).iter().filter_map(hash_of).collect();
        let n: usize = hs.len();
        hs.sort();
        hs.dedup();
        assert_eq!((n, hs.len()), (72, 72));
        let mut bad: Byml = map();
        bad.root.remove("FarDeleteGroups");
        assert!(dead_hand(&mut bad, &PLACES[0], &[], "Test").is_err());
    }

    #[test]
    fn each_place_is_listed_once_and_the_files_follow_the_places() {
        let names: std::collections::BTreeSet<&str> = PLACES.iter().map(|p| p.name).collect();
        assert_eq!(names.len(), PLACES.len());
        assert_eq!(files().len(), 5);
        assert!(files().iter().all(|f| f.ends_with("_Static.bcett.byml.zs")));
        assert_eq!(files_of(&PLACES[..1]).len(), 2);
    }

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        if p.is_dir() { Some(p) } else { None }
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn the_game_maps_take_each_encounter_alone_and_together() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        for single in 0..=PLACES.len() {
            let chosen: Vec<Place> = if single == PLACES.len() { PLACES.to_vec() } else { vec![PLACES[single]] };
            let outs: Vec<Out> = stage_places(&mut rf, &none, &chosen).unwrap();
            let mut paths: Vec<String> = Vec::new();
            for p in &chosen {
                for m in p.maps {
                    let f: String = file(p.dir, m, "Static");
                    if !paths.contains(&f) {
                        paths.push(f);
                    }
                }
            }
            assert_eq!(outs.iter().map(|o| o.path.clone()).collect::<Vec<String>>(), paths);
            for o in &outs {
                let was: Byml = rf.read_byml(&o.path).unwrap();
                let now: Byml = Byml::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap();
                let here: Vec<&Place> = chosen.iter().filter(|p| p.maps.iter().any(|m| o.path == file(p.dir, m, "Static"))).collect();
                let hands: usize = here.iter().filter(|p| p.what == What::DeadHand).count();
                let knights: usize = here.iter().filter(|p| p.what == What::IronKnuckle).count();
                let likes: usize = here.iter().filter(|p| p.what == What::LikeLike).count();
                let guays: usize = here.iter().filter(|p| p.what == What::Guay).count();
                let octos: usize = here.iter().filter(|p| p.what == What::Octorok).count();
                let scrubs: usize = here.iter().filter(|p| p.what == What::MadScrub).count();
                let beamos: usize = here.iter().filter(|p| p.what == What::Beamos).count();
                assert!(o.new > o.old);
                assert_eq!(count(&now, HAND), count(&was, HAND) + hands, "{}", o.path);
                assert_eq!(count(&now, FINGERS), count(&was, FINGERS) + 4 * hands, "{}", o.path);
                assert_eq!(count(&now, KNIGHT), count(&was, KNIGHT) + knights, "{}", o.path);
                assert_eq!(count(&now, LIKE), count(&was, LIKE) + likes, "{}", o.path);
                assert_eq!(count(&now, GUAY), count(&was, GUAY) + guays, "{}", o.path);
                assert_eq!(count(&now, OCTO), count(&was, OCTO) + octos, "{}", o.path);
                assert_eq!(count(&now, SCRUB), count(&was, SCRUB) + scrubs, "{}", o.path);
                assert_eq!(count(&now, LEAF), count(&was, LEAF) + scrubs, "{}", o.path);
                assert_eq!(count(&now, BEAMOS), count(&was, BEAMOS) + beamos, "{}", o.path);
                assert_eq!(actors(&now).len(), actors(&was).len() + 5 * hands + knights + likes + guays + octos + 2 * scrubs + beamos);
                assert_eq!(&actors(&now)[..actors(&was).len()], actors(&was), "the actors that were there are untouched");
                let mut hs: Vec<u64> = actors(&now).iter().filter_map(hash_of).collect();
                let n: usize = hs.len();
                hs.sort();
                hs.dedup();
                assert_eq!(hs.len(), n, "no hash is used twice in {}", o.path);
                for g in GROUPS {
                    let a: usize = was.root.get(g).and_then(|n| n.as_arr()).map_or(0, |n| n.len());
                    let b: usize = now.root.get(g).and_then(|n| n.as_arr()).map_or(0, |n| n.len());
                    assert_eq!(b, a + hands, "{g} in {}", o.path);
                }
                assert_eq!(now.root.get("AiGroups"), was.root.get("AiGroups"));
                for a in &actors(&now)[actors(&was).len()..] {
                    let t: [f32; 3] = vec3(a, "Translate").unwrap();
                    let near: Vec<(String, f32)> = around(&was, t);
                    if text(a, "Gyaml") == HAND || text(a, "Gyaml") == KNIGHT {
                        for (g, d) in &near {
                            if g.starts_with("Enemy_") || g.starts_with("Obj_TBox") {
                                assert!(*d > 4.0, "{} sits {d} from {g}", text(a, "Gyaml"));
                            }
                        }
                    }
                    if text(a, "Gyaml") == GUAY {
                        for (g, d) in &near {
                            if g.starts_with("Enemy_") || g.starts_with("Obj_TBox") || g.starts_with("Obj_GossipStone") || g.starts_with("Obj_GuidePost") || g.starts_with("Obj_Scarecrow") {
                                assert!(*d >= 4.0, "a Guay sits {d} from {g}");
                            }
                        }
                        assert!(o.path.contains("HyruleField_Child"), "Guays are only in the child map");
                    }
                    if text(a, "Gyaml") == LIKE {
                        for (g, d) in &near {
                            if g.starts_with("Enemy_") || g.starts_with("Obj_TBox") || g.starts_with("Obj_Scarecrow") || g.starts_with("Obj_GossipStone") || g.starts_with("Obj_SignBoard") || g.starts_with("Obj_LakeHrl_WarpPoint") || g.starts_with("Obj_Arrow") {
                                assert!(*d >= 2.0, "a Like Like sits {d} from {g}");
                            }
                        }
                        assert!(t[0] < -80.0 && t[0] > -300.0 && t[2] > 400.0 && t[2] < 590.0 && t[1] > 130.0 && t[1] < 138.0, "a Like Like is inside the lake area");
                    }
                    if text(a, "Gyaml") == OCTO {
                        assert!(t[0] < -100.0 && t[0] > -210.0 && t[2] > 430.0 && t[2] < 495.0 && t[1] == 129.0, "an Octorok is in the lake");
                        assert_eq!(a.get("Presence").is_some(), o.path.contains("_Adult"), "only the adult Octoroks wait for the full lake");
                    }
                    if text(a, "Gyaml") == SCRUB {
                        for (g, d) in &near {
                            if (g.starts_with("Enemy_") && !g.starts_with("Enemy_Scrub_Mad")) || g.contains("Tree") {
                                assert!(*d >= 3.0, "a Mad Scrub sits {d} from {g}");
                            }
                        }
                        assert!(t[0] < -70.0 && t[0] > -165.0 && t[2] > 280.0 && t[2] < 335.0, "a Mad Scrub is in the ranch trees");
                    }
                    if text(a, "Gyaml") == BEAMOS {
                        for (g, d) in &near {
                            if g.starts_with("Enemy_") || g.starts_with("Obj_TBox") || g.starts_with("Npc_") || g.starts_with("Obj_TimeTemple") || g.starts_with("Obj_Pot") {
                                assert!(*d >= 3.0, "a Beamos sits {d} from {g}");
                            }
                        }
                        assert!(o.path.contains("Market_Adult") && a.get("Presence").is_some(), "Beamos are only in the adult Market and wait for the first adult scenes");
                    }
                    if text(a, "Gyaml") == HAND {
                        let landing: &Node = actors(&was)
                            .iter()
                            .filter(|x| text(x, "Name") == "SwapRestartPosAnchor")
                            .min_by(|x, y| {
                                let d = |n: &Node| -> f32 {
                                    let l: [f32; 3] = vec3(n, "Translate").unwrap();
                                    ((l[0] - t[0]).powi(2) + (l[2] - t[2]).powi(2)).sqrt()
                                };
                                d(x).total_cmp(&d(y))
                            })
                            .unwrap();
                        let l: [f32; 3] = vec3(landing, "Translate").unwrap();
                        let d: f32 = ((l[0] - t[0]).powi(2) + (l[2] - t[2]).powi(2)).sqrt();
                        assert!(d > 6.5, "the Dead Hand sits {d} from where Link lands");
                        assert!((t[1] - l[1]).abs() < 0.6, "the Dead Hand is level with the floor");
                    }
                    if text(a, "Gyaml") == HAND || text(a, "Gyaml") == FINGERS {
                        let exit: [f32; 3] = actors(&was)
                            .iter()
                            .filter(|x| text(x, "Name") == "HidingHole_FieldExitGo")
                            .filter_map(|x| vec3(x, "Translate"))
                            .min_by(|x, y| ((x[0] - t[0]).powi(2) + (x[2] - t[2]).powi(2)).total_cmp(&((y[0] - t[0]).powi(2) + (y[2] - t[2]).powi(2))))
                            .unwrap();
                        let far: f32 = ((exit[0] - t[0]).powi(2) + (exit[2] - t[2]).powi(2)).sqrt();
                        assert!(far >= 6.0, "{} sits {far} from where Link lands", text(a, "Gyaml"));
                    }
                }
            }
        }
    }
}
