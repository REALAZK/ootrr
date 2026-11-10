use std::fs;
use std::path::Path as P;

use super::check::id_ok;
use super::model::{Setting, Value};
use super::{Settings, schema};
use crate::error::{Error, Res};
use crate::json::{self, Json};

const BUILTIN: [(&str, &str); 5] = [
    ("default", include_str!("../../../../data/presets/default.json")),
    ("easy", include_str!("../../../../data/presets/easy.json")),
    ("keysanity", include_str!("../../../../data/presets/keysanity.json")),
    ("full_shuffle", include_str!("../../../../data/presets/full_shuffle.json")),
    ("open_world", include_str!("../../../../data/presets/open_world.json")),
];

#[derive(Debug, Clone, PartialEq)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub tip: String,
    pub order: i64,
    pub values: Vec<(String, Value)>,
}

fn text_of(j: &Json, k: &str) -> Res<String> {
    match j.get(k) {
        Some(Json::Str(s)) if !s.is_empty() => Ok(s.clone()),
        _ => Err(Error::Msg(format!("preset needs a non-empty string field {k}"))),
    }
}

impl Preset {
    pub fn from_json(j: &Json) -> Res<Preset> {
        let id: String = text_of(j, "id")?;
        if !id_ok(&id) {
            return Err(Error::Msg(format!("preset id {id:?} is malformed")));
        }
        let name: String = text_of(j, "name")?;
        let tip: String = text_of(j, "tip")?;
        let order: i64 = match j.get("order") {
            None => 100,
            Some(x) => x.as_i64().ok_or_else(|| Error::Msg(format!("preset {id}: order must be an integer")))?,
        };
        let m: &[(String, Json)] = j
            .get("settings")
            .and_then(|x| x.as_obj())
            .ok_or_else(|| Error::Msg(format!("preset {id}: settings must be an object")))?;
        let mut values: Vec<(String, Value)> = Vec::with_capacity(m.len());
        for (k, x) in m {
            let s: &Setting = schema()
                .get(k)
                .ok_or_else(|| Error::Msg(format!("preset {id}: unknown setting {k}")))?;
            let v: Value = Value::from_json(x).map_err(|e| Error::Msg(format!("preset {id}: {k}: {e}")))?;
            s.check(&v).map_err(|e| Error::Msg(format!("preset {id}: {e}")))?;
            values.push((k.clone(), v));
        }
        let p: Preset = Preset { id, name, tip, order, values };
        let done: Settings = p.apply(&Settings::default())?;
        for (k, v) in &p.values {
            if done.get(k) != Some(v) {
                return Err(Error::Msg(format!(
                    "preset {}: {k} is switched off by its controlling setting, so its value would be ignored",
                    p.id
                )));
            }
        }
        Ok(p)
    }

    pub fn to_json(&self) -> Json {
        let m: Vec<(String, Json)> = self.values.iter().map(|(k, v)| (k.clone(), v.to_json())).collect();
        Json::obj()
            .with("id", self.id.as_str().into())
            .with("name", self.name.as_str().into())
            .with("tip", self.tip.as_str().into())
            .with("order", self.order.into())
            .with("settings", Json::Obj(m))
    }

    pub fn apply(&self, base: &Settings) -> Res<Settings> {
        let mut s: Settings = base.clone();
        for (k, v) in &self.values {
            s.set(k, v.clone())?;
        }
        s.finish()
    }
}

pub fn parse(text: &str) -> Res<Preset> {
    Preset::from_json(&json::parse(text)?)
}

fn sort(v: &mut [Preset]) -> Res<()> {
    v.sort_by(|a, b| (a.order, &a.id).cmp(&(b.order, &b.id)));
    for w in v.windows(2) {
        if w[0].id == w[1].id {
            return Err(Error::Msg(format!("preset id {} is used twice", w[0].id)));
        }
    }
    Ok(())
}

pub fn builtin() -> Res<Vec<Preset>> {
    let mut out: Vec<Preset> = Vec::new();
    for (stem, t) in BUILTIN {
        let p: Preset = parse(t)?;
        if p.id != stem {
            return Err(Error::Msg(format!("built-in preset {stem} declares the id {}", p.id)));
        }
        out.push(p);
    }
    sort(&mut out)?;
    Ok(out)
}

