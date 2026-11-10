use std::collections::BTreeMap as Map;
use std::fs;

use crate::byml::{Byml, Node};
use crate::error::Res;
use crate::placement::banc_files;
use crate::romfs::Romfs;

const TOPS: [&str; 14] = [
    "Event", "Banc", "Pack", "Logic", "AI", "Sequence", "Scene", "WorldMgr", "GameParameter", "Game", "Component",
    "System", "RSDB", "AISchedule",
];

#[derive(Debug, Clone, PartialEq)]
pub struct Use {
    pub file: String,
    pub gyaml: String,
    pub path: String,
}

fn ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

pub fn has_token(d: &[u8], t: &[u8]) -> bool {
    if t.is_empty() || d.len() < t.len() {
        return false;
    }
    let mut i: usize = 0;
    while i + t.len() <= d.len() {
        if d[i] == t[0] && &d[i..i + t.len()] == t {
            let before: bool = i == 0 || !ident(d[i - 1]);
            let after: bool = i + t.len() == d.len() || !ident(d[i + t.len()]);
            if before && after {
                return true;
            }
        }
        i += 1;
    }
    false
}

pub fn class_of(p: &str) -> &'static str {
    let top: &str = p.split('/').next().unwrap_or("");
    match top {
        "Event" => {
            if p.contains(".bfevfl") {
                "flow"
            } else {
                "eventparam"
            }
        }
        "Banc" => "banc",
        "Pack" => "pack",
        "RSDB" => "rsdb",
        "Logic" | "Sequence" | "AI" => "ainb",
        _ => "param",
    }
}

pub fn scan(rf: &mut Romfs, names: &[String]) -> Res<Map<String, Vec<String>>> {
    let mut out: Map<String, Vec<String>> = Map::new();
    let mut files: Vec<String> = Vec::new();
    for t in TOPS {
        if rf.root.join(t).is_dir() {
            files.extend(rf.list(t)?);
        }
    }
    files.sort();
    for p in files.iter().filter(|p| !p.ends_with(".webm") && !p.ends_with(".bwav")) {
        let Ok(raw) = fs::read(rf.root.join(p)) else { continue };
        let d: Vec<u8> = if p.ends_with(".zs") {
            match rf.zs.decompress(&raw) {
                Ok(x) => x,
                Err(_) => continue,
            }
        } else {
            raw
        };
        for n in names {
            if has_token(&d, n.as_bytes()) {
                out.entry(n.clone()).or_default().push(p.clone());
            }
        }
    }
    Ok(out)
}

fn walk(n: &Node, path: &str, f: &str, g: &str, names: &[String], out: &mut Map<String, Vec<Use>>) {
    match n {
        Node::Str(s) => {
            if names.iter().any(|x| x == s) {
                out.entry(s.clone()).or_default().push(Use { file: f.to_string(), gyaml: g.to_string(), path: path.to_string() });
            }
        }
        Node::Arr(a) => {
            for x in a {
                walk(x, path, f, g, names, out);
            }
        }
        Node::Map(m) => {
            let own: String = n.get("Gyaml").and_then(|x| x.as_str()).unwrap_or(g).to_string();
            for (k, v) in m {
                let p: String = if path.is_empty() { k.clone() } else { format!("{path}.{k}") };
                walk(v, &p, f, &own, names, out);
            }
        }
        _ => {}
    }
}

pub fn banc_uses(rf: &mut Romfs, names: &[String]) -> Res<Map<String, Vec<Use>>> {
    let mut out: Map<String, Vec<Use>> = Map::new();
    for f in banc_files(rf)? {
        let y: Byml = rf.read_byml(&f)?;
        walk(&y.root, "", &f, "", names, &mut out);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_needs_boundaries() {
        assert!(has_token(b"\0Foo_Bar\0", b"Foo_Bar"));
        assert!(!has_token(b"\0Foo_Bar_2\0", b"Foo_Bar"));
        assert!(!has_token(b"xFoo_Bar", b"Foo_Bar"));
        assert!(has_token(b"Have.Foo_Bar", b"Foo_Bar"));
        assert!(!has_token(b"", b"A"));
    }

    #[test]
    fn classes() {
        assert_eq!(class_of("Banc/A/b.zs"), "banc");
        assert_eq!(class_of("Event/EventFlow/a.bfevfl.zs"), "flow");
        assert_eq!(class_of("Logic/a.ainb"), "ainb");
    }

    #[test]
    fn banc_walk_records_the_key_path() {
        let n: Node = Node::Map(vec![(
            "Actors".to_string(),
            Node::Arr(vec![Node::Map(vec![
                ("Gyaml".to_string(), Node::Str("Wall".to_string())),
                ("Presence".to_string(), Node::Map(vec![("FlagName".to_string(), Node::Str("F".to_string()))])),
            ])]),
        )]);
        let mut out: Map<String, Vec<Use>> = Map::new();
        walk(&n, "", "Banc/x", "", &["F".to_string()], &mut out);
        assert_eq!(out["F"][0].path, "Actors.Presence.FlagName");
        assert_eq!(out["F"][0].gyaml, "Wall");
    }
}


