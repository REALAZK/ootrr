use std::collections::BTreeMap as Map;

use super::model::{Kind, Schema, Setting, Value};
use crate::error::{Error, Res};
use crate::json::Json;

pub type Vals = Map<String, Value>;

impl Schema {
    pub fn defaults(&self) -> Vals {
        let mut out: Vals = Map::new();
        for s in &self.settings {
            out.insert(s.id.clone(), s.default.clone());
        }
        out
    }

    pub fn wire_defaults(&self) -> Vals {
        let mut out: Vals = Map::new();
        for s in &self.settings {
            out.insert(s.id.clone(), s.wire_value().clone());
        }
        out
    }

    pub fn enabled(&self, s: &Setting, v: &Vals) -> bool {
        for d in &s.deps {
            match v.get(&d.on) {
                Some(x) if d.cond.holds(x) => {}
                _ => return false,
            }
        }
        true
    }

    fn canon(&self, s: &Setting, x: &mut Value) {
        if let Value::Set(xs) = x {
            match s.kind {
                Kind::Multi => {
                    xs.sort_by_key(|c| s.choice_index(c).unwrap_or(usize::MAX));
                    xs.dedup();
                }
                _ => {
                    xs.sort();
                    xs.dedup();
                }
            }
        }
    }

    pub fn normalize(&self, v: &mut Vals) {
        v.retain(|k, _| self.get(k).is_some());
        for s in &self.settings {
            if !v.contains_key(&s.id) {
                v.insert(s.id.clone(), s.default.clone());
            }
            if !self.enabled(s, v) {
                v.insert(s.id.clone(), s.off_value().clone());
            }
            if let Some(x) = v.get_mut(&s.id) {
                self.canon(s, x);
            }
        }
    }

    pub fn validate(&self, v: &Vals) -> Res<()> {
        for s in &self.settings {
            let x: &Value = v
                .get(&s.id)
                .ok_or_else(|| Error::Msg(format!("setting {} has no value", s.id)))?;
            s.check(x)?;
            if let (Some(b), Value::Int(n)) = (&s.max_by, x)
                && let Some(Value::Int(m)) = v.get(b)
                && n > m
            {
                return Err(Error::Msg(format!("setting {} is {n}, which exceeds {b} at {m}", s.id)));
            }
        }
        Ok(())
    }

    pub fn from_json(&self, j: &Json) -> Res<(Vals, Vec<String>)> {
        let m: &[(String, Json)] = j
            .as_obj()
            .ok_or_else(|| Error::Msg(format!("settings must be a json object, not a {}", j.kind())))?;
        let mut v: Vals = self.defaults();
        let mut warn: Vec<String> = Vec::new();
        for (k, x) in m {
            match self.get(k) {
                None => warn.push(format!("unknown setting {k} ignored")),
                Some(s) => {
                    let val: Value = Value::from_json(x).map_err(|e| Error::Msg(format!("setting {k}: {e}")))?;
                    s.check(&val)?;
                    v.insert(k.clone(), val);
                }
            }
        }
        self.normalize(&mut v);
        self.validate(&v)?;
        Ok((v, warn))
    }

