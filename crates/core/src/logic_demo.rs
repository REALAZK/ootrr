use std::fs;
use std::time::Instant;

use crate::error::Res;
use crate::fill::{fill, FillCfg, Filled};
use crate::logic_items::ItemId;
use crate::logic_synth::{chain_spec, synth_spec};
use crate::rng::Rng;
use crate::spoiler::{playthrough, Spoiler};
use crate::world::{Spec, World};

pub const DEMO: &str = r#"{
    "goal": "Game_Won",
    "settings": {"starting_age": "child", "ensure_tod_access": true, "open_gate": false, "keys_needed": 2},
    "helpers": {
        "is_child": "age == 'child'",
        "is_adult": "age == 'adult'",
        "is_starting_age": "age == starting_age",
        "skip_to_night": "can_play(Sun_Song)",
        "skip_to_day": "can_play(Sun_Song)",
        "can_play(song)": "Ocarina and song",
        "can_cut": "is_adult or Sword",
        "has_keys": "(Small_Key, keys_needed)"
    },
    "items": [
        {"name": "Sword", "advancement": true},
        {"name": "Bow", "advancement": true},
        {"name": "Hookshot", "advancement": true},
        {"name": "Hammer", "advancement": true},
        {"name": "Ocarina", "advancement": true},
        {"name": "Sun Song", "advancement": true},
        {"name": "Time Gem", "advancement": true},
        {"name": "Lantern", "advancement": true},
        {"name": "Boots", "advancement": true},
        {"name": "Small Key", "advancement": true, "count": 3},
        {"name": "Crown", "advancement": true},
        {"name": "Rupee", "count": 4},
        {"name": "Heart"},
        {"name": "Potion"}
    ],
    "regions": [
        {"region_name": "Root", "exits": {"Root Exits": "is_starting_age or Time_Gem"}},
        {"region_name": "Root Exits", "exits": {"Village": "is_child", "Citadel Gate": "is_adult"}},
        {"region_name": "Village", "time_passes": true,
         "locations": {
            "Village Well Chest": "True",
            "Village Shop Gift": "True",
            "Village Rooftop": "Hookshot and is_adult",
            "Fountain Fairy": "Ocarina"},
         "exits": {"Forest Path": "True", "Market": "True"}},
        {"region_name": "Forest Path",
         "locations": {
            "Forest Stump": "Sword or Boots",
            "Mossy Chest": "can_cut",
            "Night Watchman Gift": "at_night and is_child"},
         "exits": {"Deep Forest": "can_cut", "Village": "True"}},
        {"region_name": "Deep Forest",
         "locations": {
            "Hollow Log": "Lantern",
            "Spider Cache": "Boots and is_child",
            "Forest Boss": "has_keys and Bow"},
         "events": {"Forest Cleared": "has_keys and Bow"},
         "exits": {"Forest Path": "True"}},
        {"region_name": "Market",
         "locations": {
            "Market Chest": "True",
            "Bazaar Gift": "True",
            "Cat Reward": "True",
            "Song Teacher": "Ocarina",
            "Archery Prize": "Bow and is_adult"},
         "exits": {"Village": "True"}},
        {"region_name": "Citadel Gate",
         "locations": {"Gate Guard Gift": "Hookshot or Hammer"},
         "exits": {"Citadel": "open_gate or Hookshot", "Market": "True"}},
        {"region_name": "Citadel",
         "locations": {"Citadel Chest": "True", "Citadel Vault": "Hammer", "Throne Chest": "Hammer or Hookshot"},
         "events": {"Game Won": "Crown and Forest_Cleared and at('Deep Forest', Lantern)"}}
    ]
}"#;

pub fn demo_world() -> Res<World> {
    World::build(&Spec::from_bundle(DEMO)?)
}

pub fn run_demo(seed: &str) -> Res<String> {
    let w: World = demo_world()?;
    let pool: Vec<ItemId> = w.pool();
    let f: Filled = fill(&w, &pool, &mut Rng::from_text(seed), &FillCfg::new())?;
    let sp: Spoiler = playthrough(&w, &f.place)?;
    let mut out: String = format!(
        "seed {seed}: {} locations, {} items, filled in {} attempt(s)\n",
        w.real.len(),
        pool.len(),
        f.attempts
    );
    out.push_str(&sp.render(&w, &f.place));
    Ok(out)
}

