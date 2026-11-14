use std::collections::{BTreeMap as BM, HashMap as HM};
use std::fmt::Write as W;
use std::sync::OnceLock;

use crate::catalog::{Conf, Loc};
use crate::error::{Error as E, Res};
use crate::json::{self, Json};
use crate::logic_items::ItemId;
use crate::progressive::family_of;
use crate::search::{explore, Search};
use crate::settings::Settings;
use crate::slice::data::Data;
use crate::slice::support::Plan;
use crate::world::{LocId, Placement, World};

pub const SKULLS_JSON: &str = include_str!("../../../../data/skulltulas.json");
pub const SETTING: &str = "shuffle_skulltulas";
pub const KIND: &str = "gs";
pub const TOKEN: &str = "Gold Skulltula Token";
pub const NOTE: &str = "Gold Skulltulas are shuffled. A skulltula drops its new item instead of the token. The item shows the get window when it is picked up and only then is the skulltula saved, so an item that is left behind comes back with its skulltula on the next visit.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Off,
    Pinned,
    Matched,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub contested: Vec<String>,
    pub pickups: BM<String, String>,
}

pub fn table() -> Res<Table> {
    parse_table(SKULLS_JSON)
}

pub fn parse_table(t: &str) -> Res<Table> {
    let j: Json = json::parse(t)?;
    if j.get("format").and_then(|v| v.as_i64()) != Some(1) {
        return Err(E::Format("skulltula data: unsupported format"));
    }
    let mut contested: Vec<String> = Vec::new();
    for v in j.get("contested").and_then(|v| v.as_arr()).ok_or(E::Format("skulltula data: no contested list"))? {
        contested.push(v.as_str().ok_or(E::Format("skulltula data: contested entry must be text"))?.to_string());
    }
    let mut pickups: BM<String, String> = BM::new();
    for (k, v) in j.get("pickups").and_then(|v| v.as_obj()).ok_or(E::Format("skulltula data: no pickups map"))? {
        let a: &str = v.as_str().ok_or(E::Format("skulltula data: pickup must be text"))?;
        if !a.starts_with("Obj_") {
            return Err(E::Msg(format!("skulltula data: {a} is not a world pickup actor")));
        }
        pickups.insert(k.clone(), a.to_string());
    }
    Ok(Table { contested, pickups })
}

pub fn mode(s: &Settings, bad: &mut Vec<String>) -> Mode {
    match s.text(SETTING) {
        Ok("off") => Mode::Off,
        Ok("pinned") => Mode::Pinned,
        Ok("matched") => Mode::Matched,
        Ok(v) => {
            bad.push(format!("{SETTING} is {v}, which is not a known choice"));
            Mode::Off
        }
        Err(e) => {
            bad.push(e.to_string());
            Mode::Off
        }
    }
}

pub fn is_skull(d: &Data, name: &str) -> bool {
    d.cat.by_name(name).is_some_and(|c| c.kind == KIND) || crate::slice::skull_names::listed(name)
}

pub fn pickup(t: &Table, d: &Data, item: &str) -> Option<String> {
    let i: &crate::slice::data::Item = d.item(item)?;
    if family_of(&i.tiers).is_some() || i.dungeon.is_some() {
        return None;
    }
    t.pickups.get(item).cloned()
}

pub fn accepts(d: &Data, w: &World, it: ItemId, l: LocId) -> Option<bool> {
    if !is_skull(d, &w.locs[l].name) {
        return None;
    }
    let t: &Table = shared()?;
    let item: &str = &w.items.names[it];
    Some(pickup(t, d, item).is_some() || crate::slice::key_spots::placeable(d, item, crate::slice::key_spots::Spot::Skull))
}

pub fn drop_actor(t: &Table, d: &Data, item: &str, tier: Option<&str>) -> Res<String> {
    if let Some(a) = pickup(t, d, item) {
        return Ok(crate::slice::skull_kit::kit_actor(&a));
    }
    let p: crate::slice::key_spots::Pick = crate::slice::key_spots::resolve(d, item, tier)?;
    p.kit_spec()?;
    Ok(p.kit())
}

pub fn shared() -> Option<&'static Table> {
    static T: OnceLock<Option<Table>> = OnceLock::new();
    T.get_or_init(|| table().ok()).as_ref()
}

