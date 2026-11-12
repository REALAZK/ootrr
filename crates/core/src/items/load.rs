use std::collections::HashSet as HS;

use crate::error::{Error as E, Res};
use crate::items::model::{Item, Mode, Seen, Table, Via};
use crate::json::{self, Json};

pub const ITEMS_JSON: &str = include_str!("../../../../data/items.json");

const CONFS: [&str; 3] = ["exact", "convention", "approx"];

fn opt(j: &Json, k: &str) -> Option<String> {
    j.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn names(j: &Json, k: &str, who: &str) -> Res<Vec<String>> {
    let Some(v) = j.get(k) else {
        return Ok(Vec::new());
    };
    let a: &[Json] = v.as_arr().ok_or_else(|| E::Msg(format!("{who}: {k} must be a list")))?;
    let mut out: Vec<String> = Vec::new();
    for x in a {
        let s: &str = x.as_str().filter(|s| !s.is_empty()).ok_or_else(|| E::Msg(format!("{who}: {k} holds a bad name")))?;
        out.push(s.to_string());
    }
    Ok(out)
}

fn via(j: &Json, who: &str) -> Res<Via> {
    let mut out: Via = Via::default();
    let Some(v) = j.get("via") else {
        return Ok(out);
    };
    let o: &[(String, Json)] = v.as_obj().ok_or_else(|| E::Msg(format!("{who}: via must be an object")))?;
    for (k, x) in o {
        let s: Seen = x
            .as_str()
            .and_then(Seen::from_name)
            .ok_or_else(|| E::Msg(format!("{who}: via {k} has a bad value")))?;
        match k.as_str() {
            "chest" => out.chest = Some(s),
            "flow" => out.flow = Some(s),
            "world" => out.world = Some(s),
            "shop" => out.shop = Some(s),
            _ => return Err(E::Msg(format!("{who}: unknown via channel {k}"))),
        }
    }
    Ok(out)
}

pub fn name_amount(name: &str) -> Option<i64> {
    let a: usize = name.rfind('(')?;
    let b: usize = name[a..].find(')')? + a;
    let t: &str = &name[a + 1..b];
    if t.is_empty() || !t.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    t.parse().ok()
}

fn entry(j: &Json) -> Res<Item> {
    let name: String = j
        .get("name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or(E::Format("items file: an item has no name"))?
        .to_string();
    let kind: String = opt(j, "kind").ok_or_else(|| E::Msg(format!("{name}: kind is missing")))?;
    let actor: Option<String> = opt(j, "actor");
    let mode: Mode = match opt(j, "mode") {
        Some(m) => Mode::from_name(&m).ok_or_else(|| E::Msg(format!("{name}: unknown mode {m}")))?,
        None if actor.is_some() => Mode::Actor,
        None => Mode::None,
    };
    Ok(Item {
        adv: j.get("adv").and_then(|v| v.as_bool()).unwrap_or(false),
        conf: opt(j, "conf").unwrap_or_default(),
        dungeon: opt(j, "dungeon"),
        tiers: names(j, "tiers", &name)?,
        world: names(j, "world", &name)?,
        qty: j.get("qty").and_then(|v| v.as_i64()),
        via: via(j, &name)?,
        why: opt(j, "why").unwrap_or_default(),
        name,
        kind,
        actor,
        mode,
    })
}

fn bad(it: &Item, m: &str) -> E {
    E::Msg(format!("{}: {m}", it.name))
}

pub fn validate(it: &Item) -> Res<()> {
    let has: bool = it.actor.is_some();
    match it.mode {
        Mode::Actor | Mode::Generic | Mode::Bundle | Mode::Bottle | Mode::Shop => {
            if !has || !it.tiers.is_empty() {
                return Err(bad(it, "this mode needs one actor and no tiers"));
            }
        }
        Mode::Tiers => {
            if it.tiers.len() < 2 || it.actor != it.tiers.first().cloned() {
                return Err(bad(it, "tiers needs two or more actors and actor must be the first"));
            }
            let mut seen: HS<&str> = HS::new();
            if !it.tiers.iter().all(|t| seen.insert(t.as_str())) {
                return Err(bad(it, "a tier is listed twice"));
            }
        }
        Mode::Logic | Mode::None => {
            if has || !it.tiers.is_empty() || it.qty.is_some() || it.via.chest.is_some() || it.via.shop.is_some() {
                return Err(bad(it, "an item without an actor cannot carry tiers, qty or chest and shop evidence"));
            }
            if it.why.is_empty() {
                return Err(bad(it, "an item without an actor needs a reason"));
            }
        }
    }
    if has && !CONFS.contains(&it.conf.as_str()) {
        return Err(bad(it, "conf must be exact, convention or approx"));
    }
    if !has && !it.conf.is_empty() {
        return Err(bad(it, "conf without an actor"));
    }
    if it.mode == Mode::Shop && it.via.chest.is_some() {
        return Err(bad(it, "a shop slot actor has no chest channel"));
    }
    if it.mode != Mode::Tiers && it.via.chest == Some(Seen::Partial) {
        return Err(bad(it, "partial chest evidence only makes sense for tiers"));
    }
    if it.conf == "approx" && (it.qty.is_none() || it.why.is_empty()) {
        return Err(bad(it, "approx needs a qty and a reason"));
    }
    if let (Some(q), Some(n)) = (it.qty, name_amount(&it.name)) {
        if it.conf == "exact" && q != n {
            return Err(bad(it, "exact conf but qty differs from the amount in the name"));
        }
        if it.conf == "approx" && q == n {
            return Err(bad(it, "approx conf but qty equals the amount in the name"));
        }
    }
    Ok(())
}

pub fn parse(t: &str) -> Res<Table> {
    let j: Json = json::parse(t)?;
    if j.get("format").and_then(|v| v.as_i64()) != Some(1) {
        return Err(E::Format("items file: unsupported format"));
    }
    let list: &[Json] = j.get("items").and_then(|v| v.as_arr()).ok_or(E::Format("items file: no items list"))?;
    let mut items: Vec<Item> = Vec::with_capacity(list.len());
    let mut seen: HS<String> = HS::new();
    for e in list {
        let it: Item = entry(e)?;
        if !seen.insert(it.name.clone()) {
            return Err(E::Msg(format!("item {} is listed twice", it.name)));
        }
        validate(&it)?;
        items.push(it);
    }
    Ok(Table::new(items))
}

pub fn load() -> Res<Table> {
    parse(ITEMS_JSON)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(items: &str) -> String {
        format!("{{\"format\":1,\"items\":[{items}]}}")
    }

    #[test]
    fn a_small_file_parses_with_every_field() {
        let t: Table = parse(&doc(
            "{\"name\":\"Hook\",\"kind\":\"Item\",\"adv\":true,\"actor\":\"A1\",\"conf\":\"exact\",\"mode\":\"tiers\",\"tiers\":[\"A1\",\"A2\"],\"via\":{\"chest\":\"partial\",\"flow\":\"seen\"}},\
             {\"name\":\"Nuts (5)\",\"kind\":\"Item\",\"actor\":\"N\",\"conf\":\"approx\",\"mode\":\"bundle\",\"qty\":3,\"world\":[\"W\"],\"why\":\"holds 3\"},\
             {\"name\":\"Event\",\"kind\":\"Event\",\"mode\":\"logic\",\"why\":\"token\"}",
        ))
        .unwrap();
        let h: &Item = t.get("Hook").unwrap();
        assert_eq!(h.mode, Mode::Tiers);
        assert_eq!(h.tiers, vec!["A1", "A2"]);
        assert_eq!(h.via.chest, Some(Seen::Partial));
        assert_eq!(h.via.flow, Some(Seen::Seen));
        assert!(h.adv);
        let n: &Item = t.get("Nuts (5)").unwrap();
        assert_eq!((n.qty, n.world.clone()), (Some(3), vec!["W".to_string()]));
        assert_eq!(t.get("Event").unwrap().mode, Mode::Logic);
    }

    #[test]
    fn the_old_slice_format_still_loads() {
        let e: E = parse(&doc(
            "{\"name\":\"Boomerang\",\"kind\":\"Item\",\"adv\":true,\"actor\":\"Item_Boomerang\",\"conf\":\"exact\"},\
             {\"name\":\"Cojiro\",\"kind\":\"Item\",\"adv\":true}",
        ))
        .unwrap_err();
        assert!(e.to_string().contains("reason"));
        let t: Table = parse(&doc("{\"name\":\"Boomerang\",\"kind\":\"Item\",\"adv\":true,\"actor\":\"Item_Boomerang\",\"conf\":\"exact\"}")).unwrap();
        assert_eq!(t.get("Boomerang").unwrap().mode, Mode::Actor);
    }

    #[test]
    fn broken_entries_are_refused_with_the_item_name() {
        let cases: [(&str, &str); 12] = [
            ("{\"kind\":\"Item\"}", "no name"),
            ("{\"name\":\"A\"}", "kind"),
            ("{\"name\":\"A\",\"kind\":\"Item\",\"mode\":\"zzz\"}", "unknown mode"),
            ("{\"name\":\"A\",\"kind\":\"Item\",\"mode\":\"actor\",\"conf\":\"exact\"}", "one actor"),
            ("{\"name\":\"A\",\"kind\":\"Item\",\"mode\":\"tiers\",\"actor\":\"X\",\"conf\":\"exact\",\"tiers\":[\"Y\",\"Z\"]}", "first"),
            ("{\"name\":\"A\",\"kind\":\"Item\",\"mode\":\"tiers\",\"actor\":\"X\",\"conf\":\"exact\",\"tiers\":[\"X\",\"X\"]}", "twice"),
            ("{\"name\":\"A\",\"kind\":\"Item\",\"actor\":\"X\",\"conf\":\"wild\"}", "conf must"),
            ("{\"name\":\"A\",\"kind\":\"Item\",\"actor\":\"X\",\"conf\":\"approx\",\"qty\":1}", "approx needs"),
            ("{\"name\":\"A (5)\",\"kind\":\"Item\",\"actor\":\"X\",\"conf\":\"exact\",\"qty\":3}", "differs"),
            ("{\"name\":\"A (5)\",\"kind\":\"Item\",\"actor\":\"X\",\"conf\":\"approx\",\"qty\":5,\"why\":\"w\"}", "equals"),
            ("{\"name\":\"A\",\"kind\":\"Item\",\"actor\":\"X\",\"conf\":\"exact\",\"via\":{\"mail\":\"seen\"}}", "unknown via"),
            ("{\"name\":\"A\",\"kind\":\"Item\",\"actor\":\"Shop_X\",\"conf\":\"exact\",\"mode\":\"shop\",\"via\":{\"chest\":\"seen\"}}", "chest channel"),
        ];
        for (c, want) in cases {
            let e: String = parse(&doc(c)).unwrap_err().to_string();
            assert!(e.contains(want), "{c} gave {e}");
        }
        let dup: String = doc("{\"name\":\"A\",\"kind\":\"Item\",\"why\":\"w\"},{\"name\":\"A\",\"kind\":\"Item\",\"why\":\"w\"}");
        assert!(parse(&dup).unwrap_err().to_string().contains("listed twice"));
        assert!(parse("{\"format\":2,\"items\":[]}").is_err());
        assert!(parse("{\"format\":1}").is_err());
    }

    #[test]
    fn amounts_come_from_the_last_parenthesis() {
        assert_eq!(name_amount("Bombs (5)"), Some(5));
        assert_eq!(name_amount("Buy Bombs (5) for 25 Rupees"), Some(5));
        assert_eq!(name_amount("Rupees (Treasure Chest Game) (20)"), Some(20));
        assert_eq!(name_amount("Map (Ice Cavern)"), None);
        assert_eq!(name_amount("Bombchus"), None);
        assert_eq!(name_amount("Odd ()"), None);
    }

    #[test]
    fn the_shipped_file_loads_and_is_complete() {
        let t: Table = load().unwrap();
        assert_eq!(t.items.len(), 289);
        let c = t.counts();
        assert_eq!(c.mapped + c.logic + c.unmapped, c.total);
        assert!(c.mapped >= 190);
        for it in &t.items {
            assert!(it.mode.mapped() || !it.why.is_empty());
        }
    }

    #[test]
    fn the_shipped_progressive_chains_are_ordered() {
        let t: Table = load().unwrap();
        let h: &Item = t.get("Progressive Hookshot").unwrap();
        assert_eq!(h.actor_for(0), Some("Item_Hookshot"));
        assert_eq!(h.actor_for(1), Some("Item_Longshot"));
        assert_eq!(h.actor_for(2), None);
        let s: &Item = t.get("Progressive Strength Upgrade").unwrap();
        assert_eq!(s.tiers.len(), 3);
        assert_eq!(t.get("Bow").and_then(|i| i.actor.as_deref()), Some("Item_Range_Bow"));
        assert_eq!(t.get("Bomb Bag").and_then(|i| i.actor.as_deref()), Some("Item_BombBag_Small"));
    }

    #[test]
    fn the_shipped_dungeon_items_resolve_from_the_pickup_dungeon() {
        let t: Table = load().unwrap();
        assert_eq!(t.generic("Item_Dungeon_SmallKey", "Forest Temple").map(|i| i.name.as_str()), Some("Small Key (Forest Temple)"));
        assert_eq!(t.generic("Item_Dungeon_Map", "Ice Cavern").map(|i| i.name.as_str()), Some("Map (Ice Cavern)"));
        assert_eq!(t.generic("Item_Dungeon_BossKey", "Ganons Castle").map(|i| i.name.as_str()), Some("Boss Key (Ganons Castle)"));
        assert!(t.get("Small Key (Forest Temple)").is_some_and(|i| i.dungeon_scoped()));
        assert!(t.get("Small Key Ring (Forest Temple)").is_some_and(|i| i.mode == Mode::None));
    }

    #[test]
    fn the_shipped_amounts_match_their_conf() {
        let t: Table = load().unwrap();
        for n in ["Rupees (5)", "Arrows (10)", "Bombchus (20)", "Bombs (5)"] {
            let i: &Item = t.get(n).unwrap();
            assert_eq!(i.conf, "exact");
            assert_eq!(i.qty, name_amount(n));
        }
        assert_eq!(t.get("Deku Nuts (5)").map(|i| i.conf.as_str()), Some("approx"));
    }

    #[test]
    fn the_old_chest_actors_are_unchanged() {
        let t: Table = load().unwrap();
        for (n, a) in [("Kokiri Sword", "Item_Sword_Kokiri"), ("Piece of Heart", "Obj_Heart_Piece"), ("Iron Boots", "Item_HeavyBoot"), ("Rupees (50)", "TBox_Rupee_Purple")] {
            assert_eq!(t.get(n).and_then(|i| i.actor.as_deref()), Some(a));
        }
    }
}
