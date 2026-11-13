use std::collections::{BTreeMap as BM, BTreeSet as BS};

use crate::error::{Error as E, Res};
use crate::logic_items::ItemId;
use crate::progressive::{needs_every_holder, owners, tiers, FAMILIES};
use crate::search::{explore, Search};
use crate::settings::seed::SeedFile;
use crate::slice::data::Data;
use crate::slice::edit::is_sale;
use crate::world::{LocId, Placement, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Give {
    Flow,
    Fixed(usize),
    Open,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    pub loc: String,
    pub item: String,
    pub family: &'static str,
    pub give: Give,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Need {
    pub held: usize,
    pub reach: bool,
    pub tight: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Verdict {
    pub assign: BM<String, String>,
    pub bad: Vec<String>,
    pub at: Vec<String>,
}

pub fn covered(d: &Data, placed: &BM<String, String>, f: &str) -> bool {
    !needs_every_holder(f) || d.holders(f).iter().all(|h| placed.contains_key(*h) || d.wrappable(h))
}

pub fn rank(f: &str, actor: &str) -> Option<usize> {
    let t: &[&str] = tiers(f).ok()?;
    let o: &[&str] = owners(f).ok()?;
    t.iter().position(|x| *x == actor).or_else(|| o.iter().position(|x| *x == actor))
}

fn native(d: &Data, f: &str, loc: &str) -> Give {
    if let Some(l) = d.flow.by_name(loc) {
        let ks: BS<usize> = l.edits.iter().filter_map(|e| rank(f, &e.item)).collect();
        if ks.len() > 1 {
            return Give::Flow;
        }
        if let Some(k) = l.item.as_deref().and_then(|a| rank(f, a)).or_else(|| ks.first().copied()) {
            return Give::Fixed(k);
        }
    }
    Give::Fixed(d.cat.by_name(loc).and_then(|c| rank(f, &c.item)).unwrap_or(0))
}

pub fn fixed_spot(d: &Data, loc: &str) -> bool {
    crate::pot_slice::targets::is_loc(d, loc)
        || crate::slice::skulls::is_skull(d, loc)
        || d.cat.by_name(loc).is_some_and(|l| crate::slice::pickups::is_world(l) || l.kind == "bossheart")
}

pub fn slots(d: &Data, placed: &BM<String, String>) -> Vec<Slot> {
    let mut out: Vec<Slot> = Vec::new();
    let mut used: BS<&'static str> = BS::new();
    for (loc, item) in placed {
        let Some(f) = d.family(item) else {
            continue;
        };
        let give: Give = if fixed_spot(d, loc) {
            Give::Open
        } else if is_sale(d, loc) {
            if crate::scrubs_a::home(d, loc, item) {
                native(d, f, loc)
            } else if d.flow_ok(loc) && covered(d, placed, f) {
                used.insert(f);
                Give::Flow
            } else {
                Give::Open
            }
        } else if covered(d, placed, f) {
            used.insert(f);
            Give::Flow
        } else {
            Give::Open
        };
        out.push(Slot { loc: loc.clone(), item: item.clone(), family: f, give });
    }
    for (f, _) in FAMILIES.iter().copied() {
        let hs: BS<&str> = d.holders(f).into_iter().filter(|h| !placed.contains_key(*h)).collect();
        for h in hs {
            let give: Give = if d.wrappable(h) && used.contains(f) { Give::Flow } else { native(d, f, h) };
            let item: String = d.vanilla_of(h).unwrap_or("").to_string();
            out.push(Slot { loc: h.to_string(), item, family: f, give });
        }
    }
    out
}

fn held_at_start(w: &World, it: ItemId) -> usize {
    w.start.iter().filter(|x| x.0 == it).map(|x| x.1 as usize).sum()
}

pub fn need(w: &World, place: &Placement, it: ItemId, at: LocId) -> Need {
    let mut p: Placement = place.clone();
    p.item[at] = None;
    let s: Search = explore(w, &p, &[]);
    let others: Vec<LocId> = (0..w.locs.len()).filter(|x| *x != at && place.get(*x) == Some(it) && s.loc_access(*x)).collect();
    let held: usize = held_at_start(w, it) + others.len();
    let reach: bool = s.loc_access(at);
    let tight: bool = if !reach || others.is_empty() {
        true
    } else if !w.items.adv[it] {
        false
    } else {
        others.iter().all(|x| {
            let mut q: Placement = p.clone();
            q.item[*x] = None;
            !explore(w, &q, &[]).loc_access(at)
        })
    };
    Need { held, reach, tight }
}

fn top_rule(d: &Data, f: &str, mine: &[&Slot]) -> bool {
    crate::slice::key_spots::TOP_ONLY.contains(&f) && mine.iter().any(|s| s.give == Give::Open && fixed_spot(d, &s.loc))
}

pub fn judge(d: &Data, w: &World, place: &Placement, placed: &BM<String, String>) -> Res<Verdict> {
    let all: Vec<Slot> = slots(d, placed);
    let mut v: Verdict = Verdict::default();
    let fams: BS<&'static str> = all.iter().filter(|s| s.give == Give::Open).map(|s| s.family).collect();
    for f in fams {
        let t: &[&str] = tiers(f)?;
        let mine: Vec<&Slot> = all.iter().filter(|s| s.family == f).collect();
        if top_rule(d, f, &mine) {
            for s in mine.iter().filter(|s| s.give == Give::Open && fixed_spot(d, &s.loc)) {
                v.assign.insert(s.loc.clone(), t[t.len() - 1].to_string());
            }
            continue;
        }
        let name: &str = mine.iter().find(|s| s.give == Give::Open).map(|s| s.item.as_str()).unwrap_or("");
        let it: ItemId = w.item(name).ok_or_else(|| E::Msg(format!("{name} is not in the logic item list")))?;
        for s in mine.iter().filter(|s| s.give != Give::Flow) {
            let Some(l) = w.loc(&s.loc) else {
                v.bad.push(format!("{} holds {} but is not a logic location", s.loc, s.item));
                continue;
            };
            let n: Need = need(w, place, it, l);
            let want: usize = match s.give {
                Give::Fixed(k) => k,
                _ => n.held.min(t.len() - 1),
            };
            if s.give == Give::Open {
                v.assign.insert(s.loc.clone(), t[want].to_string());
            }
            if !n.reach {
                continue;
            }
            if n.held != want || !n.tight {
                v.bad.push(format!(
                    "{} hands out {} ({name} tier {}) but the player can reach it holding {} other copies{}",
                    s.loc,
                    t[want],
                    want + 1,
                    n.held,
                    if n.tight { "" } else { " or fewer" }
                ));
                v.at.push(s.loc.clone());
            }
        }
    }
    Ok(v)
}

pub fn named(w: &World, place: &Placement, free: &[LocId]) -> BM<String, String> {
    free.iter().filter_map(|l| place.get(*l).map(|i| (w.locs[*l].name.clone(), w.items.names[i].clone()))).collect()
}

pub fn fits(d: &Data, w: &World, place: &Placement, free: &[LocId]) -> Res<bool> {
    let placed: BM<String, String> = named(w, place, free);
    if !slots(d, &placed).iter().any(|s| s.give == Give::Open) {
        return Ok(true);
    }
    Ok(judge(d, w, place, &placed)?.bad.is_empty())
}

pub fn fixed_actors(d: &Data, seed: &SeedFile) -> Res<Verdict> {
    if !slots(d, &seed.placements).iter().any(|s| s.give == Give::Open) {
        return Ok(Verdict::default());
    }
    let (w, place): (World, Placement) = crate::slice::starts::rebuild(d, seed)?;
    judge(d, &w, &place, &seed.placements)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Settings, Value};
    use crate::slice::fill::vanilla_place;
    use crate::slice::generate::{generate_data, GenOpts, Generated};
    use crate::slice::logic::world_with;
    use crate::slice::support::{plan, Plan};

    fn map(pairs: &[(&str, &str)]) -> BM<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    fn give(v: &[Slot], loc: &str) -> Option<Give> {
        v.iter().find(|s| s.loc == loc).map(|s| s.give)
    }

    fn with(pairs: &[(&str, &str)]) -> Settings {
        let mut s: Settings = Settings::default();
        for (k, v) in pairs {
            s.set(k, Value::s(v)).unwrap();
        }
        s
    }

    #[test]
    fn ranks_read_tiers_and_pouch_owners() {
        assert_eq!(rank("Bow", "Item_Range_Bow"), Some(0));
        assert_eq!(rank("Bow", "Weapon_Range_Bow"), Some(0));
        assert_eq!(rank("BombBag", "Item_BombBag_Large"), Some(2));
        assert_eq!(rank("BombBag", "Item_Bomb"), None);
    }

    #[test]
    fn chests_resolve_by_ownership_and_pickup_spots_are_open() {
        let d: Data = Data::load().unwrap();
        let v: Vec<Slot> = slots(&d, &map(&[("KF Midos Top Left Chest", "Bomb Bag"), ("KF Top of Sarias Recovery Heart 1", "Bow")]));
        assert_eq!(give(&v, "KF Midos Top Left Chest"), Some(Give::Flow));
        assert_eq!(give(&v, "KF Top of Sarias Recovery Heart 1"), Some(Give::Open));
        assert_eq!(give(&v, "Water Temple Longshot Chest"), Some(Give::Fixed(1)));
        assert_eq!(give(&v, "Kak 30 Gold Skulltula Reward"), Some(Give::Fixed(1)));
    }

    #[test]
    fn an_uncovered_hookshot_copy_is_open_and_its_partner_chest_stays_fixed() {
        let d: Data = Data::load().unwrap();
        let v: Vec<Slot> = slots(&d, &map(&[("KF Midos Top Left Chest", "Progressive Hookshot"), ("Graveyard Dampe Race Hookshot Chest", "Rupees (5)")]));
        assert_eq!(give(&v, "KF Midos Top Left Chest"), Some(Give::Open));
        assert_eq!(give(&v, "Water Temple Longshot Chest"), Some(Give::Fixed(1)));
        let half: Vec<Slot> = slots(&d, &map(&[("KF Midos Top Left Chest", "Progressive Hookshot"), ("Water Temple Longshot Chest", "Rupees (5)")]));
        assert_eq!(give(&half, "KF Midos Top Left Chest"), Some(Give::Open));
        assert_eq!(give(&half, "Graveyard Dampe Race Hookshot Chest"), Some(Give::Fixed(0)));
        let both: Vec<Slot> = slots(
            &d,
            &map(&[
                ("KF Midos Top Left Chest", "Progressive Hookshot"),
                ("Water Temple Longshot Chest", "Progressive Hookshot"),
                ("Graveyard Dampe Race Hookshot Chest", "Rupees (5)"),
            ]),
        );
        assert!(both.iter().filter(|s| s.family == "Hookshot").all(|s| s.give == Give::Flow), "{both:?}");
    }

    fn world_of(d: &Data, s: &Settings) -> (World, Placement) {
        let p: Plan = plan(s).unwrap();
        let w: World = world_with(d, &p.logic()).unwrap();
        let place: Placement = vanilla_place(&w, d).unwrap();
        (w, place)
    }

    #[test]
    fn an_early_copy_gets_the_first_tier_and_the_water_chest_agrees() {
        let d: Data = Data::load().unwrap();
        let s: Settings = with(&[("shuffle_chests", "overworld")]);
        let (w, mut place): (World, Placement) = world_of(&d, &s);
        let hook: ItemId = w.item("Progressive Hookshot").unwrap();
        let five: ItemId = w.item("Rupees (5)").unwrap();
        place.put(w.loc("Graveyard Dampe Race Hookshot Chest").unwrap(), five);
        place.put(w.loc("KF Midos Top Left Chest").unwrap(), hook);
        let placed: BM<String, String> = map(&[("KF Midos Top Left Chest", "Progressive Hookshot"), ("Graveyard Dampe Race Hookshot Chest", "Rupees (5)")]);
        let v: Verdict = judge(&d, &w, &place, &placed).unwrap();
        assert!(v.bad.is_empty(), "{:?}", v.bad);
        assert_eq!(v.assign.get("KF Midos Top Left Chest").map(|s| s.as_str()), Some("Item_Hookshot"));
        let n: Need = need(&w, &place, hook, w.loc("Water Temple Longshot Chest").unwrap());
        assert_eq!((n.held, n.tight), (1, true));
    }

    #[test]
    fn an_early_longshot_beside_the_vanilla_dampe_hookshot_is_refused() {
        let d: Data = Data::load().unwrap();
        let s: Settings = with(&[("shuffle_chests", "dungeons")]);
        let (w, mut place): (World, Placement) = world_of(&d, &s);
        let hook: ItemId = w.item("Progressive Hookshot").unwrap();
        let five: ItemId = w.item("Rupees (5)").unwrap();
        place.put(w.loc("Water Temple Longshot Chest").unwrap(), five);
        place.put(w.loc("Deku Tree Map Chest").unwrap(), hook);
        let placed: BM<String, String> = map(&[("Deku Tree Map Chest", "Progressive Hookshot"), ("Water Temple Longshot Chest", "Rupees (5)")]);
        let v: Verdict = judge(&d, &w, &place, &placed).unwrap();
        assert_eq!(v.assign.get("Deku Tree Map Chest").map(|s| s.as_str()), Some("Item_Longshot"));
        assert!(v.bad.iter().any(|b| b.starts_with("Graveyard Dampe Race Hookshot Chest")), "{:?}", v.bad);
        assert!(v.bad.iter().any(|b| b.starts_with("Deku Tree Map Chest")), "{:?}", v.bad);
        let p: Plan = plan(&s).unwrap();
        let free: Vec<LocId> = crate::slice::pick::pick(&d, &w, &p).free;
        assert!(!fits(&d, &w, &place, &free).unwrap());
    }

    #[test]
    fn fifty_seeds_keep_every_fixed_tier_in_reach_order() {
        let d: Data = Data::load().unwrap();
        let mut open: usize = 0;
        let mut seeds: usize = 0;
        for (chests, rewards, k) in [("overworld", "off", 15), ("dungeons", "off", 25), ("all", "off", 8), ("dungeons", "supported", 2)] {
            let s: Settings = with(&[("shuffle_chests", chests), ("shuffle_rewards", rewards)]);
            for n in 0..k {
                let g: Generated = generate_data(&d, &s, &format!("tier order {chests} {rewards} {n}"), &GenOpts::default()).unwrap();
                seeds += 1;
                let v: Verdict = fixed_actors(&d, &g.seed_file).unwrap();
                assert!(v.bad.is_empty(), "{chests} {rewards} {n}: {:?}", v.bad);
                let sl: Vec<Slot> = slots(&d, &g.seed_file.placements);
                open += sl.iter().filter(|x| x.give == Give::Open).count();
                for x in sl.iter().filter(|x| x.give == Give::Open) {
                    assert!(v.assign.contains_key(&x.loc), "{}", x.loc);
                }
            }
        }
        assert_eq!(seeds, 50);
        assert!(open > 0, "no seed had a fixed upgrade slot, the test proves nothing");
    }
}
