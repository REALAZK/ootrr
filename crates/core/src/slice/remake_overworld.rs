use crate::slice::data::Data;

pub const SEED: &str = "Business Scrub Seed";

pub const NEW: [&str; 18] = [
    "DMC Hammer Grotto Chest",
    "Colossus Grotto Beamos Chest",
    "GF Hideout Hole Chest",
    "GV Storms Grotto Chest",
    "GV Storms Grotto Lens Chest",
    "GV Octorok Grotto Chest",
    "GC Maze Silver Chest",
    "Graveyard Dampe Race Low Rupee Chest",
    "Graveyard Dampe Race High Rupee Chest",
    "HF Lake Poe Chest",
    "HF South Poe Chest",
    "HF East Poe Chest",
    "HF Tunnel Ledge Chest",
    "HF Redead Grotto Chest",
    "Kak Windmill Roof Chest",
    "KF West Ledge Chest",
    "SFM Time Block Grotto Chest",
    "ZF Scarecrow Ledge Chest",
];

pub const PAIRED: [&str; 10] = [
    "DMC Upper Grotto Chest",
    "DMT Chest",
    "GV Chest",
    "GC Maze Left Chest",
    "GC Maze Center Chest",
    "GC Maze Right Chest",
    "HF Open Grotto Chest",
    "Kak Open Grotto Chest",
    "KF Storms Grotto Chest",
    "SFM Wolfos Grotto Chest",
];

pub const SEEDED: [&str; 3] = ["ZR Open Grotto Chest", "Wasteland Chest", "Ganons Castle Spirit Trial Crystal Switch Chest"];

pub const UNSURE: [&str; 7] = [
    "Graveyard Dampe Race Low Rupee Chest",
    "Graveyard Dampe Race High Rupee Chest",
    "HF South Poe Chest",
    "HF East Poe Chest",
    "Kak Windmill Roof Chest",
    "KF West Ledge Chest",
    "ZF Scarecrow Ledge Chest",
];

pub fn gated(name: &str) -> bool {
    NEW.contains(&name) || PAIRED.contains(&name) || SEEDED.contains(&name)
}

