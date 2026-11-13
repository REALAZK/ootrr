use std::collections::{BTreeMap as BM, BTreeSet as BS};

use crate::error::Res;
use crate::logic_items::ItemId;
use crate::rng::Rng;
use crate::search::explore;
use crate::slice::data::Data;
use crate::slice::tier_order::{fixed_spot, judge, named, slots, Slot, Verdict};
use crate::world::{LocId, Placement, World};

pub const SWAPS: usize = 32;
pub const LOOK: usize = 24;

pub struct Ctx<'a> {
    pub d: &'a Data,
    pub w: &'a World,
    pub free: &'a [LocId],
    pub keep: &'a [LocId],
    pub ok: &'a dyn Fn(ItemId, LocId) -> bool,
}

pub fn active(d: &Data, w: &World, free: &[LocId]) -> bool {
    free.iter().any(|l| fixed_spot(d, &w.locs[*l].name))
}

pub fn verdict(d: &Data, w: &World, g: &Placement, free: &[LocId]) -> Res<Verdict> {
    let placed: BM<String, String> = named(w, g, free);
    judge(d, w, g, &placed)
}

fn culprits(c: &Ctx, g: &Placement, v: &Verdict) -> Vec<LocId> {
    let placed: BM<String, String> = named(c.w, g, c.free);
    let sl: Vec<Slot> = slots(c.d, &placed);
    let fams: BS<&str> = sl.iter().filter(|s| v.at.contains(&s.loc)).map(|s| s.family).collect();
    sl.iter()
        .filter(|s| fams.contains(s.family))
        .filter_map(|s| c.w.loc(&s.loc))
        .filter(|l| c.free.contains(l) && !c.keep.contains(l))
        .collect()
}

fn better(c: &Ctx, g: &Placement, a: LocId, bad: usize, rng: &mut Rng) -> Res<Option<(Placement, Verdict)>> {
    let Some(it) = g.get(a) else {
        return Ok(None);
    };
    let fam: Option<&str> = c.d.family(&c.w.items.names[it]);
    let mut others: Vec<LocId> = c.free.iter().copied().filter(|l| *l != a && !c.keep.contains(l) && !fixed_spot(c.d, &c.w.locs[*l].name)).collect();
    rng.shuffle(&mut others);
    for b in others.into_iter().take(LOOK) {
        let Some(jt) = g.get(b) else {
            continue;
        };
        let other: &str = &c.w.items.names[jt];
        if c.d.family(other) == fam || other == crate::slice::skulls::TOKEN || !(c.ok)(it, b) || !(c.ok)(jt, a) {
            continue;
        }
        let mut next: Placement = g.clone();
        next.put(a, jt);
        next.put(b, it);
        if !explore(c.w, &next, &[]).won() {
            continue;
        }
        let v: Verdict = verdict(c.d, c.w, &next, c.free)?;
        if v.bad.len() < bad {
            return Ok(Some((next, v)));
        }
    }
    Ok(None)
}

