use std::path::PathBuf as PB;

use ootrr_core::json::Json;

use crate::body;
use crate::error::{Fail, Hr};
use crate::opener;
use crate::pathsafe;

pub fn check(b: &Json) -> Hr<PB> {
    let p: PB = pathsafe::abs(body::short(b, "path")?)?;
    if !p.is_dir() {
        return Err(Fail::new(422, "That folder does not exist yet."));
    }
    Ok(p)
}

pub fn run(b: &Json) -> Hr<Json> {
    let p: PB = check(b)?;
    let ok: bool = opener::folder(&p);
    Ok(Json::obj().with("opened", ok.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ootrr_core::json;

    #[test]
    fn only_existing_absolute_folders_pass() {
        let d: PB = std::env::temp_dir();
        let t: String = format!("{{\"path\":{}}}", Json::Str(d.display().to_string()));
        assert!(check(&json::parse(&t).unwrap()).is_ok());
        for t in [
            r#"{"path":"relative"}"#,
            r#"{"path":"/a/../b"}"#,
            r#"{"path":"/definitely/not/here/ootrr"}"#,
            r#"{"path":"C:\\definitely\\not\\here\\ootrr"}"#,
            r#"{}"#,
        ] {
            assert!(check(&json::parse(t).unwrap()).is_err(), "{t}");
        }
    }
}
