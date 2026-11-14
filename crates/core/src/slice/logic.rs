use std::collections::HashSet as HS;

use crate::error::{Error as E, Res};
use crate::logic_json::{parse, Json};
use crate::logic_val::Settings;
use crate::slice::data::{Data, SEP};
use crate::world::{Spec, World};

const SHIMS: &str = include_str!("../../../../data/logic/shims.json");
const SETTINGS: &str = include_str!("../../../../data/logic/settings.json");
const HELPERS: &str = include_str!("../../../../data/logic/LogicHelpers.json");

const TOWER: [&str; 3] = ["Ganons Castle Tower", "Ganons Castle Tower Below Boss", "Ganondorf Boss Room"];

const WORLD: [&str; 14] = [
    include_str!("../../../../data/logic/World/Overworld.json"),
    include_str!("../../../../data/logic/World/Bosses.json"),
    include_str!("../../../../data/logic/World/Deku Tree.json"),
    include_str!("../../../../data/logic/World/Dodongos Cavern.json"),
    include_str!("../../../../data/logic/World/Jabu Jabus Belly.json"),
    include_str!("../../../../data/logic/World/Forest Temple.json"),
    include_str!("../../../../data/logic/World/Fire Temple.json"),
    include_str!("../../../../data/logic/World/Water Temple.json"),
    include_str!("../../../../data/logic/World/Shadow Temple.json"),
    include_str!("../../../../data/logic/World/Spirit Temple.json"),
    include_str!("../../../../data/logic/World/Bottom of the Well.json"),
    include_str!("../../../../data/logic/World/Ice Cavern.json"),
    include_str!("../../../../data/logic/World/Gerudo Training Ground.json"),
    include_str!("../../../../data/logic/World/Ganons Castle.json"),
];

fn unique(files: Vec<Json>) -> Vec<Json> {
    let mut seen: HS<String> = HS::new();
    let mut out: Vec<Json> = Vec::new();
    for f in files {
        let Json::Arr(regions) = f else {
            out.push(f);
            continue;
        };
        let mut rs: Vec<Json> = Vec::with_capacity(regions.len());
        for r in regions {
            let Json::Obj(fields) = r else {
                rs.push(r);
                continue;
            };
            let name: String = fields
                .iter()
                .find(|(k, _)| k == "region_name")
                .and_then(|(_, v)| v.as_str())
                .unwrap_or("")
                .to_string();
            let mut nf: Vec<(String, Json)> = Vec::with_capacity(fields.len());
            for (k, v) in fields {
                if k == "locations"
                    && let Json::Obj(locs) = v
                {
                    let mut nl: Vec<(String, Json)> = Vec::with_capacity(locs.len());
                    for (ln, lr) in locs {
                        let full: String = if seen.contains(&ln) { format!("{ln}{SEP}{name}") } else { ln.clone() };
                        seen.insert(ln);
                        nl.push((full, lr));
                    }
                    nf.push((k, Json::Obj(nl)));
                } else {
                    nf.push((k, v));
                }
            }
            rs.push(Json::Obj(nf));
        }
        out.push(Json::Arr(rs));
    }
    out
}

pub fn spec(d: &Data, over: &[(&str, &str)]) -> Res<Spec> {
    let mut s: Spec = Spec::from_bundle(SHIMS)?;
    s.settings = Settings::from_json(&parse(SETTINGS)?)?;
    for (k, v) in over {
        match v.parse::<i64>() {
            Ok(n) => s.settings.set_int(k, n),
            Err(_) => s.settings.set_str(k, v),
        }
    }
    for it in &d.items {
        s.add_item(&it.name, it.adv, 0);
    }
    s.add_helpers(HELPERS)?;
    let mut files: Vec<Json> = Vec::with_capacity(WORLD.len());
    for t in WORLD {
        files.push(parse(t)?);
    }
    if crate::slice::skull_rules::wanted(over) {
        crate::slice::skull_rules::apply(&mut files)?;
    }
    s.regions.extend(unique(files));
    Ok(s)
}

pub fn world(d: &Data) -> Res<World> {
    world_with(d, &[])
}