pub fn load_dir(d: &P) -> Res<Vec<Preset>> {
    let mut names: Vec<String> = Vec::new();
    for e in fs::read_dir(d)? {
        let n: String = e?.file_name().to_string_lossy().into_owned();
        if n.ends_with(".json") {
            names.push(n);
        }
    }
    names.sort();
    let mut out: Vec<Preset> = Vec::new();
    for n in names {
        let t: String = fs::read_to_string(d.join(&n))?;
        let p: Preset = parse(&t).map_err(|e| Error::Msg(format!("{n}: {e}")))?;
        if format!("{}.json", p.id) != n {
            return Err(Error::Msg(format!("{n}: the file name must match the preset id {}", p.id)));
        }
        out.push(p);
    }
    sort(&mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root_dir() -> std::path::PathBuf {
        P::new(env!("CARGO_MANIFEST_DIR")).join("../../data/presets")
    }

    #[test]
    fn builtin_presets_load_and_apply_cleanly() {
        let v: Vec<Preset> = builtin().unwrap();
        assert!(v.len() >= 5);
        assert_eq!(v[0].id, "default");
        for p in &v {
            let s: Settings = p.apply(&Settings::default()).unwrap();
            let t: String = s.encode().unwrap();
            assert_eq!(Settings::decode(&t).unwrap().settings, s, "{}", p.id);
        }
        assert_eq!(v[0].apply(&Settings::default()).unwrap(), Settings::default());
    }

    #[test]
    fn presets_actually_differ_from_the_default() {
        let v: Vec<Preset> = builtin().unwrap();
        let d: String = Settings::default().encode().unwrap();
        let mut seen: Vec<String> = vec![d];
        for p in v.iter().skip(1) {
            let t: String = p.apply(&Settings::default()).unwrap().encode().unwrap();
            assert!(!seen.contains(&t), "{} duplicates another preset", p.id);
            seen.push(t);
        }
    }

    #[test]
    fn every_preset_file_on_disk_is_embedded() {
        let mut disk: Vec<String> = load_dir(&root_dir()).unwrap().into_iter().map(|p| p.id).collect();
        let mut emb: Vec<String> = builtin().unwrap().into_iter().map(|p| p.id).collect();
        disk.sort();
        emb.sort();
        assert_eq!(disk, emb);
        assert_eq!(load_dir(&root_dir()).unwrap(), builtin().unwrap());
    }

    #[test]
    fn json_round_trip() {
        for p in builtin().unwrap() {
            assert_eq!(Preset::from_json(&p.to_json()).unwrap(), p);
        }
    }

    #[test]
    fn bad_presets_are_rejected() {
        let head: &str = r#""id":"x","name":"X","tip":"t""#;
        let cases: Vec<String> = vec![
            format!(r#"{{{head},"settings":{{"nope":1}}}}"#),
            format!(r#"{{{head},"settings":{{"bridge":"sometimes"}}}}"#),
            format!(r#"{{{head},"settings":{{"bridge_stones":2}}}}"#),
            format!(r#"{{{head},"settings":{{"starting_hearts":99}}}}"#),
            format!(r#"{{{head},"settings":{{"triforce_hunt":true,"triforce_count_per_world":5}}}}"#),
            format!(r#"{{{head},"settings":[]}}"#),
            format!(r#"{{{head}}}"#),
            r#"{"id":"X","name":"X","tip":"t","settings":{}}"#.to_string(),
            r#"{"id":"x","name":"","tip":"t","settings":{}}"#.to_string(),
            r#"{"id":"x","name":"X","settings":{}}"#.to_string(),
            format!(r#"{{{head},"order":"first","settings":{{}}}}"#),
            "[]".to_string(),
        ];
        for t in cases {
            assert!(parse(&t).is_err(), "accepted {t}");
        }
        assert!(parse(&format!(r#"{{{head},"settings":{{"bridge":"stones","bridge_stones":2}}}}"#)).is_ok());
    }

    #[test]
    fn missing_directory_is_an_error() {
        assert!(load_dir(&root_dir().join("does_not_exist")).is_err());
    }
}
