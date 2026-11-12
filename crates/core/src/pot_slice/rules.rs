use std::collections::HashMap as HM;

use crate::logic_items::ItemId;
use crate::pot_slice::pickup;
use crate::slice::data::Data;
use crate::world::{LocId, Placement, World};

pub fn at(d: &Data, name: &str, item: &str) -> Option<bool> {
    if d.pots.has(name) {
        return Some(pickup::of(item).is_some());
    }
    None
}

pub fn verdict(d: &Data, w: &World, it: ItemId, l: LocId) -> Option<bool> {
    if d.pots.is_empty() {
        return None;
    }
    let item: &str = &w.items.names[it];
    if let Some(v) = at(d, &w.locs[l].name, item) {
        return Some(v);
    }
    if d.pots.only.contains(item) {
        return Some(false);
    }
    None
}

pub fn caps(d: &Data, w: &World, free: &[LocId], vanilla: &Placement, ok: &dyn Fn(ItemId, LocId) -> bool) -> HM<LocId, usize> {
    let mut out: HM<LocId, usize> = HM::new();
    if !free.iter().any(|l| d.pots.has(&w.locs[*l].name)) {
        return out;
    }
    let pool: Vec<ItemId> = free.iter().filter_map(|l| vanilla.get(*l)).collect();
    for l in free {
        out.insert(*l, pool.iter().filter(|i| ok(**i, *l)).count());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::explore;
    use crate::settings::{Settings, Value};
    use crate::slice::fill::vanilla_place;
    use crate::slice::generate::{generate_data, GenOpts, Generated};
    use crate::slice::logic::{world, world_with};
    use crate::slice::support::{plan, Plan};
    use crate::world::Placement;

    fn probe(d: &Data, w: &World, item: &str, loc: &str) -> Option<bool> {
        verdict(d, w, w.item(item).unwrap(), w.loc(loc).unwrap())
    }

    #[test]
    fn pots_take_only_shipped_pickups_and_pot_junk_stays_out_of_chests() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let pot: &str = "KF House of Twins Pot 1";
        let chest: &str = "KF Midos Top Left Chest";
        assert_eq!(probe(&d, &w, "Rupees (20)", pot), Some(true));
        assert_eq!(probe(&d, &w, "Fire Arrows", pot), Some(true));
        assert_eq!(probe(&d, &w, "Progressive Hookshot", pot), Some(false));
        assert_eq!(probe(&d, &w, "Piece of Heart", pot), Some(false));
        assert_eq!(probe(&d, &w, "Ocarina", pot), Some(false));
        assert_eq!(probe(&d, &w, "Small Key (Forest Temple)", pot), Some(false));
        assert_eq!(probe(&d, &w, "Deku Nuts (5)", chest), None);
        assert_eq!(probe(&d, &w, "Fairy Drop", chest), Some(false));
        assert_eq!(probe(&d, &w, "Progressive Hookshot", chest), None);
        assert_eq!(probe(&d, &w, "Rupees (5)", chest), None);
    }

    fn with(pairs: &[(&str, &str)]) -> Settings {
        let mut s: Settings = Settings::default();
        for (k, v) in pairs {
            s.set(k, Value::s(v)).unwrap();
        }
        s
    }

    fn beatable(d: &Data, s: &Settings, g: &Generated) -> bool {
        let p: Plan = plan(s).unwrap();
        let w: World = world_with(d, &p.logic()).unwrap();
        let mut pl: Placement = vanilla_place(&w, d).unwrap();
        for (loc, item) in &g.seed_file.placements {
            pl.put(w.loc(loc).unwrap(), w.item(item).unwrap());
        }
        explore(&w, &pl, &[]).won()
    }

    #[test]
    fn pot_seeds_are_beatable_and_obey_the_pickup_rules() {
        let d: Data = Data::load().unwrap();
        let mut moved: usize = 0;
        for (chests, pots) in [("all", "all"), ("off", "all"), ("all", "dungeons")] {
            let s: Settings = with(&[("shuffle_chests", chests), ("shuffle_pots", pots), ("shuffle_crates", "all")]);
            for n in 0..3 {
                let g: Generated = generate_data(&d, &s, &format!("pots {chests} {pots} {n}"), &GenOpts::default()).unwrap();
                assert!(beatable(&d, &s, &g), "{chests} {pots} {n}");
                let mut here: usize = 0;
                for (loc, item) in &g.seed_file.placements {
                    if d.pots.has(loc) {
                        here += 1;
                        assert!(pickup::of(item).is_some(), "{loc} holds {item}");
                        if d.vanilla_of(loc) != Some(item.as_str()) {
                            moved += 1;
                        }
                    } else {
                        assert!(!d.pots.only.contains(item), "{loc} holds {item}");
                    }
                }
                assert!(here > 20, "{here}");
            }
        }
        assert!(moved > 10, "{moved}");
    }

    #[test]
    fn pots_together_with_rewards_hearts_and_roaming_keys_still_generate() {
        let d: Data = Data::load().unwrap();
        let mut s: Settings = with(&[("shuffle_pots", "all"), ("shuffle_crates", "all"), ("shuffle_rewards", "supported"), ("shuffle_smallkeys", "any_dungeon")]);
        s.set("shuffle_boss_hearts", Value::Bool(true)).unwrap();
        for n in 0..2 {
            let g: Generated = generate_data(&d, &s, &format!("busy {n}"), &GenOpts::default()).unwrap();
            assert!(beatable(&d, &s, &g), "{n}");
            assert!(g.seed_file.placements.keys().any(|k| d.pots.has(k)));
        }
    }

    #[test]
    fn the_default_settings_never_touch_a_pot() {
        let d: Data = Data::load().unwrap();
        let g: Generated = generate_data(&d, &Settings::default(), "default pots", &GenOpts::default()).unwrap();
        assert!(g.seed_file.placements.keys().all(|k| !d.pots.has(k)));
    }

    #[test]
    fn a_dungeon_only_setting_keeps_pots_inside_dungeons() {
        let d: Data = Data::load().unwrap();
        let s: Settings = with(&[("shuffle_pots", "dungeons")]);
        let p: Plan = plan(&s).unwrap();
        let w: World = world_with(&d, &p.logic()).unwrap();
        let g: Generated = generate_data(&d, &s, "dungeon pots", &GenOpts::default()).unwrap();
        for k in g.seed_file.placements.keys().filter(|k| d.pots.has(k)) {
            assert!(w.regions[w.locs[w.loc(k).unwrap()].region].dungeon.is_some(), "{k}");
        }
    }

    #[test]
    fn a_data_set_without_pots_never_has_an_opinion() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let mut e: Data = d.clone();
        e.pots = Default::default();
        assert_eq!(probe(&e, &w, "Deku Nuts (5)", "KF Midos Top Left Chest"), None);
    }
}
