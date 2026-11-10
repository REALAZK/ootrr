use std::collections::HashMap as Map;

use crate::byml::{Byml, Node};
use crate::error::Res;
use crate::flags::{FlagList, KEYED};
use crate::romfs::Romfs;

pub const BY_ACTOR: &str = "placement";
pub const BY_REF: &str = "reference";
pub const BY_NEAR: &str = "placement-near";
const NEAR: u64 = 2;

#[derive(Default)]
pub struct Keys {
    pub actors: Map<u64, (String, String)>,
    pub refs: Map<u64, (String, String)>,
}

fn stem(p: &str) -> String {
    let f: &str = p.rsplit('/').next().unwrap_or(p);
    f.split('.').next().unwrap_or(f).to_string()
}

fn big(n: &Node) -> Option<u64> {
    match n {
        Node::U64(x) => Some(*x),
        Node::I64(x) => Some(*x as u64),
        _ => None,
    }
}

fn refs(n: &Node, key: &str, s: &str, out: &mut Keys) {
    match n {
        Node::Map(m) => {
            for (k, v) in m {
                refs(v, k, s, out);
            }
        }
        Node::Arr(a) => {
            for v in a {
                refs(v, key, s, out);
            }
        }
        _ => {
            if let Some(x) = big(n) {
                out.refs
                    .entry(x)
                    .or_insert_with(|| (key.to_string(), s.to_string()));
            }
        }
    }
}

fn one(y: &Byml, s: &str, out: &mut Keys) {
    for a in y.root.get("Actors").and_then(|a| a.as_arr()).unwrap_or(&[]) {
        let h: Option<u64> = a.get("Hash").and_then(big);
        let g: &str = a.get("Gyaml").and_then(|v| v.as_str()).unwrap_or("");
        if let Some(h) = h {
            out.actors
                .entry(h)
                .or_insert_with(|| (g.to_string(), s.to_string()));
        }
    }
    refs(&y.root, "", s, out);
}

pub fn collect(rf: &mut Romfs) -> Res<Keys> {
    let mut out: Keys = Keys::default();
    let mut files: Vec<String> = rf.list("Banc")?;
    files.extend(rf.list("AoC1")?);
    files.sort();
    for f in files.iter().filter(|f| f.ends_with(".byml.zs")) {
        let y: Byml = rf.read_byml(f)?;
        one(&y, &stem(f), &mut out);
    }
    Ok(out)
}

fn near(sorted: &[u64], k: &Keys, h: u64) -> Option<String> {
    let i: usize = sorted.partition_point(|x| *x < h);
    for j in [i.wrapping_sub(1), i] {
        let Some(a) = sorted.get(j) else {
            continue;
        };
        let d: u64 = a.abs_diff(h);
        if d == NEAR {
            let (g, s) = k.actors.get(a)?;
            let sign: char = if h > *a { '+' } else { '-' };
            return Some(format!("{g}@{s}{sign}{d}"));
        }
    }
    None
}

pub fn label(l: &mut FlagList, k: &Keys) -> (usize, usize, usize) {
    let mut a: usize = 0;
    let mut r: usize = 0;
    let mut n: usize = 0;
    let mut sorted: Vec<u64> = k.actors.keys().copied().collect();
    sorted.sort_unstable();
    let hs: Vec<u64> = l
        .flags
        .iter()
        .filter(|f| f.ty == KEYED)
        .map(|f| f.hash)
        .collect();
    for h in hs {
        if let Some((g, s)) = k.actors.get(&h) {
            if l.tag(h, &format!("{g}@{s}"), BY_ACTOR) {
                a += 1;
            }
        } else if let Some((key, s)) = k.refs.get(&h) {
            if l.tag(h, &format!("{key}@{s}"), BY_REF) {
                r += 1;
            }
        } else if let Some(t) = near(&sorted, k, h)
            && l.tag(h, &t, BY_NEAR)
        {
            n += 1;
        }
    }
    (a, r, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stem_drops_every_extension() {
        assert_eq!(stem("Banc/A/A_Static.bcett.byml.zs"), "A_Static");
        assert_eq!(stem("x.byml.zs"), "x");
    }

    #[test]
    fn actors_and_refs_are_collected() {
        let a: Node = Node::Map(vec![
            ("Gyaml".to_string(), Node::Str("Obj_TBox_A_01".to_string())),
            ("Hash".to_string(), Node::U64(7)),
            ("Other".to_string(), Node::U64(9)),
        ]);
        let y: Byml = Byml {
            root: Node::Map(vec![("Actors".to_string(), Node::Arr(vec![a]))]),
            version: 7,
            big: false,
        };
        let mut k: Keys = Keys::default();
        one(&y, "S", &mut k);
        assert_eq!(
            k.actors.get(&7).map(|x| x.0.as_str()),
            Some("Obj_TBox_A_01")
        );
        assert!(k.refs.contains_key(&9));
        let mut sorted: Vec<u64> = k.actors.keys().copied().collect();
        sorted.sort_unstable();
        assert_eq!(near(&sorted, &k, 9).as_deref(), Some("Obj_TBox_A_01@S+2"));
        assert_eq!(near(&sorted, &k, 5).as_deref(), Some("Obj_TBox_A_01@S-2"));
        assert!(near(&sorted, &k, 100).is_none());
    }
}
