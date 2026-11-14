use crate::flow_extra::pouch;
use crate::flow_locations::Loc;
use crate::settings::Settings;
use crate::slice::data::{Data, Item};

pub const SETTING: &str = "shuffle_scrubs_a";

pub const NOTE: &str = "DMC Deku Scrub, LW Deku Scrub Near Bridge and LW Deku Scrub Grotto Front are shuffled. Their one time sale hands out the placed item, the price stays and the sale text still names the vanilla item.";

const HOME: [&str; 1] = ["Obj_Heart_Piece"];

const REFUSED: [&str; 1] = ["Item_FairyBless_"];

pub fn is_a(l: &Loc) -> bool {
    l.gated && l.cat == "scrub_sale"
}

pub fn grants_only(l: &Loc) -> bool {
    is_a(l) && !l.edits.is_empty() && l.edits.iter().all(|e| e.role == "grant")
}

pub fn home(d: &Data, name: &str, item: &str) -> bool {
    d.flow.by_name(name).is_some_and(is_a) && d.vanilla_of(name) == Some(item) && d.family(item).is_some()
}

pub fn read(s: &Settings, rewards: bool, bad: &mut Vec<String>) -> bool {
    rewards
        && s.flag(SETTING).unwrap_or_else(|e| {
            bad.push(e.to_string());
            false
        })
}

pub fn sells(i: &Item) -> bool {
    let Some(a) = i.actor.as_deref() else {
        return false;
    };
    if REFUSED.iter().any(|r| a.starts_with(r)) {
        return false;
    }
    pouch(i) || HOME.contains(&a)
}

