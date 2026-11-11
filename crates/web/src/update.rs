use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use ootrr_core::json::{self, Json};

use crate::state::Shared;

pub const API: &str = "https://api.github.com/repos/realazk/ootrr/releases?per_page=1";
pub const PAGE: &str = "https://github.com/realazk/ootrr/";
pub const WAIT: &str = "6";
pub const KEEP: Duration = Duration::from_secs(3600);
pub const RETRY: Duration = Duration::from_secs(600);
pub const NOTE_MAX: usize = 160;
pub const BODY_MAX: usize = 1 << 20;

pub fn parts(v: &str) -> Option<[u32; 3]> {
    let t: &str = v.trim().trim_start_matches(['v', 'V']);
    let mut it = t.split('.');
    let mut out: [u32; 3] = [0; 3];
    for (i, slot) in out.iter_mut().enumerate() {
        match it.next() {
            Some(p) => {
                let d: String = p.chars().take_while(|c| c.is_ascii_digit()).collect();
                *slot = d.parse().ok()?;
            }
            None if i > 0 => break,
            None => return None,
        }
    }
    Some(out)
}

pub fn newer(current: &str, latest: &str) -> bool {
    match (parts(current), parts(latest)) {
        (Some(a), Some(b)) => b > a,
        _ => false,
    }
}

pub fn link_ok(u: &str) -> bool {
    u.starts_with(PAGE) && u.chars().all(|c| c.is_ascii_alphanumeric() || "/:.-_#".contains(c))
}

pub fn note_of(body: &str) -> String {
    let line: &str = body
        .lines()
        .map(|l| l.trim_start_matches(['#', '*', '-', ' ']).trim())
        .find(|l| !l.is_empty())
        .unwrap_or("");
    line.chars().take(NOTE_MAX).collect()
}

fn none(current: &str) -> Json {
    Json::obj().with("checked", false.into()).with("newer", false.into()).with("current", current.into())
}

pub fn newest(list: &Json) -> Option<&Json> {
    match list {
        Json::Arr(v) => v.iter().find(|r| r.get("draft").and_then(|x| x.as_bool()) != Some(true)),
        other => Some(other),
    }
}

pub fn reply(release: &Json, current: &str) -> Json {
    let tag: &str = release.get("tag_name").and_then(|x| x.as_str()).unwrap_or("");
    let url: &str = release.get("html_url").and_then(|x| x.as_str()).unwrap_or("");
    let body: &str = release.get("body").and_then(|x| x.as_str()).unwrap_or("");
    if parts(tag).is_none() || !link_ok(url) {
        return none(current);
    }
    Json::obj()
        .with("checked", true.into())
        .with("newer", newer(current, tag).into())
        .with("current", current.into())
        .with("latest", tag.trim_start_matches(['v', 'V']).into())
        .with("url", url.into())
        .with("note", note_of(body).as_str().into())
}

fn curl() -> std::path::PathBuf {
    if cfg!(windows)
        && let Some(root) = std::env::var_os("SystemRoot")
    {
        let p: std::path::PathBuf = std::path::Path::new(&root).join("System32").join("curl.exe");
        if p.is_file() {
            return p;
        }
    }
    std::path::PathBuf::from("curl")
}

fn fetch() -> Option<Json> {
    let out = Command::new(curl())
        .args([
            "-fsSL",
            "-m",
            WAIT,
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "User-Agent: ootrr-update-check",
            API,
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() || out.stdout.len() > BODY_MAX {
        return None;
    }
    json::parse(std::str::from_utf8(&out.stdout).ok()?).ok()
}

pub fn check(sh: &Shared) -> Json {
    let current: &str = env!("CARGO_PKG_VERSION");
    if !sh.updates {
        return none(current).with("disabled", true.into());
    }
    let mut c = match sh.cache.lock() {
        Ok(c) => c,
        Err(p) => p.into_inner(),
    };
    if let Some((at, ttl, j)) = c.as_ref()
        && at.elapsed() < *ttl
    {
        return j.clone();
    }
    let (j, ttl): (Json, Duration) = match fetch() {
        Some(r) => (newest(&r).map(|x| reply(x, current)).unwrap_or_else(|| none(current)), KEEP),
        None => (none(current), RETRY),
    };
    *c = Some((Instant::now(), ttl, j.clone()));
    j
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel(tag: &str, url: &str, body: &str) -> Json {
        Json::obj().with("tag_name", tag.into()).with("html_url", url.into()).with("body", body.into())
    }

    #[test]
    fn versions_compare_as_numbers_not_text() {
        assert!(newer("0.1.0", "v0.1.1"));
        assert!(newer("0.9.0", "0.10.0"));
        assert!(newer("0.1.0", "1"));
        assert!(!newer("0.1.0", "v0.1.0"));
        assert!(!newer("0.2.0", "0.1.9"));
        assert!(!newer("0.1.0", "nightly"));
        assert!(newer("0.1.0", "0.2.0-beta"));
    }

    #[test]
    fn only_release_pages_of_this_project_become_links() {
        assert!(link_ok("https://github.com/realazk/ootrr/releases/tag/v0.2.0"));
        for u in [
            "http://github.com/realazk/ootrr/",
            "https://github.com/other/ootrr/",
            "https://github.com/realazk/ootrr.evil.com/",
            "javascript:alert(1)",
            "https://github.com/realazk/ootrr/ x",
        ] {
            assert!(!link_ok(u), "{u}");
        }
    }

    #[test]
    fn the_note_is_the_first_real_line_and_is_short() {
        assert_eq!(note_of("\n## Bombs work\nmore"), "Bombs work");
        assert_eq!(note_of("- Fixes presets"), "Fixes presets");
        assert_eq!(note_of(""), "");
        assert_eq!(note_of(&"x".repeat(500)).chars().count(), NOTE_MAX);
    }

    #[test]
    fn the_newest_release_is_read_from_a_list_and_drafts_are_skipped() {
        let a: Json = rel("v9.0.0", "https://github.com/realazk/ootrr/releases/tag/v9.0.0", "x").with("draft", true.into());
        let b: Json = rel("v8.0.0", "https://github.com/realazk/ootrr/releases/tag/v8.0.0", "x");
        let list: Json = Json::Arr(vec![a, b]);
        assert_eq!(newest(&list).and_then(|r| r.get("tag_name")).and_then(|x| x.as_str()), Some("v8.0.0"));
        assert!(newest(&Json::Arr(Vec::new())).is_none());
    }

    #[test]
    fn a_newer_release_is_reported_and_a_bad_one_is_ignored() {
        let ok: Json = reply(&rel("v9.0.0", "https://github.com/realazk/ootrr/releases/tag/v9.0.0", "Big one"), "0.1.0");
        assert_eq!(ok.get("newer").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(ok.get("latest").and_then(|x| x.as_str()), Some("9.0.0"));
        assert_eq!(ok.get("note").and_then(|x| x.as_str()), Some("Big one"));
        let same: Json = reply(&rel("v0.1.0", "https://github.com/realazk/ootrr/releases/tag/v0.1.0", ""), "0.1.0");
        assert_eq!(same.get("newer").and_then(|x| x.as_bool()), Some(false));
        let bad: Json = reply(&rel("v9.0.0", "https://evil.example/x", ""), "0.1.0");
        assert_eq!(bad.get("newer").and_then(|x| x.as_bool()), Some(false));
        assert_eq!(bad.get("checked").and_then(|x| x.as_bool()), Some(false));
        assert!(bad.get("url").is_none());
    }
}
