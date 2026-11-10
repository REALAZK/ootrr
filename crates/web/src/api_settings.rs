use ootrr_core::json::Json;
use ootrr_core::settings::{Loaded, Settings};

use crate::body;
use crate::error::{Fail, Hr};

pub fn reply(l: &Loaded) -> Hr<Json> {
    let t: String = l.settings.encode()?;
    Ok(Json::obj()
        .with("settings_string", t.into())
        .with("settings", l.settings.to_json())
        .with("warnings", Json::strs(l.warnings.iter().map(|w| w.as_str()))))
}

pub fn from_body(b: &Json) -> Hr<Loaded> {
    match (b.get("settings"), body::opt_text(b, "settings_string")?) {
        (Some(_), Some(_)) => Err(Fail::bad("send either settings or settings_string, not both")),
        (Some(o), None) => Ok(Settings::from_json(o)?),
        (None, Some(t)) => {
            if t.len() > body::MAX_STRING {
                return Err(Fail::bad("the settings string is too long"));
            }
            Ok(Settings::decode(t)?)
        }
        (None, None) => Err(Fail::bad("send settings or settings_string")),
    }
}

pub fn convert(b: &Json) -> Hr<Json> {
    reply(&from_body(b)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ootrr_core::json;

    fn j(t: &str) -> Json {
        json::parse(t).unwrap()
    }

    #[test]
    fn object_to_string_and_back() {
        let a: Json = convert(&j(r#"{"settings":{"bridge":"tokens","bridge_tokens":50}}"#)).unwrap();
        let s: String = a.get("settings_string").and_then(|x| x.as_str()).unwrap().to_string();
        assert!(s.starts_with("OOTRR1-"));
        let back: Json = convert(&j(&format!("{{\"settings_string\":\"{s}\"}}"))).unwrap();
        assert_eq!(back.get("settings"), a.get("settings"));
        let st: &Json = back.get("settings").unwrap();
        assert_eq!(st.get("bridge").and_then(|x| x.as_str()), Some("tokens"));
        assert_eq!(st.get("bridge_tokens").and_then(|x| x.as_i64()), Some(50));
    }

    #[test]
    fn disabled_settings_are_normalized_in_the_reply() {
        let a: Json = convert(&j(r#"{"settings":{"bridge":"open","bridge_tokens":7}}"#)).unwrap();
        assert_eq!(a.get("settings").and_then(|x| x.get("bridge_tokens")).and_then(|x| x.as_i64()), Some(100));
    }

    #[test]
    fn unknown_keys_warn_and_bad_values_fail() {
        let a: Json = convert(&j(r#"{"settings":{"nope":1}}"#)).unwrap();
        assert_eq!(a.get("warnings").and_then(|x| x.as_arr()).map(|x| x.len()), Some(1));
        for t in [
            r#"{"settings":{"bridge":"zzz"}}"#,
            r#"{"settings":{"triforce_hunt":true,"triforce_count_per_world":10,"triforce_goal_per_world":20}}"#,
            r#"{"settings_string":"garbage"}"#,
            r#"{"settings_string":"OOTRR9-AAAA"}"#,
            r#"{"settings":[]}"#,
        ] {
            assert_eq!(convert(&j(t)).unwrap_err().status, 422, "{t}");
        }
        for t in [r#"{}"#, r#"{"settings":{},"settings_string":"x"}"#, r#"{"settings_string":5}"#] {
            assert_eq!(convert(&j(t)).unwrap_err().status, 400, "{t}");
        }
    }

    #[test]
    fn a_pasted_string_with_spaces_and_hyphens_still_decodes() {
        let a: Json = convert(&j(r#"{"settings":{"bridge":"tokens","bridge_tokens":50}}"#)).unwrap();
        let s: &str = a.get("settings_string").and_then(|x| x.as_str()).unwrap();
        let spaced: String = s.replace("OOTRR1-", "OOTRR1-\n ").to_ascii_lowercase();
        let t: String = format!("{{\"settings_string\":{}}}", Json::Str(spaced));
        assert!(convert(&j(&t)).is_ok());
    }
}
