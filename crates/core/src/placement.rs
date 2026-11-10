use crate::byml::{Byml, Node};
use crate::error::Res;
use crate::romfs::Romfs;

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub file: String,
    pub gyaml: String,
    pub name: String,
    pub hash: u64,
    pub pos: Vec<f32>,
    pub item: String,
}

pub struct Edit {
    pub file: String,
    pub old_len: usize,
    pub bytes: Vec<u8>,
}

pub fn hash_of(n: &Node) -> Option<u64> {
    match n.get("Hash")? {
        Node::U64(x) => Some(*x),
        Node::I64(x) => Some(*x as u64),
        _ => None,
    }
}

pub fn banc_files(rf: &Romfs) -> Res<Vec<String>> {
    let all: Vec<String> = rf.list("Banc")?;
    Ok(all.into_iter().filter(|p| p.ends_with(".bcett.byml.zs")).collect())
}

fn actors(y: &Byml) -> &[Node] {
    y.root.get("Actors").and_then(|a| a.as_arr()).unwrap_or(&[])
}

fn text(n: &Node, k: &str) -> String {
    n.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

pub fn drops(rf: &mut Romfs) -> Res<Vec<Hit>> {
    let mut out: Vec<Hit> = Vec::new();
    for f in banc_files(rf)? {
        let y: Byml = rf.read_byml(&f)?;
        for a in actors(&y) {
            let Some(item) = a
                .get("Dynamic")
                .and_then(|d| d.get("Drop__DropActor"))
                .and_then(|v| v.as_str())
            else {
                continue;
            };
            let pos: Vec<f32> = a
                .get("Translate")
                .and_then(|t| t.as_arr())
                .map(|t| {
                    t.iter()
                        .filter_map(|x| if let Node::F32(v) = x { Some(*v) } else { None })
                        .collect()
                })
                .unwrap_or_default();
            out.push(Hit {
                file: f.clone(),
                gyaml: text(a, "Gyaml"),
                name: text(a, "Name"),
                hash: hash_of(a).unwrap_or(0),
                pos,
                item: item.to_string(),
            });
        }
    }
    Ok(out)
}

pub fn set_drop(rf: &mut Romfs, hash: u64, item: &str, clear_event: bool) -> Res<Vec<Edit>> {
    let mut out: Vec<Edit> = Vec::new();
    for f in banc_files(rf)? {
        let raw: Vec<u8> = rf.read_zs(&f)?;
        let mut y: Byml = Byml::parse(&raw)?;
        let mut hit: bool = false;
        if let Some(Node::Arr(list)) = y.root.get_mut("Actors") {
            for a in list.iter_mut() {
                if hash_of(a) != Some(hash) {
                    continue;
                }
                if let Some(d) = a.get_mut("Dynamic")
                    && d.get("Drop__DropActor").is_some()
                    && d.set("Drop__DropActor", Node::Str(item.to_string()))
                {
                    hit = true;
                    if clear_event {
                        d.remove("GetEventName");
                    }
                }
            }
        }
        if hit {
            out.push(Edit { file: f, old_len: raw.len(), bytes: y.write()? });
        }
    }
    Ok(out)
}
