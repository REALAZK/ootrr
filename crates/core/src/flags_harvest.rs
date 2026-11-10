use std::collections::HashMap as Map;
use std::collections::HashSet as Set;

use crate::error::Res;
use crate::flags::{Body, FlagList, KEYED, hash_name};
use crate::flags_scan::{Walked, class_of, runs, walk};
use crate::murmur::murmur3_32;
use crate::romfs::Romfs;

pub type Mem = Map<u32, String>;

const FLAG: u8 = 1;
const MEMBER: u8 = 2;
const MIN_LEN: usize = 3;
const MIN_NAME: usize = 4;
const MAX_LEN: usize = 120;

pub type Evidence = Map<String, Vec<(String, String)>>;

pub struct Harvest {
    targets: Map<u32, u8>,
    seen: Set<u64>,
    want: Set<String>,
    at: (String, String),
    pub evidence: Evidence,
    pub tested: u64,
    pub strings: u64,
    pub members: Mem,
}

pub fn targets_of(l: &FlagList) -> Map<u32, u8> {
    let mut t: Map<u32, u8> = Map::new();
    for f in &l.flags {
        if f.ty != KEYED {
            *t.entry(f.hash as u32).or_insert(0) |= FLAG;
        }
        if let Body::Members(m) = &f.body {
            for x in m {
                *t.entry(x.0).or_insert(0) |= MEMBER;
            }
        }
    }
    t
}

fn plausible(s: &str) -> bool {
    let b: &[u8] = s.as_bytes();
    let shape: bool = b
        .iter()
        .all(|c| c.is_ascii_alphanumeric() || *c == b'_' || *c == b'.' || *c == b'-');
    let word: bool =
        b.iter().any(|c| c.is_ascii_uppercase()) || b.iter().any(|c| *c == b'_' || *c == b'.');
    shape
        && word
        && b.len() >= MIN_NAME
        && b.len() <= MAX_LEN
        && b.iter().any(|c| c.is_ascii_alphabetic())
}

fn fingerprint(s: &str) -> u64 {
    ((murmur3_32(s.as_bytes(), 1) as u64) << 32) | murmur3_32(s.as_bytes(), 2) as u64
}

impl Harvest {
    pub fn new(l: &FlagList) -> Harvest {
        Harvest {
            targets: targets_of(l),
            seen: Set::new(),
            want: Set::new(),
            at: (String::new(), String::new()),
            evidence: Evidence::new(),
            tested: 0,
            strings: 0,
            members: Mem::new(),
        }
    }

    fn test(&mut self, l: &mut FlagList, c: &str, how: &str) {
        if c.len() < MIN_LEN {
            return;
        }
        self.tested += 1;
        self.note(c);
        let h: u32 = hash_name(c);
        let Some(k) = self.targets.get(&h).copied() else {
            return;
        };
        if !plausible(c) {
            return;
        }
        if k & FLAG != 0 {
            l.name(h as u64, c, how);
        }
        if k & MEMBER != 0 {
            self.members.entry(h).or_insert_with(|| c.to_string());
        }
    }

    pub fn want(&mut self, names: Set<String>) {
        self.want = names;
    }

    fn note(&mut self, s: &str) {
        if self.want.contains(s) {
            let at: (String, String) = self.at.clone();
            let v: &mut Vec<(String, String)> = self.evidence.entry(s.to_string()).or_default();
            if v.len() < 4 && !v.iter().any(|x| x.0 == at.0) {
                v.push(at);
            }
        }
    }

    pub fn feed(&mut self, l: &mut FlagList, s: &str, class: &str) {
        self.note(s);
        if s.len() > MAX_LEN || !s.is_ascii() || !self.seen.insert(fingerprint(s)) {
            return;
        }
        self.strings += 1;
        let how: String = format!("string:{class}");
        self.test(l, s, &how);
        if s.len() > MIN_LEN {
            self.test(l, &s[1..], &how);
        }
        if s.len() > MIN_LEN + 1 {
            self.test(l, &s[2..], &how);
        }
        for piece in
            s.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-'))
        {
            if piece.len() != s.len() {
                self.test(l, piece, &how);
            }
        }
    }

    pub fn run(&mut self, rf: &mut Romfs, l: &mut FlagList) -> Res<Walked> {
        let w: Walked = walk(rf, &mut |p, d| {
            let cl: &'static str = class_of(p);
            self.at = (cl.to_string(), p.to_string());
            runs(d, MIN_LEN, &mut |s| self.feed(l, s, cl));
        })?;
        Ok(w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::Flag;
    use crate::json::Json;

    fn flag(n: &str) -> Flag {
        Flag {
            hash: hash_name(n) as u64,
            ty: "Bool".to_string(),
            def: Json::Bool(false),
            reset: 0,
            save: 0,
            extra: None,
            size: None,
            body: Body::Plain,
            name: None,
            how: None,
        }
    }

    #[test]
    fn feed_names_by_piece() {
        let mut l: FlagList =
            FlagList::from_flags(vec![flag("IsGet.Item_Bomb"), flag("Other_Flag")]);
        let mut h: Harvest = Harvest::new(&l);
        h.feed(&mut l, "Event/IsGet.Item_Bomb:Other_Flag", "t");
        assert_eq!(l.named(), 2);
        assert_eq!(l.flags[0].how.as_deref(), Some("string:t"));
    }

    #[test]
    fn junk_is_not_named() {
        let mut l: FlagList = FlagList::from_flags(vec![
            flag("%To"),
            flag("zSA"),
            flag("mooona"),
            flag("123456789"),
        ]);
        let mut h: Harvest = Harvest::new(&l);
        for s in ["%To", "zSA", "mooona", "123456789"] {
            h.feed(&mut l, s, "t");
        }
        assert_eq!(l.named(), 0);
    }

    #[test]
    fn plausible_shapes() {
        assert!(plausible("IsGet"));
        assert!(plausible("Npc_001"));
        assert!(plausible("12-Npc_U_DekuTreeChild-A-0"));
        assert!(!plausible("abcd"));
        assert!(!plausible("Ab"));
    }
}