pub fn repair(c: &Ctx, g: &Placement, rng: &mut Rng) -> Res<Option<Placement>> {
    let mut cur: Placement = g.clone();
    let mut v: Verdict = verdict(c.d, c.w, &cur, c.free)?;
    let mut n: usize = 0;
    while !v.bad.is_empty() && n < SWAPS {
        n += 1;
        let mut cands: Vec<LocId> = culprits(c, &cur, &v);
        rng.shuffle(&mut cands);
        let mut step: Option<(Placement, Verdict)> = None;
        for a in cands {
            step = better(c, &cur, a, v.bad.len(), rng)?;
            if step.is_some() {
                break;
            }
        }
        let Some((next, nv)) = step else {
            return Ok(None);
        };
        cur = next;
        v = nv;
    }
    Ok(v.bad.is_empty().then_some(cur))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Settings, Value};
    use crate::slice::fill::vanilla_place;
    use crate::slice::generate::allowed;
    use crate::slice::logic::world_with;
    use crate::slice::pick::{pick, Pick};
    use crate::slice::support::{plan, Plan};

    fn with(pairs: &[(&str, &str)]) -> Settings {
        let mut s: Settings = Settings::default();
        for (k, v) in pairs {
            s.set(k, Value::s(v)).unwrap();
        }
        s
    }

    #[test]
    fn an_out_of_order_hookshot_at_a_pot_is_repaired_by_swaps_and_the_seed_stays_beatable() {
        let d: Data = Data::load().unwrap();
        let s: Settings = with(&[("shuffle_chests", "all"), ("shuffle_pots", "all")]);
        let p: Plan = plan(&s).unwrap();
        let w: World = world_with(&d, &p.logic()).unwrap();
        let pk: Pick = pick(&d, &w, &p);
        assert!(active(&d, &w, &pk.free));
        let hook: ItemId = w.item("Progressive Hookshot").unwrap();
        let mut g: Placement = vanilla_place(&w, &d).unwrap();
        let ok = |i: ItemId, l: LocId| allowed(&d, &w, &p, i, l);
        let mut pots: Vec<LocId> = pk.free.iter().copied().filter(|l| d.pots.has(&w.locs[*l].name) && ok(hook, *l)).collect();
        pots.sort_by(|a, b| w.locs[*a].name.cmp(&w.locs[*b].name));
        let dampe: LocId = w.loc("Water Temple Longshot Chest").unwrap();
        let mut seen: bool = false;
        for pot in pots {
            let mut t: Placement = g.clone();
            let was: ItemId = t.get(pot).unwrap();
            t.put(pot, hook);
            t.put(dampe, was);
            if explore(&w, &t, &[]).won() && !verdict(&d, &w, &t, &pk.free).unwrap().bad.is_empty() {
                g = t;
                seen = true;
                break;
            }
        }
        assert!(seen, "no pot gave an out of order hookshot");
        let c: Ctx = Ctx { d: &d, w: &w, free: &pk.free, keep: &[], ok: &ok };
        let fixed: Placement = repair(&c, &g, &mut Rng::from_text("repair test")).unwrap().expect("a repair");
        assert!(verdict(&d, &w, &fixed, &pk.free).unwrap().bad.is_empty());
        assert!(explore(&w, &fixed, &[]).won());
        let again: Placement = repair(&c, &g, &mut Rng::from_text("repair test")).unwrap().unwrap();
        assert_eq!(again.item, fixed.item);
        let all: Ctx = Ctx { keep: &pk.free, ..c };
        assert!(repair(&all, &g, &mut Rng::from_text("x")).unwrap().is_none());
    }

    #[test]
    fn fifty_widened_seeds_keep_every_tier_at_a_pickup_spot_in_reach_order_with_a_pack() {
        use crate::slice::generate::{generate_data, GenOpts, Generated};
        use crate::slice::tier_order::{fixed_actors, Give};
        let d: Data = Data::load().unwrap();
        let mut seeds: usize = 0;
        let mut open: usize = 0;
        let mut packs: usize = 0;
        let sets: [(&str, &str, &str, &str, usize); 4] = [
            ("all", "all", "off", "off", 15),
            ("overworld", "off", "all", "off", 15),
            ("all", "all", "all", "off", 10),
            ("dungeons", "off", "off", "matched", 10),
        ];
        for (chests, free, pots, skulls, k) in sets {
            let s: Settings = with(&[("shuffle_rewards", "off"), ("shuffle_chests", chests), ("shuffle_freestanding_items", free), ("shuffle_pots", pots), ("shuffle_crates", pots), ("shuffle_skulltulas", skulls)]);
            for n in 0..k {
                let g: Generated = generate_data(&d, &s, &format!("wide tiers {chests} {free} {pots} {skulls} {n}"), &GenOpts::default()).unwrap();
                seeds += 1;
                let v: Verdict = fixed_actors(&d, &g.seed_file).unwrap();
                assert!(v.bad.is_empty(), "{chests} {free} {pots} {skulls} {n}: {:?}", v.bad);
                for x in slots(&d, &g.seed_file.placements).iter().filter(|x| x.give == Give::Open && fixed_spot(&d, &x.loc)) {
                    open += 1;
                    let tier: &str = v.assign.get(&x.loc).map(|t| t.as_str()).unwrap_or("");
                    if crate::slice::key_spots::top_only(&d, &x.item) {
                        assert_eq!(Some(tier), crate::slice::key_spots::top(x.family), "{}", x.loc);
                    }
                    let p: crate::slice::key_spots::Pick = crate::slice::key_spots::resolve(&d, &x.item, Some(tier)).unwrap_or_else(|e| panic!("{}: {e}", x.loc));
                    assert!(p.pack.starts_with("Obj_"), "{}", p.pack);
                    packs += 1;
                }
            }
        }
        assert_eq!(seeds, 50);
        assert_eq!(open, packs);
        assert!(open > 0, "no seed put a tier item at a pickup spot, the test proves nothing");
    }
}
