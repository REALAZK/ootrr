use ootrr_core::TITLE_ID;
use ootrr_core::json::Json;
use ootrr_core::settings::export::schema_json;
use ootrr_core::settings::presets::{self, Preset};
use ootrr_core::settings::Settings;
use ootrr_core::slice::support::plan;
use ootrr_core::settings::schema;

use crate::error::Hr;
use crate::state::Shared;

pub fn info(sh: &Shared) -> Json {
    let os: &str = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    Json::obj()
        .with("name", "ootrr".into())
        .with("version", env!("CARGO_PKG_VERSION").into())
        .with("os", os.into())
        .with("sep", std::path::MAIN_SEPARATOR.to_string().into())
        .with("lan", sh.guard.lan.into())
        .with("title_id", TITLE_ID.into())
}

fn gaps(p: &Preset) -> Vec<String> {
    let s: Settings = match p.apply(&Settings::default()) {
        Ok(s) => s,
        Err(e) => return vec![e.to_string()],
    };
    match plan(&s) {
        Ok(_) => Vec::new(),
        Err(e) => e.to_string().lines().skip(1).map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect(),
    }
}

fn with_support(p: &Preset) -> Json {
    let g: Vec<String> = gaps(p);
    p.to_json()
        .with("supported", g.is_empty().into())
        .with("unsupported", Json::Arr(g.iter().map(|x| x.as_str().into()).collect()))
}

pub fn schema_reply() -> Hr<Json> {
    let ps: Vec<Preset> = presets::builtin()?;
    let mut j: Json = schema_json(schema(), &ps);
    j.put("presets", Json::Arr(ps.iter().map(with_support).collect()));
    Ok(j)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guard::Guard;
    use crate::jobs::Jobs;

    #[test]
    fn info_has_the_basics_and_no_token() {
        let sh: Shared = Shared { guard: Guard { port: 1, token: "tok123".into(), lan: false }, jobs: Jobs::new() };
        let j: Json = info(&sh);
        assert_eq!(j.get("lan").and_then(|x| x.as_bool()), Some(false));
        assert!(j.get("version").is_some());
        assert!(!j.to_string().contains("tok123"));
    }

    #[test]
    fn schema_reply_matches_the_core_dump() {
        let j: Json = schema_reply().unwrap();
        assert_eq!(j.get("string_prefix").and_then(|x| x.as_str()), Some("OOTRR1-"));
        assert!(j.get("settings").and_then(|x| x.as_arr()).map(|a| a.len()).unwrap_or(0) >= 30);
        assert!(j.get("groups").is_some() && j.get("presets").is_some());
    }

    #[test]
    fn every_preset_says_whether_the_build_supports_it() {
        let j: Json = schema_reply().unwrap();
        let ps: &[Json] = j.get("presets").and_then(|x| x.as_arr()).unwrap();
        for p in ps {
            let ok: bool = p.get("supported").and_then(|x| x.as_bool()).unwrap();
            let n: usize = p.get("unsupported").and_then(|x| x.as_arr()).map(|a| a.len()).unwrap();
            assert_eq!(ok, n == 0);
        }
        let d: &Json = ps.iter().find(|p| p.get("id").and_then(|x| x.as_str()) == Some("default")).unwrap();
        assert_eq!(d.get("supported").and_then(|x| x.as_bool()), Some(true));
    }
}
