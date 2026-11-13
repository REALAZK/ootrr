use crate::settings::Settings;

pub const SETTING: &str = "bombchus_in_logic";
pub const LOGIC_KEY: &str = "free_bombchu_drops";

pub const NOTE: &str = "Bombchus count as explosives in the logic without a Bomb Bag. Experimental and untested in the game. In our test Bombchus only worked once a Bomb Bag and its ammo maximum were set, so a seed may expect Bombchus where the game will not let you use them.";

pub fn mode(s: &Settings, bad: &mut Vec<String>) -> bool {
    s.flag(SETTING).unwrap_or_else(|e| {
        bad.push(e.to_string());
        false
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic_items::ItemId;
    use crate::rules::{eval, Age, Ctx};
    use crate::search::{explore, Search};
    use crate::settings::Value;
    use crate::slice::data::Data;
    use crate::slice::fill::vanilla_place;
    use crate::slice::generate::{generate_data, GenOpts, Generated};
    use crate::slice::logic::{spec, world, world_with};
    use crate::slice::support::{plan, Plan};
    use crate::world::{Exit, Placement, World};

    const WALL: &str = "ZR Front -> Zora River";

    fn on() -> Settings {
        let mut s: Settings = Settings::default();
        s.set(SETTING, Value::Bool(true)).unwrap();
        s
    }

    fn open(s: &Search, w: &World, a: Age) -> bool {
        let x: &Exit = w.exits.iter().find(|x| x.name == WALL).unwrap();
        let c: Ctx = Ctx { cnt: &s.cnt, age: Some(a), tod: 0, region: x.from, q: Some(s) };
        x.rule.always || (!x.rule.never && eval(&x.rule.node, &c))
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

    fn chu_world(d: &Data) -> World {
        world_with(d, &[(LOGIC_KEY, "true")]).unwrap()
    }

    fn beatable(d: &Data, logic: &[(&str, &str)], g: &Generated) -> bool {
        let w: World = world_with(d, logic).unwrap();
        let mut p: Placement = vanilla_place(&w, d).unwrap();
        for (loc, item) in &g.seed_file.placements {
            p.put(w.loc(loc).unwrap(), w.item(item).unwrap());
        }
        explore(&w, &p, &[]).won()
    }

    #[test]
    fn off_is_the_default_and_changes_nothing() {
        let p: Plan = plan(&Settings::default()).unwrap();
        assert!(!p.chus);
        assert_eq!(p.logic(), vec![("open_door_of_time", "open")]);
        assert!(!p.notes.iter().any(|n| n == NOTE));
    }

    #[test]
    fn on_is_accepted_sets_the_logic_flag_and_adds_the_caution() {
        let p: Plan = plan(&on()).unwrap();
        assert!(p.chus);
        assert_eq!(p.logic(), vec![("open_door_of_time", "open"), (LOGIC_KEY, "true")]);
        assert!(p.notes.iter().any(|n| n == NOTE));
        assert!(NOTE.contains("Bomb Bag") && NOTE.contains("Experimental"));
    }

    #[test]
    fn the_schema_entry_is_an_experimental_toggle_with_the_caution_in_its_tip() {
        let st: &crate::settings::model::Setting = crate::settings::schema().get(SETTING).unwrap();
        assert_eq!(st.default, Value::Bool(false));
        assert!(st.label.to_lowercase().contains("experimental"));
        for w in ["Bomb Bag", "ammo", "count of 0", "Dodongo"] {
            assert!(st.tip.contains(w), "{w}");
        }
        assert!(!st.tip.contains(';') && !st.tip.contains('\u{2014}'));
    }

    #[test]
    fn the_logic_flag_is_off_in_the_shipped_data_and_the_override_flips_it() {
        let d: Data = Data::load().unwrap();
        assert!(!spec(&d, &[]).unwrap().settings.flag(LOGIC_KEY));
        assert!(spec(&d, &[(LOGIC_KEY, "true")]).unwrap().settings.flag(LOGIC_KEY));
        assert_eq!(chu_world(&d).real.len(), world(&d).unwrap().real.len());
    }

    #[test]
    fn bombchus_open_a_blast_wall_without_a_bomb_bag_only_when_ticked() {
        let d: Data = Data::load().unwrap();
        let off: World = world(&d).unwrap();
        let on: World = chu_world(&d);
        let chus: [&str; 4] = ["Bombchus", "Bombchus (5)", "Bombchus (10)", "Bombchus (20)"];
        let mut gone: Vec<&str> = chus.to_vec();
        gone.push("Bomb Bag");
        for (w, ticked) in [(&off, false), (&on, true)] {
            let p: Placement = without(w, &d, &gone);
            let none: Search = explore(w, &p, &[]);
            assert!(!open(&none, w, Age::Child), "{ticked} no explosive");
            for c in chus {
                let have: Search = explore(w, &p, &[w.item(c).unwrap()]);
                assert_eq!(open(&have, w, Age::Child), ticked, "{ticked} {c}");
            }
            let bag: Search = explore(w, &without(w, &d, &chus), &[]);
            assert!(open(&bag, w, Age::Child), "{ticked} bag");
        }
    }

    #[test]
    fn a_bomb_bag_only_matters_for_bombchus_when_not_ticked() {
        let d: Data = Data::load().unwrap();
        let off: World = world(&d).unwrap();
        let on: World = chu_world(&d);
        let p_off: Placement = without(&off, &d, &["Bomb Bag"]);
        let p_on: Placement = without(&on, &d, &["Bomb Bag"]);
        let chu: ItemId = off.item("Bombchus (20)").unwrap();
        let s_off: Search = explore(&off, &p_off, &[chu]);
        let s_on: Search = explore(&on, &p_on, &[on.item("Bombchus (20)").unwrap()]);
        let l_off: usize = off.real.iter().filter(|l| s_off.loc_access(**l)).count();
        let l_on: usize = on.real.iter().filter(|l| s_on.loc_access(**l)).count();
        assert!(l_on > l_off, "{l_on} vs {l_off}");
    }

    #[test]
    fn the_settings_string_round_trips_and_the_default_string_is_unchanged() {
        let t: String = on().encode().unwrap();
        let back: Settings = Settings::decode(&t).unwrap().settings;
        assert_eq!(back, on());
        assert!(back.flag(SETTING).unwrap());
        let def: String = Settings::default().encode().unwrap();
        assert_ne!(t, def);
        assert!(!Settings::decode(&def).unwrap().settings.flag(SETTING).unwrap());
        let mut off: Settings = on();
        off.set(SETTING, Value::Bool(false)).unwrap();
        assert_eq!(off.encode().unwrap(), def);
    }

    #[test]
    fn off_matches_the_default_and_both_settings_generate_beatable_seeds() {
        let d: Data = Data::load().unwrap();
        let mut explicit: Settings = Settings::default();
        explicit.set(SETTING, Value::Bool(false)).unwrap();
        let p_on: Plan = plan(&on()).unwrap();
        let p_off: Plan = plan(&Settings::default()).unwrap();
        for n in 0..4 {
            let seed: String = format!("chu {n}");
            let a: Generated = generate_data(&d, &Settings::default(), &seed, &GenOpts::default()).unwrap();
            let b: Generated = generate_data(&d, &explicit, &seed, &GenOpts::default()).unwrap();
            assert_eq!(a.seed_file.to_text(), b.seed_file.to_text());
            assert_eq!(a.report, b.report);
            let c: Generated = generate_data(&d, &on(), &seed, &GenOpts::default()).unwrap();
            assert!(beatable(&d, &p_off.logic(), &a), "{seed} off");
            assert!(beatable(&d, &p_on.logic(), &c), "{seed} on");
            assert!(c.report.contains(NOTE));
            assert!(!a.report.contains(NOTE));
        }
    }

    fn wide(tick: bool) -> Settings {
        let mut s: Settings = Settings::default();
        for (k, v) in [
            ("shuffle_smallkeys", "keysanity"),
            ("shuffle_pots", "all"),
            ("shuffle_crates", "all"),
            ("shuffle_freestanding_items", "all"),
            ("shuffle_remake_chests", "on"),
        ] {
            s.set(k, Value::s(v)).unwrap();
        }
        s.set(SETTING, Value::Bool(tick)).unwrap();
        s
    }

    #[test]
    fn a_ticked_seed_can_need_bombchus_that_the_unticked_logic_cannot_use() {
        let d: Data = Data::load().unwrap();
        let p_on: Plan = plan(&wide(true)).unwrap();
        let p_off: Plan = plan(&wide(false)).unwrap();
        let w: World = world_with(&d, &p_on.logic()).unwrap();
        let chus: Vec<ItemId> = ["Bombchus", "Bombchus (5)", "Bombchus (10)", "Bombchus (20)"].iter().map(|x| w.item(x).unwrap()).collect();
        let f: ItemId = w.item("Rupees (5)").unwrap();
        let mut stuck_off: usize = 0;
        let mut stuck_bare: usize = 0;
        for n in 0..3 {
            let seed: String = format!("probe {n}");
            let on: Generated = generate_data(&d, &wide(true), &seed, &GenOpts::default()).unwrap();
            let off: Generated = generate_data(&d, &wide(false), &seed, &GenOpts::default()).unwrap();
            assert!(beatable(&d, &p_on.logic(), &on), "{seed} on");
            assert!(beatable(&d, &p_off.logic(), &off), "{seed} off");
            if !beatable(&d, &p_off.logic(), &on) {
                stuck_off += 1;
            }
            let mut p: Placement = vanilla_place(&w, &d).unwrap();
            for (loc, item) in &on.seed_file.placements {
                p.put(w.loc(loc).unwrap(), w.item(item).unwrap());
            }
            for l in &w.real {
                if p.get(*l).is_some_and(|i| chus.contains(&i)) {
                    p.put(*l, f);
                }
            }
            if !explore(&w, &p, &[]).won() {
                stuck_bare += 1;
            }
        }
        assert!(stuck_off >= 1, "{stuck_off}");
        assert!(stuck_bare >= 1, "{stuck_bare}");
    }
}
