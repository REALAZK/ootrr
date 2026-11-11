use std::collections::{BTreeMap as BM, BTreeSet as BS};

use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::murmur::murmur3_32;

pub const LIST: &str = "GameData/GameDataList.Product.110.byml.zs";
pub const OPEN: &str = "IsTreasureBoxOpen";
pub const ROOMS: &str = "DungeonRoomStep";
pub const SPARE_MAP: &str = "TestField";

pub fn key(s: &str) -> u32 {
    murmur3_32(s.as_bytes(), 0)
}

pub fn member(hash: u64) -> u32 {
    key(&hash.to_string())
}

fn num(n: &Node) -> Option<u32> {
    match n {
        Node::U32(x) => Some(*x),
        Node::I32(x) => Some(*x as u32),
        _ => None,
    }
}

fn field(n: &Node, k: &str) -> Option<u32> {
    n.get(k).and_then(num)
}

fn list<'a>(y: &'a Byml, kind: &str) -> &'a [Node] {
    y.root.get("Data").and_then(|d| d.get(kind)).and_then(|l| l.as_arr()).unwrap_or(&[])
}

fn members<'a>(y: &'a Byml, hash: u32) -> &'a [Node] {
    list(y, "Struct")
        .iter()
        .find(|s| field(s, "Hash") == Some(hash))
        .and_then(|s| s.get("DefaultValue"))
        .and_then(|m| m.as_arr())
        .unwrap_or(&[])
}

fn value(m: &[Node], k: u32) -> Option<u32> {
    m.iter().find(|x| field(x, "Hash") == Some(k)).and_then(|x| field(x, "Value"))
}

pub fn spares(y: &Byml) -> Vec<u32> {
    let bools: BS<u32> = list(y, "Bool").iter().filter_map(|b| field(b, "Hash")).collect();
    let used: BS<u32> = members(y, key(OPEN)).iter().filter_map(|m| field(m, "Value")).collect();
    let Some(map) = value(members(y, key(ROOMS)), key(SPARE_MAP)) else {
        return Vec::new();
    };
    let mut out: BS<u32> = BS::new();
    for room in members(y, map) {
        let Some(r) = field(room, "Value") else {
            continue;
        };
        for m in members(y, r) {
            if let Some(v) = field(m, "Value") {
                if bools.contains(&v) && !used.contains(&v) {
                    out.insert(v);
                }
            }
        }
    }
    out.into_iter().collect()
}

pub fn has(y: &Byml, hash: u64) -> bool {
    value(members(y, key(OPEN)), member(hash)).is_some()
}

pub fn names_of(hashes: impl Iterator<Item = u64>) -> BM<u32, String> {
    hashes.map(|h| (member(h), h.to_string())).collect()
}

pub fn add(y: &mut Byml, hash: u64, spare: u32, names: &BM<u32, String>) -> Res<()> {
    let want: u32 = key(OPEN);
    let all: &mut Vec<Node> = y
        .root
        .get_mut("Data")
        .and_then(|d| d.get_mut("Struct"))
        .and_then(|l| l.as_arr_mut())
        .ok_or(Error::Format("flag list has no struct table"))?;
    let m: &mut Vec<Node> = all
        .iter_mut()
        .find(|s| field(s, "Hash") == Some(want))
        .and_then(|s| s.get_mut("DefaultValue"))
        .and_then(|v| v.as_arr_mut())
        .ok_or(Error::Format("flag list has no opened chest struct"))?;
    let mine: String = hash.to_string();
    let known: Vec<(usize, &String)> = m
        .iter()
        .enumerate()
        .filter_map(|(i, x)| field(x, "Hash").and_then(|k| names.get(&k)).map(|n| (i, n)))
        .collect();
    if known.windows(2).any(|w| w[0].1 > w[1].1) {
        return Err(Error::Format("opened chest members are not in name order"));
    }
    let at: usize = known.iter().find(|(_, n)| **n > mine).map(|(i, _)| *i).unwrap_or(m.len());
    m.insert(
        at,
        Node::Map(vec![("Hash".to_string(), Node::U32(member(hash))), ("Value".to_string(), Node::U32(spare))]),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(h: u32, v: u32) -> Node {
        Node::Map(vec![("Hash".to_string(), Node::U32(h)), ("Value".to_string(), Node::U32(v))])
    }

    fn st(h: u32, m: Vec<Node>) -> Node {
        Node::Map(vec![("DefaultValue".to_string(), Node::Arr(m)), ("Hash".to_string(), Node::U32(h))])
    }

    fn flag(h: u32) -> Node {
        Node::Map(vec![("Hash".to_string(), Node::U32(h))])
    }

    fn sample() -> Byml {
        let structs: Vec<Node> = vec![
            st(key(OPEN), vec![pair(member(5), 900)]),
            st(key(ROOMS), vec![pair(key(SPARE_MAP), 50), pair(key("FireTemple"), 60)]),
            st(50, vec![pair(key("DungeonRoom000"), 51)]),
            st(51, vec![pair(key("Step"), 700), pair(key("ClearRoom"), 902), pair(key("ClearAutoSave"), 901)]),
            st(60, vec![pair(key("DungeonRoom000"), 61)]),
            st(61, vec![pair(key("ClearRoom"), 903)]),
        ];
        let bools: Vec<Node> = vec![flag(900), flag(901), flag(902), flag(903)];
        let data: Node = Node::Map(vec![
            ("Bool".to_string(), Node::Arr(bools)),
            ("Struct".to_string(), Node::Arr(structs)),
        ]);
        Byml { root: Node::Map(vec![("Data".to_string(), data)]), version: 2, big: false }
    }

    #[test]
    fn the_chest_flag_is_keyed_by_the_decimal_actor_hash() {
        assert_eq!(key("IsTreasureBoxOpen.10831981390482426294"), 0x316d8a20);
        assert_eq!(key("BeenInTheRoom"), 0xed5a77bc);
    }

    #[test]
    fn only_bools_of_the_unused_test_map_are_spare() {
        assert_eq!(spares(&sample()), vec![901, 902]);
    }

    #[test]
    fn a_new_chest_takes_a_spare_and_the_spare_is_then_used() {
        let mut y: Byml = sample();
        assert!(!has(&y, 7));
        add(&mut y, 7, 901, &names_of([5u64, 7].into_iter())).unwrap();
        assert!(has(&y, 7));
        assert!(has(&y, 5));
        assert_eq!(spares(&y), vec![902]);
    }

    #[test]
    fn a_new_member_lands_in_decimal_name_order() {
        let mut y: Byml = sample();
        let names: BM<u32, String> = names_of([5u64, 30, 7, 100].into_iter());
        add(&mut y, 30, 901, &names).unwrap();
        add(&mut y, 7, 902, &names).unwrap();
        let m: &[Node] = members(&y, key(OPEN));
        let order: Vec<u32> = m.iter().filter_map(|x| field(x, "Hash")).collect();
        assert_eq!(order, vec![member(30), member(5), member(7)]);
    }
}
