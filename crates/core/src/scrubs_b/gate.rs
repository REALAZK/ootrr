use crate::flow_locations::Loc;
use crate::scrubs_b::data::book;
use crate::settings::Settings;
use crate::slice::data::{Data, Item};

pub const SETTING: &str = "shuffle_scrubs_b";
pub const CAT: &str = "scrub_menu";

pub const NOTE: &str = "LW Deku Scrub Grotto Rear and Jabu Jabus Belly Deku Scrub become one time sales of the placed item. Each gets a new event row, its menu is gone once it holds another item, and the sale text still names the heart piece. Experimental and untested in the game.";

pub fn is_b(l: &Loc) -> bool {
    l.gated && l.cat == CAT
}

pub fn ok(d: &Data, l: &Loc) -> bool {
    if !is_b(l) || !l.edits.is_empty() {
        return false;
    }
    let Ok(b) = book() else {
        return false;
    };
    let Some(v) = d.vanilla_of(&l.name).and_then(|v| d.item(v)) else {
        return false;
    };
    b.by_location(&l.name).is_some() && l.item.is_some() && l.item == v.actor && d.family(&v.name).is_none()
}

pub fn read(s: &Settings, rewards: bool, bad: &mut Vec<String>) -> bool {
    rewards
        && s.flag(SETTING).unwrap_or_else(|e| {
            bad.push(e.to_string());
            false
        })
}

