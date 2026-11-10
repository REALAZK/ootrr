use std::collections::BTreeSet as BS;

use crate::items::dump::Dump;
use crate::items::model::{Item, Seen, Table};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    pub issues: Vec<String>,
    pub notes: Vec<String>,
    pub actors: usize,
    pub in_actor_info: usize,
    pub in_packs: usize,
    pub in_pouch: usize,
    pub qty_checked: usize,
    pub claims: usize,
}

pub fn chest_evidence(it: &Item, d: &Dump) -> Seen {
    let own: Vec<&str> = it.own_actors();
    let n: usize = own.iter().filter(|a| d.chest.contains(**a)).count();
    if n == 0 {
        Seen::Guess
    } else if n == own.len() {
        Seen::Seen
    } else {
        Seen::Partial
    }
}

pub fn flow_evidence(it: &Item, d: &Dump) -> bool {
    it.actors().iter().any(|a| d.flow.contains(*a))
}

pub fn world_evidence(it: &Item, d: &Dump) -> bool {
    it.actors()
        .iter()
        .filter(|a| !a.starts_with("Shop_"))
        .any(|a| d.placed.contains(*a) || d.dropped.contains(*a))
}

pub fn shop_evidence(it: &Item, d: &Dump) -> bool {
    it.actors().iter().filter(|a| a.starts_with("Shop_")).any(|a| d.placed.contains(*a))
}

fn flag(r: &mut Report, it: &Item, ch: &str, have: Option<Seen>, want: bool) {
    if have.is_some() {
        r.claims += 1;
    }
    match (have, want) {
        (Some(Seen::Seen), true) | (None, false) => {}
        (Some(Seen::Seen), false) => r.issues.push(format!("{}: {ch} marked seen but the dump has no evidence", it.name)),
        (None, true) => r.issues.push(format!("{}: {ch} evidence exists in the dump but is not recorded", it.name)),
        (Some(_), _) => r.issues.push(format!("{}: {ch} may only be seen or absent", it.name)),
    }
}

pub fn verify(t: &Table, d: &Dump) -> Report {
    let mut r: Report = Report::default();
    let mut all: BS<&str> = BS::new();
    for it in &t.items {
        for a in it.actors() {
            all.insert(a);
        }
    }
    r.actors = all.len();
    let mut no_pack: BS<&str> = BS::new();
    let mut no_pouch: BS<&str> = BS::new();
    for a in &all {
        if d.actors.contains(*a) {
            r.in_actor_info += 1;
        }
        if d.packs.contains(*a) {
            r.in_packs += 1;
        } else {
            no_pack.insert(a);
        }
        if d.pouch.contains(*a) {
            r.in_pouch += 1;
        } else {
            no_pouch.insert(a);
        }
    }
    for it in &t.items {
        for a in it.actors() {
            if !d.actors.contains(a) {
                r.issues.push(format!("{}: actor {a} is not an ActorInfo row", it.name));
            }
        }
        if let (Some(q), Some(a)) = (it.qty, it.actor.as_deref()) {
            if let Some(n) = d.bundle.get(a) {
                r.qty_checked += 1;
                if *n != q {
                    r.issues.push(format!("{}: qty {q} but {a} has BundleActorNum {n}", it.name));
                }
            }
        }
        if it.via.chest.is_some() {
            r.claims += 1;
            let want: Seen = chest_evidence(it, d);
            if it.via.chest != Some(want) {
                r.issues.push(format!("{}: chest is {} but the dump says {}", it.name, it.via.chest.map(|s| s.name()).unwrap_or(""), want.name()));
            }
        }
        flag(&mut r, it, "flow", it.via.flow, flow_evidence(it, d));
        flag(&mut r, it, "world", it.via.world, world_evidence(it, d));
        flag(&mut r, it, "shop", it.via.shop, shop_evidence(it, d));
    }
    for a in no_pack {
        r.notes.push(format!("{a} has no pack file"));
    }
    for a in no_pouch {
        r.notes.push(format!("{a} has no PouchActorInfo row"));
    }
    r
}

