use std::collections::BTreeSet as Set;

use super::model::{Kind, Schema, Setting, Value};
use crate::error::{Error, Res};

pub const ITEM_MAX_BYTES: usize = 96;
pub const LIST_MAX_ITEMS: usize = 4096;

pub fn id_ok(s: &str) -> bool {
    let mut c: std::str::Chars = s.chars();
    match c.next() {
        Some(f) if f.is_ascii_lowercase() => {}
        _ => return false,
    }
    s.len() <= 48 && c.all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || x == '_')
}

pub fn list_item_ok(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= ITEM_MAX_BYTES
        && s.trim() == s
        && s.chars().all(|c| c.is_ascii_alphanumeric() || " _.:'()/-".contains(c))
}

fn bad(s: &Setting, m: &str) -> Error {
    Error::Msg(format!("setting {}: {m}", s.id))
}

impl Setting {
    pub fn check(&self, v: &Value) -> Res<()> {
        match (self.kind, v) {
            (Kind::Toggle, Value::Bool(_)) => Ok(()),
            (Kind::Choice, Value::Str(x)) => {
                if self.choice_index(x).is_some() {
                    Ok(())
                } else {
                    Err(bad(self, &format!("{x:?} is not one of its choices")))
                }
            }
            (Kind::Number, Value::Int(n)) => {
                if *n < self.min || *n > self.max {
                    Err(bad(self, &format!("{n} is outside {}..={}", self.min, self.max)))
                } else {
                    Ok(())
                }
            }
            (Kind::Multi, Value::Set(xs)) => {
                for x in xs {
                    if self.choice_index(x).is_none() {
                        return Err(bad(self, &format!("{x:?} is not one of its choices")));
                    }
                }
                Ok(())
            }
            (Kind::List, Value::Set(xs)) => {
                if xs.len() > self.max_items {
                    return Err(bad(self, &format!("more than {} entries", self.max_items)));
                }
                for x in xs {
                    if !list_item_ok(x) {
                        return Err(bad(self, &format!("entry {x:?} has characters or a length that is not allowed")));
                    }
                }
                Ok(())
            }
            _ => Err(bad(self, &format!("expected a {} value", self.kind.name()))),
        }
    }
}

impl Schema {
    pub fn problems(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut gids: Set<&str> = Set::new();
        for g in &self.groups {
            if !id_ok(&g.id) {
                out.push(format!("group id {:?} is malformed", g.id));
            }
            if !gids.insert(&g.id) {
                out.push(format!("group id {} is duplicated", g.id));
            }
            if g.label.is_empty() {
                out.push(format!("group {} has no label", g.id));
            }
        }
        let mut ids: Set<&str> = Set::new();
        let mut nums: Set<u16> = Set::new();
        let mut seen: Vec<&Setting> = Vec::new();
        for s in &self.settings {
            let n: &str = &s.id;
            if !id_ok(n) {
                out.push(format!("setting id {n:?} is malformed"));
            }
            if !ids.insert(n) {
                out.push(format!("setting id {n} is duplicated"));
            }
            if s.num == 0 || !nums.insert(s.num) {
                out.push(format!("setting {n} has a zero or duplicated num {}", s.num));
            }
            if s.label.is_empty() {
                out.push(format!("setting {n} has no label"));
            }
            if s.tip.is_empty() {
                out.push(format!("setting {n} has no tooltip"));
            }
            if !gids.contains(s.group.as_str()) {
                out.push(format!("setting {n} names the missing group {}", s.group));
            }
            self.problems_kind(s, &mut out);
            if let Err(e) = s.check(&s.default) {
                out.push(format!("default is invalid: {e}"));
            }
            if let Some(d) = &s.disabled
                && let Err(e) = s.check(d)
            {
                out.push(format!("disabled value is invalid: {e}"));
            }
            if let Some(b) = &s.max_by {
                match seen.iter().find(|x| &x.id == b) {
                    Some(t) if t.kind == Kind::Number && s.kind == Kind::Number => {
                        if let (Value::Int(d), Value::Int(m)) = (&s.default, &t.default)
                            && d > m
                        {
                            out.push(format!("setting {n} default exceeds the default of {b}"));
                        }
                    }
                    _ => out.push(format!("setting {n} max_by {b} must be an earlier number setting")),
                }
            }
            for d in &s.deps {
                match seen.iter().find(|x| x.id == d.on) {
                    None => out.push(format!("setting {n} depends on {} which is missing or listed later", d.on)),
                    Some(t) => {
                        for v in d.cond.values() {
                            if t.check(v).is_err() {
                                out.push(format!("setting {n} compares {} with the invalid value {v:?}", d.on));
                            }
                        }
                    }
                }
            }
            seen.push(s);
        }
        out
    }

