use std::collections::{BTreeMap as BM, BTreeSet as BS};

use crate::byml::{Byml, Node};
use crate::error::{Error as E, Res};
use crate::sarc::Entry;

pub const PARENT: &str = "$parent";
pub const COMPONENTS: &str = "Components";
pub const ACTOR_DIR: &str = "Actor/";
const DEPTH: usize = 16;
const STRING_TABLE: usize = 0xC2;
const MAX_TEXTS: usize = 1 << 16;

pub fn actor_file(name: &str) -> String {
    format!("Actor/{name}.engine__actor__ActorParam.bgyml")
}

pub fn norm(s: &str) -> Option<String> {
    let t: &str = s.strip_prefix('?').or_else(|| s.strip_prefix("Work/"))?;
    Some(match t.strip_suffix(".gyml") {
        Some(x) => format!("{x}.bgyml"),
        None => t.to_string(),
    })
}

pub fn work(path: &str) -> String {
    match path.strip_suffix(".bgyml") {
        Some(x) => format!("Work/{x}.gyml"),
        None => format!("Work/{path}"),
    }
}

pub fn local(path: &str) -> String {
    format!("?{path}")
}

pub fn is_byml(d: &[u8]) -> bool {
    d.get(..2) == Some(b"YB".as_slice())
}

fn cut() -> E {
    E::Format("pickup pack: a parameter file is truncated")
}

fn u32_at(d: &[u8], i: usize) -> Res<usize> {
    let s: &[u8] = d.get(i..i.saturating_add(4)).ok_or_else(cut)?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as usize)
}

pub fn texts(d: &[u8]) -> Res<Vec<String>> {
    if !is_byml(d) {
        return Err(E::Format("pickup pack: not a little endian parameter file"));
    }
    let at: usize = u32_at(d, 8)?;
    if at == 0 {
        return Ok(Vec::new());
    }
    let head: usize = u32_at(d, at)?;
    if head & 0xFF != STRING_TABLE {
        return Err(E::Format("pickup pack: the string table is missing"));
    }
    let n: usize = head >> 8;
    let mut out: Vec<String> = Vec::with_capacity(n.min(MAX_TEXTS));
    for i in 0..n {
        let from: usize = at.checked_add(u32_at(d, at + 4 + 4 * i)?).ok_or_else(cut)?;
        let tail: &[u8] = d.get(from..).ok_or_else(cut)?;
        let len: usize = tail.iter().position(|c| *c == 0).ok_or_else(cut)?;
        out.push(String::from_utf8_lossy(&tail[..len]).into_owned());
    }
    Ok(out)
}

#[derive(Clone, Debug, Default)]
pub struct Pool {
    pub files: BM<String, Vec<u8>>,
}

impl Pool {
    pub fn add(&mut self, name: &str, data: &[u8]) -> Res<()> {
        match self.files.get(name) {
            Some(d) if d.as_slice() != data => Err(E::Msg(format!("{name} has two different contents in the source packs"))),
            Some(_) => Ok(()),
            None => {
                self.files.insert(name.to_string(), data.to_vec());
                Ok(())
            }
        }
    }

    pub fn add_pack(&mut self, es: &[Entry]) -> Res<()> {
        for e in es {
            self.add(&e.name, &e.data)?;
        }
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.files.get(name).map(|d| d.as_slice())
    }

