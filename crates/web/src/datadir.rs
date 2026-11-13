use std::env;
use std::path::PathBuf as PB;

use crate::pathsafe;

pub const OVERRIDE: &str = "OOTRR_DATA_DIR";

fn var(k: &str) -> Option<PB> {
    env::var(k).ok().and_then(|s| pathsafe::abs(&s).ok())
}

pub fn base() -> Option<PB> {
    if let Some(p) = var(OVERRIDE) {
        return Some(p);
    }
    if cfg!(windows) {
        return var("LOCALAPPDATA").or_else(|| var("APPDATA"));
    }
    if cfg!(target_os = "macos") {
        return var("HOME").map(|h| h.join("Library").join("Application Support"));
    }
    var("XDG_DATA_HOME").or_else(|| var("HOME").map(|h| h.join(".local").join("share")))
}

pub fn cache_root(base: &PB) -> PB {
    base.join("ootrr").join("tracker-cache")
}

pub fn game_dir_name(fp: &str) -> String {
    fp.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fingerprint_becomes_a_safe_folder_name() {
        assert_eq!(game_dir_name("oot-remake:c944ba62294a93fa"), "oot-remake-c944ba62294a93fa");
        assert_eq!(game_dir_name("a/../b"), "a----b");
        let r: PB = cache_root(&PB::from("base"));
        assert!(r.ends_with(PB::from("ootrr").join("tracker-cache")));
    }
}
