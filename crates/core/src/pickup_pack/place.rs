use crate::byml::{Byml, Node};
use crate::catalog::Loc;
use crate::error::{Error as E, Res};
use crate::placement::hash_of;
use crate::pots::{self, TABLE};
use crate::slice::copies::hashes;
use crate::slice::data::Data;
use crate::slice::pickups::{swap_gyaml, world_actor};

pub const SWAPPABLE: [&str; 6] = ["Piece of Heart", "Recovery Heart", "Rupee (1)", "Rupees (5)", "Rupees (20)", "Rupees (50)"];

pub fn classes() -> Vec<&'static str> {
    SWAPPABLE.iter().filter_map(|i| world_actor(i)).collect()
}

pub fn saved(list: &Byml, hash: u64) -> Res<(i64, i64)> {
    let rows: &[Node] = list
        .root
        .get("Data")
        .and_then(|d| d.get(TABLE))
        .and_then(|t| t.as_arr())
        .ok_or(E::Format("pickup pack: the flag list has no 64-bit key table"))?;
    let r: &Node = rows
        .iter()
        .find(|r| hash_of(r) == Some(hash))
        .ok_or_else(|| E::Msg(format!("actor {hash} has no saved row, a pickup placed there would come back on every load")))?;
    Ok(pots::saved(r))
}

pub fn permanent(list: &Byml, hash: u64) -> Res<()> {
    let row: (i64, i64) = saved(list, hash)?;
    if !pots::permanent(row) {
        return Err(E::Msg(format!("actor {hash} has a saved row of reset type {} that the game clears, a pickup placed there would come back", row.0)));
    }
    Ok(())
}

pub fn single(d: &Data, hash: u64) -> Res<()> {
    let twin: Option<&Loc> = d.cat.locs.iter().chain(d.pots.locs.iter()).find(|l| hashes(l) > 1 && l.refs.iter().any(|r| r.hash == hash));
    match twin {
        Some(l) => Err(E::Msg(format!(
            "actor {hash} is one of {} copies of {}, the spot exists once per age with its own saved row, so an item placed there could be taken twice",
            hashes(l),
            l.name
        ))),
        None => Ok(()),
    }
}

pub fn has_hash(raw: &[u8], hash: u64) -> bool {
    let b: [u8; 8] = hash.to_le_bytes();
    raw.windows(8).any(|w| w == b)
}

pub fn current(y: &Byml, hash: u64) -> Option<String> {
    let list: &[Node] = y.root.get("Actors").and_then(|a| a.as_arr())?;
    let a: &Node = list.iter().find(|a| hash_of(a) == Some(hash))?;
    a.get("Gyaml").and_then(|v| v.as_str()).map(|s| s.to_string())
}

pub fn swap(y: &mut Byml, hash: u64, new: &str) -> Res<Option<String>> {
    let Some(old) = current(y, hash) else {
        return Ok(None);
    };
    if old == new {
        return Ok(Some(old));
    }
    if !classes().contains(&old.as_str()) {
        return Err(E::Msg(format!("actor {hash} is {old}, only a freestanding rupee, heart or heart piece spot can take a pickup")));
    }
    if swap_gyaml(y, hash, &old, new) == 0 {
        return Err(E::Msg(format!("actor {hash} could not be changed")));
    }
    Ok(Some(old))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn banc(rows: &[(u64, &str)]) -> Byml {
        let actors: Vec<Node> = rows
            .iter()
            .map(|r| {
                Node::Map(vec![
                    ("Gyaml".to_string(), Node::Str(r.1.to_string())),
                    ("Hash".to_string(), Node::U64(r.0)),
                    ("Translate".to_string(), Node::Arr(vec![Node::F32(1.0), Node::F32(2.0), Node::F32(3.0)])),
                ])
            })
            .collect();
        Byml { root: Node::Map(vec![("Actors".to_string(), Node::Arr(actors))]), version: 7, big: false }
    }

    #[test]
    fn the_six_freestanding_classes_come_from_the_existing_swap_table() {
        let c: Vec<&str> = classes();
        assert_eq!(c.len(), 6);
        assert!(c.contains(&"Obj_Rupee_Green") && c.contains(&"Obj_Heart_Piece"));
    }

    #[test]
    fn only_the_named_spot_changes_and_only_its_actor_name() {
        let mut y: Byml = banc(&[(11, "Obj_Rupee_Green"), (12, "Obj_Rupee_Green"), (13, "Obj_TBox_A")]);
        let before: Byml = y.clone();
        assert_eq!(swap(&mut y, 11, "Obj_Hookshot").unwrap().as_deref(), Some("Obj_Rupee_Green"));
        let a: &[Node] = y.root.get("Actors").unwrap().as_arr().unwrap();
        assert_eq!(a[0].get("Gyaml").and_then(|v| v.as_str()), Some("Obj_Hookshot"));
        assert_eq!(a[0].get("Hash"), Some(&Node::U64(11)));
        assert_eq!(a[0].get("Translate"), before.root.get("Actors").unwrap().as_arr().unwrap()[0].get("Translate"));
        assert_eq!(a[1], before.root.get("Actors").unwrap().as_arr().unwrap()[1]);
        assert_eq!(swap(&mut y, 11, "Obj_Hookshot").unwrap().as_deref(), Some("Obj_Hookshot"));
        assert_eq!(swap(&mut y, 99, "Obj_Hookshot").unwrap(), None);
        assert!(swap(&mut y, 13, "Obj_Hookshot").is_err());
    }

    #[test]
    fn a_spot_needs_a_permanent_saved_row() {
        let row = |h: u64, kind: i32, extra: bool| {
            let mut r: Node = Node::Map(vec![("Hash".to_string(), Node::U64(h)), ("ResetTypeValue".to_string(), Node::I32(kind))]);
            if extra {
                r.set("ExtraByte", Node::I32(7));
            }
            r
        };
        let table: Node = Node::Arr(vec![row(1, 24, false), row(2, 88, true), row(3, 24, true)]);
        let list: Byml = Byml { root: Node::Map(vec![("Data".to_string(), Node::Map(vec![(TABLE.to_string(), table)]))]), version: 7, big: true };
        assert_eq!(saved(&list, 1).unwrap(), (24, 0));
        assert_eq!(saved(&list, 3).unwrap(), (24, 7));
        assert!(permanent(&list, 1).is_ok());
        assert!(permanent(&list, 2).is_err());
        assert!(permanent(&list, 3).is_err());
        assert!(permanent(&list, 4).is_err());
    }

    #[test]
    fn a_spot_that_exists_once_per_age_is_refused() {
        let d: Data = Data::load().unwrap();
        let twin: &Loc = d.cat.by_name("KF Sarias House Recovery Heart 1").unwrap();
        assert_eq!(hashes(twin), 2);
        for r in &twin.refs {
            assert!(single(&d, r.hash).unwrap_err().to_string().contains("KF Sarias House Recovery Heart 1"));
        }
        let one: &Loc = d.cat.by_name("LLR Freestanding PoH").unwrap();
        assert!(single(&d, one.refs[0].hash).is_ok());
        assert!(single(&d, 11219803099333988363).is_ok());
        assert!(single(&d, 1).is_ok());
    }

    #[test]
    fn the_hash_is_searched_as_little_endian_bytes() {
        let y: Byml = banc(&[(0x1122_3344_5566_7788, "Obj_Rupee_Green")]);
        let raw: Vec<u8> = y.write().unwrap();
        assert!(has_hash(&raw, 0x1122_3344_5566_7788));
        assert!(!has_hash(&raw, 0x1122_3344_5566_7789));
    }
}
