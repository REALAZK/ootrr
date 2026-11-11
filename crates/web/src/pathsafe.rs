use std::path::{Component, Path as P, PathBuf as PB, Prefix};

use ootrr_core::error::{Error, Res};

pub const MAX_PATH_BYTES: usize = 1024;

const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2",
    "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

fn bad(m: &str) -> Error {
    Error::Msg(m.to_string())
}

fn seg_ok(n: &str) -> bool {
    if n.is_empty() || n.len() > 255 || n.chars().any(|c| c.is_control()) {
        return false;
    }
    if cfg!(windows) {
        if n.chars().any(|c| matches!(c, ':' | '<' | '>' | '"' | '|' | '?' | '*')) {
            return false;
        }
        if n.ends_with('.') || n.ends_with(' ') {
            return false;
        }
        let stem: String = n.split('.').next().unwrap_or("").trim_end().to_ascii_uppercase();
        if RESERVED.contains(&stem.as_str()) {
            return false;
        }
    }
    true
}

pub fn abs(s: &str) -> Res<PB> {
    let s: &str = s.strip_prefix('"').and_then(|x| x.strip_suffix('"')).unwrap_or(s);
    if s.is_empty() || s.len() > MAX_PATH_BYTES {
        return Err(bad("the path is empty or too long"));
    }
    if s.chars().any(|c| c.is_control()) {
        return Err(bad("the path has control characters"));
    }
    let win: bool = cfg!(windows);
    if s.split(|c: char| c == '/' || (win && c == '\\')).any(|x| x == ".." || x == ".") {
        return Err(bad("the path may not contain . or .. parts"));
    }
    let p: &P = P::new(s);
    if !p.is_absolute() {
        return Err(bad("the path must be absolute"));
    }
    let mut out: PB = PB::new();
    for c in p.components() {
        match c {
            Component::Prefix(x) => match x.kind() {
                Prefix::Disk(_) => out.push(c.as_os_str()),
                _ => return Err(bad("network and device paths are not allowed")),
            },
            Component::RootDir => out.push(c.as_os_str()),
            Component::Normal(n) => {
                let t: &str = n.to_str().ok_or_else(|| bad("the path is not valid text"))?;
                if !seg_ok(t) {
                    return Err(bad(&format!("the path part {t:?} is not allowed")));
                }
                out.push(n);
            }
            _ => return Err(bad("the path may not contain . or .. parts")),
        }
    }
    Ok(out)
}

pub fn mod_name_ok(s: &str) -> bool {
    let ok_chars: bool = s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-' | '!' | '.'));
    !s.is_empty() && s.len() <= 64 && ok_chars && !s.contains("..") && s.trim() == s
}

pub fn text(p: &P) -> String {
    p.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_errors_everywhere() {
        for s in ["", "relative", "a/b", "./x", "../x", "x\0y", "x\ny"] {
            assert!(abs(s).is_err(), "{s:?}");
        }
        assert!(abs(&"a".repeat(2000)).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn unix_paths() {
        assert_eq!(abs("/home/x//y/").unwrap(), PB::from("/home/x/y"));
        assert!(abs("/").is_ok());
        for s in ["/a/../b", "/a/./b", "/a/..", "/.."] {
            assert!(abs(s).is_err(), "{s}");
        }
        assert!(abs("/a\\..\\b").is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn one_pair_of_surrounding_quotes_is_dropped() {
        let d: String = std::env::temp_dir().display().to_string();
        assert_eq!(abs(&format!("\"{d}\"")).unwrap(), abs(&d).unwrap());
        assert!(abs(&format!("\"{d}")).is_err());
    }

    #[test]
    fn windows_paths() {
        assert_eq!(abs("C:\\Users\\x\\\\y").unwrap(), PB::from("C:\\Users\\x\\y"));
        assert_eq!(abs("C:/Users/x").unwrap(), PB::from("C:\\Users\\x"));
        assert!(abs("C:\\").is_ok());
        for s in [
            "C:\\Users\\..\\x",
            "C:\\a\\..",
            "C:/a/../b",
            "C:\\a\\.\\b",
            "\\\\?\\C:\\x",
            "\\\\.\\C:\\x",
            "\\\\server\\share\\x",
            "\\\\?\\UNC\\server\\share",
            "C:foo",
            "\\foo",
            "C:\\CON",
            "C:\\a\\nul.txt",
            "C:\\a\\com1",
            "C:\\a:b",
            "C:\\a\\b:stream",
            "C:\\a\\b?",
            "C:\\a\\b.",
            "C:\\a\\b ",
            "C:\\a\\<b>",
        ] {
            assert!(abs(s).is_err(), "{s}");
        }
    }

    #[test]
    fn mod_names() {
        for s in ["Test Mod", "my-mod_1", "Seed!", "a.b"] {
            assert!(mod_name_ok(s), "{s}");
        }
        for s in ["", "..", "a/b", "a\\b", "a:b", " x", "x ", "a..b", "<x>"] {
            assert!(!mod_name_ok(s), "{s}");
        }
        assert!(!mod_name_ok(&"a".repeat(65)));
    }
}
