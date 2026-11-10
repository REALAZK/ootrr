use std::collections::HashMap as HM;

use crate::error::Res;
use crate::json::{self, Json};
use crate::settings::digest::sha256_hex;

#[derive(Debug, Clone, PartialEq)]
pub struct Loc {
    pub name: String,
    pub conf: String,
    pub kind: String,
    pub vanilla: String,
    pub item: String,
    pub area: String,
    pub state: String,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Snap {
    pub locations: String,
    pub slice: String,
    pub by_hash: HM<u64, Loc>,
}

fn text(j: &Json, k: &str) -> String {
    j.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn stamp(b: &[u8]) -> String {
    format!("{} bytes sha256 {}", b.len(), &sha256_hex(b)[..16])
}

pub fn parse(locations: &[u8], slice: &[u8]) -> Res<Snap> {
    let l: Json = json::parse(&String::from_utf8_lossy(locations))?;
    let s: Json = json::parse(&String::from_utf8_lossy(slice))?;
    let mut states: HM<String, String> = HM::new();
    for c in s.get("chests").and_then(|x| x.as_arr()).unwrap_or(&[]) {
        states.insert(text(c, "name"), text(c, "state"));
    }
    let mut by_hash: HM<u64, Loc> = HM::new();
    for x in l.get("locations").and_then(|v| v.as_arr()).unwrap_or(&[]) {
        let name: String = text(x, "name");
        let loc: Loc = Loc {
            conf: text(x, "conf"),
            kind: text(x, "kind"),
            vanilla: text(x, "vanilla"),
            item: text(x, "item"),
            area: text(x, "area"),
            state: states.get(&name).cloned().unwrap_or_default(),
            name,
        };
        for r in x.get("refs").and_then(|v| v.as_arr()).unwrap_or(&[]) {
            if let Some(h) = r.get("hash").and_then(|v| v.as_str()).and_then(|v| v.parse::<u64>().ok()) {
                by_hash.insert(h, loc.clone());
            }
        }
    }
    Ok(Snap { locations: stamp(locations), slice: stamp(slice), by_hash })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_locations_with_slice_states_by_hash() {
        let l: &str = r#"{"locations":[{"name":"A Chest","area":"Area","kind":"chest","vanilla":"V","item":"I","conf":"item","refs":[{"hash":"42","files":["x"]}]}]}"#;
        let s: &str = r#"{"chests":[{"name":"A Chest","state":"event"}]}"#;
        let n: Snap = parse(l.as_bytes(), s.as_bytes()).unwrap();
        assert_eq!(n.by_hash[&42].name, "A Chest");
        assert_eq!(n.by_hash[&42].state, "event");
        assert!(n.locations.starts_with(&format!("{} bytes", l.len())));
        assert!(parse(b"{", b"{}").is_err());
    }
}
