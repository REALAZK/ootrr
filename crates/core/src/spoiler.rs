use std::fmt::Write as W;

use crate::error::{Error as E, Res};
use crate::logic_items::ItemId;
use crate::search::{explore, Search};
use crate::world::{LocId, Placement, World};

#[derive(Clone, Debug)]
pub struct Spoiler {
    pub spheres: Vec<Vec<(LocId, ItemId)>>,
    pub required: Vec<LocId>,
}

fn spheres_of(w: &World, place: &Placement) -> (Vec<Vec<LocId>>, bool) {
    let mut s: Search = Search::new(w);
    let mut out: Vec<Vec<LocId>> = Vec::new();
    let mut found: Vec<LocId> = s.settle(place, false);
    if s.won() {
        return (out, true);
    }
    while !found.is_empty() {
        found.sort_unstable();
        for l in &found {
            if let Some(i) = place.get(*l) {
                s.add(i, 1);
            }
        }
        out.push(found);
        if s.won() {
            break;
        }
        found = s.settle(place, false);
    }
    let won: bool = s.won();
    (out, won)
}

pub fn playthrough(w: &World, place: &Placement) -> Res<Spoiler> {
    let (first, won): (Vec<Vec<LocId>>, bool) = spheres_of(w, place);
    if !won {
        return Err(E::Format("the game is not beatable with this placement"));
    }
    let mut keep: Placement = place.clone();
    let mut required: Vec<LocId> = Vec::new();
    for sphere in first.iter().rev() {
        for l in sphere.iter().rev() {
            let it: Option<ItemId> = keep.item[*l];
            keep.item[*l] = None;
            if !explore(w, &keep, &[]).won() {
                keep.item[*l] = it;
                required.push(*l);
            }
        }
    }
    required.sort_unstable();
    let (last, _): (Vec<Vec<LocId>>, bool) = spheres_of(w, &keep);
    let mut spheres: Vec<Vec<(LocId, ItemId)>> = Vec::with_capacity(last.len());
    for sphere in &last {
        let mut row: Vec<(LocId, ItemId)> = Vec::with_capacity(sphere.len());
        for l in sphere {
            if let Some(i) = keep.get(*l) {
                row.push((*l, i));
            }
        }
        if !row.is_empty() {
            spheres.push(row);
        }
    }
    Ok(Spoiler { spheres, required })
}

impl Spoiler {
    pub fn render(&self, w: &World, place: &Placement) -> String {
        let mut out: String = String::new();
        let _ = writeln!(
            out,
            "Playthrough: {} required locations in {} spheres",
            self.required.len(),
            self.spheres.len()
        );
        for (n, sphere) in self.spheres.iter().enumerate() {
            let _ = writeln!(out, "Sphere {}", n + 1);
            for (l, i) in sphere {
                let _ = writeln!(out, "  {}: {}", w.locs[*l].name, w.items.names[*i]);
            }
        }
        let _ = writeln!(out, "Placement of {} locations", w.real.len());
        for l in &w.real {
            let name: &str = match place.get(*l) {
                Some(i) => w.items.names[i].as_str(),
                None => "(empty)",
            };
            let _ = writeln!(out, "  {}: {}", w.locs[*l].name, name);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Spec;

    fn world(text: &str) -> World {
        World::build(&Spec::from_bundle(text).unwrap()).unwrap()
    }

    fn put(w: &World, p: &mut Placement, loc: &str, item: &str) {
        p.put(w.loc(loc).unwrap(), w.item(item).unwrap());
    }

    const CHAIN: &str = r#"{
        "goal": "Win",
        "items": [{"name": "Key1", "advancement": true}, {"name": "Key2", "advancement": true},
                  {"name": "Crown", "advancement": true}, {"name": "Extra", "advancement": true}],
        "regions": [{"region_name": "Root",
            "locations": {"L1": "True", "L2": "Key1", "L3": "Key2", "Decoy": "True"},
            "events": {"Win": "Crown"}}]
    }"#;

    #[test]
    fn spheres_follow_the_dependency_chain_and_drop_decoys() {
        let w: World = world(CHAIN);
        let mut p: Placement = Placement::new(w.locs.len());
        put(&w, &mut p, "L1", "Key1");
        put(&w, &mut p, "L2", "Key2");
        put(&w, &mut p, "L3", "Crown");
        put(&w, &mut p, "Decoy", "Extra");
        let sp: Spoiler = playthrough(&w, &p).unwrap();
        let names: Vec<Vec<&str>> = sp
            .spheres
            .iter()
            .map(|s| s.iter().map(|(l, _)| w.locs[*l].name.as_str()).collect())
            .collect();
        assert_eq!(names, vec![vec!["L1"], vec!["L2"], vec!["L3"]]);
        assert_eq!(sp.required.len(), 3);
        assert!(!sp.required.contains(&w.loc("Decoy").unwrap()));
    }

    #[test]
    fn unbeatable_placements_are_refused() {
        let w: World = world(CHAIN);
        let mut p: Placement = Placement::new(w.locs.len());
        put(&w, &mut p, "L1", "Extra");
        put(&w, &mut p, "L2", "Key2");
        put(&w, &mut p, "L3", "Crown");
        assert!(playthrough(&w, &p).is_err());
    }

    #[test]
    fn spare_copies_of_a_counted_item_are_not_required() {
        let w: World = world(
            r#"{
            "goal": "Win",
            "items": [{"name": "Key", "advancement": true, "count": 3}],
            "regions": [{"region_name": "Root",
                "locations": {"K1": "True", "K2": "True", "K3": "True"},
                "events": {"Win": "(Key, 2)"}}]
        }"#,
        );
        let mut p: Placement = Placement::new(w.locs.len());
        for n in ["K1", "K2", "K3"] {
            put(&w, &mut p, n, "Key");
        }
        let sp: Spoiler = playthrough(&w, &p).unwrap();
        assert_eq!(sp.required.len(), 2);
        assert_eq!(sp.spheres.len(), 1);
        assert_eq!(sp.spheres[0].len(), 2);
    }

