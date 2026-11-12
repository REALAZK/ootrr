pub const RELEASED: [&str; 12] = [
    "Bottom of the Well Back Left Bombable Chest",
    "Bottom of the Well Front Left Fake Wall Chest",
    "Bottom of the Well Right Bottom Fake Wall Chest",
    "Bottom of the Well Front Center Bombable Chest",
    "Bottom of the Well Center Skulltula Chest",
    "Bottom of the Well Fire Keese Chest",
    "Graveyard Shield Grave Chest",
    "Spirit Temple Child Bridge Chest",
    "Deku Tree Basement Chest",
    "Ganons Castle Water Trial Right Chest",
    "Shadow Temple Spike Walls Left Chest",
    "Ice Cavern Iron Boots Chest",
];

pub fn gated(name: &str) -> bool {
    RELEASED.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Settings, Value};
    use crate::slice::data::{Data, State};
    use crate::slice::generate::{allowed, generate_data, GenOpts, Generated};
    use crate::slice::logic::world;
    use crate::slice::pick::pick;
    use crate::slice::remake_chests::{self as rc, SETTING};
    use crate::slice::support::{plan, Plan};
    use crate::world::World;

    fn on() -> Settings {
        let mut s: Settings = Settings::default();
        s.set(SETTING, Value::s("on")).unwrap();
        s
    }

    #[test]
    fn every_released_chest_is_open_in_the_slice_and_in_the_logic() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        for n in RELEASED {
            assert!(d.cat.by_name(n).is_some_and(|c| c.kind == "chest"), "{n}");
            assert!(w.loc(n).is_some(), "{n}");
            assert!(!rc::gated(n), "{n}");
            let st: Option<State> = d.state_of(n);
            assert!(st == Some(State::Eligible) || (st == Some(State::Event) && d.event_ok(n)), "{n} is {st:?}");
            let v: &str = d.vanilla_of(n).unwrap();
            assert!(d.item(v).is_some_and(|i| i.actor.as_deref() == d.cat.by_name(n).map(|c| c.item.as_str())), "{n} holds {v}");
        }
    }

    #[test]
    fn the_default_keeps_them_vanilla_and_the_remake_setting_frees_them() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let p: Plan = plan(&Settings::default()).unwrap();
        let off: Vec<String> = pick(&d, &w, &p).free.iter().map(|l| w.locs[*l].name.clone()).collect();
        assert!(off.iter().all(|n| !gated(n)));
        let q: Plan = plan(&on()).unwrap();
        let got: Vec<String> = pick(&d, &w, &q).free.iter().map(|l| w.locs[*l].name.clone()).collect();
        for n in RELEASED {
            assert!(got.iter().any(|g| g == n), "{n}");
        }
    }

    #[test]
    fn seeds_with_the_setting_place_only_allowed_items_in_released_chests() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let s: Settings = on();
        let q: Plan = plan(&s).unwrap();
        let mut moved: usize = 0;
        for n in 0..4 {
            let g: Generated = generate_data(&d, &s, &format!("held {n}"), &GenOpts::default()).unwrap();
            for name in RELEASED {
                let it: &str = &g.seed_file.placements[name];
                assert!(allowed(&d, &w, &q, w.item(it).unwrap(), w.loc(name).unwrap()), "{it} at {name}");
                if d.vanilla_of(name) != Some(it) {
                    moved += 1;
                }
            }
        }
        assert!(moved > 0);
    }
}
