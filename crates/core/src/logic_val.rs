use std::collections::BTreeMap as BM;

use crate::error::{Error as E, Res};
use crate::logic_json::Json;

#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Val>),
    Map(BM<String, Val>),
}

impl Val {
    pub fn truthy(&self) -> bool {
        match self {
            Val::Bool(b) => *b,
            Val::Int(n) => *n != 0,
            Val::Float(f) => *f != 0.0,
            Val::Str(s) => !s.is_empty(),
            Val::List(v) => !v.is_empty(),
            Val::Map(m) => !m.is_empty(),
        }
    }

    pub fn num(&self) -> Option<f64> {
        match self {
            Val::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            Val::Int(n) => Some(*n as f64),
            Val::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn from_json(j: &Json) -> Res<Val> {
        match j {
            Json::Null => Ok(Val::Bool(false)),
            Json::Bool(b) => Ok(Val::Bool(*b)),
            Json::Num(n) => {
                if n.fract() == 0.0 && n.abs() < 9.0e15 {
                    Ok(Val::Int(*n as i64))
                } else {
                    Ok(Val::Float(*n))
                }
            }
            Json::Str(s) => Ok(Val::Str(s.clone())),
            Json::Arr(a) => {
                let mut out: Vec<Val> = Vec::with_capacity(a.len());
                for x in a {
                    out.push(Val::from_json(x)?);
                }
                Ok(Val::List(out))
            }
            Json::Obj(o) => {
                let mut out: BM<String, Val> = BM::new();
                for (k, v) in o {
                    out.insert(k.clone(), Val::from_json(v)?);
                }
                Ok(Val::Map(out))
            }
        }
    }
}

pub fn same(a: &Val, b: &Val) -> bool {
    match (a.num(), b.num()) {
        (Some(x), Some(y)) => x == y,
        _ => a == b,
    }
}

#[derive(Clone, Debug, Default)]
pub struct Settings {
    pub vars: BM<String, Val>,
}

impl Settings {
    pub fn new() -> Settings {
        Settings { vars: BM::new() }
    }

    pub fn set(&mut self, k: &str, v: Val) {
        self.vars.insert(k.to_string(), v);
    }

    pub fn set_bool(&mut self, k: &str, v: bool) {
        self.set(k, Val::Bool(v));
    }

    pub fn set_int(&mut self, k: &str, v: i64) {
        self.set(k, Val::Int(v));
    }

    pub fn set_str(&mut self, k: &str, v: &str) {
        self.set(k, Val::Str(v.to_string()));
    }

    pub fn get(&self, k: &str) -> Option<&Val> {
        self.vars.get(k)
    }

    pub fn flag(&self, k: &str) -> bool {
        self.vars.get(k).is_some_and(|v| v.truthy())
    }

    pub fn from_json(j: &Json) -> Res<Settings> {
        let o: &[(String, Json)] = j.as_obj().ok_or(E::Format("settings must be an object"))?;
        let mut out: Settings = Settings::new();
        for (k, v) in o {
            out.set(k, Val::from_json(v)?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic_json::parse;

    #[test]
    fn json_converts() {
        let j: Json = parse(r#"{"a": true, "b": 3, "c": 0.5, "d": "x", "e": [1, "y"], "f": {"g": 2}}"#).unwrap();
        let s: Settings = Settings::from_json(&j).unwrap();
        assert_eq!(s.get("a"), Some(&Val::Bool(true)));
        assert_eq!(s.get("b"), Some(&Val::Int(3)));
        assert_eq!(s.get("c"), Some(&Val::Float(0.5)));
        assert_eq!(s.get("d"), Some(&Val::Str("x".to_string())));
        assert!(matches!(s.get("e"), Some(Val::List(v)) if v.len() == 2));
        assert!(matches!(s.get("f"), Some(Val::Map(m)) if m.len() == 1));
    }

    #[test]
    fn truthiness_follows_python() {
        assert!(!Val::Int(0).truthy());
        assert!(Val::Int(2).truthy());
        assert!(!Val::Str(String::new()).truthy());
        assert!(Val::Str("a".to_string()).truthy());
        assert!(!Val::List(Vec::new()).truthy());
    }

    #[test]
    fn numbers_compare_across_kinds() {
        assert!(same(&Val::Bool(true), &Val::Int(1)));
        assert!(same(&Val::Int(2), &Val::Float(2.0)));
        assert!(!same(&Val::Int(2), &Val::Str("2".to_string())));
    }
}