fn key(w: &World, name: &str) -> Option<(usize, String)> {
    let l: LocId = w.loc(name)?;
    Some((w.locs[l].region, format!("{:?}", w.locs[l].rule.node)))
}

fn pool_ok(d: &Data, w: &World, _p: &Plan, pool: &str) -> bool {
    let members: Vec<&Loc> = d.cat.locs.iter().filter(|l| l.pool.as_deref() == Some(pool)).collect();
    if members.is_empty() || members.iter().any(|l| l.kind != KIND) {
        return false;
    }
    let first: Option<(usize, String)> = key(w, &members[0].name);
    first.is_some() && members.iter().all(|l| key(w, &l.name) == first)
}

fn gate(w: &World, p: &Plan, name: &str) -> Option<String> {
    if p.skulls == Mode::Off {
        return Some("skulltula shuffle is off".to_string());
    }
    if w.loc(name).is_none() {
        return Some("location is not in the logic world".to_string());
    }
    None
}

pub fn why_extra(w: &World, p: &Plan, name: &str) -> Option<String> {
    if let Some(m) = gate(w, p, name) {
        return Some(m);
    }
    crate::slice::skull_names::verdict(p.skulls, name).unwrap_or_else(|| Some(crate::slice::skull_names::HELD.to_string()))
}

pub fn why(d: &Data, w: &World, p: &Plan, t: &Table, l: &Loc) -> Option<String> {
    if let Some(m) = gate(w, p, &l.name) {
        return Some(m);
    }
    if let Some(v) = crate::slice::skull_names::verdict(p.skulls, &l.name) {
        return v;
    }
    if t.contested.contains(&l.name) {
        return Some("the actor research pairs this skulltula with another remake actor".to_string());
    }
    if d.vanilla_of(&l.name) != Some(TOKEN) {
        return Some("its vanilla item is not a token".to_string());
    }
    match (l.conf, p.skulls) {
        (Conf::Item | Conf::Forced, _) => None,
        (Conf::Pool, Mode::Pinned) => Some("match is a guess inside a pool".to_string()),
        (Conf::Pool, _) => {
            let pool: &str = l.pool.as_deref().unwrap_or("");
            if pool_ok(d, w, p, pool) {
                None
            } else {
                Some("match is a guess inside a pool whose skulltulas differ in logic".to_string())
            }
        }
    }
}

pub fn pick(d: &Data, w: &World, p: &Plan, left: &mut BM<String, usize>) -> (Vec<LocId>, usize) {
    if p.skulls == Mode::Off {
        return (Vec::new(), 0);
    }
    let t: Table = match table() {
        Ok(t) => t,
        Err(e) => {
            *left.entry(format!("skulltula data is broken: {e}")).or_insert(0) += 1;
            return (Vec::new(), 0);
        }
    };
    let mut free: Vec<LocId> = Vec::new();
    let mut n: usize = 0;
    for l in d.cat.locs.iter().filter(|l| l.kind == KIND) {
        n += 1;
        match why(d, w, p, &t, l) {
            None => {
                if let Some(x) = w.loc(&l.name) {
                    free.push(x);
                }
            }
            Some(m) => *left.entry(m).or_insert(0) += 1,
        }
    }
    if p.skulls == Mode::Matched {
        for name in crate::slice::skull_names::extra(d) {
            n += 1;
            match why_extra(w, p, name) {
                None => {
                    if let Some(x) = w.loc(name) {
                        free.push(x);
                    }
                }
                Some(m) => *left.entry(m).or_insert(0) += 1,
            }
        }
    }
    (free, n)
}

pub fn token_spots(w: &World, got: &Placement) -> Vec<LocId> {
    let Some(tok) = w.item(TOKEN) else {
        return Vec::new();
    };
    w.real.iter().copied().filter(|l| got.get(*l) == Some(tok)).collect()
}

pub fn reachable_tokens(w: &World, place: &Placement) -> usize {
    let s: Search = explore(w, place, &[]);
    token_spots(w, place).into_iter().filter(|l| s.loc_access(*l)).count()
}