pub fn run_bench(locs: usize, seed: u64, chain: bool) -> Res<String> {
    let t0: Instant = Instant::now();
    let spec: Spec = if chain { chain_spec(locs)? } else { synth_spec(locs, seed)? };
    let w: World = World::build(&spec)?;
    let t1: Instant = Instant::now();
    let pool: Vec<ItemId> = w.pool();
    let f: Filled = fill(&w, &pool, &mut Rng::from_seed(seed), &FillCfg::new())?;
    let t2: Instant = Instant::now();
    let sp: Spoiler = playthrough(&w, &f.place)?;
    let t3: Instant = Instant::now();
    Ok(format!(
        "locations: {}  regions: {}  exits: {}  items: {}  progression: {}\nbuild: {:.1} ms  fill: {:.1} ms ({} attempt(s))  playthrough: {:.1} ms ({} required, {} spheres)",
        w.real.len(),
        w.regions.len(),
        w.exits.len(),
        pool.len(),
        pool.iter().filter(|i| w.items.adv[**i]).count(),
        (t1 - t0).as_secs_f64() * 1000.0,
        (t2 - t1).as_secs_f64() * 1000.0,
        f.attempts,
        (t3 - t2).as_secs_f64() * 1000.0,
        sp.required.len(),
        sp.spheres.len()
    ))
}

pub fn run_load(root: &str, dir: &str, mode: &str) -> Res<String> {
    let mut s: Spec = Spec::new();
    s.lenient = true;
    s.add_helpers(&fs::read_to_string(format!("{root}/LogicHelpers.json"))?)?;
    let mut files: Vec<String> = Vec::new();
    for e in fs::read_dir(format!("{root}/{dir}"))? {
        files.push(e?.path().to_string_lossy().to_string());
    }
    files.sort();
    let mut n: usize = 0;
    let mut rules: usize = 0;
    for f in &files {
        let mq: bool = f.ends_with(" MQ.json");
        let dungeon: bool = !(f.ends_with("Overworld.json") || f.ends_with("Bosses.json"));
        if (mode == "vanilla" && mq) || (mode == "mq" && dungeon && !mq) {
            continue;
        }
        let t: String = fs::read_to_string(f)?;
        rules += t.matches("\": \"").count();
        s.add_regions(&t)?;
        n += 1;
    }
    let t0: Instant = Instant::now();
    let w: World = World::build(&s)?;
    Ok(format!(
        "files: {n}  rule strings: {rules}  regions: {}  exits: {}  locations: {} ({} fillable)  items seen: {}
built in {:.1} ms",
        w.regions.len(),
        w.exits.len(),
        w.locs.len(),
        w.real.len(),
        w.items.len(),
        t0.elapsed().as_secs_f64() * 1000.0
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::explore;

    #[test]
    fn the_demo_world_loads_with_matching_pool_and_locations() {
        let w: World = demo_world().unwrap();
        assert_eq!(w.pool().len(), w.real.len());
    }

    #[test]
    fn the_demo_fills_and_prints_for_many_seeds() {
        let w: World = demo_world().unwrap();
        let pool: Vec<ItemId> = w.pool();
        for n in 0..60u64 {
            let f: Filled = fill(&w, &pool, &mut Rng::from_seed(n), &FillCfg::new()).unwrap();
            assert!(explore(&w, &f.place, &[]).won());
            assert!(playthrough(&w, &f.place).is_ok());
        }
        let t: String = run_demo("test").unwrap();
        assert!(t.contains("Sphere 1"));
        assert!(t.contains("Crown"));
    }

    #[test]
    fn bench_reports_numbers() {
        let t: String = run_bench(200, 3, false).unwrap();
        assert!(t.contains("locations: 200"));
        let c: String = run_bench(200, 3, true).unwrap();
        assert!(c.contains("49 spheres") || c.contains("spheres"));
    }
}
