use std::collections::BTreeMap as Map;

use crate::byml::{Byml, Node};
use crate::error::Res;
use crate::placement::{banc_files, hash_of};
use crate::romfs::Romfs;

#[derive(Debug, Clone, PartialEq)]
pub struct Chest {
    pub hash: u64,
    pub gyaml: String,
    pub files: Vec<String>,
    pub pos: Vec<f32>,
    pub drop: String,
    pub event: String,
    pub extra: Vec<(String, String)>,
    pub agree: bool,
}

fn text(n: &Node, k: &str) -> String {
    n.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn pos_of(n: &Node) -> Vec<f32> {
    n.get("Translate")
        .and_then(|t| t.as_arr())
        .map(|t| t.iter().filter_map(|x| if let Node::F32(v) = x { Some(*v) } else { None }).collect())
        .unwrap_or_default()
}

fn scalar(n: &Node) -> String {
    match n {
        Node::Str(s) => s.clone(),
        Node::Bool(b) => b.to_string(),
        Node::I32(x) => x.to_string(),
        Node::U32(x) => x.to_string(),
        Node::F32(x) => x.to_string(),
        _ => "?".to_string(),
    }
}

pub fn read(f: &str, a: &Node) -> Option<Chest> {
    let d: &Node = a.get("Dynamic")?;
    let event: String = d.get("GetEventName")?.as_str()?.to_string();
    let extra: Vec<(String, String)> = match d {
        Node::Map(m) => m
            .iter()
            .filter(|e| e.0 != "GetEventName" && e.0 != "Drop__DropActor")
            .map(|e| (e.0.clone(), scalar(&e.1)))
            .collect(),
        _ => Vec::new(),
    };
    Some(Chest {
        hash: hash_of(a)?,
        gyaml: text(a, "Gyaml"),
        files: vec![f.to_string()],
        pos: pos_of(a),
        drop: text(d, "Drop__DropActor"),
        event,
        extra,
        agree: true,
    })
}

pub fn merge(out: &mut Map<u64, Chest>, c: Chest) {
    match out.get_mut(&c.hash) {
        None => {
            out.insert(c.hash, c);
        }
        Some(x) => {
            x.agree &= x.drop == c.drop && x.event == c.event && x.pos == c.pos;
            x.files.extend(c.files);
        }
    }
}

pub fn chests(rf: &mut Romfs) -> Res<Vec<Chest>> {
    let mut out: Map<u64, Chest> = Map::new();
    for f in banc_files(rf)? {
        let y: Byml = rf.read_byml(&f)?;
        for a in y.root.get("Actors").and_then(|x| x.as_arr()).unwrap_or(&[]) {
            if let Some(c) = read(&f, a) {
                merge(&mut out, c);
            }
        }
    }
    let mut v: Vec<Chest> = out.into_values().collect();
    v.sort_by(|a, b| (a.files[0].as_str(), a.event.as_str(), a.hash).cmp(&(b.files[0].as_str(), b.event.as_str(), b.hash)));
    Ok(v)
}

pub fn folder(c: &Chest) -> String {
    c.files.first().and_then(|f| f.split('/').nth(1)).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(h: u64, ev: Option<&str>, drop: &str) -> Node {
        let mut d: Vec<(String, Node)> = vec![("Drop__DropActor".to_string(), Node::Str(drop.to_string()))];
        if let Some(e) = ev {
            d.push(("GetEventName".to_string(), Node::Str(e.to_string())));
        }
        Node::Map(vec![
            ("Dynamic".to_string(), Node::Map(d)),
            ("Gyaml".to_string(), Node::Str("Obj_TBox_A_02".to_string())),
            ("Hash".to_string(), Node::U64(h)),
            ("Translate".to_string(), Node::Arr(vec![Node::F32(1.0), Node::F32(2.0), Node::F32(3.0)])),
        ])
    }

    #[test]
    fn reads_only_actors_with_an_event() {
        assert!(read("Banc/A/x", &actor(1, None, "I")).is_none());
        let c: Chest = read("Banc/A/x", &actor(1, Some("E"), "I")).unwrap();
        assert_eq!((c.event.as_str(), c.drop.as_str(), c.pos.len()), ("E", "I", 3));
        assert!(c.extra.is_empty());
    }

    #[test]
    fn merges_files_and_flags_disagreement() {
        let mut m: Map<u64, Chest> = Map::new();
        merge(&mut m, read("Banc/A/a", &actor(1, Some("E"), "I")).unwrap());
        merge(&mut m, read("Banc/A/b", &actor(1, Some("E"), "I")).unwrap());
        assert!(m[&1].agree && m[&1].files.len() == 2);
        merge(&mut m, read("Banc/A/c", &actor(1, Some("E"), "J")).unwrap());
        assert!(!m[&1].agree);
        assert_eq!(folder(&m[&1]), "A");
    }
}