pub fn check_tokens(w: &World, vanilla: &Placement, got: &Placement) -> Res<usize> {
    let want: usize = reachable_tokens(w, vanilla);
    let have: usize = reachable_tokens(w, got);
    if have < want {
        return Err(E::Msg(format!(
            "only {have} Gold Skulltula Tokens can be reached against {want} in the vanilla game, the House of Skulltula rewards would be lost"
        )));
    }
    Ok(have)
}

fn named(w: &World, got: &Placement, l: LocId) -> String {
    got.get(l).map(|i| w.items.names[i].clone()).unwrap_or_else(|| "(nothing)".to_string())
}

pub fn spoiler(d: &Data, w: &World, p: &Plan, free: &[LocId], got: &Placement) -> String {
    spoiler_with(d, w, p, free, got, &BM::new())
}

pub fn spoiler_with(d: &Data, w: &World, p: &Plan, free: &[LocId], got: &Placement, fixed: &BM<String, String>) -> String {
    let mut out: String = String::new();
    if p.skulls == Mode::Off {
        return out;
    }
    let t: Option<Table> = table().ok();
    let cat: HM<&str, &Loc> = d.cat.locs.iter().filter(|l| l.kind == KIND).map(|l| (l.name.as_str(), l)).collect();
    let mut names: Vec<&str> = w.real.iter().map(|l| w.locs[*l].name.as_str()).filter(|n| d.vanilla_of(n) == Some(TOKEN)).collect();
    names.sort_unstable();
    let _ = writeln!(out, "Gold Skulltulas, {} in the logic", names.len());
    for n in names {
        let Some(l) = w.loc(n) else {
            continue;
        };
        let item: String = named(w, got, l);
        if free.contains(&l) {
            let drop: String = match t.as_ref().map(|t| drop_actor(t, d, &item, fixed.get(n).map(|s| s.as_str()))) {
                _ if item == TOKEN => "its token drop is left as it is".to_string(),
                Some(Ok(a)) => format!("drops {a}"),
                _ => "no pickup".to_string(),
            };
            let _ = writeln!(out, "  {n}: {item} (shuffled, {drop})");
            continue;
        }
        let reason: String = match (cat.get(n), t.as_ref()) {
            (Some(c), Some(t)) => why(d, w, p, t, c).unwrap_or_else(|| "not picked".to_string()),
            (None, _) if p.skulls == Mode::Matched && crate::slice::skull_names::listed(n) => why_extra(w, p, n).unwrap_or_else(|| "not picked".to_string()),
            (None, _) => "no remake actor is matched to it".to_string(),
            (Some(_), None) => "skulltula data is broken".to_string(),
        };
        let _ = writeln!(out, "  {n}: {item} (vanilla, {reason})");
    }
    let spots: Vec<LocId> = token_spots(w, got);
    let home: usize = spots.iter().filter(|l| d.vanilla_of(&w.locs[**l].name) == Some(TOKEN)).count();
    let _ = writeln!(out, "Tokens: {} in the world, {} at a skulltula, {} elsewhere", spots.len(), home, spots.len() - home);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Value;
    use crate::slice::generate::{generate_data, GenOpts, Generated};
    use crate::slice::logic::world;
    use crate::slice::support::plan;

    fn with(v: &str) -> Settings {
        let mut s: Settings = Settings::default();
        s.set("shuffle_rewards", Value::s("off")).unwrap();
        s.set(SETTING, Value::s(v)).unwrap();
        s
    }

    #[test]
    fn the_shipped_table_parses_and_names_real_items_and_locations() {
        let d: Data = Data::load().unwrap();
        let t: Table = table().unwrap();
        assert_eq!(t.contested.len(), 5);
        for c in &t.contested {
            assert!(is_skull(&d, c), "{c}");
        }
        for (k, v) in &t.pickups {
            assert!(d.item(k).is_some(), "{k}");
            assert!(v.starts_with("Obj_"), "{v}");
        }
        assert_eq!(pickup(&t, &d, TOKEN).as_deref(), Some("Obj_Token_Gold"));
        assert!(pickup(&t, &d, "Progressive Hookshot").is_none());
        assert!(pickup(&t, &d, "Small Key (Forest Temple)").is_none());
        assert!(parse_table("{\"format\":1,\"contested\":[],\"pickups\":{\"A\":\"Item_A\"}}").is_err());
        assert!(parse_table("{\"format\":2}").is_err());
    }

    #[test]
    fn the_vanilla_game_reaches_all_tokens_but_the_one_above_the_theater() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let v: Placement = crate::slice::fill::vanilla_place(&w, &d).unwrap();
        assert_eq!(token_spots(&w, &v).len(), 100);
        assert_eq!(reachable_tokens(&w, &v), 99);
        assert_eq!(check_tokens(&w, &v, &v).unwrap(), 99);
    }

    #[test]
    fn the_default_leaves_every_skulltula_alone() {
        let d: Data = Data::load().unwrap();
        let p: Plan = plan(&Settings::default()).unwrap();
        assert_eq!(p.skulls, Mode::Off);
        let g: Generated = generate_data(&d, &Settings::default(), "skulls off", &GenOpts::default()).unwrap();
        assert!(!g.seed_file.placements.keys().any(|k| is_skull(&d, k)));
    }

    #[test]
    fn pinned_takes_the_confident_matches_and_matched_adds_equal_pools() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let mut left: BM<String, usize> = BM::new();
        let (pin, n): (Vec<LocId>, usize) = pick(&d, &w, &plan(&with("pinned")).unwrap(), &mut left);
        assert_eq!(n, 96);
        assert_eq!(pin.len(), 44);
        let (all, m): (Vec<LocId>, usize) = pick(&d, &w, &plan(&with("matched")).unwrap(), &mut BM::new());
        assert_eq!(m, 100);
        assert_eq!(all.len(), 77);
        assert!(pin.iter().all(|l| all.contains(l)));
        for l in &all {
            assert_eq!(d.vanilla_of(&w.locs[*l].name), Some(TOKEN));
        }
        let names: Vec<&str> = all.iter().map(|l| w.locs[*l].name.as_str()).collect();
        for n in ["LH GS Tree", "LH GS Lab Wall", "Shadow Temple GS Invisible Blades Room", "Colossus GS Hill", "Fire Temple GS Boss Key Loop", "Jabu Jabus Belly GS Near Boss"] {
            assert!(names.contains(&n), "{n}");
            assert!(!pin.iter().any(|l| w.locs[*l].name == n), "{n}");
        }
        for n in ["Water Temple GS Central Pillar", "Shadow Temple GS Triple Giant Pot", "Colossus GS Tree", "ZR GS Tree", "HF GS Cow Grotto", "Jabu Jabus Belly GS Water Switch Room"] {
            assert!(!names.contains(&n), "{n}");
        }
    }

    #[test]
    fn skulltulas_only_hold_shipped_pickups_and_every_token_stays_reachable() {
        let d: Data = Data::load().unwrap();
        let t: Table = table().unwrap();
        for v in ["pinned", "matched"] {
            let s: Settings = with(v);
            let mut moved: usize = 0;
            for n in 0..3 {
                let o: GenOpts = GenOpts { spoiler: true, ..GenOpts::default() };
                let g: Generated = generate_data(&d, &s, &format!("skulls {v} {n}"), &o).unwrap();
                let tokens: usize = g.seed_file.placements.values().filter(|i| i.as_str() == TOKEN).count();
                let held: usize = g.seed_file.placements.keys().filter(|k| is_skull(&d, k)).count();
                assert_eq!(tokens, held);
                for (loc, item) in &g.seed_file.placements {
                    if is_skull(&d, loc) {
                        let ok: bool = pickup(&t, &d, item).is_some() || crate::slice::key_spots::placeable(&d, item, crate::slice::key_spots::Spot::Skull);
                        assert!(ok, "{loc} holds {item}");
                        if item != TOKEN {
                            moved += 1;
                        }
                    }
                }
                let sp: String = g.spoiler.unwrap();
                assert!(sp.contains("Gold Skulltulas, 100 in the logic"), "{sp}");
                assert!(sp.contains("KF GS Know It All House:"));
                let held: String = if v == "matched" { crate::slice::skull_names::HELD.to_string() } else { "no remake actor".to_string() };
                assert!(sp.contains(&format!("ZR GS Tree: Gold Skulltula Token (vanilla, {held}")), "{sp}");
            }
            assert!(moved > 0, "{v}");
        }
    }
}
