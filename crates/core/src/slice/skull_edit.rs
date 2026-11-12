use std::collections::BTreeSet as BS;

use crate::byml::{Byml, Node};
use crate::error::{Error as E, Res};
use crate::placement::hash_of;
use crate::pots::DROP;
use crate::romfs::Romfs;
use crate::settings::seed::SeedFile;
use crate::slice::data::Data;
use crate::slice::edit::Edited;
use crate::slice::skulls::{is_skull, pickup, table, Table, TOKEN};

pub const SKULL: &str = "Enemy_Skulltula_Gold";
pub const LISTED: usize = 10;

#[derive(Debug, Clone, PartialEq)]
pub struct Drop {
    pub name: String,
    pub item: String,
    pub hash: u64,
    pub actor: String,
}

pub fn vanilla_actor(d: &Data, loc: &str) -> Option<String> {
    if !is_skull(d, loc) {
        return None;
    }
    d.item(TOKEN).and_then(|i| i.actor.clone())
}

pub fn drops(d: &Data, seed: &SeedFile) -> Res<Vec<Drop>> {
    let t: Table = table()?;
    let mut out: Vec<Drop> = Vec::new();
    for (loc, item) in seed.placements.iter().filter(|(l, _)| is_skull(d, l)) {
        if item == TOKEN {
            continue;
        }
        let actor: String = pickup(&t, d, item).ok_or_else(|| E::Msg(format!("{item} has no world pickup, so {loc} cannot drop it")))?;
        let l: &crate::catalog::Loc = d.cat.by_name(loc).ok_or_else(|| E::Msg(format!("{loc} is not in the catalog")))?;
        for r in &l.refs {
            out.push(Drop { name: loc.clone(), item: item.clone(), hash: r.hash, actor: actor.clone() });
        }
    }
    Ok(out)
}

pub fn set_drop(y: &mut Byml, hash: u64, actor: &str) -> Res<usize> {
    let Some(list) = y.root.get_mut("Actors").and_then(|a| a.as_arr_mut()) else {
        return Ok(0);
    };
    let mut n: usize = 0;
    for a in list.iter_mut() {
        if hash_of(a) != Some(hash) {
            continue;
        }
        if a.get("Gyaml").and_then(|g| g.as_str()) != Some(SKULL) {
            return Err(E::Msg(format!("actor {hash} is not a Gold Skulltula, the game files differ")));
        }
        match a.get_mut("Dynamic") {
            Some(d) => {
                d.set(DROP, Node::Str(actor.to_string()));
            }
            None => {
                a.set("Dynamic", Node::Map(vec![(DROP.to_string(), Node::Str(actor.to_string()))]));
            }
        }
        n += 1;
    }
    Ok(n)
}

pub fn apply(file: &str, raw: &[u8], prev: Option<Edited>, ds: &[Drop], hit: &mut BS<u64>) -> Res<Option<Edited>> {
    if ds.is_empty() {
        return Ok(prev);
    }
    let mut y: Byml = match &prev {
        Some(e) => Byml::parse(&e.bytes)?,
        None => Byml::parse(raw)?,
    };
    let mut n: usize = 0;
    for d in ds {
        let k: usize = set_drop(&mut y, d.hash, &d.actor)?;
        if k > 0 {
            hit.insert(d.hash);
            n += k;
        }
    }
    if n == 0 {
        return Ok(prev);
    }
    let bytes: Vec<u8> = y.write()?;
    Ok(Some(match prev {
        Some(mut e) => {
            e.bytes = bytes;
            e
        }
        None => Edited {
            file: file.to_string(),
            old_len: raw.len(),
            bytes,
            hits: Vec::new(),
            used: Vec::new(),
            hearts: Vec::new(),
            wraps: Vec::new(),
            touched: Vec::new(),
        },
    }))
}

pub fn all_found(ds: &[Drop], hit: &BS<u64>) -> Res<()> {
    match ds.iter().find(|d| !hit.contains(&d.hash)) {
        Some(d) => Err(E::Msg(format!("{} (actor {}) was not found in the scenes, the game files differ", d.name, d.hash))),
        None => Ok(()),
    }
}

pub fn check_packs(rf: &Romfs, ds: &[Drop]) -> Res<()> {
    let actors: BS<&str> = ds.iter().map(|d| d.actor.as_str()).collect();
    for a in actors {
        if !rf.root.join("Pack").join("Actor").join(format!("{a}.pack.zs")).is_file() {
            return Err(E::Msg(format!("the game folder has no pickup actor {a}, a skulltula cannot drop it")));
        }
    }
    Ok(())
}

