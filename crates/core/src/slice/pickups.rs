use std::collections::BTreeMap as BM;

use crate::byml::{Byml, Node};
use crate::catalog::{Loc, Via};
use crate::error::{Error as E, Res};
use crate::logic_items::ItemId;
use crate::pickup_pack::default_name;
use crate::pickup_persist::Extra;
use crate::placement::hash_of;
use crate::settings::Settings;
use crate::slice::data::Data;
use crate::slice::edit::Target;
use crate::slice::key_spots as ks;
use crate::slice::support::{Mode, Plan};
use crate::world::{LocId, World};

pub const SETTING: &str = "shuffle_freestanding_items";

pub const HELD: [(&str, bool); 3] = [
    ("HF Tektite Grotto Freestanding PoH", false),
    ("GV Crate Freestanding PoH", false),
    ("Graveyard Dampe Race Freestanding PoH", true),
];

pub const NOTE: &str = "Freestanding items are shuffled. Heart pieces, rupees and recovery hearts on the ground swap their actor for another world pickup, and three heart piece chests join the chest pool. Rupees, recovery hearts, heart pieces, the two shields and every item with a built pickup pack can lie on the ground. Experimental and untested in the game.";

pub const WORLD_ONLY: [&str; 1] = ["Recovery Heart"];

pub const UNSURE: [&str; 2] = ["HF Tektite Grotto Freestanding PoH", "GV Crate Freestanding PoH"];

const ACTORS: [(&str, &str); 6] = [
    ("Piece of Heart", "Obj_Heart_Piece"),
    ("Recovery Heart", "Obj_Heart"),
    ("Rupee (1)", "Obj_Rupee_Green"),
    ("Rupees (5)", "Obj_Rupee_Blue"),
    ("Rupees (20)", "Obj_Rupee_Red"),
    ("Rupees (50)", "Obj_Rupee_Purple"),
];

pub fn world_actor(item: &str) -> Option<&'static str> {
    ACTORS.iter().find(|a| a.0 == item).map(|a| a.1)
}

pub fn is_world(l: &Loc) -> bool {
    l.via == Via::Actor && matches!(l.kind.as_str(), "heart" | "freestanding")
}

pub fn held(name: &str) -> Option<bool> {
    HELD.iter().find(|h| h.0 == name).map(|h| h.1)
}

pub fn is_held(name: &str) -> bool {
    held(name).is_some()
}

pub fn listed(d: &Data, name: &str) -> bool {
    is_held(name) || d.cat.by_name(name).is_some_and(is_world)
}

pub fn keeps_out(d: &Data, name: &str, item: &str) -> bool {
    UNSURE.contains(&name) && d.vanilla_of(name) != Some(item) && d.item(item).is_some_and(|i| i.adv)
}

pub fn mode(s: &Settings, bad: &mut Vec<String>) -> Mode {
    match s.text(SETTING) {
        Ok("off") => Mode::Off,
        Ok("overworld") => Mode::Overworld,
        Ok("dungeons") => Mode::Dungeons,
        Ok("all") => Mode::All,
        Ok(v) => {
            bad.push(format!("{SETTING} is {v}, which is not a known choice"));
            Mode::Off
        }
        Err(e) => {
            bad.push(e.to_string());
            Mode::Off
        }
    }
}

pub fn pool_actor(d: &Data, name: &str) -> Option<String> {
    let l: &Loc = d.cat.by_name(name)?;
    if !is_world(l) {
        return None;
    }
    d.vanilla_of(name).and_then(|v| d.item(v)).and_then(|i| i.actor.clone())
}

pub fn swap_gyaml(y: &mut Byml, hash: u64, old: &str, new: &str) -> usize {
    let Some(list) = y.root.get_mut("Actors").and_then(|a| a.as_arr_mut()) else {
        return 0;
    };
    let mut n: usize = 0;
    for a in list.iter_mut() {
        if hash_of(a) != Some(hash) || a.get("Gyaml").and_then(|v| v.as_str()) != Some(old) {
            continue;
        }
        a.set("Gyaml", Node::Str(new.to_string()));
        n += 1;
    }
    n
}

pub fn pack_actor(d: &Data, extra: &Extra, item: &str, tier: Option<&str>) -> Option<String> {
    let i: &crate::slice::data::Item = d.item(item)?;
    let content: &str = match tier {
        Some(t) => t,
        None if i.tiers.is_empty() => i.actor.as_deref()?,
        None => return None,
    };
    let name: String = ks::pick(content).map(|p| p.pack).unwrap_or_else(|| default_name(content));
    extra.has(&name).then_some(name)
}

pub fn targets(d: &Data, l: &Loc, item: &str) -> Res<Vec<Target>> {
    targets_with(d, l, item, &Extra::default(), None)
}