    pub fn byml(&self, name: &str) -> Res<Byml> {
        let d: &[u8] = self.get(name).ok_or_else(|| E::Msg(format!("{name} is not in the source packs")))?;
        Byml::parse(d)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chain {
    pub files: Vec<String>,
    pub comps: BM<String, String>,
}

pub fn chain(p: &Pool, actor: &str) -> Res<Chain> {
    let mut files: Vec<String> = Vec::new();
    let mut maps: Vec<Vec<(String, String)>> = Vec::new();
    let mut at: String = actor_file(actor);
    loop {
        if files.len() >= DEPTH || files.contains(&at) {
            return Err(E::Msg(format!("{actor} has a parent chain that does not end")));
        }
        let y: Byml = p.byml(&at)?;
        let mut own: Vec<(String, String)> = Vec::new();
        if let Some(Node::Map(m)) = y.root.get(COMPONENTS) {
            for (k, v) in m {
                if let Node::Str(s) = v {
                    own.push((k.clone(), s.clone()));
                }
            }
        }
        maps.push(own);
        files.push(at.clone());
        match y.root.get(PARENT).and_then(|v| v.as_str()).and_then(norm) {
            Some(up) => at = up,
            None => break,
        }
    }
    let mut comps: BM<String, String> = BM::new();
    for m in maps.into_iter().rev() {
        comps.extend(m);
    }
    Ok(Chain { files, comps })
}

pub fn select(p: &Pool, actor: &str) -> Res<Vec<String>> {
    let c: Chain = chain(p, actor)?;
    let mut seen: BS<String> = c.files.iter().cloned().collect();
    let mut todo: Vec<String> = Vec::new();
    let mut found: Vec<String> = c.comps.values().cloned().collect();
    loop {
        for s in found.drain(..) {
            let Some(t) = norm(&s) else {
                continue;
            };
            if t.starts_with(ACTOR_DIR) || !p.files.contains_key(&t) || !seen.insert(t.clone()) {
                continue;
            }
            todo.push(t);
        }
        let Some(f) = todo.pop() else {
            break;
        };
        let d: &[u8] = p.get(&f).unwrap_or(&[]);
        if is_byml(d) {
            found.extend(texts(d)?);
        }
    }
    Ok(seen.into_iter().collect())
}

pub fn dangling(p: &Pool, files: &[String], known: &BS<String>) -> Res<Vec<String>> {
    let c: Chain = Chain { files: Vec::new(), comps: BM::new() };
    dangling_with(p, files, known, &c)
}

pub fn dangling_with(p: &Pool, files: &[String], known: &BS<String>, c: &Chain) -> Res<Vec<String>> {
    let mut out: BS<String> = BS::new();
    for f in files {
        let d: &[u8] = p.get(f).ok_or_else(|| E::Msg(format!("{f} is not in the source packs")))?;
        if !is_byml(d) {
            continue;
        }
        let mut found: Vec<String> = Vec::new();
        if c.files.contains(f) {
            found.extend(c.comps.values().cloned());
        } else {
            found.extend(texts(d)?);
        }
        for s in found {
            let Some(t) = norm(&s) else {
                continue;
            };
            if !t.starts_with(ACTOR_DIR) && !files.contains(&t) && known.contains(&t) {
                out.insert(t);
            }
        }
    }
    Ok(out.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(pairs: &[(&str, Node)]) -> Vec<u8> {
        let root: Node = Node::Map(pairs.iter().map(|p| (p.0.to_string(), p.1.clone())).collect());
        Byml { root, version: 7, big: false }.write().unwrap()
    }

    fn s(x: &str) -> Node {
        Node::Str(x.to_string())
    }

    fn pool() -> Pool {
        let mut p: Pool = Pool::default();
        let base: Vec<u8> = file(&[(
            COMPONENTS,
            Node::Map(vec![
                ("ModelInfoRef".to_string(), s("?Component/ModelInfo/Dummy.bgyml")),
                ("PhysicsRef".to_string(), s("?Component/Physics/Base.bgyml")),
            ]),
        )]);
        let kid: Vec<u8> = file(&[
            (PARENT, s("Work/Actor/Base.engine__actor__ActorParam.gyml")),
            (
                COMPONENTS,
                Node::Map(vec![
                    ("ActorNameRef".to_string(), s("?ActorSystem/ActorName/Item.bgyml")),
                    ("ModelInfoRef".to_string(), s("?Component/ModelInfo/Kid.bgyml")),
                    ("PouchContentRef".to_string(), s("?Component/Pouch/Kid.bgyml")),
                ]),
            ),
        ]);
        p.add(&actor_file("Base"), &base).unwrap();
        p.add(&actor_file("Kid"), &kid).unwrap();
        p.add(&actor_file("Item"), &file(&[("A", Node::Bool(true))])).unwrap();
        p.add("Component/ModelInfo/Dummy.bgyml", &file(&[("A", Node::Bool(true))])).unwrap();
        p.add("Component/ModelInfo/Kid.bgyml", &file(&[("A", Node::Bool(true))])).unwrap();
        p.add("Component/Physics/Base.bgyml", &file(&[("Set", s("Work/Phive/Set.gyml"))])).unwrap();
        p.add("Phive/Set.bgyml", &file(&[("A", Node::Bool(true))])).unwrap();
        p.add(
            "Component/Pouch/Kid.bgyml",
            &file(&[
                (PARENT, s("Work/Component/Pouch/Item.gyml")),
                ("PouchActor", s("Work/Actor/Item.engine__actor__ActorParam.gyml")),
            ]),
        )
        .unwrap();
        p.add("Component/Pouch/Item.bgyml", &file(&[("A", Node::Bool(true))])).unwrap();
        p
    }

    #[test]
    fn references_map_to_pack_paths_and_back() {
        assert_eq!(norm("?A/B.bgyml").as_deref(), Some("A/B.bgyml"));
        assert_eq!(norm("Work/A/B.x__Y.gyml").as_deref(), Some("A/B.x__Y.bgyml"));
        assert_eq!(norm("Work/AS/Item.root.asb").as_deref(), Some("AS/Item.root.asb"));
        assert_eq!(norm("Material_Stone"), None);
        assert_eq!(work("A/B.x__Y.bgyml"), "Work/A/B.x__Y.gyml");
        assert_eq!(local("A/B.bgyml"), "?A/B.bgyml");
    }

    #[test]
    fn a_child_component_replaces_the_parent_one() {
        let c: Chain = chain(&pool(), "Kid").unwrap();
        assert_eq!(c.files, vec![actor_file("Kid"), actor_file("Base")]);
        assert_eq!(c.comps["ModelInfoRef"], "?Component/ModelInfo/Kid.bgyml");
        assert_eq!(c.comps["PhysicsRef"], "?Component/Physics/Base.bgyml");
    }

    #[test]
    fn the_file_list_follows_parents_and_skips_replaced_and_foreign_actor_files() {
        let got: Vec<String> = select(&pool(), "Kid").unwrap();
        let want: Vec<&str> = vec![
            "Actor/Base.engine__actor__ActorParam.bgyml",
            "Actor/Kid.engine__actor__ActorParam.bgyml",
            "Component/ModelInfo/Kid.bgyml",
            "Component/Physics/Base.bgyml",
            "Component/Pouch/Item.bgyml",
            "Component/Pouch/Kid.bgyml",
            "Phive/Set.bgyml",
        ];
        assert_eq!(got, want);
    }

    #[test]
    fn a_reference_to_a_known_file_left_out_is_reported() {
        let p: Pool = pool();
        let c: Chain = chain(&p, "Kid").unwrap();
        let mut files: Vec<String> = select(&p, "Kid").unwrap();
        let known: BS<String> = p.files.keys().cloned().collect();
        assert!(dangling_with(&p, &files, &known, &c).unwrap().is_empty());
        assert_eq!(dangling(&p, &files, &known).unwrap(), vec!["Component/ModelInfo/Dummy.bgyml".to_string()]);
        files.retain(|f| f != "Phive/Set.bgyml");
        assert_eq!(dangling_with(&p, &files, &known, &c).unwrap(), vec!["Phive/Set.bgyml".to_string()]);
    }

    #[test]
    fn the_string_table_gives_every_text_value_and_no_key() {
        let d: Vec<u8> = file(&[("Key", s("Work/A/B.gyml")), ("List", Node::Arr(vec![s("?C/D.bgyml"), Node::I32(4)]))]);
        assert_eq!(texts(&d).unwrap(), vec!["?C/D.bgyml".to_string(), "Work/A/B.gyml".to_string()]);
        assert!(texts(&file(&[("A", Node::Bool(true))])).unwrap().is_empty());
        assert!(texts(b"SARC").is_err());
        assert!(texts(&d[..20]).is_err());
    }

    #[test]
    fn two_contents_for_one_path_are_refused() {
        let mut p: Pool = pool();
        assert!(p.add("Phive/Set.bgyml", &[1, 2, 3]).is_err());
        assert!(chain(&p, "Nobody").is_err());
    }
}
