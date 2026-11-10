use std::ffi::OsStr;
use std::path::Path as P;
use std::process::{Command, Stdio};

fn spawn(prog: &str, args: &[&OsStr]) -> bool {
    Command::new(prog)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

pub fn url_ok(u: &str) -> bool {
    u.starts_with("http://") && u.chars().all(|c| c.is_ascii_alphanumeric() || ":/.#=-_[]".contains(c))
}

pub fn browser(url: &str) -> bool {
    if !url_ok(url) {
        return false;
    }
    let u: &OsStr = OsStr::new(url);
    if cfg!(windows) {
        spawn("cmd", &[OsStr::new("/c"), OsStr::new("start"), OsStr::new(""), u])
    } else if cfg!(target_os = "macos") {
        spawn("open", &[u])
    } else {
        spawn("xdg-open", &[u])
    }
}

pub fn folder(p: &P) -> bool {
    let a: &OsStr = p.as_os_str();
    if cfg!(windows) {
        spawn("explorer", &[a])
    } else if cfg!(target_os = "macos") {
        spawn("open", &[a])
    } else {
        spawn("xdg-open", &[a])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_http_urls_are_launched() {
        assert!(url_ok("http://127.0.0.1:7878/#token=abc123"));
        for u in [
            "https://x",
            "file:///c:/x",
            "javascript:alert(1)",
            "http://127.0.0.1:1/ & calc",
            "http://a/\"b",
            "http://a/%00",
            "calc.exe",
            "",
        ] {
            assert!(!url_ok(u), "{u}");
            assert!(!browser(u), "{u}");
        }
    }
}
