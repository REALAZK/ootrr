use ootrr_core::TITLE_ID;
use ootrr_core::json::Json;
use ootrr_core::settings::export::schema_json;
use ootrr_core::settings::presets;
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

pub fn schema_reply() -> Hr<Json> {
    Ok(schema_json(schema(), &presets::builtin()?))
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
}