pub fn targets_with(d: &Data, l: &Loc, item: &str, extra: &Extra, tier: Option<&str>) -> Res<Vec<Target>> {
    let vanilla: &str = d.vanilla_of(&l.name).ok_or_else(|| E::Msg(format!("{} has no vanilla item", l.name)))?;
    if item == vanilla {
        return Ok(Vec::new());
    }
    let new: String = match world_actor(item).or_else(|| ks::shield(item)) {
        Some(a) => a.to_string(),
        None => pack_actor(d, extra, item, tier).ok_or_else(|| E::Msg(format!("{item} has no world pickup and cannot be placed at {}", l.name)))?,
    };
    let mut out: Vec<Target> = Vec::new();
    for r in &l.refs {
        out.push(Target {
            name: l.name.clone(),
            hash: r.hash,
            files: r.files.clone(),
            old_item: vanilla.to_string(),
            new_item: item.to_string(),
            old_actor: l.item.clone(),
            new_actor: new.to_string(),
            ev: None,
            family: None,
            key: None,
        });
    }
    Ok(out)
}

fn key_of(w: &World, name: &str) -> Option<(usize, String)> {
    let l: LocId = w.loc(name)?;
    Some((w.locs[l].region, format!("{:?}", w.locs[l].rule.node)))
}

fn pool_ready(d: &Data, w: &World, l: &Loc) -> bool {
    let Some(pool) = l.pool.as_deref() else {
        return true;
    };
    let members: Vec<&Loc> = d.cat.locs.iter().filter(|x| x.pool.as_deref() == Some(pool)).collect();
    let first: Option<(usize, String)> = key_of(w, &l.name);
    first.is_some() && members.iter().all(|m| key_of(w, &m.name) == first)
}

fn count(left: &mut BM<String, usize>, why: &str) {
    *left.entry(why.to_string()).or_insert(0) += 1;
}

fn mode_ok(m: Mode, dungeon: bool) -> bool {
    match m {
        Mode::Off => false,
        Mode::Overworld => !dungeon,
        Mode::Dungeons => dungeon,
        Mode::All => true,
    }
}

pub fn select(d: &Data, w: &World, p: &Plan, free: &mut Vec<LocId>, left: &mut BM<String, usize>, flow: &mut usize) {
    if p.world == Mode::Off {
        return;
    }
    let mut names: Vec<&Loc> = d.cat.locs.iter().filter(|l| is_world(l) || is_held(&l.name)).collect();
    names.sort_by(|a, b| a.name.cmp(&b.name));
    for l in names {
        *flow += 1;
        let Some(x) = w.loc(&l.name) else {
            count(left, "location is not in the logic world");
            continue;
        };
        let dungeon: bool = w.regions[w.locs[x].region].dungeon.is_some();
        if p.excluded.iter().any(|e| *e == l.name) {
            count(left, "excluded by the player");
        } else if !mode_ok(p.world, dungeon) {
            count(left, "freestanding item shuffle does not cover its area");
        } else if held(&l.name) == Some(true) && !d.event_ok(&l.name) {
            count(left, "chest runs a story event that can grant its own item");
        } else if !pool_ready(d, w, l) {
            count(left, "match is a guess inside a pool");
        } else {
            free.push(x);
        }
    }
}

pub fn at(d: &Data, name: &str, item: &str) -> Option<bool> {
    if d.cat.by_name(name).is_some_and(is_world) {
        let known: bool = world_actor(item).is_some() || ks::shield(item).is_some() || ks::placeable(d, item, ks::Spot::Free);
        return Some(known && crate::slice::copies::fits(d, name, item));
    }
    None
}

