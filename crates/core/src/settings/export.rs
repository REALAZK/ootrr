use super::model::{Cond, Dep, Group, Kind, Schema, Setting, Value};
use super::presets::Preset;
use super::string::{PREFIX, STRING_VERSION};
use crate::json::Json;

fn group_json(g: &Group) -> Json {
    Json::obj()
        .with("id", g.id.as_str().into())
        .with("label", g.label.as_str().into())
        .with("tip", g.tip.as_str().into())
}

fn dep_json(d: &Dep) -> Json {
    let o: Json = Json::obj().with("on", d.on.as_str().into());
    match &d.cond {
        Cond::Is(v) => o.with("is", v.to_json()),
        Cond::IsNot(v) => o.with("not", v.to_json()),
        Cond::In(vs) => o.with("in", Json::Arr(vs.iter().map(Value::to_json).collect())),
    }
}

fn setting_json(s: &Setting) -> Json {
    let mut o: Json = Json::obj()
        .with("id", s.id.as_str().into())
        .with("num", Json::Int(s.num as i64))
        .with("label", s.label.as_str().into())
        .with("group", s.group.as_str().into())
        .with("kind", s.kind.name().into())
        .with("tag", s.tag.name().into())
        .with("tip", s.tip.as_str().into())
        .with("default", s.default.to_json());
    if matches!(s.kind, Kind::Choice | Kind::Multi) {
        let cs: Vec<Json> = s
            .choices
            .iter()
            .map(|c| {
                let mut x: Json = Json::obj().with("id", c.id.as_str().into()).with("label", c.label.as_str().into());
                if !c.tip.is_empty() {
                    x.put("tip", c.tip.as_str().into());
                }
                x
            })
            .collect();
        o.put("choices", Json::Arr(cs));
    }
    if s.kind == Kind::Number {
        o.put("min", s.min.into());
        o.put("max", s.max.into());
        if let Some(b) = &s.max_by {
            o.put("max_by", b.as_str().into());
        }
    }
    if s.kind == Kind::List {
        o.put("max_items", Json::Int(s.max_items as i64));
    }
    o.put("deps", Json::Arr(s.deps.iter().map(dep_json).collect()));
    if let Some(d) = &s.disabled {
        o.put("disabled", d.to_json());
    }
    o
}

pub fn schema_json(sch: &Schema, presets: &[Preset]) -> Json {
    Json::obj()
        .with("schema_version", Json::Int(sch.version as i64))
        .with("string_prefix", format!("{PREFIX}{STRING_VERSION}-").into())
        .with("groups", Json::Arr(sch.groups.iter().map(group_json).collect()))
        .with("settings", Json::Arr(sch.settings.iter().map(setting_json).collect()))
        .with("presets", Json::Arr(presets.iter().map(Preset::to_json).collect()))
}

#[cfg(test)]
mod tests {
    use super::super::presets::builtin;
    use super::super::schema;
    use super::*;
    use crate::json;

    #[test]
    fn dump_is_valid_json_with_every_setting() {
        let j: Json = schema_json(schema(), &builtin().unwrap());
        let back: Json = json::parse(&j.pretty()).unwrap();
        assert_eq!(back, j);
        let n: usize = back.get("settings").and_then(|x| x.as_arr()).map(|a| a.len()).unwrap();
        assert_eq!(n, schema().settings.len());
        let p: usize = back.get("presets").and_then(|x| x.as_arr()).map(|a| a.len()).unwrap();
        assert_eq!(p, 5);
        assert_eq!(back.get("string_prefix").and_then(|x| x.as_str()), Some("OOTRR1-"));
    }

    #[test]
    fn dump_is_deterministic() {
        let a: String = schema_json(schema(), &builtin().unwrap()).to_string();
        let b: String = schema_json(schema(), &builtin().unwrap()).to_string();
        assert_eq!(a, b);
    }

    #[test]
    fn every_entry_has_the_fields_the_ui_needs() {
        let j: Json = schema_json(schema(), &[]);
        let groups: Vec<&str> = j
            .get("groups")
            .and_then(|x| x.as_arr())
            .unwrap()
            .iter()
            .filter_map(|g| g.get("id").and_then(|x| x.as_str()))
            .collect();
        for s in j.get("settings").and_then(|x| x.as_arr()).unwrap() {
            for k in ["id", "num", "label", "group", "kind", "tag", "tip", "default", "deps"] {
                assert!(s.get(k).is_some(), "missing {k} in {s}");
            }
            let g: &str = s.get("group").and_then(|x| x.as_str()).unwrap();
            assert!(groups.contains(&g));
            match s.get("kind").and_then(|x| x.as_str()).unwrap() {
                "choice" | "multi" => assert!(s.get("choices").is_some()),
                "number" => assert!(s.get("min").is_some() && s.get("max").is_some()),
                "list" => assert!(s.get("max_items").is_some()),
                _ => {}
            }
        }
    }

    #[test]
    fn dependency_shapes() {
        let j: Json = schema_json(schema(), &[]);
        let all: &[Json] = j.get("settings").and_then(|x| x.as_arr()).unwrap();
        let st: &Json = all.iter().find(|s| s.get("id").and_then(|x| x.as_str()) == Some("bridge_stones")).unwrap();
        assert_eq!(st.get("deps").unwrap().to_string(), r#"[{"on":"bridge","is":"stones"}]"#);
        let tr: &Json = all.iter().find(|s| s.get("id").and_then(|x| x.as_str()) == Some("trials")).unwrap();
        assert_eq!(tr.get("deps").unwrap().to_string(), r#"[{"on":"trials_random","is":false}]"#);
    }
}