pub fn fits(d: &Data, name: &str, i: &Item) -> bool {
    match d.flow.by_name(name) {
        Some(l) if is_b(l) => crate::scrubs_a::sells(i) || d.vanilla_of(name) == Some(i.name.as_str()),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scrubs_b::data::Book;
    use crate::settings::Value;
    use crate::slice::generate::{generate_data, GenOpts, Generated};
    use crate::slice::plando::Entry;
    use crate::slice::support::plan;

    const NEW: [&str; 2] = ["LW Deku Scrub Grotto Rear", "Jabu Jabus Belly Deku Scrub"];

    fn with(rewards: bool, b: Option<bool>) -> Settings {
        let mut s: Settings = Settings::default();
        s.set("shuffle_rewards", Value::s(if rewards { "supported" } else { "off" })).unwrap();
        if let Some(x) = b {
            s.set(SETTING, Value::Bool(x)).unwrap();
        }
        s
    }

    #[test]
    fn the_setting_is_off_by_default_needs_supported_rewards_and_keeps_the_settings_string() {
        assert!(!plan(&Settings::default()).unwrap().scrubs_b);
        assert!(!plan(&with(false, Some(true))).unwrap().scrubs_b);
        assert!(!plan(&with(true, Some(false))).unwrap().scrubs_b);
        assert!(plan(&with(true, Some(true))).unwrap().scrubs_b);
        assert!(plan(&with(true, Some(true))).unwrap().notes.iter().any(|n| n == NOTE));
        assert_eq!(Settings::default().encode().unwrap(), with(true, Some(false)).encode().unwrap());
        assert_eq!(with(false, None).encode().unwrap(), with(false, Some(false)).encode().unwrap());
        let e: String = plan(&{
            let mut s: Settings = with(true, Some(true));
            s.set("shuffle_scrubs", Value::s("low")).unwrap();
            s
        })
        .err()
        .unwrap()
        .to_string();
        assert!(e.contains("shuffle_scrubs is low"));
    }

    #[test]
    fn the_entries_are_gated_menu_scrubs_with_a_ready_book_entry() {
        let d: Data = Data::load().unwrap();
        let b: &Book = book().unwrap();
        let mut got: Vec<&str> = d.flow.locs.iter().filter(|l| is_b(l)).map(|l| l.name.as_str()).collect();
        got.sort_unstable();
        let mut want: Vec<&str> = NEW.to_vec();
        want.sort_unstable();
        assert_eq!(got, want);
        for n in NEW {
            let l: &Loc = d.flow.by_name(n).unwrap();
            assert!(ok(&d, l) && d.flow_ok(n) && !l.supported && !crate::scrubs_a::is_a(l), "{n}");
            assert!(b.by_location(n).is_some(), "{n}");
            assert!(crate::slice::edit::is_sale(&d, n), "{n}");
        }
        assert_eq!(d.vanilla_of("LW Deku Scrub Grotto Rear"), Some("Deku Seeds (30)"));
        assert_eq!(d.vanilla_of("Jabu Jabus Belly Deku Scrub"), Some("Deku Nuts (5)"));
        for s in b.scrubs.iter().filter(|s| !s.ready()) {
            assert!(s.location.is_empty() && s.flag.is_empty(), "{}", s.key);
        }
        assert!(!crate::flow_extra::bound(&d, "Deku Seeds (30)") && !crate::flow_extra::bound(&d, "Deku Nuts (5)"));
    }

    #[test]
    fn a_menu_scrub_takes_pouch_items_and_its_own_vanilla_item_only() {
        let d: Data = Data::load().unwrap();
        for x in ["Hover Boots", "Lens of Truth", "Bottle", "Piece of Heart"] {
            let i: &Item = d.item(x).unwrap();
            assert!(fits(&d, "Jabu Jabus Belly Deku Scrub", i) && d.flow_accepts(i), "{x}");
        }
        for x in ["Rupees (50)", "Zeldas Lullaby", "Deku Shield", "Double Defense", "Goron Tunic", "Deku Seeds (30)"] {
            assert!(!fits(&d, "Jabu Jabus Belly Deku Scrub", d.item(x).unwrap()), "{x}");
        }
        assert!(fits(&d, "LW Deku Scrub Grotto Rear", d.item("Deku Seeds (30)").unwrap()));
        assert!(fits(&d, "Jabu Jabus Belly Deku Scrub", d.item("Deku Nuts (5)").unwrap()));
        for x in ["Progressive Hookshot", "Deku Stick Capacity", "Progressive Wallet"] {
            assert!(!crate::slice::flow_rules::allows(&d, "Jabu Jabus Belly Deku Scrub", d.item(x).unwrap()), "{x}");
        }
    }

    #[test]
    fn menu_scrubs_join_the_pool_only_with_the_setting_and_reports_stay_the_same_without_it() {
        let d: Data = Data::load().unwrap();
        for (r, x) in [(false, None), (true, None), (true, Some(false)), (false, Some(true))] {
            let g: Generated = generate_data(&d, &with(r, x), "scrubs b off", &GenOpts::default()).unwrap();
            assert!(NEW.iter().all(|n| !g.seed_file.placements.contains_key(*n)), "{r} {x:?}");
        }
        let a: Generated = generate_data(&d, &with(true, None), "same", &GenOpts::default()).unwrap();
        let b: Generated = generate_data(&d, &with(true, Some(false)), "same", &GenOpts::default()).unwrap();
        assert_eq!(a.report, b.report);
        assert_eq!(a.seed_file.placements, b.seed_file.placements);
        assert_eq!(a.counts, b.counts);
        for n in 0..3 {
            let g: Generated = generate_data(&d, &with(true, Some(true)), &format!("scrubs b {n}"), &GenOpts::default()).unwrap();
            assert_eq!(g.seed_file.placements.len(), a.seed_file.placements.len() + NEW.len());
            for x in NEW {
                let it: &str = &g.seed_file.placements[x];
                assert!(fits(&d, x, d.item(it).unwrap()), "{x} holds {it}");
                assert!(d.family(it).is_none(), "{x} holds {it}");
            }
            assert!(g.report.contains(NOTE));
        }
    }

    #[test]
    fn both_scrub_settings_and_the_extra_setting_work_together() {
        let d: Data = Data::load().unwrap();
        let mut s: Settings = with(true, Some(true));
        s.set(crate::scrubs_a::SETTING, Value::Bool(true)).unwrap();
        s.set(crate::flow_extra::SETTING, Value::Bool(true)).unwrap();
        let g: Generated = generate_data(&d, &s, "all scrubs", &GenOpts::default()).unwrap();
        for x in NEW.iter().chain(["DMC Deku Scrub", "LW Deku Scrub Near Bridge", "GC Medigoron"].iter()) {
            assert!(g.seed_file.placements.contains_key(*x), "{x}");
        }
    }

    #[test]
    fn plando_can_put_an_item_at_a_menu_scrub_but_not_a_refused_one() {
        let d: Data = Data::load().unwrap();
        let s: Settings = with(true, Some(true));
        let o: GenOpts = GenOpts { plando: vec![Entry { location: "Jabu Jabus Belly Deku Scrub".to_string(), item: "Hover Boots".to_string() }], ..GenOpts::default() };
        assert_eq!(generate_data(&d, &s, "plando", &o).unwrap().seed_file.placements["Jabu Jabus Belly Deku Scrub"], "Hover Boots");
        let bad: GenOpts = GenOpts { plando: vec![Entry { location: "Jabu Jabus Belly Deku Scrub".to_string(), item: "Rupees (50)".to_string() }], ..GenOpts::default() };
        assert!(generate_data(&d, &s, "plando", &bad).is_err());
        let back: GenOpts = GenOpts { plando: vec![Entry { location: "LW Deku Scrub Grotto Rear".to_string(), item: "Deku Seeds (30)".to_string() }], ..GenOpts::default() };
        assert_eq!(generate_data(&d, &s, "plando", &back).unwrap().seed_file.placements["LW Deku Scrub Grotto Rear"], "Deku Seeds (30)");
    }
}
