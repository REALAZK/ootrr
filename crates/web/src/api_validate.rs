use std::fs;
use std::path::{Path as P, PathBuf as PB};

use ootrr_core::json::Json;

use crate::error::Hr;
use crate::pathsafe;

const MARKERS: [(&str, bool); 2] = [("Pack/ZsDic.pack.zs", false), ("Banc", true)];

pub struct Report {
    pub ok: bool,
    pub romfs: Option<PB>,
    pub checks: Vec<(String, bool)>,
    pub problems: Vec<String>,
}

fn has(base: &P, rel: &str, dir: bool) -> bool {
    let t: PB = rel.split('/').fold(base.to_path_buf(), |a, s| a.join(s));
    match fs::metadata(&t) {
        Ok(m) => {
            if dir {
                m.is_dir()
            } else {
                m.is_file()
            }
        }
        Err(_) => false,
    }
}

fn complete(c: &P) -> bool {
    MARKERS.iter().all(|(r, d)| has(c, r, *d))
}

pub fn inspect(p: &P) -> Report {
    let mut rep: Report = Report { ok: false, romfs: None, checks: Vec::new(), problems: Vec::new() };
    match fs::metadata(p) {
        Ok(m) if m.is_dir() => {}
        Ok(_) => {
            rep.problems.push("That path is a file, not a folder.".to_string());
            return rep;
        }
        Err(_) => {
            rep.problems.push("That folder does not exist or cannot be read.".to_string());
            return rep;
        }
    }
    let found: Option<PB> = [p.to_path_buf(), p.join("romfs")].into_iter().find(|c| complete(c));
    let base: PB = found.clone().unwrap_or_else(|| p.to_path_buf());
    for (r, d) in MARKERS {
        let ok: bool = has(&base, r, d);
        rep.checks.push((format!("{r}{}", if d { "/" } else { "" }), ok));
    }
    match found {
        Some(f) => {
            rep.ok = true;
            rep.romfs = Some(f);
        }
        None => rep
            .problems
            .push("This does not look like an extracted game folder. Pick the folder that contains the Pack and Banc folders.".to_string()),
    }
    rep
}

pub fn run(path: Option<&str>) -> Hr<Json> {
    let raw: &str = path.unwrap_or("");
    let p: PB = pathsafe::abs(raw)?;
    let rep: Report = inspect(&p);
    let checks: Vec<Json> = rep
        .checks
        .iter()
        .map(|(l, ok)| Json::obj().with("label", l.as_str().into()).with("ok", (*ok).into()))
        .collect();
    Ok(Json::obj()
        .with("path", pathsafe::text(&p).into())
        .with("ok", rep.ok.into())
        .with("romfs", rep.romfs.as_deref().map(|x| Json::Str(pathsafe::text(x))).unwrap_or(Json::Null))
        .with("checks", Json::Arr(checks))
        .with("problems", Json::strs(rep.problems.iter().map(|s| s.as_str())))
        .with("version", Json::Null)
        .with(
            "version_note",
            "The game version cannot be read from folder names. It is checked when the mod is built.".into(),
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn tmp(tag: &str) -> PB {
        let p: PB = env::temp_dir().join(format!("ootrr_web_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn fake_romfs(root: &P) {
        fs::create_dir_all(root.join("Pack")).unwrap();
        fs::create_dir_all(root.join("Banc")).unwrap();
        fs::write(root.join("Pack").join("ZsDic.pack.zs"), b"").unwrap();
    }

    #[test]
    fn finds_a_folder_by_names_alone() {
        let d: PB = tmp("direct");
        fake_romfs(&d);
        let r: Report = inspect(&d);
        assert!(r.ok);
        assert_eq!(r.romfs.as_deref(), Some(d.as_path()));
        assert!(r.checks.iter().all(|c| c.1));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn finds_romfs_one_level_down() {
        let d: PB = tmp("nested");
        fake_romfs(&d.join("romfs"));
        let r: Report = inspect(&d);
        assert!(r.ok);
        assert_eq!(r.romfs.as_deref(), Some(d.join("romfs").as_path()));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn empty_and_partial_folders_fail_with_a_plain_message() {
        let d: PB = tmp("empty");
        let r: Report = inspect(&d);
        assert!(!r.ok && r.romfs.is_none() && r.problems.len() == 1);
        assert!(r.checks.iter().all(|c| !c.1));
        fs::create_dir_all(d.join("Banc")).unwrap();
        let r: Report = inspect(&d);
        assert!(!r.ok);
        assert_eq!(r.checks.iter().filter(|c| c.1).count(), 1);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn files_and_missing_paths_fail() {
        let d: PB = tmp("file");
        fs::write(d.join("x.txt"), b"hi").unwrap();
        assert!(!inspect(&d.join("x.txt")).ok);
        assert!(!inspect(&d.join("nope")).ok);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn the_report_never_carries_file_contents_and_names_no_source() {
        let d: PB = tmp("json");
        fake_romfs(&d);
        fs::write(d.join("Pack").join("ZsDic.pack.zs"), b"SECRET-CONTENT").unwrap();
        let j: Json = run(d.to_str()).unwrap();
        let t: String = j.to_string();
        assert!(!t.contains("SECRET-CONTENT"));
        assert!(!t.to_ascii_lowercase().contains("download"));
        assert_eq!(j.get("ok").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(j.get("version"), Some(&Json::Null));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn unsafe_paths_are_refused_before_touching_the_disk() {
        for p in [None, Some(""), Some("relative"), Some("/a/../b"), Some("C:\\a\\..\\b")] {
            assert!(run(p).is_err(), "{p:?}");
        }
    }
}