pub fn accepts(d: &Data, w: &World, it: ItemId, l: LocId) -> Option<bool> {
    let name: &str = &w.locs[l].name;
    let item: &str = &w.items.names[it];
    if let Some(ok) = at(d, name, item) {
        return Some(ok);
    }
    if WORLD_ONLY.contains(&item) {
        return Some(false);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Value;
    use crate::slice::generate::{generate_data, GenOpts, Generated};
    use crate::slice::logic::world;
    use crate::slice::support::plan;

    fn on(v: &str) -> Settings {
        let mut s: Settings = Settings::default();
        s.set(SETTING, Value::s(v)).unwrap();
        s
    }

    fn actor(g: &str, h: u64) -> Node {
        Node::Map(vec![("Gyaml".to_string(), Node::Str(g.to_string())), ("Hash".to_string(), Node::U64(h))])
    }

    #[test]
    fn only_the_named_actor_with_the_expected_name_is_swapped() {
        let mut y: Byml = Byml {
            root: Node::Map(vec![("Actors".to_string(), Node::Arr(vec![actor("Obj_Heart_Piece", 1), actor("Obj_Heart_Piece", 2), actor("Obj_TBox_A_02", 3)]))]),
            version: 2,
            big: false,
        };
        assert_eq!(swap_gyaml(&mut y, 1, "Obj_Heart_Piece", "Obj_Rupee_Blue"), 1);
        assert_eq!(swap_gyaml(&mut y, 3, "Obj_Heart_Piece", "Obj_Rupee_Blue"), 0);
        assert_eq!(swap_gyaml(&mut y, 9, "Obj_Heart_Piece", "Obj_Rupee_Blue"), 0);
        let a: &[Node] = y.root.get("Actors").unwrap().as_arr().unwrap();
        assert_eq!(a[0].get("Gyaml").and_then(|v| v.as_str()), Some("Obj_Rupee_Blue"));
        assert_eq!(a[1].get("Gyaml").and_then(|v| v.as_str()), Some("Obj_Heart_Piece"));
        assert_eq!(a[2].get("Gyaml").and_then(|v| v.as_str()), Some("Obj_TBox_A_02"));
    }

    #[test]
    fn the_world_table_only_names_known_items_and_pickups_that_ship() {
        let d: Data = Data::load().unwrap();
        for (item, a) in ACTORS.iter() {
            assert!(d.item(item).is_some(), "{item}");
            assert!(a.starts_with("Obj_"), "{a}");
        }
        assert_eq!(world_actor("Rupees (5)"), Some("Obj_Rupee_Blue"));
        assert_eq!(world_actor("Bomb Bag"), None);
        assert_eq!(world_actor("Piece of Heart (Out of Logic)"), None);
    }

    #[test]
    fn every_world_location_is_in_the_logic_with_a_pickup_for_its_vanilla_item() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let mut n: usize = 0;
        for l in d.cat.locs.iter().filter(|l| is_world(l)) {
            n += 1;
            assert!(w.loc(&l.name).is_some(), "{}", l.name);
            let v: &str = d.vanilla_of(&l.name).unwrap();
            assert!(world_actor(v).is_some(), "{} holds {v}", l.name);
            assert!(!l.refs.is_empty() && l.refs.iter().all(|r| !r.files.is_empty()), "{}", l.name);
        }
        assert_eq!(n, 14 + 7 + 8 + 1);
        for (name, _) in HELD {
            let l: &Loc = d.cat.by_name(name).unwrap();
            assert_eq!((l.kind.as_str(), l.via), ("chest", Via::Drop), "{name}");
            assert!(w.loc(name).is_some(), "{name}");
        }
    }

    #[test]
    fn a_swapped_location_lists_every_hash_of_its_spot() {
        let d: Data = Data::load().unwrap();
        let kf: &Loc = d.cat.by_name("KF Sarias House Recovery Heart 1").unwrap();
        let t: Vec<Target> = targets(&d, kf, "Rupees (50)").unwrap();
        assert_eq!(t.len(), 2);
        assert!(t.iter().all(|x| x.old_actor == "Obj_Heart" && x.new_actor == "Obj_Rupee_Purple"));
        assert!(targets(&d, kf, "Recovery Heart").unwrap().is_empty());
        assert!(targets(&d, kf, "Bomb Bag").is_err());
        let gv: &Loc = d.cat.by_name("GV Waterfall Freestanding PoH").unwrap();
        assert!(targets(&d, gv, "Piece of Heart").unwrap().is_empty());
        assert_eq!(pool_actor(&d, "GV Waterfall Freestanding PoH").as_deref(), Some("Obj_Heart_Piece"));
        assert_eq!(pool_actor(&d, "Dampe Race"), None);
    }

    #[test]
    fn nothing_changes_while_the_setting_is_off() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let p: Plan = plan(&Settings::default()).unwrap();
        let mut free: Vec<LocId> = Vec::new();
        let mut left: BM<String, usize> = BM::new();
        let mut flow: usize = 0;
        select(&d, &w, &p, &mut free, &mut left, &mut flow);
        assert!(free.is_empty() && left.is_empty() && flow == 0);
        assert!(!p.notes.iter().any(|n| n == NOTE));
    }

    #[test]
    fn the_setting_opens_the_checks_by_area_and_pool() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let count_of = |v: &str| {
            let p: Plan = plan(&on(v)).unwrap();
            let mut free: Vec<LocId> = Vec::new();
            let mut left: BM<String, usize> = BM::new();
            let mut flow: usize = 0;
            select(&d, &w, &p, &mut free, &mut left, &mut flow);
            (free.len(), flow)
        };
        let (all, seen): (usize, usize) = count_of("all");
        assert_eq!(seen, 33);
        assert!(all >= 30, "{all}");
        assert_eq!(count_of("overworld").0, all);
        assert_eq!(count_of("dungeons").0, 0);
    }

    #[test]
    fn world_locations_only_take_shipped_pickups_and_recovery_hearts_stay_off_chests() {
        let d: Data = Data::load().unwrap();
        let s: Settings = on("all");
        let p: Plan = plan(&s).unwrap();
        let w: World = world(&d).unwrap();
        let mut hearts: usize = 0;
        for n in 0..5 {
            let g: Generated = generate_data(&d, &s, &format!("world {n}"), &GenOpts::default()).unwrap();
            for (loc, item) in &g.seed_file.placements {
                let l: LocId = w.loc(loc).unwrap();
                assert!(crate::slice::generate::allowed(&d, &w, &p, w.item(item).unwrap(), l), "{item} at {loc}");
                let world_loc: bool = d.cat.by_name(loc).is_some_and(is_world);
                if world_loc {
                    let ok: bool = world_actor(item).is_some() || ks::shield(item).is_some() || ks::placeable(&d, item, ks::Spot::Free);
                    assert!(ok, "{loc} holds {item}");
                }
                if item == "Recovery Heart" {
                    assert!(world_loc, "{loc} holds a recovery heart");
                    hearts += 1;
                }
            }
            assert!(g.seed_file.placements.keys().any(|k| d.cat.by_name(k).is_some_and(is_world)));
        }
        assert!(hearts > 0);
    }

    #[test]
    fn the_default_seed_never_touches_a_world_location() {
        let d: Data = Data::load().unwrap();
        for n in 0..3 {
            let g: Generated = generate_data(&d, &Settings::default(), &format!("plain {n}"), &GenOpts::default()).unwrap();
            for k in g.seed_file.placements.keys() {
                assert!(!d.cat.by_name(k).is_some_and(is_world), "{k}");
                assert!(!is_held(k), "{k}");
            }
        }
    }

    #[test]
    fn the_file_editor_swaps_world_actors_and_still_sets_chest_drops() {
        use crate::slice::edit::edit_file;
        let chest: Node = Node::Map(vec![
            ("Dynamic".to_string(), Node::Map(vec![("Drop__DropActor".to_string(), Node::Str("Obj_Heart_Piece".to_string()))])),
            ("Gyaml".to_string(), Node::Str("Obj_TBox_A_02".to_string())),
            ("Hash".to_string(), Node::U64(2)),
        ]);
        let y: Byml = Byml { root: Node::Map(vec![("Actors".to_string(), Node::Arr(vec![actor("Obj_Heart_Piece", 1), chest]))]), version: 7, big: false };
        let raw: Vec<u8> = y.write().unwrap();
        let mk = |h: u64, old: &str, new: &str| Target {
            name: "L".to_string(),
            hash: h,
            files: vec!["Banc/Area/a.bcett.byml.zs".to_string()],
            old_item: "Piece of Heart".to_string(),
            new_item: "Rupee (1)".to_string(),
            old_actor: old.to_string(),
            new_actor: new.to_string(),
            ev: None,
            family: None,
            key: None,
        };
        let ts: Vec<Target> = vec![mk(1, "Obj_Heart_Piece", "Obj_Rupee_Green"), mk(2, "Obj_Heart_Piece", "TBox_Rupee_Green")];
        let e: crate::slice::edit::Edited = edit_file("Banc/Area/a.bcett.byml.zs", &raw, &ts).unwrap().unwrap();
        assert_eq!(e.hits.len(), 2);
        let back: Byml = Byml::parse(&e.bytes).unwrap();
        let a: &[Node] = back.root.get("Actors").unwrap().as_arr().unwrap();
        assert_eq!(a[0].get("Gyaml").and_then(|v| v.as_str()), Some("Obj_Rupee_Green"));
        assert_eq!(a[1].get("Gyaml").and_then(|v| v.as_str()), Some("Obj_TBox_A_02"));
        assert_eq!(a[1].get("Dynamic").and_then(|d| d.get("Drop__DropActor")).and_then(|v| v.as_str()), Some("TBox_Rupee_Green"));
    }

    #[test]
    fn the_two_unsure_heart_chests_never_hold_progression_and_the_note_follows_the_pool() {
        let d: Data = Data::load().unwrap();
        for n in 0..4 {
            let g: Generated = generate_data(&d, &on("all"), &format!("unsure {n}"), &GenOpts::default()).unwrap();
            for name in UNSURE {
                let it: &str = &g.seed_file.placements[name];
                assert!(d.vanilla_of(name) == Some(it) || !d.item(it).unwrap().adv, "{name} holds {it}");
            }
            assert!(g.report.contains(NOTE));
        }
        let g: Generated = generate_data(&d, &on("dungeons"), "unsure dungeons", &GenOpts::default()).unwrap();
        let any: bool = g.seed_file.placements.keys().any(|k| listed(&d, k));
        assert_eq!(g.report.contains(NOTE), any);
    }
}
