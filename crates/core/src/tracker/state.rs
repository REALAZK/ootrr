use crate::json::Json;
use crate::tracker::peek::Peek;
use crate::tracker::rules::{Hit, Rule, eval, rule_of};
use crate::tracker::targets::{Life, Target};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum S {
    Open,
    Found,
    Lost,
    Weak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum By {
    Save,
    User,
    Sticky,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mark {
    pub s: S,
    pub by: Option<By>,
    pub at: u64,
    pub rev: bool,
}

impl Mark {
    pub const OPEN: Mark = Mark { s: S::Open, by: None, at: 0, rev: false };
}

impl S {
    pub fn name(self) -> &'static str {
        match self {
            S::Open => "open",
            S::Found => "found",
            S::Lost => "lost",
            S::Weak => "weak",
        }
    }

    pub fn parse(s: &str) -> Option<S> {
        match s {
            "open" => Some(S::Open),
            "found" => Some(S::Found),
            "lost" => Some(S::Lost),
            "weak" => Some(S::Weak),
            _ => None,
        }
    }
}

impl By {
    pub fn name(self) -> &'static str {
        match self {
            By::Save => "save",
            By::User => "user",
            By::Sticky => "sticky",
        }
    }

    pub fn parse(s: &str) -> Option<By> {
        match s {
            "save" => Some(By::Save),
            "user" => Some(By::User),
            "sticky" => Some(By::Sticky),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub ids: Vec<String>,
    pub rules: Vec<Rule>,
    pub life: Vec<Life>,
    pub marks: Vec<Mark>,
    pub last: Vec<Hit>,
}

impl Track {
    pub fn new(ts: &[Target]) -> Track {
        Track {
            ids: ts.iter().map(|t| t.id.clone()).collect(),
            rules: ts.iter().map(rule_of).collect(),
            life: ts.iter().map(|t| t.life).collect(),
            marks: vec![Mark::OPEN; ts.len()],
            last: vec![Hit::Off; ts.len()],
        }
    }

    fn step(&mut self, i: usize, h: Hit, seq: u64) -> bool {
        let o: Hit = self.last[i];
        self.last[i] = h;
        if h == o {
            return false;
        }
        let m: Mark = self.marks[i];
        let next: Mark = match h {
            Hit::On if m.s == S::Found && m.by != Some(By::Sticky) => m,
            Hit::On => Mark { s: S::Found, by: Some(By::Save), at: seq, rev: true },
            Hit::Weak if m.s == S::Open => Mark { s: S::Weak, by: Some(By::Save), at: seq, rev: true },
            Hit::Off if o == Hit::On && m.s == S::Found && m.by == Some(By::Save) => {
                if self.life[i].volatile() {
                    Mark { by: Some(By::Sticky), at: seq, ..m }
                } else {
                    Mark { s: S::Lost, by: Some(By::Save), at: seq, rev: true }
                }
            }
            Hit::Off if o == Hit::Weak && m.s == S::Weak => Mark { s: S::Open, by: None, at: seq, rev: m.rev },
            _ => m,
        };
        if next == m {
            return false;
        }
        self.marks[i] = next;
        true
    }

    pub fn apply(&mut self, p: &Peek, seq: u64) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        for i in 0..self.rules.len() {
            let h: Hit = eval(&self.rules[i], p);
            if self.step(i, h, seq) {
                out.push(i);
            }
        }
        out
    }

    pub fn user(&mut self, i: usize, on: bool, seq: u64) -> bool {
        let Some(m) = self.marks.get(i).copied() else {
            return false;
        };
        let next: Mark = if on {
            Mark { s: S::Found, by: Some(By::User), at: seq, rev: true }
        } else {
            Mark { s: S::Open, by: None, at: seq, rev: m.rev }
        };
        if (next.s, next.by) == (m.s, m.by) {
            return false;
        }
        self.marks[i] = next;
        true
    }

    pub fn find(&self, id: &str) -> Option<usize> {
        self.ids.iter().position(|x| x == id)
    }

    pub fn to_json(&self) -> Json {
        let rows: Vec<Json> = self
            .marks
            .iter()
            .enumerate()
            .filter(|(_, m)| m.s != S::Open || m.rev)
            .map(|(i, m)| {
                Json::obj()
                    .with("id", self.ids[i].as_str().into())
                    .with("s", m.s.name().into())
                    .with("by", m.by.map(|b| Json::from(b.name())).unwrap_or(Json::Null))
                    .with("rev", m.rev.into())
            })
            .collect();
        Json::obj().with("format", Json::Int(1)).with("marks", Json::Arr(rows))
    }

    pub fn load(&mut self, j: &Json) -> usize {
        let mut n: usize = 0;
        for r in j.get("marks").and_then(|x| x.as_arr()).unwrap_or(&[]) {
            let Some(i) = r.get("id").and_then(|x| x.as_str()).and_then(|id| self.find(id)) else {
                continue;
            };
            let Some(s) = r.get("s").and_then(|x| x.as_str()).and_then(S::parse) else {
                continue;
            };
            let by: Option<By> = r.get("by").and_then(|x| x.as_str()).and_then(By::parse);
            let rev: bool = r.get("rev").and_then(|x| x.as_bool()).unwrap_or(s != S::Open);
            self.marks[i] = Mark { s, by, at: 0, rev };
            self.last[i] = match (s, by) {
                (S::Found, Some(By::Save)) => Hit::On,
                (S::Weak, _) => Hit::Weak,
                _ => Hit::Off,
            };
            n += 1;
        }
        n
    }

    pub fn found(&self) -> usize {
        self.marks.iter().filter(|m| m.s == S::Found).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::murmur::murmur3_32;
    use crate::save::fixture::{Cell, raw};
    use crate::tracker::peek::{Lim, peek_with};
    use crate::tracker::targets::{How, all};

    fn pick(kind: &str) -> (usize, &'static Target) {
        all().unwrap().iter().enumerate().find(|(_, t)| t.kind == kind && t.how != How::None).unwrap()
    }

    fn save(flags: &[&str], keys: &[u64]) -> Peek {
        let cells: Vec<(u32, Cell)> = flags.iter().map(|f| (murmur3_32(f.as_bytes(), 0), Cell::Inline(1))).collect();
        let mut ks: Vec<u64> = keys.to_vec();
        ks.sort();
        peek_with(&raw(&cells, &ks), &Lim::ANY).unwrap()
    }

    #[test]
    fn a_new_check_is_found_by_the_save() {
        let mut t: Track = Track::new(all().unwrap());
        let (i, c): (usize, &Target) = pick("chest");
        assert!(t.apply(&save(&[], &[]), 1).is_empty());
        assert_eq!(t.apply(&save(&[c.flags[0].as_str()], &[]), 2), vec![i]);
        assert_eq!(t.marks[i], Mark { s: S::Found, by: Some(By::Save), at: 2, rev: true });
        assert!(t.apply(&save(&[c.flags[0].as_str()], &[]), 3).is_empty());
    }

    #[test]
    fn a_permanent_check_that_vanishes_is_lost_on_reload() {
        let mut t: Track = Track::new(all().unwrap());
        let (i, c): (usize, &Target) = pick("chest");
        t.apply(&save(&[c.flags[0].as_str()], &[]), 1);
        assert_eq!(t.apply(&save(&[], &[]), 2), vec![i]);
        assert_eq!(t.marks[i].s, S::Lost);
        t.apply(&save(&[c.flags[0].as_str()], &[]), 3);
        assert_eq!(t.marks[i].s, S::Found);
    }

    #[test]
    fn volatile_checks_are_sticky_until_the_user_clears_them() {
        let mut t: Track = Track::new(all().unwrap());
        let (i, p): (usize, &Target) = pick("pot");
        t.apply(&save(&[], &[p.keys[0]]), 1);
        assert_eq!(t.marks[i].by, Some(By::Save));
        assert_eq!(t.apply(&save(&[], &[]), 2), vec![i]);
        assert_eq!((t.marks[i].s, t.marks[i].by), (S::Found, Some(By::Sticky)));
        assert!(t.apply(&save(&[], &[]), 3).is_empty());
        assert_eq!(t.marks[i].s, S::Found);
        assert!(t.user(i, false, 4));
        assert_eq!(t.marks[i].s, S::Open);
        assert!(t.marks[i].rev);
        t.apply(&save(&[], &[p.keys[0]]), 5);
        assert_eq!((t.marks[i].s, t.marks[i].by), (S::Found, Some(By::Save)));
    }

    #[test]
    fn a_user_clear_is_not_undone_while_the_save_stays_the_same() {
        let mut t: Track = Track::new(all().unwrap());
        let (i, c): (usize, &Target) = pick("chest");
        let s: Peek = save(&[c.flags[0].as_str()], &[]);
        t.apply(&s, 1);
        assert!(t.user(i, false, 2));
        assert!(t.apply(&s, 3).is_empty());
        assert_eq!(t.marks[i].s, S::Open);
        assert!(!t.user(i, false, 4));
        assert!(t.user(i, true, 5));
        assert_eq!(t.marks[i].by, Some(By::User));
        assert!(t.apply(&save(&[], &[]), 6).is_empty());
        assert_eq!(t.marks[i].s, S::Found);
    }

    #[test]
    fn weak_evidence_comes_and_goes() {
        let mut t: Track = Track::new(all().unwrap());
        let (i, c): (usize, &Target) = all().unwrap().iter().enumerate().find(|(_, t)| t.weak).unwrap();
        t.apply(&save(&[c.flags[0].as_str()], &[]), 1);
        assert_eq!(t.marks[i].s, S::Weak);
        t.apply(&save(&[], &[]), 2);
        assert_eq!(t.marks[i].s, S::Open);
    }

    #[test]
    fn marks_survive_a_round_trip_and_rebuild_the_baseline() {
        let mut t: Track = Track::new(all().unwrap());
        let (i, c): (usize, &Target) = pick("chest");
        let (j, p): (usize, &Target) = pick("pot");
        t.apply(&save(&[c.flags[0].as_str()], &[p.keys[0]]), 1);
        t.apply(&save(&[c.flags[0].as_str()], &[]), 2);
        let mut u: Track = Track::new(all().unwrap());
        assert_eq!(u.load(&t.to_json()), 2);
        assert_eq!((u.marks[i].s, u.marks[j].by), (S::Found, Some(By::Sticky)));
        assert_eq!(u.apply(&save(&[], &[]), 3), vec![i]);
        assert_eq!(u.marks[i].s, S::Lost);
        assert_eq!(u.found(), 1);
    }
}