pub fn notes(ds: &[Drop]) -> Vec<String> {
    if ds.is_empty() {
        return Vec::new();
    }
    let names: BS<(&str, &str, &str)> = ds.iter().map(|d| (d.name.as_str(), d.item.as_str(), d.actor.as_str())).collect();
    let mut out: Vec<String> = vec![format!(
        "{} Gold Skulltulas drop another item than the token and the skulltula death table now keeps every skulltula dead once killed. Check these in game ({} of {}):",
        names.len(),
        names.len().min(LISTED),
        names.len()
    )];
    for (n, i, a) in names.iter().take(LISTED) {
        out.push(format!("  {n} | drops {i} ({a})"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Settings, Value};
    use crate::slice::generate::{generate_data, GenOpts, Generated};

    fn actor(gyaml: &str, h: u64, dynamic: Option<Node>) -> Node {
        let mut m: Vec<(String, Node)> = vec![("Gyaml".to_string(), Node::Str(gyaml.to_string())), ("Hash".to_string(), Node::U64(h))];
        if let Some(d) = dynamic {
            m.push(("Dynamic".to_string(), d));
        }
        Node::Map(m)
    }

    fn raw(list: Vec<Node>) -> Vec<u8> {
        Byml { root: Node::Map(vec![("Actors".to_string(), Node::Arr(list))]), version: 7, big: false }.write().unwrap()
    }

    fn drop(h: u64, a: &str) -> Drop {
        Drop { name: "GS".to_string(), item: "Piece of Heart".to_string(), hash: h, actor: a.to_string() }
    }

    #[test]
    fn a_skulltula_gets_a_drop_and_keeps_its_other_dynamic_keys() {
        let night: Node = Node::Map(vec![("Enemy_Skulltula_Gold_Nocturnal".to_string(), Node::Bool(true))]);
        let b: Vec<u8> = raw(vec![actor(SKULL, 1, Some(night)), actor(SKULL, 2, None), actor("Obj_Pot_A_01", 3, None)]);
        let mut hit: BS<u64> = BS::new();
        let e: Edited = apply("Banc/A/a.bcett.byml.zs", &b, None, &[drop(1, "Obj_Heart_Piece"), drop(2, "Obj_Rupee_Red")], &mut hit).unwrap().unwrap();
        assert_eq!(hit.len(), 2);
        assert!(e.hits.is_empty());
        let y: Byml = Byml::parse(&e.bytes).unwrap();
        let a: &[Node] = y.root.get("Actors").unwrap().as_arr().unwrap();
        let d0: &Node = a[0].get("Dynamic").unwrap();
        assert_eq!(d0.get(DROP).and_then(|v| v.as_str()), Some("Obj_Heart_Piece"));
        assert_eq!(d0.get("Enemy_Skulltula_Gold_Nocturnal"), Some(&Node::Bool(true)));
        assert_eq!(a[1].get("Dynamic").and_then(|d| d.get(DROP)).and_then(|v| v.as_str()), Some("Obj_Rupee_Red"));
        assert!(a[2].get("Dynamic").is_none());
        assert!(all_found(&[drop(1, "x"), drop(2, "x")], &hit).is_ok());
        assert!(all_found(&[drop(9, "x")], &hit).is_err());
    }

    #[test]
    fn a_hash_on_another_actor_is_refused_and_a_miss_changes_nothing() {
        let b: Vec<u8> = raw(vec![actor("Obj_Pot_A_01", 3, None)]);
        let mut hit: BS<u64> = BS::new();
        assert!(apply("f", &b, None, &[drop(3, "Obj_Heart")], &mut hit).is_err());
        assert!(apply("f", &b, None, &[drop(4, "Obj_Heart")], &mut hit).unwrap().is_none());
        assert!(apply("f", &b, None, &[], &mut hit).unwrap().is_none());
    }

    #[test]
    fn a_generated_seed_turns_into_drops_with_world_pickups_only() {
        let d: Data = Data::load().unwrap();
        let mut s: Settings = Settings::default();
        s.set("shuffle_skulltulas", Value::s("matched")).unwrap();
        let mut seen: usize = 0;
        for n in 0..3 {
            let g: Generated = generate_data(&d, &s, &format!("drops {n}"), &GenOpts::default()).unwrap();
            let ds: Vec<Drop> = drops(&d, &g.seed_file).unwrap();
            for x in &ds {
                assert!(x.actor.starts_with("Obj_"), "{}", x.actor);
                assert_ne!(x.item, TOKEN);
                assert!(is_skull(&d, &x.name));
            }
            seen += ds.len();
            assert_eq!(vanilla_actor(&d, "KF GS Bean Patch").as_deref(), Some("Item_Token_Gold"));
            assert!(vanilla_actor(&d, "KF Midos Top Left Chest").is_none());
        }
        assert!(seen > 0);
        assert!(!notes(&[drop(1, "Obj_Heart")]).is_empty());
    }
}