    fn problems_kind(&self, s: &Setting, out: &mut Vec<String>) {
        let n: &str = &s.id;
        let has_choices: bool = matches!(s.kind, Kind::Choice | Kind::Multi);
        if has_choices && s.choices.is_empty() {
            out.push(format!("setting {n} has no choices"));
        }
        if !has_choices && !s.choices.is_empty() {
            out.push(format!("setting {n} has choices but is a {}", s.kind.name()));
        }
        let mut cids: Set<&str> = Set::new();
        for c in &s.choices {
            if !id_ok(&c.id) {
                out.push(format!("setting {n} choice id {:?} is malformed", c.id));
            }
            if !cids.insert(&c.id) {
                out.push(format!("setting {n} choice {} is duplicated", c.id));
            }
            if c.label.is_empty() {
                out.push(format!("setting {n} choice {} has no label", c.id));
            }
        }
        if s.kind == Kind::Number && s.min > s.max {
            out.push(format!("setting {n} has min above max"));
        }
        if s.kind == Kind::List && (s.max_items == 0 || s.max_items > LIST_MAX_ITEMS) {
            out.push(format!("setting {n} needs max_items in 1..={LIST_MAX_ITEMS}"));
        }
        if s.kind == Kind::Multi && !matches!(s.default, Value::Set(_)) {
            out.push(format!("setting {n} default must be a set"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::{Cond, Group};
    use super::*;

    fn one(s: Vec<Setting>) -> Schema {
        Schema {
            version: 1,
            groups: vec![Group { id: "g".into(), label: "G".into(), tip: String::new() }],
            settings: s,
        }
    }

    fn ok_choice() -> Setting {
        Setting::choice(1, "a", "g", "A", "x").tip("t").opts(&[("x", "X"), ("y", "Y")])
    }

    #[test]
    fn clean_schema_has_no_problems() {
        let sch: Schema = one(vec![
            ok_choice(),
            Setting::number(2, "n", "g", "N", 3, 1, 5).tip("t").when("a", "y"),
            Setting::number(3, "m", "g", "M", 2, 1, 5).tip("t").max_by("n"),
        ]);
        assert_eq!(sch.problems(), Vec::<String>::new());
    }

    #[test]
    fn broken_schemas_are_reported() {
        let cases: Vec<(Vec<Setting>, &str)> = vec![
            (vec![ok_choice(), ok_choice()], "duplicated"),
            (vec![Setting::choice(1, "Bad", "g", "A", "x").tip("t").opts(&[("x", "X")])], "malformed"),
            (vec![Setting::choice(1, "a", "g", "A", "z").tip("t").opts(&[("x", "X")])], "default is invalid"),
            (vec![Setting::choice(1, "a", "nope", "A", "x").tip("t").opts(&[("x", "X")])], "missing group"),
            (vec![Setting::choice(1, "a", "g", "A", "x").opts(&[("x", "X")])], "no tooltip"),
            (vec![Setting::choice(1, "a", "g", "A", "x").tip("t")], "no choices"),
            (vec![ok_choice().when("later", "x"), Setting::toggle(2, "later", "g", "L", false).tip("t")], "missing or listed later"),
            (vec![ok_choice(), Setting::toggle(2, "t", "g", "T", false).tip("t").when("a", "nope")], "invalid value"),
            (vec![ok_choice(), Setting::number(2, "n", "g", "N", 9, 1, 5).tip("t")], "default is invalid"),
            (vec![Setting::number(1, "n", "g", "N", 1, 5, 1).tip("t")], "min above max"),
            (vec![ok_choice(), Setting::number(2, "n", "g", "N", 1, 0, 5).tip("t").max_by("zzz")], "max_by"),
            (vec![Setting::number(1, "n", "g", "N", 5, 0, 9).tip("t"), Setting::number(2, "m", "g", "M", 7, 0, 9).tip("t").max_by("n")], "exceeds"),
            (vec![Setting::list(1, "l", "g", "L", 0).tip("t")], "max_items"),
            (vec![Setting::choice(0, "a", "g", "A", "x").tip("t").opts(&[("x", "X")])], "num"),
            (vec![ok_choice().dep("a", Cond::Is(Value::s("x")))], "missing or listed later"),
        ];
        for (s, want) in cases {
            let p: Vec<String> = one(s).problems();
            assert!(p.iter().any(|m| m.contains(want)), "wanted {want:?} in {p:?}");
        }
    }

    #[test]
    fn value_checks() {
        let c: Setting = ok_choice();
        assert!(c.check(&Value::s("y")).is_ok());
        assert!(c.check(&Value::s("q")).is_err());
        assert!(c.check(&Value::Int(1)).is_err());
        let n: Setting = Setting::number(1, "n", "g", "N", 3, 1, 5);
        assert!(n.check(&Value::Int(5)).is_ok());
        assert!(n.check(&Value::Int(6)).is_err());
        assert!(n.check(&Value::Int(0)).is_err());
        let m: Setting = Setting::multi(1, "m", "g", "M").opts(&[("a", "A"), ("b", "B")]);
        assert!(m.check(&Value::set(&["a", "b"])).is_ok());
        assert!(m.check(&Value::set(&["c"])).is_err());
        let l: Setting = Setting::list(1, "l", "g", "L", 2);
        assert!(l.check(&Value::set(&["Deku Tree Compass Chest", "a_b"])).is_ok());
        assert!(l.check(&Value::set(&["a", "b", "c"])).is_err());
        assert!(l.check(&Value::set(&[""])).is_err());
        assert!(l.check(&Value::set(&["bad\"quote"])).is_err());
        assert!(l.check(&Value::set(&[" pad"])).is_err());
        assert!(l.check(&Value::Set(vec!["x".repeat(97)])).is_err());
        assert!(Setting::toggle(1, "t", "g", "T", false).check(&Value::s("true")).is_err());
    }

    #[test]
    fn id_rules() {
        assert!(id_ok("shuffle_chests"));
        assert!(!id_ok(""));
        assert!(!id_ok("9a"));
        assert!(!id_ok("a-b"));
        assert!(!id_ok("A"));
    }
}
