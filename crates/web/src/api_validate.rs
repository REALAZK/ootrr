use std::fs;
use std::path::{Path as P, PathBuf as PB};

use ootrr_core::json::Json;

use crate::error::Hr;
use crate::pathsafe;

const MARKERS: [(&str, bool); 2] = [("Pack/ZsDic.pack.zs", false), ("Banc", true)];
pub const VERSION: &str = "System/Resource/ResourceSizeTable.Product.111.Nin_Ounce_NVN2.rsizetable.zs";

#[cfg(test)]
pub(crate) fn fake_version(root: &P) {
    let data: ootrr_core::slice::data::Data = ootrr_core::slice::data::Data::load().unwrap();
    for r in ootrr_core::required::list(&data) {
        let t: PB = r.split('/').fold(root.to_path_buf(), |a, s| a.join(s));
        if t.is_file() {
            continue;
        }
        fs::create_dir_all(t.parent().unwrap()).unwrap();
        fs::write(&t, b"").unwrap();
    }
}

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
    if found.is_some() {
        rep.checks.push((VERSION.to_string(), has(&base, VERSION, false)));
    }
    match found {
        Some(f) if !has(&f, VERSION, false) => rep.problems.push(
            "This folder is missing the files of game version 1.1.1 (System/Resource/ResourceSizeTable.Product.111...). The mod build needs the game extracted with the 1.1.1 update installed. Extract it again from the emulator with the update applied and pick that folder.".to_string(),
        ),
        Some(f) => {
            let gone: Vec<String> = match ootrr_core::slice::data::Data::load() {
                Ok(d) => ootrr_core::required::missing(&f, &d),
                Err(_) => Vec::new(),
            };
            if gone.is_empty() {
                rep.ok = true;
                rep.romfs = Some(f);
            } else {
                rep.checks.push((format!("{} files the build reads are missing", gone.len()), false));
                let shown: Vec<&str> = gone.iter().take(6).map(|s| s.as_str()).collect();
                rep.problems.push(format!(
                    "This folder is missing {} files the mod build reads, for example {}. The folder is probably an incomplete or older extraction of the game.",
                    gone.len(),
                    shown.join(", ")
                ));
            }
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
        fake_version(root);
    }

    #[test]
    fn a_folder_without_the_1_1_1_files_is_refused_with_a_reason() {
        let d: PB = tmp("old");
        fake_romfs(&d);
        fs::remove_dir_all(d.join("System")).unwrap();
        let r: Report = inspect(&d);
        assert!(!r.ok);
        assert!(r.romfs.is_none());
        assert!(r.problems.iter().any(|p| p.contains("1.1.1")));
        assert!(r.checks.iter().any(|(l, ok)| l.contains("Product.111") && !*ok));
    }

    #[test]
    fn a_folder_with_a_few_files_missing_is_refused_and_names_them() {
        let d: PB = tmp("partial");
        fake_romfs(&d);
        fs::remove_file(d.join("Event").join("EventFlow").join("Dm_SY_Treasurebox.bfevfl.zs")).unwrap();
        let r: Report = inspect(&d);
        assert!(!r.ok);
        assert!(r.problems.iter().any(|p| p.contains("Dm_SY_Treasurebox") || p.contains("missing 1 files")));
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
