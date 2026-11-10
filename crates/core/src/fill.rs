use crate::error::{Error as E, Res};
use crate::logic_items::ItemId;
use crate::rng::Rng;
use crate::search::{explore, Search};
use crate::world::{LocId, Placement, World};

pub struct FillCfg<'a> {
    pub attempts: u32,
    pub beatable_only: bool,
    pub allow: Option<&'a dyn Fn(ItemId, LocId) -> bool>,
}

impl Default for FillCfg<'_> {
    fn default() -> Self {
        FillCfg::new()
    }
}

impl FillCfg<'_> {
    pub fn new() -> Self {
        FillCfg { attempts: 20, beatable_only: false, allow: None }
    }
}

pub struct Filled {
    pub place: Placement,
    pub attempts: u32,
}

fn ok_here(cfg: &FillCfg, i: ItemId, l: LocId) -> bool {
    match cfg.allow {
        Some(f) => f(i, l),
        None => true,
    }
}

fn once(w: &World, pool: &[ItemId], rng: &mut Rng, cfg: &FillCfg) -> Res<Placement> {
    let mut place: Placement = Placement::new(w.locs.len());
    let mut items: Vec<ItemId> = pool.to_vec();
    rng.shuffle(&mut items);
    let mut prog: Vec<ItemId> = items.iter().copied().filter(|i| w.items.adv[*i]).collect();
    let rest: Vec<ItemId> = items.iter().copied().filter(|i| !w.items.adv[*i]).collect();
    let mut free: Vec<LocId> = w.real.clone();
    let mut base: Search = Search::new(w);
    base.settle(&place, true);
    let mut assumed: Search = base.clone();
    assumed.add_all(&prog);
    while let Some(it) = prog.pop() {
        rng.shuffle(&mut free);
        assumed.remove(it);
        let mut mx: Search = assumed.clone();
        mx.settle(&place, true);
        let check: bool = if cfg.beatable_only { !mx.won() } else { true };
        let pick: Option<usize> = free
            .iter()
            .position(|l| ok_here(cfg, it, *l) && (!check || mx.loc_access(*l)));
        match pick {
            Some(k) => {
                let l: LocId = free.swap_remove(k);
                place.put(l, it);
            }
            None => {
                return Err(E::Msg(format!(
                    "game unbeatable: no spot for {} among {} free locations with {} progression items left",
                    w.items.names[it],
                    free.len(),
                    prog.len()
                )));
            }
        }
    }
    rng.shuffle(&mut free);
    for it in rest {
        match free.iter().position(|l| ok_here(cfg, it, *l)) {
            Some(k) => {
                let l: LocId = free.swap_remove(k);
                place.put(l, it);
            }
            None => return Err(E::Msg(format!("no spot for {}", w.items.names[it]))),
        }
    }
    if !explore(w, &place, &[]).won() {
        return Err(E::Format("game unbeatable after the fill"));
    }
    Ok(place)
}

