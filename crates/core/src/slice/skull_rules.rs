use crate::error::{Error as E, Res};
use crate::logic_json::Json;
use crate::slice::skull_names::{shared, Rule};

pub const KEY: &str = "skull_rules";
pub const ON: &str = "strict";

pub fn wanted(over: &[(&str, &str)]) -> bool {
    over.iter().any(|(k, v)| *k == KEY && *v == ON)
}

fn put(files: &mut [Json], r: &Rule) -> usize {
    let mut n: usize = 0;
    for f in files.iter_mut() {
        let Json::Arr(regions) = f else {
            continue;
        };
        for reg in regions.iter_mut() {
            let Json::Obj(fields) = reg else {
                continue;
            };
            for (k, v) in fields.iter_mut() {
                if k != "locations" {
                    continue;
                }
                let Json::Obj(locs) = v else {
                    continue;
                };
                for (name, rule) in locs.iter_mut() {
                    if *name == r.name {
                        *rule = Json::Str(r.rule.clone());
                        n += 1;
                    }
                }
            }
        }
    }
    n
}

pub fn apply_with(files: &mut [Json], rules: &[Rule]) -> Res<usize> {
    for r in rules {
        match put(files, r) {
            1 => {}
            n => return Err(E::Msg(format!("skulltula rules: {} matched {n} logic locations instead of one", r.name))),
        }
    }
    Ok(rules.len())
}

pub fn apply(files: &mut [Json]) -> Res<usize> {
    let t: &crate::slice::skull_names::Names = shared().ok_or(E::Format("skulltula names: the shipped table is broken"))?;
    apply_with(files, &t.rules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic_items::ItemId;
    use crate::search::{explore, Search};
    use crate::settings::{Settings, Value};
    use crate::slice::data::Data;
    use crate::slice::fill::vanilla_place;
    use crate::slice::logic::{world, world_with};
    use crate::slice::support::plan;
    use crate::world::{LocId, Placement, World};

    fn plan_of(v: &str) -> Vec<(&'static str, &'static str)> {
        let mut s: Settings = Settings::default();
        s.set("shuffle_skulltulas", Value::s(v)).unwrap();
        plan(&s).unwrap().logic()
    }

    #[test]
    fn the_rules_switch_on_with_the_skulltula_shuffle_only() {
        assert!(!wanted(&plan_of("off")));
        assert!(wanted(&plan_of("pinned")));
        assert!(wanted(&plan_of("matched")));
    }

    #[test]
    fn a_rule_must_hit_exactly_one_location() {
        let mut f: Vec<Json> = vec![crate::logic_json::parse("[{\"region_name\":\"R\",\"locations\":{\"A\":\"True\"}}]").unwrap()];
        let ok: Rule = Rule { name: "A".to_string(), rule: "Hookshot".to_string() };
        assert_eq!(apply_with(&mut f, std::slice::from_ref(&ok)).unwrap(), 1);
        assert_eq!(f[0], crate::logic_json::parse("[{\"region_name\":\"R\",\"locations\":{\"A\":\"Hookshot\"}}]").unwrap());
        let gone: Rule = Rule { name: "B".to_string(), rule: "True".to_string() };
        assert!(apply_with(&mut f, &[gone]).is_err());
    }

    fn without(w: &World, d: &Data, item: &str, n: usize) -> Placement {
        let mut p: Placement = vanilla_place(w, d).unwrap();
        let it: ItemId = w.item(item).unwrap();
        let junk: ItemId = w.item("Rupees (5)").unwrap();
        let mut left: usize = n;
        for l in w.real.iter().rev() {
            if left > 0 && p.get(*l) == Some(it) {
                p.put(*l, junk);
                left -= 1;
            }
        }
        p
    }

    #[test]
    fn every_strict_rule_is_used_and_never_looser() {
        let d: Data = Data::load().unwrap();
        let base: World = world(&d).unwrap();
        let strict: World = world_with(&d, &[(KEY, ON)]).unwrap();
        let names: Vec<String> = shared().unwrap().rules.iter().map(|r| r.name.clone()).collect();
        for n in &names {
            let a: LocId = base.loc(n).unwrap();
            let b: LocId = strict.loc(n).unwrap();
            assert_ne!(format!("{:?}", base.locs[a].rule.node), format!("{:?}", strict.locs[b].rule.node), "{n}");
        }
        let v: Placement = vanilla_place(&strict, &d).unwrap();
        let s: Search = explore(&strict, &v, &[]);
        let gone: [(&str, usize); 12] = [
            ("Progressive Hookshot", 1),
            ("Progressive Hookshot", 2),
            ("Iron Boots", 1),
            ("Lens of Truth", 1),
            ("Magic Meter", 1),
            ("Gerudo Membership Card", 1),
            ("Boomerang", 1),
            ("Slingshot", 1),
            ("Bomb Bag", 1),
            ("Dins Fire", 1),
            ("Kokiri Sword", 1),
            ("Zeldas Lullaby", 1),
        ];
        let mut checked: usize = 0;
        for (item, n) in gone {
            let a: Search = explore(&strict, &without(&strict, &d, item, n), &[]);
            let b: Search = explore(&base, &without(&base, &d, item, n), &[]);
            for name in &names {
                let la: LocId = strict.loc(name).unwrap();
                let lb: LocId = base.loc(name).unwrap();
                if a.loc_access(la) {
                    assert!(b.loc_access(lb), "{name} is looser without {item}");
                }
                if !a.loc_access(la) && b.loc_access(lb) {
                    checked += 1;
                }
            }
        }
        assert!(checked > 0);
        assert!(s.won());
        let lost: Vec<&str> = crate::slice::skulls::token_spots(&strict, &v).into_iter().filter(|l| !s.loc_access(*l)).map(|l| strict.locs[l].name.as_str()).collect();
        assert_eq!(lost, vec!["LW GS Above Theater", "ZR GS Near Raised Grottos"]);
        assert_eq!(crate::slice::skulls::reachable_tokens(&strict, &v), 98);
    }
}