pub fn world_with(d: &Data, over: &[(&str, &str)]) -> Res<World> {
    let s: Spec = spec(d, over)?;
    let mut w: World = World::build(&s).map_err(|e| E::Msg(format!("logic world: {e}")))?;
    for r in w.regions.iter_mut() {
        if r.dungeon.is_none() && TOWER.contains(&r.name.as_str()) {
            r.dungeon = Some("Ganons Castle".to_string());
        }
    }
    Ok(w)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic_items::ItemId;
    use crate::rules::{eval, Age, Ctx};
    use crate::search::{explore, Search};
    use crate::slice::fill::vanilla_place;
    use crate::world::{Exit, Placement};

    #[test]
    fn the_vanilla_world_builds_strictly() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        assert!(w.real.len() > 1000);
        assert!(w.loc("Deku Tree Map Chest").is_some());
        assert!(w.loc("Ganon").is_some());
    }

    fn vanilla_without(w: &World, d: &Data, gone: &[&str]) -> Placement {
        let mut p: Placement = vanilla_place(w, d).unwrap();
        let g: Vec<ItemId> = gone.iter().map(|n| w.item(n).unwrap()).collect();
        let f: ItemId = w.item("Rupees (5)").unwrap();
        for l in &w.real {
            if p.get(*l).is_some_and(|i| g.contains(&i)) {
                p.put(*l, f);
            }
        }
        p
    }

    fn gauntlets(w: &World) -> Vec<ItemId> {
        vec![w.item("Progressive Strength Upgrade").unwrap(); 3]
    }

    fn seen(s: &Search, w: &World, name: &str) -> bool {
        s.loc_access(w.loc(name).unwrap())
    }

    fn exit_open(s: &Search, w: &World, name: &str, a: Age) -> bool {
        let x: &Exit = w.exits.iter().find(|x| x.name == name).unwrap();
        let c: Ctx = Ctx { cnt: &s.cnt, age: Some(a), tod: 0, region: x.from, q: Some(s) };
        x.rule.always || (!x.rule.never && eval(&x.rule.node, &c))
    }

    #[test]
    fn an_open_door_of_time_reaches_the_master_sword_without_the_song_of_time() {
        let d: Data = Data::load().unwrap();
        let closed: World = world(&d).unwrap();
        let s: Search = explore(&closed, &vanilla_without(&closed, &d, &["Song of Time"]), &gauntlets(&closed));
        assert!(!seen(&s, &closed, "Master Sword Pedestal"));
        let open: World = world_with(&d, &[("open_door_of_time", "open")]).unwrap();
        let s: Search = explore(&open, &vanilla_without(&open, &d, &["Song of Time"]), &gauntlets(&open));
        assert!(seen(&s, &open, "Master Sword Pedestal"));
    }

    #[test]
    fn every_audit_gate_can_be_satisfied_in_the_vanilla_world() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let s: Search = explore(&w, &vanilla_place(&w, &d).unwrap(), &gauntlets(&w));
        for n in [
            "Water Temple Gate Open from Ice Cavern End",
            "Light Arrows Scene Done from Temple of Time",
            "Defeat Bongo Bongo from Bongo Bongo Boss Room",
            "Defeat Twinrova from Twinrova Boss Room",
            "Defeat Phantom Ganon from Phantom Ganon Boss Room",
            "ToT Light Arrows Cutscene",
            "Sheik at Temple",
            "Colossus Great Fairy Reward",
            "OGC Great Fairy Reward",
            "Big Poe Kill",
            "LH Sun",
            "Market Lost Dog",
            "HF Ocarina of Time Item",
            "Song from Ocarina of Time",
            "Water Temple Map Chest",
        ] {
            assert!(seen(&s, &w, n), "{n}");
        }
        for r in ["Water Temple Lobby", "Market Bombchu Shop", "Ganons Castle Grounds"] {
            assert!(s.region_reached(w.region(r).unwrap(), None), "{r}");
        }
        assert!(exit_open(&s, &w, "ZR Front -> Zora River", Age::Adult));
        assert!(exit_open(&s, &w, "ZR Front -> Zora River", Age::Child));
        assert!(s.won());
    }

    #[test]
    fn open_kakariko_opens_the_death_mountain_gate_for_a_child_without_the_letter() {
        let d: Data = Data::load().unwrap();
        let mut s: crate::settings::Settings = crate::settings::Settings::default();
        let shut: World = world_with(&d, &crate::slice::support::plan(&s).unwrap().logic()).unwrap();
        s.set("open_forest", crate::settings::Value::s("closed_deku")).unwrap();
        s.set("open_kakariko", crate::settings::Value::s("open")).unwrap();
        let open: World = world_with(&d, &crate::slice::support::plan(&s).unwrap().logic()).unwrap();
        let gate: &str = "Kakariko Village -> Kak Behind Gate";
        let a: Search = explore(&shut, &vanilla_without(&shut, &d, &["Zeldas Letter"]), &[]);
        let b: Search = explore(&open, &vanilla_without(&open, &d, &["Zeldas Letter"]), &[]);
        assert!(!exit_open(&a, &shut, gate, Age::Child));
        assert!(exit_open(&b, &open, gate, Age::Child));
        assert!(exit_open(&a, &shut, gate, Age::Adult));
        let c: Search = explore(&shut, &vanilla_place(&shut, &d).unwrap(), &[]);
        assert!(exit_open(&c, &shut, gate, Age::Child));
    }

    #[test]
    fn the_wallet_and_hookshot_gates_bind() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let s: Search = explore(&w, &vanilla_without(&w, &d, &["Progressive Wallet"]), &gauntlets(&w));
        assert!(!seen(&s, &w, "Colossus Great Fairy Reward"));
        assert!(!seen(&s, &w, "OGC Great Fairy Reward"));
        let s: Search = explore(&w, &vanilla_without(&w, &d, &["Progressive Hookshot"]), &[]);
        assert!(!seen(&s, &w, "Big Poe Kill"));
        assert!(!s.region_reached(w.region("Water Temple Lobby").unwrap(), None));
    }

    #[test]
    fn the_water_temple_gate_needs_the_ice_cavern() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let boots: ItemId = w.item("Iron Boots").unwrap();
        let lobby: usize = w.region("Water Temple Lobby").unwrap();
        let cut: Placement = vanilla_without(&w, &d, &["Silver Rupee (Ice Cavern Push Block)", "Silver Rupee Pouch (Ice Cavern Push Block)"]);
        let s: Search = explore(&w, &cut, &[boots]);
        assert!(!seen(&s, &w, "Water Temple Gate Open from Ice Cavern End"));
        assert!(!s.region_reached(lobby, None));
        let s: Search = explore(&w, &vanilla_place(&w, &d).unwrap(), &[boots]);
        assert!(seen(&s, &w, "Water Temple Gate Open from Ice Cavern End"));
        assert!(s.region_reached(lobby, None));
    }

    const REAL_KEYS: [(&str, &str, usize); 3] = [
        ("Small Key (Water Temple)", "Water Temple", 4),
        ("Small Key (Bottom of the Well)", "Bottom of the Well", 2),
        ("Small Key (Ganons Castle)", "Ganons Castle", 1),
    ];

    fn count_in(w: &World, p: &Placement, item: &str) -> usize {
        let i: ItemId = w.item(item).unwrap();
        w.real.iter().filter(|l| p.get(**l) == Some(i)).count()
    }

    fn all_but(w: &World, skip: &str) -> Vec<ItemId> {
        let mut out: Vec<ItemId> = Vec::new();
        for (i, n) in w.items.names.iter().enumerate() {
            if w.items.adv[i] && !n.starts_with(skip) {
                out.extend([i; 3]);
            }
        }
        out
    }

    #[test]
    fn the_remake_key_counts_are_truthful() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let p: Placement = vanilla_place(&w, &d).unwrap();
        for (k, _, n) in REAL_KEYS {
            assert_eq!(count_in(&w, &p, k), n, "{k}");
        }
        for gone in ["Water Temple Cracked Wall Chest", "Water Temple River Chest"] {
            assert!(w.loc(gone).is_none(), "{gone}");
        }
        for k in ["Water Temple Torches Chest", "Water Temple Compass Room Chest", "Water Temple Dragon Chest", "Water Temple Central Bow Target Chest"] {
            assert_eq!(d.vanilla_of(k), Some("Small Key (Water Temple)"), "{k}");
        }
    }

    #[test]
    fn the_real_keys_open_every_dungeon_location_under_both_layouts() {
        let d: Data = Data::load().unwrap();
        for lay in ["vanilla", "shuffled"] {
            let w: World = world_with(&d, &[("water_key_layout", lay)]).unwrap();
            let p: Placement = vanilla_place(&w, &d).unwrap();
            for (k, dg, _) in REAL_KEYS {
                let s: Search = explore(&w, &p, &all_but(&w, k));
                for l in w.real.iter().filter(|l| w.regions[w.locs[**l].region].dungeon.as_deref() == Some(dg)) {
                    assert!(s.loc_access(*l), "{lay} {}", w.locs[*l].name);
                }
            }
        }
    }

    #[test]
    fn three_water_keys_never_open_the_strict_water_doors() {
        let d: Data = Data::load().unwrap();
        let w: World = world_with(&d, &[("water_key_layout", "shuffled")]).unwrap();
        let k: ItemId = w.item("Small Key (Water Temple)").unwrap();
        let mut p: Placement = vanilla_place(&w, &d).unwrap();
        for l in &w.real {
            if p.get(*l) == Some(k) {
                p.item[*l] = None;
            }
        }
        let mut extra: Vec<ItemId> = all_but(&w, "Small Key (Water Temple)");
        extra.extend([k; 3]);
        let s: Search = explore(&w, &p, &extra);
        assert!(!s.region_reached(w.region("Water Temple Falling Platform Room").unwrap(), None));
        assert!(!s.region_reached(w.region("Water Temple North Basement").unwrap(), None));
        extra.push(k);
        let s: Search = explore(&w, &p, &extra);
        assert!(s.region_reached(w.region("Water Temple Dark Link Region").unwrap(), None));
        assert!(s.region_reached(w.region("Water Temple Boss Key Chest Room").unwrap(), None));
    }

    fn final_count(w: &World, d: &Data, placed: &std::collections::BTreeMap<String, String>, item: &str) -> usize {
        let v: Placement = vanilla_place(w, d).unwrap();
        let i: ItemId = w.item(item).unwrap();
        let kept: usize = w.real.iter().filter(|l| v.get(**l) == Some(i) && !placed.contains_key(&w.locs[**l].name)).count();
        kept + placed.values().filter(|x| x.as_str() == item).count()
    }

    #[test]
    fn vanilla_layout_and_roaming_key_seeds_hold_exactly_the_real_keys() {
        let d: Data = Data::load().unwrap();
        for (mode, remake) in [("dungeon", "off"), ("dungeon", "on"), ("keysanity", "off"), ("any_dungeon", "on"), ("keysanity", "on")] {
            let mut s: crate::settings::Settings = crate::settings::Settings::default();
            s.set("shuffle_smallkeys", crate::settings::Value::s(mode)).unwrap();
            s.set(crate::slice::remake_chests::SETTING, crate::settings::Value::s(remake)).unwrap();
            let g: crate::slice::Generated = crate::slice::generate::generate_data(&d, &s, "keys1", &crate::slice::GenOpts::default()).unwrap();
            let w: World = world_with(&d, &crate::slice::support::plan(&s).unwrap().logic()).unwrap();
            let placed: std::collections::BTreeMap<String, String> = g.seed_file.placements.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            for (k, _, n) in REAL_KEYS {
                assert_eq!(final_count(&w, &d, &placed, k), n, "{mode} {remake} {k}");
            }
        }
    }

    #[test]
    fn the_zora_river_wall_needs_a_blast_or_a_smash() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let s: Search = explore(&w, &vanilla_without(&w, &d, &["Bomb Bag"]), &[]);
        let h: ItemId = w.item("Megaton Hammer").unwrap();
        assert_eq!(exit_open(&s, &w, "ZR Front -> Zora River", Age::Adult), s.cnt[h] > 0);
        assert!(!exit_open(&s, &w, "ZR Front -> Zora River", Age::Child));
    }
}
