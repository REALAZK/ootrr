use crate::flow_locations::Loc;
use crate::progressive::family_of;
use crate::slice::data::{Data, Item};

pub fn grants_only(l: &Loc) -> bool {
    l.supported && !l.edits.is_empty() && l.edits.iter().all(|e| e.role == "grant")
}

pub fn tiered(l: &Loc, actor: &str) -> bool {
    l.edits.iter().any(|e| e.item != actor)
}

pub fn linked(i: &Item, actor: &str) -> bool {
    i.actor.as_deref() == Some(actor) || i.tiers.iter().any(|t| t == actor) || i.world.iter().any(|w| w == actor)
}

pub fn ok(d: &Data, name: &str) -> bool {
    let Some(l) = d.flow.by_name(name) else {
        return false;
    };
    if crate::flow_extra::trade::is(name) {
        return l.gated && d.vanilla_of(name).is_some_and(|v| l.item.as_deref() != d.item(v).and_then(|i| i.actor.as_deref()));
    }
    if crate::scrubs_b::is_b(l) {
        return crate::scrubs_b::ok(d, l);
    }
    if !(l.supported || l.gated) || l.conf == "guess" || l.edits.is_empty() {
        return false;
    }
    let Some(actor) = l.item.as_deref() else {
        return false;
    };
    let Some(i) = d.vanilla_of(name).and_then(|v| d.item(v)) else {
        return false;
    };
    if family_of(&i.tiers).is_some() {
        return (grants_only(l) || crate::scrubs_a::grants_only(l)) && linked(i, actor) && d.flow_accepts(i);
    }
    if tiered(l, actor) {
        return grants_only(l);
    }
    linked(i, actor)
}

pub fn allows(d: &Data, name: &str, i: &Item) -> bool {
    let Some(l) = d.flow.by_name(name) else {
        return true;
    };
    if family_of(&i.tiers).is_some() {
        return (grants_only(l) && l.cat != "scrub_sale" && d.flow_accepts(i)) || crate::scrubs_a::home(d, name, &i.name);
    }
    d.flow_accepts(i) || d.vanilla_of(name) == Some(i.name.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Settings, Value};
    use crate::slice::generate::{generate_data, GenOpts, Generated};

    const NEW: [&str; 17] = [
        "Graveyard Dampe Gravedigging Tour",
        "Kak 40 Gold Skulltula Reward",
        "Kak 100 Gold Skulltula Reward",
        "LW Target in Woods",
        "Market Shooting Gallery Reward",
        "Kak Shooting Gallery Reward",
        "Market Bombchu Bowling First Prize",
        "GF HBA 1500 Points",
        "Deku Theater Skull Mask",
        "Deku Theater Mask of Truth",
        "LLR Talons Chickens",
        "Kak 10 Gold Skulltula Reward",
        "Kak 30 Gold Skulltula Reward",
        "DMT Great Fairy Reward",
        "DMC Great Fairy Reward",
        "ZD Diving Minigame",
        "LH Adult Fishing",
    ];

    const PINNED: [&str; 8] = [
        "GC Darunias Joy",
        "LW Gift from Saria",
        "HF Ocarina of Time Item",
        "GC Rolling Goron as Child",
        "DMT Biggoron",
        "GC Medigoron",
        "KF Links House Cow",
        "Kak 20 Gold Skulltula Reward",
    ];

    fn item<'a>(d: &'a Data, n: &str) -> &'a Item {
        d.item(n).unwrap()
    }

    #[test]
    fn the_new_locations_are_free_and_the_risky_ones_stay_pinned() {
        let d: Data = Data::load().unwrap();
        for n in NEW {
            assert!(d.flow_ok(n), "{n}");
        }
        let off: crate::slice::support::Plan = crate::slice::support::plan(&Settings::default()).unwrap();
        for n in PINNED {
            let open: bool = d.flow.by_name(n).is_some_and(|l| crate::flow_extra::open(l, &off));
            assert!(!(d.flow_ok(n) && open), "{n}");
        }
        assert!(!d.flow_ok("GC Rolling Goron as Child"));
        for n in ["Kak 20 Gold Skulltula Reward", "DMT Biggoron", "GC Medigoron"] {
            assert!(d.flow_ok(n) && d.flow.by_name(n).is_some_and(|l| l.gated), "{n}");
        }
    }

    #[test]
    fn tiered_locations_name_the_lower_tier_and_hold_a_junk_vanilla_item() {
        let d: Data = Data::load().unwrap();
        for n in ["LW Target in Woods", "Market Shooting Gallery Reward", "Kak Shooting Gallery Reward", "Market Bombchu Bowling First Prize", "GF HBA 1500 Points"] {
            let l: &Loc = d.flow.by_name(n).unwrap();
            assert_eq!(l.edits.len(), 2, "{n}");
            assert!(tiered(l, l.item.as_deref().unwrap()), "{n}");
            assert_eq!(d.vanilla_of(n), Some("Rupees (5)"), "{n}");
        }
        assert_eq!(d.flow.by_name("LW Target in Woods").map(|l| l.conf.as_str()), Some("scene"));
    }

    #[test]
    fn family_items_go_only_to_grant_only_flow_locations_with_accepted_tiers() {
        let d: Data = Data::load().unwrap();
        let wallet: &Item = item(&d, "Progressive Wallet");
        assert!(allows(&d, "Kak 50 Gold Skulltula Reward", wallet));
        assert!(allows(&d, "LW Target in Woods", wallet));
        assert!(!allows(&d, "ToT Light Arrows Cutscene", wallet));
        assert!(!allows(&d, "HF Deku Scrub Grotto", wallet));
        assert!(!allows(&d, "Song from Impa", wallet));
        assert!(!allows(&d, "Kak 50 Gold Skulltula Reward", item(&d, "Progressive Strength Upgrade")));
        assert!(!allows(&d, "Kak 50 Gold Skulltula Reward", item(&d, "Progressive Hookshot")));
        assert!(allows(&d, "Some Chest", item(&d, "Progressive Hookshot")));
    }

    #[test]
    fn a_location_always_accepts_its_own_vanilla_item_but_the_item_goes_nowhere_else() {
        let d: Data = Data::load().unwrap();
        let rupees: &Item = item(&d, "Rupees (5)");
        assert!(allows(&d, "LW Target in Woods", rupees));
        assert!(!allows(&d, "Kak 50 Gold Skulltula Reward", rupees));
        let milk: &Item = item(&d, "Bottle with Milk");
        assert!(allows(&d, "LLR Talons Chickens", milk));
        assert!(!allows(&d, "Kak 50 Gold Skulltula Reward", milk));
        let gold: &Item = item(&d, "Rupees (200)");
        assert!(allows(&d, "Kak 100 Gold Skulltula Reward", gold));
        assert!(!allows(&d, "Market Lost Dog", gold));
    }

    #[test]
    fn seeds_with_rewards_on_put_the_new_locations_in_the_pool_and_stay_valid() {
        let d: Data = Data::load().unwrap();
        let mut s: Settings = Settings::default();
        s.set("shuffle_chests", Value::s("all")).unwrap();
        s.set("shuffle_rewards", Value::s("supported")).unwrap();
        for n in 0..2 {
            let g: Generated = generate_data(&d, &s, &format!("flow rules {n}"), &GenOpts::default()).unwrap();
            for l in NEW {
                assert!(g.seed_file.placements.contains_key(l), "{l}");
            }
            for l in PINNED {
                assert!(!g.seed_file.placements.contains_key(l), "{l}");
            }
        }
    }
}
