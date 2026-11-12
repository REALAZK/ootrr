use crate::byml::{Byml, Node};
use crate::error::Res;
use crate::placement::hash_of;
use crate::romfs::Romfs;
use crate::settings::digest::sha256_hex;
use crate::slice::data::Ev;

#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub file: String,
    pub gyaml: String,
    pub pos: Vec<f32>,
    pub drop: Option<String>,
    pub event: bool,
    pub check: bool,
    pub sub: bool,
}

pub fn actors(y: &Byml) -> &[Node] {
    y.root.get("Actors").and_then(|a| a.as_arr()).unwrap_or(&[])
}

pub fn describe(file: &str, a: &Node) -> Found {
    let dynamic: Option<&Node> = a.get("Dynamic");
    let has = |k: &str| dynamic.and_then(|d| d.get(k)).is_some();
    let pos: Vec<f32> = a
        .get("Translate")
        .and_then(|t| t.as_arr())
        .map(|t| {
            t.iter()
                .filter_map(|x| if let Node::F32(v) = x { Some(*v) } else { None })
                .collect()
        })
        .unwrap_or_default();
    Found {
        file: file.to_string(),
        gyaml: a.get("Gyaml").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        pos,
        drop: dynamic
            .and_then(|d| d.get("Drop__DropActor"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        event: has("GetEventName"),
        check: has("Drop__CheckActor") || has("Drop__DropActor_Substitute"),
        sub: has("Drop__DropActor_Substitute"),
    }
}

pub fn find(y: &Byml, file: &str, hash: u64) -> Vec<Found> {
    actors(y)
        .iter()
        .filter(|a| hash_of(a) == Some(hash))
        .map(|a| describe(file, a))
        .collect()
}

pub fn set_drop(y: &mut Byml, hash: u64, old: &str, new: &str) -> Res<usize> {
    set_drop_ev(y, hash, old, new, None)
}

pub fn set_drop_ev(y: &mut Byml, hash: u64, old: &str, new: &str, ev: Option<Ev>) -> Res<usize> {
    let mut n: usize = 0;
    let Some(list) = y.root.get_mut("Actors").and_then(|a| a.as_arr_mut()) else {
        return Ok(0);
    };
    for a in list.iter_mut() {
        if hash_of(a) != Some(hash) {
            continue;
        }
        let Some(d) = a.get_mut("Dynamic") else {
            continue;
        };
        if d.get("Drop__DropActor").is_none() {
            continue;
        }
        let have: String = d.get("Drop__DropActor").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if have != old {
            return Err(crate::error::Error::Msg(format!(
                "actor {hash} holds {have} where {old} was expected, the game files differ from the ones the data was made for"
            )));
        }
        if d.get("GetEventName").is_some() && ev.is_none() {
            return Err(crate::error::Error::Msg(format!(
                "actor {hash} runs a story event, it cannot be shuffled in this slice"
            )));
        }
        d.set("Drop__DropActor", Node::Str(new.to_string()));
        if d.get("Drop__DropActor_Substitute").is_some() {
            d.set("Drop__DropActor_Substitute", Node::Str(new.to_string()));
        }
        if ev == Some(Ev::Strip) {
            d.remove("GetEventName");
        }
        n += 1;
    }
    Ok(n)
}

pub fn fingerprint(rf: &Romfs, files: &[String]) -> Res<String> {
    let mut sorted: Vec<&String> = files.iter().collect();
    sorted.sort();
    let mut buf: Vec<u8> = Vec::new();
    for f in sorted {
        let b: Vec<u8> = rf.read(f)?;
        buf.extend_from_slice(f.as_bytes());
        buf.push(0);
        buf.extend_from_slice(&(b.len() as u64).to_le_bytes());
        buf.extend_from_slice(&b);
    }
    let h: String = sha256_hex(&buf);
    Ok(format!("oot-remake:{}", &h[..16]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(hash: u64, drop: &str, event: bool) -> Node {
        let mut dy: Vec<(String, Node)> = vec![("Drop__DropActor".to_string(), Node::Str(drop.to_string()))];
        if event {
            dy.push(("GetEventName".to_string(), Node::Str("Ev".to_string())));
        }
        Node::Map(vec![
            ("Dynamic".to_string(), Node::Map(dy)),
            ("Gyaml".to_string(), Node::Str("Obj_TBox_A_02".to_string())),
            ("Hash".to_string(), Node::U64(hash)),
            ("Translate".to_string(), Node::Arr(vec![Node::F32(1.0), Node::F32(2.5), Node::F32(-3.0)])),
        ])
    }

    fn file(list: Vec<Node>) -> Byml {
        Byml {
            root: Node::Map(vec![("Actors".to_string(), Node::Arr(list))]),
            version: 7,
            big: false,
        }
    }

    #[test]
    fn describes_and_finds_actors() {
        let y: Byml = file(vec![actor(1, "A", false), actor(2, "B", true)]);
        let f: Vec<Found> = find(&y, "x", 2);
        assert_eq!(f.len(), 1);
        assert!(f[0].event && !f[0].check);
        assert_eq!(f[0].pos, vec![1.0, 2.5, -3.0]);
        assert_eq!(f[0].drop.as_deref(), Some("B"));
        assert!(find(&y, "x", 3).is_empty());
    }

    #[test]
    fn set_drop_checks_before_writing() {
        let mut y: Byml = file(vec![actor(1, "A", false), actor(2, "B", true)]);
        assert_eq!(set_drop(&mut y, 1, "A", "Z").unwrap(), 1);
        assert_eq!(find(&y, "x", 1)[0].drop.as_deref(), Some("Z"));
        assert!(set_drop(&mut y, 1, "A", "Y").is_err());
        assert!(set_drop(&mut y, 2, "B", "Y").is_err());
        assert_eq!(set_drop(&mut y, 9, "A", "Y").unwrap(), 0);
    }

    #[test]
    fn a_check_chest_gets_the_new_item_on_both_branches() {
        let dy: Vec<(String, Node)> = vec![
            ("Drop__CheckActor".to_string(), Node::Str("TBox_Shield_Deku".to_string())),
            ("Drop__DropActor".to_string(), Node::Str("TBox_Shield_Deku".to_string())),
            ("Drop__DropActor_Substitute".to_string(), Node::Str("TBox_Rupee_Blue".to_string())),
        ];
        let a: Node = Node::Map(vec![
            ("Dynamic".to_string(), Node::Map(dy)),
            ("Gyaml".to_string(), Node::Str("Obj_TBox_A_06".to_string())),
            ("Hash".to_string(), Node::U64(5)),
        ]);
        let mut y: Byml = file(vec![a, actor(1, "A", false)]);
        let f: Found = find(&y, "x", 5).remove(0);
        assert!(f.check && f.sub);
        assert!(!find(&y, "x", 1)[0].sub);
        assert!(set_drop(&mut y, 5, "TBox_Rupee_Blue", "Item_Hookshot").is_err());
        assert_eq!(set_drop(&mut y, 5, "TBox_Shield_Deku", "Item_Hookshot").unwrap(), 1);
        let d: &Node = actors(&y)[0].get("Dynamic").unwrap();
        assert_eq!(d.get("Drop__DropActor").and_then(|v| v.as_str()), Some("Item_Hookshot"));
        assert_eq!(d.get("Drop__DropActor_Substitute").and_then(|v| v.as_str()), Some("Item_Hookshot"));
        assert_eq!(d.get("Drop__CheckActor").and_then(|v| v.as_str()), Some("TBox_Shield_Deku"));
        set_drop(&mut y, 1, "A", "Z").unwrap();
        assert!(actors(&y)[1].get("Dynamic").unwrap().get("Drop__DropActor_Substitute").is_none());
    }

    #[test]
    fn an_edited_file_reparses_with_only_the_drop_changed() {
        let mut y: Byml = file(vec![actor(1, "AAAA", false), actor(2, "B", false)]);
        let before: Vec<u8> = y.write().unwrap();
        set_drop(&mut y, 1, "AAAA", "ZZZZ").unwrap();
        let after: Vec<u8> = y.write().unwrap();
        assert_eq!(before.len(), after.len());
        let back: Byml = Byml::parse(&after).unwrap();
        assert_eq!(find(&back, "x", 1)[0].drop.as_deref(), Some("ZZZZ"));
        assert_eq!(find(&back, "x", 2)[0].drop.as_deref(), Some("B"));
    }
}