pub fn unclaimed(t: &Table, d: &Dump) -> Vec<String> {
    let mut used: BS<&str> = BS::new();
    for it in &t.items {
        for a in it.actors() {
            used.insert(a);
        }
    }
    d.pouch
        .iter()
        .filter(|a| !["Pouch_", "Weapon_", "Armor_"].iter().any(|p| a.starts_with(p)))
        .filter(|a| !used.contains(a.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::load::parse;

    fn table() -> Table {
        parse(
            "{\"format\":1,\"items\":[\
            {\"name\":\"Hook\",\"kind\":\"Item\",\"actor\":\"A1\",\"conf\":\"exact\",\"mode\":\"tiers\",\"tiers\":[\"A1\",\"A2\"],\"via\":{\"chest\":\"partial\",\"flow\":\"seen\"}},\
            {\"name\":\"Nuts (5)\",\"kind\":\"Item\",\"actor\":\"N\",\"conf\":\"approx\",\"mode\":\"bundle\",\"qty\":3,\"world\":[\"W\"],\"why\":\"holds 3\",\"via\":{\"chest\":\"guess\",\"world\":\"seen\"}},\
            {\"name\":\"Buy\",\"kind\":\"Shop\",\"actor\":\"Shop_B\",\"conf\":\"exact\",\"mode\":\"shop\",\"qty\":10,\"via\":{\"shop\":\"seen\"}},\
            {\"name\":\"Event\",\"kind\":\"Event\",\"mode\":\"logic\",\"why\":\"token\"}\
            ]}",
        )
        .unwrap()
    }

    fn dump() -> Dump {
        let mut d: Dump = Dump::default();
        for a in ["A1", "A2", "N", "W", "Shop_B", "Extra_Item"] {
            d.actors.insert(a.to_string());
        }
        d.packs.insert("A1".to_string());
        d.pouch = ["A1", "N", "Shop_B", "Extra_Item", "Pouch_X", "Weapon_Y"].iter().map(|s| s.to_string()).collect();
        d.bundle.insert("N".to_string(), 3);
        d.bundle.insert("Shop_B".to_string(), 10);
        d.chest.insert("A1".to_string());
        d.placed.insert("W".to_string());
        d.placed.insert("Shop_B".to_string());
        d.flow.insert("A2".to_string());
        d
    }

    #[test]
    fn matching_evidence_passes() {
        let r: Report = verify(&table(), &dump());
        assert!(r.issues.is_empty(), "{:?}", r.issues);
        assert_eq!(r.actors, 5);
        assert_eq!(r.in_actor_info, 5);
        assert_eq!(r.in_packs, 1);
        assert_eq!(r.qty_checked, 2);
        assert!(r.notes.iter().any(|n| n.contains("A2 has no pack file")));
    }

    #[test]
    fn a_missing_actor_is_an_issue() {
        let mut d: Dump = dump();
        d.actors.remove("A2");
        let r: Report = verify(&table(), &d);
        assert!(r.issues.iter().any(|i| i.contains("A2 is not an ActorInfo row")));
    }

    #[test]
    fn wrong_quantities_are_an_issue() {
        let mut d: Dump = dump();
        d.bundle.insert("N".to_string(), 4);
        let r: Report = verify(&table(), &d);
        assert!(r.issues.iter().any(|i| i.contains("BundleActorNum 4")));
    }

    #[test]
    fn claims_and_evidence_must_agree_both_ways() {
        let mut d: Dump = dump();
        d.chest.insert("A2".to_string());
        d.flow.clear();
        d.placed.remove("W");
        d.placed.insert("Shop_B".to_string());
        d.dropped.insert("N".to_string());
        let r: Report = verify(&table(), &d);
        assert!(r.issues.iter().any(|i| i.contains("Hook: chest is partial but the dump says seen")));
        assert!(r.issues.iter().any(|i| i.contains("Hook: flow marked seen")));
        assert!(r.issues.iter().all(|i| !i.contains("Nuts (5): world")));
        assert!(r.issues.iter().all(|i| !i.contains("Buy: shop")));
    }

    #[test]
    fn unrecorded_evidence_is_an_issue() {
        let mut d: Dump = dump();
        d.flow.insert("N".to_string());
        let r: Report = verify(&table(), &d);
        assert!(r.issues.iter().any(|i| i.contains("Nuts (5): flow evidence exists")));
    }

    #[test]
    fn unclaimed_lists_pouch_rows_nothing_maps_to() {
        let u: Vec<String> = unclaimed(&table(), &dump());
        assert_eq!(u, vec!["Extra_Item"]);
    }
}
