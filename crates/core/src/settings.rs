use std::sync::OnceLock;

use crate::error::{Error, Res};
use crate::json::Json;

pub mod check;
mod codec;
pub mod digest;
pub mod export;
pub mod list;
pub mod model;
pub mod presets;
pub mod seed;
pub mod state;
pub mod string;

pub use model::{Cond, Kind, Schema, Setting, Tag, Value};
pub use state::Vals;

static SCHEMA: OnceLock<Schema> = OnceLock::new();

pub fn schema() -> &'static Schema {
    SCHEMA.get_or_init(list::build)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    vals: Vals,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub settings: Settings,
    pub warnings: Vec<String>,
}

impl Default for Settings {
    fn default() -> Settings {
        let mut vals: Vals = schema().defaults();
        schema().normalize(&mut vals);
        Settings { vals }
    }
}

fn missing(id: &str) -> Error {
    Error::Msg(format!("no setting named {id}"))
}

fn mismatch(id: &str, want: &str) -> Error {
    Error::Msg(format!("setting {id} is not a {want}"))
}

impl Settings {
    pub fn values(&self) -> &Vals {
        &self.vals
    }

    pub fn get(&self, id: &str) -> Option<&Value> {
        self.vals.get(id)
    }

    pub fn set(&mut self, id: &str, v: Value) -> Res<()> {
        let s: &Setting = schema().get(id).ok_or_else(|| missing(id))?;
        s.check(&v)?;
        self.vals.insert(id.to_string(), v);
        Ok(())
    }

    pub fn flag(&self, id: &str) -> Res<bool> {
        match self.get(id) {
            Some(Value::Bool(b)) => Ok(*b),
            Some(_) => Err(mismatch(id, "toggle")),
            None => Err(missing(id)),
        }
    }

    pub fn text(&self, id: &str) -> Res<&str> {
        match self.get(id) {
            Some(Value::Str(s)) => Ok(s),
            Some(_) => Err(mismatch(id, "choice")),
            None => Err(missing(id)),
        }
    }

    pub fn int(&self, id: &str) -> Res<i64> {
        match self.get(id) {
            Some(Value::Int(n)) => Ok(*n),
            Some(_) => Err(mismatch(id, "number")),
            None => Err(missing(id)),
        }
    }

    pub fn items(&self, id: &str) -> Res<&[String]> {
        match self.get(id) {
            Some(Value::Set(v)) => Ok(v),
            Some(_) => Err(mismatch(id, "multi-select or list")),
            None => Err(missing(id)),
        }
    }

    pub fn is_enabled(&self, id: &str) -> bool {
        schema().get(id).is_some_and(|s| schema().enabled(s, &self.vals))
    }

    pub fn normalize(&mut self) {
        schema().normalize(&mut self.vals);
    }

    pub fn validate(&self) -> Res<()> {
        schema().validate(&self.vals)
    }

    pub fn finish(mut self) -> Res<Settings> {
        self.normalize();
        self.validate()?;
        Ok(self)
    }

    pub fn to_json(&self) -> Json {
        schema().to_json(&self.vals)
    }

    pub fn from_json(j: &Json) -> Res<Loaded> {
        let (vals, warnings) = schema().from_json(j)?;
        Ok(Loaded { settings: Settings { vals }, warnings })
    }

    pub fn encode(&self) -> Res<String> {
        schema().encode(&self.vals)
    }