pub fn keeps_out(d: &Data, name: &str, item: &str) -> bool {
    UNSURE.contains(&name) && d.vanilla_of(name) != Some(item) && d.item(item).is_some_and(|i| i.adv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{Conf, Loc};
    use crate::logic_items::ItemId;
    use crate::search::{explore, Search};
    use crate::settings::{Settings, Value};
    use crate::slice::data::State;
    use crate::slice::fill::vanilla_place;
    use crate::slice::generate::{allowed, generate_data, GenOpts, Generated};
    use crate::slice::logic::world;
    use crate::slice::pick::pick;
    use crate::slice::remake_chests::{self as rc, OFF, SETTING};
    use crate::slice::support::{plan, Plan};
    use crate::world::{LocId, Placement, World};

    fn on() -> Settings {
        let mut s: Settings = Settings::default();
        s.set(SETTING, Value::s("on")).unwrap();
        s
    }

    fn all() -> Vec<&'static str> {
        NEW.iter().chain(PAIRED.iter()).chain(SEEDED.iter()).copied().collect()
    }

    fn without(w: &World, d: &Data, gone: &[&str]) -> Placement {
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

    #[test]
    fn every_gated_name_is_an_eligible_catalog_chest_and_a_logic_location() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        for n in all() {
            let l: &Loc = d.cat.by_name(n).unwrap();
            assert_eq!((l.kind.as_str(), l.conf), ("chest", Conf::Forced), "{n}");
            assert!(w.loc(n).is_some(), "{n}");
            let st: Option<State> = d.state_of(n);
            assert!(st == Some(State::Eligible) || (st == Some(State::Event) && d.event_ok(n) && n.starts_with("Graveyard Dampe Race")), "{n}");
            assert!(rc::gated(n), "{n}");
        }
        for n in NEW {
            assert!(d.cat.by_name(n).is_some_and(|c| c.ev.iter().any(|e| e == "new")), "{n}");
            assert!(d.remake.contains_key(n), "{n}");
        }
        for n in PAIRED {
            assert!(d.cat.by_name(n).is_some_and(|c| c.ev.iter().any(|e| e.starts_with("ol:"))), "{n}");
            assert!(!d.cat.gap_oot.iter().any(|g| g.name == n), "{n}");
        }
    }

    #[test]
    fn twin_hashes_are_edited_with_their_chest() {
        let d: Data = Data::load().unwrap();
        for (n, h) in [("Kak Windmill Roof Chest", 13080955777249750889u64), ("GV Storms Grotto Lens Chest", 6205043616869463865u64)] {
            let l: &Loc = d.cat.by_name(n).unwrap();
            assert_eq!(l.refs.len(), 2, "{n}");
            assert!(l.refs.iter().any(|r| r.hash == h), "{n}");
            assert!(!d.cat.gap_rem.iter().any(|g| g.refs.iter().any(|r| r.hash == h)), "{n}");
        }
        for h in [6781536951709667179u64, 10944569637726152230, 9159710900681116662] {
            assert!(!d.cat.locs.iter().any(|l| l.refs.iter().any(|r| r.hash == h)), "{h}");
        }
    }

    #[test]
    fn off_frees_none_and_on_frees_them_all() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let p: Plan = plan(&Settings::default()).unwrap();
        let off: Vec<String> = pick(&d, &w, &p).free.iter().map(|l| w.locs[*l].name.clone()).collect();
        assert!(off.iter().all(|n| !gated(n)));
        assert!(pick(&d, &w, &p).left.get(OFF).is_some_and(|n| *n >= all().len()));
        let q: Plan = plan(&on()).unwrap();
        let got: Vec<String> = pick(&d, &w, &q).free.iter().map(|l| w.locs[*l].name.clone()).collect();
        for n in all() {
            assert!(got.iter().any(|g| g == n), "{n}");
        }
    }

    #[test]
    fn the_vanilla_world_reaches_every_gated_chest() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let p: Placement = vanilla_place(&w, &d).unwrap();
        let s: Search = explore(&w, &p, &[w.item("Progressive Strength Upgrade").unwrap(); 3]);
        assert!(s.won());
        for n in all() {
            assert!(s.loc_access(w.loc(n).unwrap()), "{n}");
        }
    }

    #[test]
    fn the_local_rules_bind() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let g: Vec<ItemId> = vec![w.item("Progressive Strength Upgrade").unwrap(); 3];
        let s: Search = explore(&w, &without(&w, &d, &["Dins Fire", "Fire Arrows"]), &g);
        assert!(!s.loc_access(w.loc("HF Open Grotto Chest").unwrap()));
        let s: Search = explore(&w, &without(&w, &d, &["Song of Storms"]), &g);
        for n in ["KF Storms Grotto Chest", "SFM Wolfos Grotto Chest", "HF Redead Grotto Chest", "GV Storms Grotto Chest"] {
            assert!(!s.loc_access(w.loc(n).unwrap()), "{n}");
        }
        let s: Search = explore(&w, &without(&w, &d, &["Bomb Bag", "Megaton Hammer"]), &g);
        for n in ["DMT Chest", "GC Maze Center Chest", "GC Maze Silver Chest", "SFM Time Block Grotto Chest", "DMC Upper Grotto Chest"] {
            assert!(!s.loc_access(w.loc(n).unwrap()), "{n}");
        }
        assert!(s.loc_access(w.loc("HF Tunnel Ledge Chest").unwrap()));
        assert!(s.loc_access(w.loc("Kak Open Grotto Chest").unwrap()));
    }

    #[test]
    fn the_seed_item_is_defined_and_every_pinned_seed_chest_holds_it() {
        let d: Data = Data::load().unwrap();
        let it: &crate::slice::data::Item = d.item(SEED).unwrap();
        assert_eq!((it.actor.as_deref(), it.adv), (Some("Item_ScrubBusinessSeed"), false));
        for l in d.cat.locs.iter().filter(|l| l.item == "Item_ScrubBusinessSeed" && l.conf != Conf::Pool) {
            assert_eq!(d.vanilla_of(&l.name), Some(SEED), "{}", l.name);
        }
    }

    #[test]
    fn seeds_with_the_setting_on_win_and_keep_progression_off_the_unsure_chests() {
        let d: Data = Data::load().unwrap();
        let s: Settings = on();
        let p: Plan = plan(&s).unwrap();
        let w: World = world(&d).unwrap();
        let mut moved: usize = 0;
        for n in 0..8 {
            let g: Generated = generate_data(&d, &s, &format!("overworld {n}"), &GenOpts::default()).unwrap();
            for (loc, item) in &g.seed_file.placements {
                let l: LocId = w.loc(loc).unwrap();
                assert!(allowed(&d, &w, &p, w.item(item).unwrap(), l), "{item} at {loc}");
                if UNSURE.contains(&loc.as_str()) {
                    assert!(d.vanilla_of(loc) == Some(item.as_str()) || !d.item(item).unwrap().adv, "{loc} holds {item}");
                }
                if gated(loc) && d.vanilla_of(loc) != Some(item.as_str()) {
                    moved += 1;
                }
            }
        }
        assert!(moved > 0);
    }

    #[test]
    fn the_default_seed_never_moves_a_gated_chest() {
        let d: Data = Data::load().unwrap();
        for n in 0..3 {
            let g: Generated = generate_data(&d, &Settings::default(), &format!("plain {n}"), &GenOpts::default()).unwrap();
            assert!(g.seed_file.placements.keys().all(|k| !gated(k)));
        }
    }
}