pub fn fits(d: &Data, name: &str, i: &Item) -> bool {
    match d.flow.by_name(name) {
        Some(l) if l.cat == "scrub_sale" => sells(i) || d.vanilla_of(name) == Some(i.name.as_str()),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Value;
    use crate::slice::data::Data;
    use crate::slice::generate::{generate_data, GenOpts, Generated};
    use crate::slice::plando::Entry;
    use crate::slice::support::plan;

    const NEW: [&str; 3] = ["DMC Deku Scrub", "LW Deku Scrub Near Bridge", "LW Deku Scrub Grotto Front"];

    fn with(rewards: bool, scrubs: Option<bool>) -> Settings {
        let mut s: Settings = Settings::default();
        s.set("shuffle_rewards", Value::s(if rewards { "supported" } else { "off" })).unwrap();
        if let Some(x) = scrubs {
            s.set(SETTING, Value::Bool(x)).unwrap();
        }
        s
    }

    #[test]
    fn the_setting_is_off_by_default_needs_supported_rewards_and_keeps_the_settings_string() {
        assert!(!plan(&Settings::default()).unwrap().scrubs);
        assert!(!plan(&with(false, Some(true))).unwrap().scrubs);
        assert!(!plan(&with(true, Some(false))).unwrap().scrubs);
        assert!(plan(&with(true, Some(true))).unwrap().scrubs);
        assert!(plan(&with(true, Some(true))).unwrap().notes.iter().any(|n| n == NOTE));
        assert_eq!(Settings::default().encode().unwrap(), with(true, Some(false)).encode().unwrap());
        assert_eq!(with(false, None).encode().unwrap(), with(false, Some(false)).encode().unwrap());
        let e: String = plan(&{
            let mut s: Settings = Settings::default();
            s.set("shuffle_scrubs", Value::s("low")).unwrap();
            s
        })
        .err()
        .unwrap()
        .to_string();
        assert!(e.contains("shuffle_scrubs is low"));
    }

    #[test]
    fn the_three_entries_are_gated_scrub_sales_with_grant_edits_only() {
        let d: Data = Data::load().unwrap();
        let mut got: Vec<&str> = d.flow.locs.iter().filter(|l| is_a(l)).map(|l| l.name.as_str()).collect();
        got.sort_unstable();
        let mut want: Vec<&str> = NEW.to_vec();
        want.sort_unstable();
        assert_eq!(got, want);
        let edits: usize = d.flow.locs.iter().filter(|l| is_a(l)).map(|l| l.edits.len()).sum();
        assert_eq!(edits, 6);
        for n in NEW {
            let l: &Loc = d.flow.by_name(n).unwrap();
            assert!(grants_only(l) && !l.supported, "{n}");
            assert!(l.edits.iter().all(|e| e.file == "Enemy_Scrub_Business_Common.bfevfl" && e.via.ends_with(":SimpleShopSub_Scrub_Business_OneTime")), "{n}");
            assert!(d.flow_ok(n), "{n}");
            assert!(d.flow.locs.iter().filter(|m| m.name != n).all(|m| m.edits.iter().all(|e| l.edits.iter().all(|x| x.file != e.file || x.index != e.index))), "{n}");
        }
        assert!(!is_a(d.flow.by_name("HF Deku Scrub Grotto").unwrap()));
        assert_eq!(d.vanilla_of("DMC Deku Scrub"), Some("Piece of Heart"));
    }

    #[test]
    fn a_scrub_sale_takes_pouch_items_and_its_own_vanilla_item_only() {
        let d: Data = Data::load().unwrap();
        for x in ["Hover Boots", "Lens of Truth", "Dins Fire", "Bottle", "Piece of Heart"] {
            let i: &Item = d.item(x).unwrap();
            assert!(sells(i) && d.flow_accepts(i) && fits(&d, "LW Deku Scrub Near Bridge", i), "{x}");
        }
        for x in ["Rupees (50)", "Rupees (200)", "Zeldas Lullaby", "Deku Shield", "Double Defense", "Goron Tunic"] {
            let i: &Item = d.item(x).unwrap();
            assert!(!sells(i), "{x}");
            assert!(!fits(&d, "DMC Deku Scrub", i), "{x}");
            assert!(fits(&d, "Kak 50 Gold Skulltula Reward", i), "{x}");
        }
        assert!(fits(&d, "DMC Deku Scrub", d.item("Piece of Heart").unwrap()));
        let stick: &Item = d.item("Deku Stick Capacity").unwrap();
        let nut: &Item = d.item("Deku Nut Capacity").unwrap();
        let wallet: &Item = d.item("Progressive Wallet").unwrap();
        assert!(crate::slice::flow_rules::allows(&d, "LW Deku Scrub Near Bridge", stick));
        assert!(crate::slice::flow_rules::allows(&d, "LW Deku Scrub Grotto Front", nut));
        assert!(!crate::slice::flow_rules::allows(&d, "LW Deku Scrub Near Bridge", nut));
        assert!(!crate::slice::flow_rules::allows(&d, "LW Deku Scrub Grotto Front", stick));
        assert!(!crate::slice::flow_rules::allows(&d, "DMC Deku Scrub", stick));
        assert!(!crate::slice::flow_rules::allows(&d, "LW Deku Scrub Near Bridge", wallet));
        assert!(home(&d, "LW Deku Scrub Near Bridge", "Deku Stick Capacity"));
        assert!(!home(&d, "Deku Theater Skull Mask", "Deku Stick Capacity"));
        assert!(!home(&d, "DMC Deku Scrub", "Piece of Heart"));
    }

    #[test]
    fn a_bag_scrub_holding_its_own_family_item_is_never_wrapped() {
        let d: Data = Data::load().unwrap();
        let mut m: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
        m.insert("LW Deku Scrub Near Bridge".to_string(), "Deku Stick Capacity".to_string());
        m.insert("LW Deku Scrub Grotto Front".to_string(), "Deku Nut Capacity".to_string());
        m.insert("Deku Theater Skull Mask".to_string(), "Deku Nut Capacity".to_string());
        let s: crate::settings::seed::SeedFile = crate::settings::seed::SeedFile::new("t", &with(true, Some(true)), &d.scan.game, m).unwrap();
        let w: std::collections::BTreeMap<String, &'static str> = crate::slice::flow_wrap::wraps(&d, &s);
        assert!(!w.contains_key("LW Deku Scrub Near Bridge") && !w.contains_key("LW Deku Scrub Grotto Front"));
        assert!(w.contains_key("Deku Theater Skull Mask"));
    }

    #[test]
    fn scrubs_join_the_pool_only_with_the_setting_and_reports_stay_the_same_without_it() {
        let d: Data = Data::load().unwrap();
        for (r, x) in [(false, None), (true, None), (true, Some(false)), (false, Some(true))] {
            let g: Generated = generate_data(&d, &with(r, x), "scrubs off", &GenOpts::default()).unwrap();
            assert!(NEW.iter().all(|n| !g.seed_file.placements.contains_key(*n)), "{r} {x:?}");
        }
        let a: Generated = generate_data(&d, &with(true, None), "same", &GenOpts::default()).unwrap();
        let b: Generated = generate_data(&d, &with(true, Some(false)), "same", &GenOpts::default()).unwrap();
        assert_eq!(a.report, b.report);
        assert_eq!(a.seed_file.placements, b.seed_file.placements);
        assert_eq!(a.counts, b.counts);
        for n in 0..3 {
            let g: Generated = generate_data(&d, &with(true, Some(true)), &format!("scrubs {n}"), &GenOpts::default()).unwrap();
            assert_eq!(g.seed_file.placements.len(), a.seed_file.placements.len() + NEW.len());
            for x in NEW.iter().chain(["HF Deku Scrub Grotto"].iter()) {
                let it: &str = &g.seed_file.placements[*x];
                let i: &Item = d.item(it).unwrap();
                assert!(fits(&d, x, i) && d.flow_accepts(i), "{x} holds {it}");
                assert!(d.family(it).is_none() || home(&d, x, it), "{x} holds {it}");
            }
            assert!(g.report.contains(NOTE));
        }
    }

    #[test]
    fn the_extra_setting_and_the_scrubs_setting_work_together() {
        let d: Data = Data::load().unwrap();
        let mut s: Settings = with(true, Some(true));
        s.set(crate::flow_extra::SETTING, Value::Bool(true)).unwrap();
        let g: Generated = generate_data(&d, &s, "both", &GenOpts::default()).unwrap();
        for x in NEW.iter().chain(["GC Medigoron", "DMT Biggoron"].iter()) {
            assert!(g.seed_file.placements.contains_key(*x), "{x}");
        }
        assert!(!crate::flow_extra::bound(&d, "Piece of Heart"));
        assert!(!crate::flow_extra::bound(&d, "Deku Stick Capacity"));
    }

    #[test]
    fn plando_can_put_an_item_at_a_scrub_but_not_a_refused_one() {
        let d: Data = Data::load().unwrap();
        let s: Settings = with(true, Some(true));
        let o: GenOpts = GenOpts { plando: vec![Entry { location: "DMC Deku Scrub".to_string(), item: "Hover Boots".to_string() }], ..GenOpts::default() };
        let g: Generated = generate_data(&d, &s, "plando", &o).unwrap();
        assert_eq!(g.seed_file.placements["DMC Deku Scrub"], "Hover Boots");
        let bad: GenOpts = GenOpts { plando: vec![Entry { location: "LW Deku Scrub Near Bridge".to_string(), item: "Deku Nut Capacity".to_string() }], ..GenOpts::default() };
        assert!(generate_data(&d, &s, "plando", &bad).is_err());
        let back: GenOpts = GenOpts { plando: vec![Entry { location: "LW Deku Scrub Near Bridge".to_string(), item: "Deku Stick Capacity".to_string() }], ..GenOpts::default() };
        assert_eq!(generate_data(&d, &s, "plando", &back).unwrap().seed_file.placements["LW Deku Scrub Near Bridge"], "Deku Stick Capacity");
    }
}
