use std::collections::HashSet as HS;

use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::json::{self, Json};
use crate::placement::{banc_files, hash_of, Edit};
use crate::romfs::Romfs;

#[derive(Debug, Clone, PartialEq)]
pub struct Pick {
    pub hash: u64,
    pub item: String,
    pub strip: bool,
}

pub fn picks(text: &str) -> Res<Vec<Pick>> {
    let j: Json = json::parse(text)?;
    let mut out: Vec<Pick> = Vec::new();
    for t in j.get("test_batch").and_then(|x| x.as_arr()).unwrap_or(&[]) {
        let hash: u64 = t
            .get("hash")
            .and_then(|x| x.as_str())
            .and_then(|x| x.parse().ok())
            .ok_or(Error::Format("test batch entry without a hash"))?;
        let item: String = t.get("new_item").and_then(|x| x.as_str()).unwrap_or("").to_string();
        if item.is_empty() {
            return Err(Error::Format("test batch entry without an item"));
        }
        let strip: bool = t.get("set_drop_flag").and_then(|x| x.as_str()) == Some("no-event");
        out.push(Pick { hash, item, strip });
    }
    Ok(out)
}

pub fn apply(y: &mut Byml, list: &[Pick]) -> Vec<u64> {
    let mut n: Vec<u64> = Vec::new();
    let Some(actors) = y.root.get_mut("Actors").and_then(|a| a.as_arr_mut()) else { return n };
    for a in actors.iter_mut() {
        let Some(h) = hash_of(a) else { continue };
        let Some(p) = list.iter().find(|p| p.hash == h) else { continue };
        let Some(d) = a.get_mut("Dynamic") else { continue };
        if d.get("Drop__DropActor").is_none() {
            continue;
        }
        d.set("Drop__DropActor", Node::Str(p.item.clone()));
        if p.strip {
            d.remove("GetEventName");
        }
        n.push(h);
    }
    n
}

pub fn edits(rf: &mut Romfs, list: &[Pick]) -> Res<Vec<Edit>> {
    let mut out: Vec<Edit> = Vec::new();
    let mut done: HS<u64> = HS::new();
    for f in banc_files(rf)? {
        let raw: Vec<u8> = rf.read_zs(&f)?;
        let mut y: Byml = Byml::parse(&raw)?;
        let hit: Vec<u64> = apply(&mut y, list);
        if !hit.is_empty() {
            done.extend(hit);
            out.push(Edit { file: f, old_len: raw.len(), bytes: y.write()? });
        }
    }
    let want: HS<u64> = list.iter().map(|p| p.hash).collect();
    if done != want {
        return Err(Error::Msg(format!("{} test batch chest(s) were not found", want.difference(&done).count())));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(h: u64, drop: &str, ev: bool) -> Node {
        let mut d: Vec<(String, Node)> = vec![("Drop__DropActor".to_string(), Node::Str(drop.to_string()))];
        if ev {
            d.push(("GetEventName".to_string(), Node::Str("E".to_string())));
        }
        Node::Map(vec![("Dynamic".to_string(), Node::Map(d)), ("Hash".to_string(), Node::U64(h))])
    }

    #[test]
    fn reads_the_batch_from_json() {
        let t: &str = r#"{"test_batch":[{"hash":"5","new_item":"I","set_drop_flag":"no-event"},{"hash":"6","new_item":"J","set_drop_flag":""}]}"#;
        let p: Vec<Pick> = picks(t).unwrap();
        assert_eq!(p, vec![Pick { hash: 5, item: "I".to_string(), strip: true }, Pick { hash: 6, item: "J".to_string(), strip: false }]);
        assert!(picks(r#"{"test_batch":[{"new_item":"I"}]}"#).is_err());
        assert!(picks(r#"{"test_batch":[{"hash":"1"}]}"#).is_err());
    }

    #[test]
    fn several_chests_in_one_file_are_edited_together() {
        let mut y: Byml = Byml {
            root: Node::Map(vec![("Actors".to_string(), Node::Arr(vec![actor(1, "A", true), actor(2, "B", true), actor(3, "C", true)]))]),
            version: 7,
            big: false,
        };
        let list: Vec<Pick> = vec![
            Pick { hash: 1, item: "X".to_string(), strip: true },
            Pick { hash: 2, item: "Y".to_string(), strip: false },
        ];
        assert_eq!(apply(&mut y, &list), vec![1, 2]);
        let a: &[Node] = y.root.get("Actors").and_then(|x| x.as_arr()).unwrap_or(&[]);
        assert!(a[0].get("Dynamic").and_then(|d| d.get("GetEventName")).is_none());
        assert!(a[1].get("Dynamic").and_then(|d| d.get("GetEventName")).is_some());
        assert_eq!(a[1].get("Dynamic").and_then(|d| d.get("Drop__DropActor")).and_then(|x| x.as_str()), Some("Y"));
        assert_eq!(a[2].get("Dynamic").and_then(|d| d.get("Drop__DropActor")).and_then(|x| x.as_str()), Some("C"));
        let b: Vec<u8> = y.write().unwrap();
        assert_eq!(Byml::parse(&b).unwrap(), y);
    }
}