    #[test]
    fn events_do_not_cost_a_sphere() {
        let w: World = world(
            r#"{
            "goal": "Win",
            "items": [{"name": "Lens", "advancement": true}, {"name": "Crown", "advancement": true}],
            "regions": [{"region_name": "Root",
                "locations": {"A": "True", "B": "Lit"},
                "events": {"Lit": "Lens", "Win": "Crown"}}]
        }"#,
        );
        let mut p: Placement = Placement::new(w.locs.len());
        put(&w, &mut p, "A", "Lens");
        put(&w, &mut p, "B", "Crown");
        let sp: Spoiler = playthrough(&w, &p).unwrap();
        assert_eq!(sp.spheres.len(), 2);
    }

    #[test]
    fn render_lists_spheres_and_placement() {
        let w: World = world(CHAIN);
        let mut p: Placement = Placement::new(w.locs.len());
        put(&w, &mut p, "L1", "Key1");
        put(&w, &mut p, "L2", "Key2");
        put(&w, &mut p, "L3", "Crown");
        let sp: Spoiler = playthrough(&w, &p).unwrap();
        let t: String = sp.render(&w, &p);
        assert!(t.contains("Playthrough: 3 required locations in 3 spheres"));
        assert!(t.contains("Sphere 2\n  L2: Key2"));
        assert!(t.contains("Decoy: (empty)"));
    }

    #[test]
    fn sphere_indexes_are_tight_on_generated_worlds() {
        use crate::fill::{fill, FillCfg, Filled};
        use crate::logic_synth::synth_world;
        use crate::rng::Rng;
        let mut deepest: usize = 0;
        for k in 0..12u64 {
            let w: World = synth_world(60 + (k as usize) * 40, k).unwrap();
            let pool: Vec<ItemId> = w.pool();
            let f: Filled = fill(&w, &pool, &mut Rng::from_seed(k + 100), &FillCfg::new()).unwrap();
            let sp: Spoiler = playthrough(&w, &f.place).unwrap();
            deepest = deepest.max(sp.spheres.len());
            let mut have: Placement = Placement::new(w.locs.len());
            let mut before: Placement = Placement::new(w.locs.len());
            for (n, sphere) in sp.spheres.iter().enumerate() {
                let now: Search = explore(&w, &have, &[]);
                for (l, _) in sphere {
                    assert!(now.loc_access(*l), "world {k}: {} not open in sphere {}", w.locs[*l].name, n + 1);
                    if n >= 1 {
                        let earlier: Search = explore(&w, &before, &[]);
                        assert!(!earlier.loc_access(*l), "world {k}: {} opens a sphere early", w.locs[*l].name);
                    }
                }
                before = have.clone();
                for (l, i) in sphere {
                    have.put(*l, *i);
                }
            }
            assert!(explore(&w, &have, &[]).won());
        }
        assert!(deepest >= 3);
    }

    #[test]
    fn already_won_worlds_have_no_spheres() {
        let w: World = world(
            r#"{"goal": "True", "items": ["Junk"],
                "regions": [{"region_name": "Root", "locations": {"A": "True"}}]}"#,
        );
        let mut p: Placement = Placement::new(w.locs.len());
        put(&w, &mut p, "A", "Junk");
        let sp: Spoiler = playthrough(&w, &p).unwrap();
        assert!(sp.spheres.is_empty() && sp.required.is_empty());
    }
}
