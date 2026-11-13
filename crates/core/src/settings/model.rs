use crate::error::{Error, Res};
use crate::json::Json;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Str(String),
    Int(i64),
    Set(Vec<String>),
}

impl Value {
    pub fn s(x: &str) -> Value {
        Value::Str(x.to_string())
    }

    pub fn set(x: &[&str]) -> Value {
        Value::Set(x.iter().map(|s| s.to_string()).collect())
    }

    pub fn to_json(&self) -> Json {
        match self {
            Value::Bool(b) => Json::Bool(*b),
            Value::Str(s) => Json::Str(s.clone()),
            Value::Int(n) => Json::Int(*n),
            Value::Set(v) => Json::Arr(v.iter().map(|s| Json::Str(s.clone())).collect()),
        }
    }

    pub fn from_json(j: &Json) -> Res<Value> {
        match j {
            Json::Bool(b) => Ok(Value::Bool(*b)),
            Json::Str(s) => Ok(Value::Str(s.clone())),
            Json::Int(n) => Ok(Value::Int(*n)),
            Json::Num(f) if f.fract() == 0.0 && f.abs() < 9.0e15 => Ok(Value::Int(*f as i64)),
            Json::Arr(a) => {
                let mut out: Vec<String> = Vec::with_capacity(a.len());
                for x in a {
                    match x {
                        Json::Str(s) => out.push(s.clone()),
                        _ => return Err(Error::Msg("a list value must hold only strings".into())),
                    }
                }
                Ok(Value::Set(out))
            }
            other => Err(Error::Msg(format!("a setting value cannot be a json {}", other.kind()))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Toggle,
    Choice,
    Number,
    Multi,
    List,
}

impl Kind {
    pub fn name(&self) -> &'static str {
        match self {
            Kind::Toggle => "toggle",
            Kind::Choice => "choice",
            Kind::Number => "number",
            Kind::Multi => "multi",
            Kind::List => "list",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tag {
    Core,
    Common,
    Niche,
}

impl Tag {
    pub fn name(&self) -> &'static str {
        match self {
            Tag::Core => "core",
            Tag::Common => "common",
            Tag::Niche => "niche",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    pub id: String,
    pub label: String,
    pub tip: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub id: String,
    pub label: String,
    pub tip: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Cond {
    Is(Value),
    IsNot(Value),
    In(Vec<Value>),
}

impl Cond {
    pub fn holds(&self, v: &Value) -> bool {
        match self {
            Cond::Is(x) => v == x,
            Cond::IsNot(x) => v != x,
            Cond::In(xs) => xs.contains(v),
        }
    }

    pub fn values(&self) -> Vec<&Value> {
        match self {
            Cond::Is(x) | Cond::IsNot(x) => vec![x],
            Cond::In(xs) => xs.iter().collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dep {
    pub on: String,
    pub cond: Cond,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Setting {
    pub num: u16,
    pub id: String,
    pub label: String,
    pub group: String,
    pub kind: Kind,
    pub tag: Tag,
    pub tip: String,
    pub default: Value,
    pub choices: Vec<Choice>,
    pub min: i64,
    pub max: i64,
    pub max_by: Option<String>,
    pub max_items: usize,
    pub deps: Vec<Dep>,
    pub disabled: Option<Value>,
    pub wire: Option<Value>,
}

impl Setting {
    fn base(num: u16, id: &str, group: &str, label: &str, kind: Kind, default: Value) -> Setting {
        Setting {
            num,
            id: id.to_string(),
            label: label.to_string(),
            group: group.to_string(),
            kind,
            tag: Tag::Common,
            tip: String::new(),
            default,
            choices: Vec::new(),
            min: 0,
            max: 0,
            max_by: None,
            max_items: 0,
            deps: Vec::new(),
            disabled: None,
            wire: None,
        }
    }

    pub fn toggle(num: u16, id: &str, group: &str, label: &str, default: bool) -> Setting {
        Setting::base(num, id, group, label, Kind::Toggle, Value::Bool(default))
    }

    pub fn choice(num: u16, id: &str, group: &str, label: &str, default: &str) -> Setting {
        Setting::base(num, id, group, label, Kind::Choice, Value::s(default))
    }

    pub fn number(num: u16, id: &str, group: &str, label: &str, default: i64, min: i64, max: i64) -> Setting {
        let mut s: Setting = Setting::base(num, id, group, label, Kind::Number, Value::Int(default));
        s.min = min;
        s.max = max;
        s
    }

    pub fn multi(num: u16, id: &str, group: &str, label: &str) -> Setting {
        Setting::base(num, id, group, label, Kind::Multi, Value::Set(Vec::new()))
    }

    pub fn list(num: u16, id: &str, group: &str, label: &str, max_items: usize) -> Setting {
        let mut s: Setting = Setting::base(num, id, group, label, Kind::List, Value::Set(Vec::new()));
        s.max_items = max_items;
        s
    }

    pub fn tag(mut self, t: Tag) -> Setting {
        self.tag = t;
        self
    }

    pub fn tip(mut self, t: &str) -> Setting {
        self.tip = t.to_string();
        self
    }

    pub fn opt(mut self, id: &str, label: &str, tip: &str) -> Setting {
        self.choices.push(Choice { id: id.to_string(), label: label.to_string(), tip: tip.to_string() });
        self
    }

    pub fn opts(mut self, items: &[(&str, &str)]) -> Setting {
        for (id, label) in items {
            self.choices.push(Choice { id: id.to_string(), label: label.to_string(), tip: String::new() });
        }
        self
    }

    pub fn max_by(mut self, id: &str) -> Setting {
        self.max_by = Some(id.to_string());
        self
    }

    pub fn dep(mut self, on: &str, cond: Cond) -> Setting {
        self.deps.push(Dep { on: on.to_string(), cond });
        self
    }

    pub fn when(self, on: &str, v: &str) -> Setting {
        self.dep(on, Cond::Is(Value::s(v)))
    }

    pub fn when_on(self, on: &str) -> Setting {
        self.dep(on, Cond::Is(Value::Bool(true)))
    }

    pub fn when_off(self, on: &str) -> Setting {
        self.dep(on, Cond::Is(Value::Bool(false)))
    }

    pub fn disabled_as(mut self, v: Value) -> Setting {
        self.disabled = Some(v);
        self
    }

    pub fn wire_as(mut self, v: Value) -> Setting {
        self.wire = Some(v);
        self
    }

    pub fn wire_value(&self) -> &Value {
        self.wire.as_ref().unwrap_or(&self.default)
    }

    pub fn choice_index(&self, id: &str) -> Option<usize> {
        self.choices.iter().position(|c| c.id == id)
    }

    pub fn off_value(&self) -> &Value {
        self.disabled.as_ref().unwrap_or(&self.default)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Schema {
    pub version: u32,
    pub groups: Vec<Group>,
    pub settings: Vec<Setting>,
}

impl Schema {
    pub fn get(&self, id: &str) -> Option<&Setting> {
        self.settings.iter().find(|s| s.id == id)
    }

    pub fn by_num(&self, num: u16) -> Option<&Setting> {
        self.settings.iter().find(|s| s.num == num)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_json_round_trip() {
        for v in [
            Value::Bool(true),
            Value::s("x"),
            Value::Int(-4),
            Value::set(&["a", "b"]),
            Value::Set(Vec::new()),
        ] {
            assert_eq!(Value::from_json(&v.to_json()).unwrap(), v);
        }
    }

    #[test]
    fn value_from_json_rejects_odd_shapes() {
        assert!(Value::from_json(&Json::Null).is_err());
        assert!(Value::from_json(&Json::obj()).is_err());
        assert!(Value::from_json(&Json::Num(1.5)).is_err());
        assert!(Value::from_json(&Json::Arr(vec![Json::Int(1)])).is_err());
        assert_eq!(Value::from_json(&Json::Num(3.0)).unwrap(), Value::Int(3));
    }

    #[test]
    fn conditions() {
        let c: Cond = Cond::In(vec![Value::s("a"), Value::s("b")]);
        assert!(c.holds(&Value::s("a")));
        assert!(!c.holds(&Value::s("c")));
        assert!(Cond::IsNot(Value::Bool(true)).holds(&Value::Bool(false)));
        assert_eq!(c.values().len(), 2);
    }

    #[test]
    fn builder_sets_fields() {
        let s: Setting = Setting::choice(1, "x", "g", "X", "a")
            .tag(Tag::Core)
            .tip("t")
            .opt("a", "A", "")
            .opts(&[("b", "B")])
            .when_on("y");
        assert_eq!(s.choices.len(), 2);
        assert_eq!(s.choice_index("b"), Some(1));
        assert_eq!(s.deps.len(), 1);
        assert_eq!(s.tag, Tag::Core);
        assert_eq!(s.off_value(), &Value::s("a"));
    }
}