pub fn fill(w: &World, pool: &[ItemId], rng: &mut Rng, cfg: &FillCfg) -> Res<Filled> {
    if pool.len() > w.real.len() {
        return Err(E::Msg(format!(
            "{} items do not fit into {} locations",
            pool.len(),
            w.real.len()
        )));
    }
    let mut all: Search = Search::new(w);
    all.add_all(pool);
    all.settle(&Placement::new(w.locs.len()), true);
    if !all.won() {
        return Err(E::Format("goal is unreachable even with every item"));
    }
    let mut last: E = E::Format("no fill attempts were made");
    for n in 1..=cfg.attempts.max(1) {
        match once(w, pool, rng, cfg) {
            Ok(place) => return Ok(Filled { place, attempts: n }),
            Err(e) => last = e,
        }
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Spec;
    use std::cell::Cell;

    fn world(text: &str) -> World {
        World::build(&Spec::from_bundle(text).unwrap()).unwrap()
    }

    fn run(w: &World, seed: u64) -> Res<Filled> {
        let pool: Vec<ItemId> = w.pool();
        fill(w, &pool, &mut Rng::from_seed(seed), &FillCfg::new())
    }

    fn name_at(w: &World, f: &Filled, loc: &str) -> String {
        let l: LocId = w.loc(loc).unwrap();
        match f.place.get(l) {
            Some(i) => w.items.names[i].clone(),
            None => String::new(),
        }
    }

    const PAIR: &str = r#"{
        "goal": "Win",
        "items": [{"name": "Sword", "advancement": true}, {"name": "Boots", "advancement": true}, "Rupee"],
        "regions": [{"region_name": "Root",
            "locations": {"Start Chest": "True", "Sword Chest": "Sword", "Boots Chest": "Boots"},
            "events": {"Win": "Sword and Boots"}}]
    }"#;

    #[test]
    fn gated_pair_is_always_beatable_and_varies() {
        let w: World = world(PAIR);
        let mut starts: Vec<String> = Vec::new();
        for seed in 0..200u64 {
            let f: Filled = run(&w, seed).unwrap();
            let s: String = name_at(&w, &f, "Start Chest");
            assert!(s == "Sword" || s == "Boots", "seed {seed} put {s} at the start");
            assert!(explore(&w, &f.place, &[]).won());
            if !starts.contains(&s) {
                starts.push(s);
            }
        }
        assert_eq!(starts.len(), 2);
    }

    #[test]
    fn circular_dependency_is_unbeatable() {
        let w: World = world(
            r#"{
            "goal": "Win",
            "items": [{"name": "Key A", "advancement": true}, {"name": "Key B", "advancement": true}],
            "regions": [{"region_name": "Root",
                "locations": {"A Chest": "Key_B", "B Chest": "Key_A"},
                "events": {"Win": "Key_A and Key_B"}}]
        }"#,
        );
        assert!(run(&w, 1).is_err());
    }

    #[test]
    fn goal_without_its_item_is_refused_fast() {
        let w: World = world(
            r#"{
            "goal": "Win",
            "items": [{"name": "Hookshot", "advancement": true, "count": 0}, "Rupee", "Rupee 2"],
            "regions": [{"region_name": "Root", "locations": {"A": "True", "B": "True"},
                "events": {"Win": "Hookshot"}}]
        }"#,
        );
        let pool: Vec<ItemId> = w.pool_of(&[("Rupee", 1), ("Rupee 2", 1)]).unwrap();
        let e: E = fill(&w, &pool, &mut Rng::from_seed(3), &FillCfg::new()).err().unwrap();
        assert!(e.to_string().contains("unreachable"));
    }

    #[test]
    fn counted_gate_forces_the_prize_into_the_vault() {
        let w: World = world(
            r#"{
            "goal": "Win",
            "items": [{"name": "Key", "advancement": true, "count": 3}, {"name": "Prize", "advancement": true}],
            "regions": [{"region_name": "Root",
                "locations": {"Free 1": "True", "Free 2": "True", "Free 3": "True", "Vault": "(Key, 3)"},
                "events": {"Win": "Prize"}}]
        }"#,
        );
        for seed in 0..100u64 {
            let f: Filled = run(&w, seed).unwrap();
            assert_eq!(name_at(&w, &f, "Vault"), "Prize");
        }
    }

    #[test]
    fn ages_keep_time_travel_out_of_adult_only_spots() {
        let w: World = world(
            r#"{
            "goal": "Win",
            "settings": {"starting_age": "child"},
            "helpers": {"is_child": "age == 'child'", "is_adult": "age == 'adult'",
                        "is_starting_age": "age == starting_age"},
            "items": [{"name": "Time_Travel", "advancement": true}, {"name": "Bow", "advancement": true}, "Junk"],
            "regions": [
                {"region_name": "Root", "exits": {"Hub": "is_starting_age or Time_Travel"}},
                {"region_name": "Hub",
                 "locations": {"Hub A": "True", "Hub B": "is_adult", "Hub C": "True"},
                 "events": {"Win": "is_adult and Bow"}}]
        }"#,
        );
        for seed in 0..100u64 {
            let f: Filled = run(&w, seed).unwrap();
            assert_ne!(name_at(&w, &f, "Hub B"), "Time_Travel");
            assert!(explore(&w, &f.place, &[]).won());
        }
    }

    #[test]
    fn settings_decide_whether_the_world_can_be_won() {
        let text: &str = r#"{
            "goal": "Win",
            "items": ["Junk", "Junk 2"],
            "regions": [
                {"region_name": "Root", "exits": {"Vault": "open_door"}, "locations": {"A": "True", "B": "True"}},
                {"region_name": "Vault", "events": {"Win": "True"}}]
        }"#;
        let mut open: Spec = Spec::from_bundle(text).unwrap();
        open.settings.set_bool("open_door", true);
        let mut shut: Spec = Spec::from_bundle(text).unwrap();
        shut.settings.set_bool("open_door", false);
        let a: World = World::build(&open).unwrap();
        let b: World = World::build(&shut).unwrap();
        assert!(run(&a, 5).is_ok());
        assert!(run(&b, 5).is_err());
    }

    #[test]
    fn remote_subrules_and_shared_events_work() {
        let w: World = world(
            r#"{
            "goal": "Win",
            "items": [{"name": "Hammer", "advancement": true}, {"name": "Lens", "advancement": true}, "Junk"],
            "regions": [
                {"region_name": "Root", "exits": {"Tower": "True"},
                 "locations": {"Yard": "True", "Shed": "True", "Gate Chest": "at('Tower', Lens)"},
                 "events": {"Win": "Lit and Hammer"}},
                {"region_name": "Tower", "events": {"Lit": "Lens"}, "locations": {"Tower Top": "Lit"}}]
        }"#,
        );
        assert!(w.loc("Tower Subrule 1").is_some());
        for seed in 0..100u64 {
            let f: Filled = run(&w, seed).unwrap();
            let l: String = name_at(&w, &f, "Yard");
            let m: String = name_at(&w, &f, "Shed");
            assert!(l == "Lens" || m == "Lens", "seed {seed}: yard {l}, shed {m}");
            assert!(explore(&w, &f.place, &[]).won());
        }
    }

    #[test]
    fn same_seed_same_fill_and_other_seeds_differ() {
        let w: World = world(PAIR);
        let a: Filled = run(&w, 42).unwrap();
        let b: Filled = run(&w, 42).unwrap();
        assert_eq!(a.place.item, b.place.item);
        let mut distinct: Vec<Vec<Option<ItemId>>> = Vec::new();
        for seed in 0..40u64 {
            let f: Filled = run(&w, seed).unwrap();
            if !distinct.contains(&f.place.item) {
                distinct.push(f.place.item);
            }
        }
        assert!(distinct.len() > 1);
    }

    #[test]
    fn oversized_pool_is_an_error_and_a_short_pool_leaves_gaps() {
        let w: World = world(PAIR);
        let big: Vec<ItemId> = w.pool_of(&[("Sword", 2), ("Boots", 2)]).unwrap();
        assert!(fill(&w, &big, &mut Rng::from_seed(1), &FillCfg::new()).is_err());
        let small: Vec<ItemId> = w.pool_of(&[("Sword", 1), ("Boots", 1)]).unwrap();
        let f: Filled = fill(&w, &small, &mut Rng::from_seed(1), &FillCfg::new()).unwrap();
        let empty: usize = w.real.iter().filter(|l| f.place.get(**l).is_none()).count();
        assert_eq!(empty, 1);
    }

    #[test]
    fn failed_attempts_are_retried() {
        let w: World = world(
            r#"{
            "goal": "Win",
            "items": [{"name": "X", "advancement": true}, {"name": "Y", "advancement": true}],
            "regions": [{"region_name": "Root", "locations": {"A": "True", "B": "True"},
                "events": {"Win": "X and Y"}}]
        }"#,
        );
        let calls: Cell<u32> = Cell::new(0);
        let gate = |_i: ItemId, _l: LocId| -> bool {
            let c: u32 = calls.get();
            calls.set(c + 1);
            c >= 3
        };
        let cfg: FillCfg = FillCfg { attempts: 5, beatable_only: false, allow: Some(&gate) };
        let pool: Vec<ItemId> = w.pool();
        let f: Filled = fill(&w, &pool, &mut Rng::from_seed(9), &cfg).unwrap();
        assert_eq!(f.attempts, 2);
        calls.set(0);
        let one: FillCfg = FillCfg { attempts: 1, beatable_only: false, allow: Some(&gate) };
        assert!(fill(&w, &pool, &mut Rng::from_seed(9), &one).is_err());
    }

    #[test]
    fn beatable_only_lets_spare_progression_items_go_unreachable() {
        let text: &str = r#"{
            "goal": "Win",
            "items": [{"name": "Crown", "advancement": true}, {"name": "Extra", "advancement": true}],
            "regions": [{"region_name": "Root", "locations": {"Open": "True", "Dead": "False"},
                "events": {"Win": "Crown"}}]
        }"#;
        let w: World = world(text);
        let pool: Vec<ItemId> = w.pool();
        assert!(fill(&w, &pool, &mut Rng::from_seed(4), &FillCfg::new()).is_err());
        let cfg: FillCfg = FillCfg { attempts: 20, beatable_only: true, allow: None };
        for seed in 0..30u64 {
            let f: Filled = fill(&w, &pool, &mut Rng::from_seed(seed), &cfg).unwrap();
            assert_eq!(name_at(&w, &f, "Open"), "Crown");
            assert_eq!(name_at(&w, &f, "Dead"), "Extra");
        }
    }

    #[test]
    fn restrictions_are_respected() {
        let w: World = world(PAIR);
        let sword: ItemId = w.item("Sword").unwrap();
        let start: LocId = w.loc("Start Chest").unwrap();
        let ban = move |i: ItemId, l: LocId| -> bool { !(i == sword && l == start) };
        let cfg: FillCfg = FillCfg { attempts: 5, beatable_only: false, allow: Some(&ban) };
        let pool: Vec<ItemId> = w.pool();
        for seed in 0..50u64 {
            let f: Filled = fill(&w, &pool, &mut Rng::from_seed(seed), &cfg).unwrap();
            assert_eq!(name_at(&w, &f, "Start Chest"), "Boots");
        }
    }
}
