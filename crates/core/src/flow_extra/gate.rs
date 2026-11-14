use crate::flow_locations::Loc;
use crate::slice::data::{Data, Item};
use crate::slice::support::Plan;

pub const POUCH_ONLY: [&str; 1] = ["GC Medigoron"];

pub fn open(l: &Loc, p: &Plan) -> bool {
    if crate::scrubs_a::is_a(l) {
        return p.scrubs;
    }
    if crate::scrubs_b::is_b(l) {
        return p.scrubs_b;
    }
    if crate::flow_extra::trade::is(&l.name) {
        return p.trade;
    }
    if crate::open_world::skips::is_bean_sale(&l.name) {
        return p.open.beans;
    }
    !l.gated || p.extra
}

pub fn pouch(i: &Item) -> bool {
    i.actor.as_deref().is_some_and(|a| (a.starts_with("Item_") || a.starts_with("Weapon_")) && !a.starts_with("Item_Melody_"))
}

pub fn bound(d: &Data, item: &str) -> bool {
    d.flow.locs.iter().any(|l| l.gated && !crate::flow_extra::trade::is(&l.name) && !crate::scrubs_a::is_a(l) && !crate::scrubs_b::is_b(l) && !crate::open_world::skips::is_bean_sale(&l.name) && d.vanilla_of(&l.name) == Some(item) && d.flow_ok(&l.name))
}

pub fn fits(d: &Data, name: &str, i: &Item) -> bool {
    if POUCH_ONLY.contains(&name) && !pouch(i) {
        return false;
    }
    !bound(d, &i.name) || d.flow.by_name(name).is_some_and(|l| l.gated)
}

#[cfg(test)]
mod tests {
    use crate::settings::{Settings, Value};
    use crate::slice::data::Data;
    use crate::slice::generate::{generate_data, GenOpts, Generated};
    use crate::slice::support::plan;

    const SHUFFLED: [&str; 3] = ["Kak 20 Gold Skulltula Reward", "DMT Biggoron", "GC Medigoron"];

    fn with(extra: Option<bool>) -> Settings {
        let mut s: Settings = Settings::default();
        if let Some(x) = extra {
            s.set("shuffle_rewards", Value::s("supported")).unwrap();
            s.set(crate::flow_extra::SETTING, Value::Bool(x)).unwrap();
        }
        s
    }

    fn gated(d: &Data) -> Vec<String> {
        d.flow.locs.iter().filter(|l| l.gated && !crate::flow_extra::trade::is(&l.name) && !crate::scrubs_a::is_a(l) && !crate::scrubs_b::is_b(l) && !crate::open_world::skips::is_bean_sale(&l.name)).map(|l| l.name.clone()).collect()
    }

    #[test]
    fn the_setting_is_off_by_default_and_needs_supported_rewards() {
        assert!(!plan(&Settings::default()).unwrap().extra);
        assert!(!plan(&with(Some(false))).unwrap().extra);
        assert!(plan(&with(Some(true))).unwrap().extra);
        let mut s: Settings = Settings::default();
        s.set("shuffle_rewards", Value::s("off")).unwrap();
        s.set(crate::flow_extra::SETTING, Value::Bool(true)).unwrap();
        assert!(!plan(&s).unwrap().extra);
        assert_eq!(Settings::default().encode().unwrap(), with(None).encode().unwrap());
    }

    #[test]
    fn gated_locations_stay_vanilla_unless_the_setting_is_on() {
        let d: Data = Data::load().unwrap();
        let names: Vec<String> = gated(&d);
        assert_eq!(names.len(), 4);
        for (extra, n) in [(None, 0), (Some(false), 1)] {
            let g: Generated = generate_data(&d, &with(extra), &format!("gate {n}"), &GenOpts::default()).unwrap();
            assert!(names.iter().all(|x| !g.seed_file.placements.contains_key(x)), "{extra:?}");
        }
    }

    #[test]
    fn the_setting_adds_three_beatable_locations_and_rolling_goron_waits() {
        let d: Data = Data::load().unwrap();
        let s: Settings = with(Some(true));
        let off: Generated = generate_data(&d, &with(Some(false)), "extra", &GenOpts::default()).unwrap();
        for n in 0..3 {
            let g: Generated = generate_data(&d, &s, &format!("extra {n}"), &GenOpts::default()).unwrap();
            for x in SHUFFLED {
                assert!(g.seed_file.placements.contains_key(x), "{x}");
            }
            assert!(!g.seed_file.placements.contains_key("GC Rolling Goron as Child"));
            assert_eq!(g.seed_file.placements.len(), off.seed_file.placements.len() + SHUFFLED.len());
            assert!(g.report.contains("structural event edits"));
            for x in SHUFFLED {
                let it: &str = &g.seed_file.placements[x];
                assert!(d.item(it).is_some_and(|i| d.flow_accepts(i)), "{x} holds {it}");
            }
            for (loc, it) in &g.seed_file.placements {
                if super::bound(&d, it) {
                    assert!(SHUFFLED.contains(&loc.as_str()), "{loc} holds {it}");
                }
            }
            let medi: &str = &g.seed_file.placements["GC Medigoron"];
            assert!(d.item(medi).is_some_and(super::pouch), "GC Medigoron holds {medi}");
        }
    }

    #[test]
    fn only_the_vanilla_items_of_the_pooled_gated_locations_are_bound() {
        let d: Data = Data::load().unwrap();
        let mut got: Vec<&str> = ["Stone of Agony", "Biggoron Sword", "Giants Knife"].into_iter().filter(|x| super::bound(&d, x)).collect();
        got.sort_unstable();
        assert_eq!(got, vec!["Biggoron Sword", "Giants Knife", "Stone of Agony"]);
        for x in ["Rupees (5)", "Bomb Bag", "Progressive Hookshot", "Piece of Heart"] {
            assert!(!super::bound(&d, x), "{x}");
        }
        assert!(!super::pouch(d.item("Rupees (50)").unwrap()));
        assert!(!super::pouch(d.item("Piece of Heart").unwrap()));
        assert!(super::pouch(d.item("Giants Knife").unwrap()));
    }
}