    pub fn to_json(&self, v: &Vals) -> Json {
        let mut out: Vec<(String, Json)> = Vec::with_capacity(self.settings.len());
        for s in &self.settings {
            let x: &Value = v.get(&s.id).unwrap_or(&s.default);
            out.push((s.id.clone(), x.to_json()));
        }
        Json::Obj(out)
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::Group;
    use super::*;
    use crate::json;

    fn sch() -> Schema {
        Schema {
            version: 1,
            groups: vec![Group { id: "g".into(), label: "G".into(), tip: "t".into() }],
            settings: vec![
                Setting::choice(1, "mode", "g", "Mode", "off").tip("t").opts(&[("off", "Off"), ("stones", "Stones"), ("both", "Both")]),
                Setting::number(2, "stones", "g", "Stones", 3, 0, 3).tip("t").when("mode", "stones"),
                Setting::toggle(3, "hunt", "g", "Hunt", false).tip("t"),
                Setting::number(4, "count", "g", "Count", 30, 1, 99).tip("t").when_on("hunt"),
                Setting::number(5, "goal", "g", "Goal", 20, 1, 99).tip("t").when_on("hunt").max_by("count"),
                Setting::multi(6, "items", "g", "Items").tip("t").opts(&[("b", "B"), ("a", "A"), ("c", "C")]),
                Setting::list(7, "skip", "g", "Skip", 3).tip("t"),
                Setting::number(8, "forced", "g", "Forced", 1, 0, 9).tip("t").when_off("hunt").disabled_as(Value::Int(7)),
            ],
        }
    }

    #[test]
    fn defaults_are_normalized_and_valid() {
        let s: Schema = sch();
        let mut v: Vals = s.defaults();
        let before: Vals = v.clone();
        s.normalize(&mut v);
        assert!(s.validate(&v).is_ok());
        assert_eq!(v.get("stones"), Some(&Value::Int(3)));
        assert_eq!(v.get("count"), Some(&Value::Int(30)));
        assert_eq!(v.get("forced"), Some(&Value::Int(1)));
        assert_eq!(before.len(), v.len());
    }

    #[test]
    fn disabled_settings_reset_to_their_off_value() {
        let s: Schema = sch();
        let mut v: Vals = s.defaults();
        v.insert("stones".into(), Value::Int(1));
        v.insert("count".into(), Value::Int(50));
        s.normalize(&mut v);
        assert_eq!(v["stones"], Value::Int(3));
        assert_eq!(v["count"], Value::Int(30));
        v.insert("hunt".into(), Value::Bool(true));
        s.normalize(&mut v);
        assert_eq!(v["forced"], Value::Int(7));
        assert!(s.enabled(s.get("count").unwrap(), &v));
    }

    #[test]
    fn sets_are_canonical() {
        let s: Schema = sch();
        let mut v: Vals = s.defaults();
        v.insert("items".into(), Value::set(&["c", "a", "b", "a"]));
        v.insert("skip".into(), Value::set(&["z", "m", "z"]));
        s.normalize(&mut v);
        assert_eq!(v["items"], Value::set(&["b", "a", "c"]));
        assert_eq!(v["skip"], Value::set(&["m", "z"]));
    }

    #[test]
    fn from_json_applies_and_warns() {
        let s: Schema = sch();
        let j: Json = json::parse(r#"{"mode":"stones","stones":2,"mystery":5,"hunt":true,"count":10,"goal":10}"#).unwrap();
        let (v, w) = s.from_json(&j).unwrap();
        assert_eq!(v["stones"], Value::Int(2));
        assert_eq!(v["goal"], Value::Int(10));
        assert_eq!(w, vec!["unknown setting mystery ignored".to_string()]);
    }

    #[test]
    fn from_json_rejects_bad_values() {
        let s: Schema = sch();
        for t in [
            r#"{"mode":"nope"}"#,
            r#"{"mode":1}"#,
            r#"{"hunt":"yes"}"#,
            r#"{"hunt":true,"count":0}"#,
            r#"{"hunt":true,"count":10,"goal":11}"#,
            r#"{"items":["zzz"]}"#,
            r#"{"skip":["a","b","c","d"]}"#,
            r#"{"hunt":null}"#,
            r#"[1]"#,
        ] {
            assert!(s.from_json(&json::parse(t).unwrap()).is_err(), "accepted {t}");
        }
    }

    #[test]
    fn to_json_lists_every_setting_in_schema_order() {
        let s: Schema = sch();
        let v: Vals = s.from_json(&Json::obj()).unwrap().0;
        let j: Json = s.to_json(&v);
        let keys: Vec<&str> = j.as_obj().unwrap().iter().map(|e| e.0.as_str()).collect();
        assert_eq!(keys, vec!["mode", "stones", "hunt", "count", "goal", "items", "skip", "forced"]);
        assert_eq!(s.from_json(&j).unwrap().0, v);
    }

    #[test]
    fn validate_reports_missing_values() {
        let s: Schema = sch();
        let mut v: Vals = s.defaults();
        v.remove("mode");
        assert!(s.validate(&v).is_err());
    }
}
