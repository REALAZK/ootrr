use std::collections::BTreeMap as BM;

use crate::byml::Byml;
use crate::error::{Error as E, Res};
use crate::pot_slice::scan::peek;
use crate::pot_slice::targets::Give;
use crate::pots::give;
use crate::romfs::Romfs;
use crate::slice::banc::fingerprint;
use crate::slice::data::Data;
use crate::slice::edit::Edited;

pub fn give_one(y: &mut Byml, t: &Give) -> Res<usize> {
    let seen: Vec<crate::pot_slice::scan::Peek> = peek(y, t.hash);
    if let Some(p) = seen.iter().find(|p| p.drop != t.was) {
        return Err(E::Msg(format!(
            "actor {} of {} holds {:?} where {:?} was expected, the game files differ from the ones the data was made for",
            t.hash, t.name, p.drop, t.was
        )));
    }
    Ok(give(y, t.hash, t.actor))
}

pub fn check_game(rf: &Romfs, d: &Data) -> Res<()> {
    let have: String = fingerprint(rf, &d.pots.files)?;
    if have != d.pots.game {
        return Err(E::Msg(format!(
            "the pot data was made for the game {} but this folder is {}",
            d.pots.game, have
        )));
    }
    Ok(())
}

pub fn apply(rf: &mut Romfs, d: &Data, ts: &[Give], edits: &mut Vec<Edited>) -> Res<usize> {
    if ts.is_empty() {
        return Ok(0);
    }
    check_game(rf, d)?;
    let mut by: BM<&str, Vec<&Give>> = BM::new();
    for t in ts {
        for f in &t.files {
            by.entry(f.as_str()).or_default().push(t);
        }
    }
    let mut n: usize = 0;
    for (f, list) in by {
        let at: Option<usize> = edits.iter().position(|e| e.file == f);
        let (raw, old): (Vec<u8>, usize) = match at {
            Some(i) => (edits[i].bytes.clone(), edits[i].old_len),
            None => {
                let b: Vec<u8> = rf.read_zs(f)?;
                let l: usize = b.len();
                (b, l)
            }
        };
        let mut y: Byml = Byml::parse(&raw)?;
        for t in list {
            if give_one(&mut y, t)? == 0 {
                return Err(E::Msg(format!("{} was not found in {f}, the game files differ", t.name)));
            }
            n += 1;
        }
        let bytes: Vec<u8> = y.write()?;
        match at {
            Some(i) => edits[i].bytes = bytes,
            None => edits.push(Edited {
                file: f.to_string(),
                old_len: old,
                bytes,
                hits: Vec::new(),
                used: Vec::new(),
                hearts: Vec::new(),
                wraps: Vec::new(),
                touched: Vec::new(),
            }),
        }
    }
    Ok(n)
}

pub fn hashes(ts: &[Give]) -> Vec<u64> {
    let mut v: Vec<u64> = ts.iter().map(|t| t.hash).collect();
    v.sort_unstable();
    v.dedup();
    v
}

pub fn lines(ts: &[Give], limit: usize) -> Vec<String> {
    let mut seen: Vec<&str> = Vec::new();
    let mut out: Vec<String> = Vec::new();
    for t in ts {
        if seen.contains(&t.name.as_str()) {
            continue;
        }
        seen.push(&t.name);
        if out.len() < limit {
            out.push(format!("  {} | now {} ({}) | drop before {}", t.name, t.item, t.actor, if t.was.is_empty() { "a table" } else { t.was.as_str() }));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byml::Node;
    use crate::pot_slice::data::Kind;

    fn pot(h: u64, drop: Option<&str>) -> Node {
        let mut m: Vec<(String, Node)> = vec![("Gyaml".to_string(), Node::Str("Obj_Pot_A_01".to_string())), ("Hash".to_string(), Node::U64(h))];
        if let Some(x) = drop {
            m.push(("Dynamic".to_string(), Node::Map(vec![("Drop__DropActor".to_string(), Node::Str(x.to_string()))])));
        }
        Node::Map(m)
    }

    fn file(list: Vec<Node>) -> Byml {
        Byml { root: Node::Map(vec![("Actors".to_string(), Node::Arr(list))]), version: 7, big: false }
    }

    fn give_of(h: u64, was: &str, actor: &'static str) -> Give {
        Give { name: "Pot".to_string(), kind: Kind::Pot, hash: h, files: vec!["f".to_string()], was: was.to_string(), item: "Rupees (5)".to_string(), actor }
    }

    #[test]
    fn a_bare_pot_and_a_pot_with_a_drop_both_take_the_new_pickup() {
        let mut y: Byml = file(vec![pot(1, None), pot(2, Some("Obj_Rupee_Green")), pot(3, Some("Obj_Heart"))]);
        assert_eq!(give_one(&mut y, &give_of(1, "", "Obj_Rupee_Blue")).unwrap(), 1);
        assert_eq!(give_one(&mut y, &give_of(2, "Obj_Rupee_Green", "Obj_Rupee_Blue")).unwrap(), 1);
        assert_eq!(give_one(&mut y, &give_of(9, "", "Obj_Rupee_Blue")).unwrap(), 0);
        let p = peek(&y, 1);
        assert_eq!(p[0].drop, "Obj_Rupee_Blue");
        assert_eq!(peek(&y, 2)[0].drop, "Obj_Rupee_Blue");
        assert_eq!(peek(&y, 3)[0].drop, "Obj_Heart");
    }

    #[test]
    fn a_pot_that_holds_something_else_is_a_changed_game() {
        let mut y: Byml = file(vec![pot(1, Some("Obj_Heart"))]);
        assert!(give_one(&mut y, &give_of(1, "", "Obj_Rupee_Blue")).is_err());
        assert!(give_one(&mut y, &give_of(1, "Obj_Rupee_Green", "Obj_Rupee_Blue")).is_err());
    }

    #[test]
    fn hashes_and_report_lines_are_deduplicated() {
        let a: Give = give_of(5, "", "Obj_Heart");
        let mut b: Give = give_of(5, "", "Obj_Heart");
        b.files = vec!["g".to_string()];
        let c: Give = give_of(6, "Obj_Heart", "Obj_Heart");
        assert_eq!(hashes(&[a.clone(), b.clone(), c.clone()]), vec![5, 6]);
        assert_eq!(lines(&[a, b, c], 10).len(), 1);
    }
}
