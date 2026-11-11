use std::collections::BTreeMap as BM;
use std::fmt::Write as W;
use std::path::{Path as P, PathBuf as PB};

use crate::byml::{Byml, Node};
use crate::chest_slot::LIST;
use crate::error::{Error, Res};
use crate::placement::{banc_files, hash_of};
use crate::romfs::Romfs;
use crate::staging::{banc_out, banc_raw, commit, rewrite, Committed, Out};

pub const DROP: &str = "Drop__DropActor";
pub const KEPT: i32 = 24;
pub const TABLE: &str = "Bool64bitKey";

#[derive(Clone, Debug)]
pub struct Opts {
    pub rst_from: Option<PB>,
    pub banc_from: Option<PB>,
}

pub fn give(y: &mut Byml, hash: u64, item: &str) -> usize {
    let Some(list) = y.root.get_mut("Actors").and_then(|a| a.as_arr_mut()) else {
        return 0;
    };
    let mut n: usize = 0;
    for a in list.iter_mut() {
        if hash_of(a) != Some(hash) {
            continue;
        }
        match a.get_mut("Dynamic") {
            Some(d) => {
                d.set(DROP, Node::Str(item.to_string()));
            }
            None => {
                a.set("Dynamic", Node::Map(vec![(DROP.to_string(), Node::Str(item.to_string()))]));
            }
        }
        n += 1;
    }
    n
}

fn kept(r: &mut Node) {
    let v: Node = match r.get("ResetTypeValue") {
        Some(Node::U32(_)) => Node::U32(KEPT as u32),
        _ => Node::I32(KEPT),
    };
    r.set("ResetTypeValue", v);
    r.remove("ExtraByte");
}

pub fn one_time(y: &mut Byml, hashes: &[u64]) -> Res<usize> {
    let rows: &mut Vec<Node> = y
        .root
        .get_mut("Data")
        .and_then(|d| d.get_mut(TABLE))
        .and_then(|t| t.as_arr_mut())
        .ok_or(Error::Format("pots: the flag list has no 64-bit key table"))?;
    let mut n: usize = 0;
    for h in hashes {
        let r: &mut Node = rows
            .iter_mut()
            .find(|r| hash_of(r) == Some(*h))
            .ok_or_else(|| Error::Msg(format!("the flag list has no collected row for actor {h}")))?;
        kept(r);
        n += 1;
    }
    Ok(n)
}

pub struct Built {
    pub report: String,
    pub files: usize,
    pub grown: usize,
}

pub fn build(romfs: &P, load: &P, name: &str, picks: &[(u64, String)], o: &Opts) -> Res<Built> {
    let mut rf: Romfs = Romfs::open(romfs)?;
    let want: BM<u64, &String> = picks.iter().map(|(h, i)| (*h, i)).collect();
    let mut outs: Vec<Out> = Vec::new();
    let mut report: String = String::new();
    let mut found: usize = 0;
    for f in banc_files(&rf)? {
        let raw: Vec<u8> = banc_raw(&mut rf, &f, o.banc_from.as_ref())?;
        let mut y: Byml = Byml::parse(&raw)?;
        let mut hit: bool = false;
        for (h, item) in &want {
            if give(&mut y, *h, item) > 0 {
                hit = true;
                found += 1;
                let _ = writeln!(report, "{f}: actor {h} now drops {item}");
            }
        }
        if hit {
            let bytes: Vec<u8> = y.write()?;
            outs.push(banc_out(&mut rf, &f, raw.len(), &bytes)?);
        }
    }
    if found == 0 {
        return Err(Error::Format("pots: none of the actor hashes was found in the scenes"));
    }
    let hashes: Vec<u64> = want.keys().copied().collect();
    outs.push(rewrite(&mut rf, LIST, |raw| {
        let mut y: Byml = Byml::parse(raw)?;
        one_time(&mut y, &hashes)?;
        y.write()
    })?);
    let c: Committed = commit(&mut rf, load, name, &outs, o.rst_from.as_ref())?;
    report.push_str(&c.report);
    Ok(Built { report, files: c.files, grown: c.grown })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(h: u64, dynamic: Option<Node>) -> Node {
        let mut m: Vec<(String, Node)> = vec![
            ("Gyaml".to_string(), Node::Str("Obj_Pot_A_01".to_string())),
            ("Hash".to_string(), Node::U64(h)),
        ];
        if let Some(d) = dynamic {
            m.push(("Dynamic".to_string(), d));
        }
        Node::Map(m)
    }

    #[test]
    fn a_pot_without_a_drop_key_gets_one_and_the_others_keep_theirs() {
        let kept_key: Node = Node::Map(vec![(DROP.to_string(), Node::Str("Obj_Rupee_Green".to_string()))]);
        let mut y: Byml = Byml {
            root: Node::Map(vec![("Actors".to_string(), Node::Arr(vec![actor(1, None), actor(2, Some(kept_key))]))]),
            version: 2,
            big: false,
        };
        assert_eq!(give(&mut y, 1, "Obj_Arrow_Fire"), 1);
        assert_eq!(give(&mut y, 9, "Obj_Arrow_Fire"), 0);
        let a: &[Node] = y.root.get("Actors").unwrap().as_arr().unwrap();
        assert_eq!(a[0].get("Dynamic").and_then(|d| d.get(DROP)).and_then(|v| v.as_str()), Some("Obj_Arrow_Fire"));
        assert_eq!(a[1].get("Dynamic").and_then(|d| d.get(DROP)).and_then(|v| v.as_str()), Some("Obj_Rupee_Green"));
    }

    #[test]
    fn the_collected_row_becomes_permanent_without_a_scene_byte() {
        let row = |h: u64| {
            Node::Map(vec![
                ("ExtraByte".to_string(), Node::I32(7)),
                ("Hash".to_string(), Node::U64(h)),
                ("ResetTypeValue".to_string(), Node::I32(88)),
                ("SaveFileIndex".to_string(), Node::I32(0)),
            ])
        };
        let data: Node = Node::Map(vec![(TABLE.to_string(), Node::Arr(vec![row(5), row(6)]))]);
        let mut y: Byml = Byml { root: Node::Map(vec![("Data".to_string(), data)]), version: 2, big: false };
        assert_eq!(one_time(&mut y, &[6]).unwrap(), 1);
        let t: &[Node] = y.root.get("Data").unwrap().get(TABLE).unwrap().as_arr().unwrap();
        assert_eq!(t[1].get("ResetTypeValue"), Some(&Node::I32(24)));
        assert!(t[1].get("ExtraByte").is_none());
        assert_eq!(t[0].get("ResetTypeValue"), Some(&Node::I32(88)));
        assert!(one_time(&mut y, &[99]).is_err());
    }
}