    pub fn decode(text: &str) -> Res<Loaded> {
        let (vals, warnings) = schema().decode(text)?;
        Ok(Loaded { settings: Settings { vals }, warnings })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json;

    #[test]
    fn the_shipped_schema_is_consistent() {
        let p: Vec<String> = schema().problems();
        assert!(p.is_empty(), "{p:#?}");
        assert!(schema().settings.len() >= 30);
    }

    #[test]
    fn every_dependency_names_a_real_setting() {
        for s in &schema().settings {
            for d in &s.deps {
                assert!(schema().get(&d.on).is_some(), "{} depends on missing {}", s.id, d.on);
            }
            if let Some(b) = &s.max_by {
                assert!(schema().get(b).is_some(), "{} max_by missing {b}", s.id);
            }
            assert!(schema().groups.iter().any(|g| g.id == s.group));
        }
    }

    #[test]
    fn every_default_is_valid_and_stable_under_normalize() {
        for s in &schema().settings {
            assert!(s.check(&s.default).is_ok(), "{}", s.id);
            if let Some(d) = &s.disabled {
                assert!(s.check(d).is_ok(), "{}", s.id);
            }
        }
        let d: Settings = Settings::default();
        assert!(d.validate().is_ok());
        let mut n: Settings = d.clone();
        n.normalize();
        assert_eq!(n, d);
    }

    #[test]
    fn nums_and_choice_order_are_pinned() {
        let want: [(&str, u16, &[&str]); 4] = [
            ("shuffle_chests", 8, &["off", "overworld", "dungeons", "all"]),
            ("shuffle_smallkeys", 14, &["remove", "vanilla", "dungeon", "regional", "overworld", "any_dungeon", "keysanity"]),
            ("bridge", 24, &["open", "vanilla", "stones", "medallions", "dungeons", "tokens", "hearts", "random"]),
            ("triforce_goal_per_world", 34, &[]),
        ];
        for (id, num, ch) in want {
            let s: &Setting = schema().get(id).unwrap();
            assert_eq!(s.num, num, "{id}");
            let got: Vec<&str> = s.choices.iter().map(|c| c.id.as_str()).collect();
            assert_eq!(got, ch, "{id}");
        }
    }

    #[test]
    fn every_kind_is_used_and_every_tag_is_used() {
        for k in [Kind::Toggle, Kind::Choice, Kind::Number, Kind::Multi, Kind::List] {
            assert!(schema().settings.iter().any(|s| s.kind == k), "{k:?}");
        }
        for t in [Tag::Core, Tag::Common, Tag::Niche] {
            assert!(schema().settings.iter().any(|s| s.tag == t), "{t:?}");
        }
    }

    #[test]
    fn dependencies_switch_settings_off() {
        let mut s: Settings = Settings::default();
        assert!(!s.is_enabled("bridge_stones"));
        assert!(s.is_enabled("bridge_medallions"));
        s.set("bridge", Value::s("stones")).unwrap();
        s.set("bridge_stones", Value::Int(2)).unwrap();
        s.set("bridge_medallions", Value::Int(4)).unwrap();
        s.normalize();
        assert!(s.is_enabled("bridge_stones"));
        assert_eq!(s.int("bridge_stones").unwrap(), 2);
        assert_eq!(s.int("bridge_medallions").unwrap(), 6);
    }

    #[test]
    fn typed_getters_and_errors() {
        let mut s: Settings = Settings::default();
        assert_eq!(s.text("shuffle_chests").unwrap(), "all");
        assert!(!s.flag("triforce_hunt").unwrap());
        assert_eq!(s.int("starting_hearts").unwrap(), 3);
        assert!(s.items("starting_items").unwrap().is_empty());
        assert!(s.flag("shuffle_chests").is_err());
        assert!(s.text("nope").is_err());
        assert!(s.set("shuffle_chests", Value::s("sometimes")).is_err());
        assert!(s.set("starting_hearts", Value::Int(21)).is_err());
        assert!(s.set("nope", Value::Bool(true)).is_err());
    }

    #[test]
    fn cross_field_limit_is_enforced() {
        let mut s: Settings = Settings::default();
        s.set("triforce_hunt", Value::Bool(true)).unwrap();
        s.set("triforce_count_per_world", Value::Int(10)).unwrap();
        assert!(s.clone().finish().is_err());
        s.set("triforce_goal_per_world", Value::Int(10)).unwrap();
        assert!(s.finish().is_ok());
    }

    #[test]
    fn json_round_trip_and_unknown_keys() {
        let mut s: Settings = Settings::default();
        s.set("bridge", Value::s("tokens")).unwrap();
        s.set("starting_songs", Value::set(&["song_of_time", "sarias_song"])).unwrap();
        let s: Settings = s.finish().unwrap();
        let l: Loaded = Settings::from_json(&s.to_json()).unwrap();
        assert_eq!(l.settings, s);
        assert!(l.warnings.is_empty());
        let j: Json = json::parse(r#"{"bridge":"open","removed_in_a_future_version":1}"#).unwrap();
        let l: Loaded = Settings::from_json(&j).unwrap();
        assert_eq!(l.settings.text("bridge").unwrap(), "open");
        assert_eq!(l.warnings.len(), 1);
    }

    #[test]
    fn string_round_trip_for_a_busy_configuration() {
        let mut s: Settings = Settings::default();
        s.set("shuffle_chests", Value::s("dungeons")).unwrap();
        s.set("shuffle_pots", Value::s("all")).unwrap();
        s.set("shuffle_smallkeys", Value::s("keysanity")).unwrap();
        s.set("shuffle_ganon_bosskey", Value::s("hearts")).unwrap();
        s.set("ganon_bosskey_hearts", Value::Int(12)).unwrap();
        s.set("bridge", Value::s("stones")).unwrap();
        s.set("bridge_stones", Value::Int(1)).unwrap();
        s.set("triforce_hunt", Value::Bool(true)).unwrap();
        s.set("triforce_count_per_world", Value::Int(40)).unwrap();
        s.set("triforce_goal_per_world", Value::Int(25)).unwrap();
        s.set("starting_items", Value::set(&["hookshot", "ocarina", "bottle"])).unwrap();
        s.set("starting_hearts", Value::Int(5)).unwrap();
        s.set("disabled_locations", Value::set(&["Skulltula House 10 Tokens", "ZR Magic Bean Salesman"])).unwrap();
        let s: Settings = s.finish().unwrap();
        let t: String = s.encode().unwrap();
        let l: Loaded = Settings::decode(&t).unwrap();
        assert_eq!(l.settings, s);
        assert!(l.warnings.is_empty());
        assert_eq!(l.settings.encode().unwrap(), t);
    }

    #[test]
    fn frozen_settings_string_keeps_decoding() {
        let l: Loaded = Settings::decode("OOTRR1-BEAQGGABAUOACZBEAMBAABWEOQ").unwrap();
        assert!(l.warnings.is_empty());
        let s: Settings = l.settings;
        assert_eq!(s.text("bridge").unwrap(), "tokens");
        assert_eq!(s.int("bridge_tokens").unwrap(), 50);
        assert_eq!(s.text("shuffle_pots").unwrap(), "all");
        assert_eq!(s.items("starting_items").unwrap(), ["ocarina", "hookshot"]);
        assert_eq!(s.text("shuffle_chests").unwrap(), "all");
        assert_eq!(s.encode().unwrap(), "OOTRR1-BEAQGGABAUOACZBEAMBAABWEOQ");
    }

    #[test]
    fn default_string_is_tiny_and_stable() {
        let t: String = Settings::default().encode().unwrap();
        assert_eq!(t, Settings::default().encode().unwrap());
        assert!(t.len() < 16, "{t}");
        assert_eq!(Settings::decode(&t).unwrap().settings, Settings::default());
    }
}
