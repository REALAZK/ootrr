use std::cmp::Ordering;
use std::env;
use std::fs;
use std::path::{Path as P, PathBuf as PB};

use ootrr_core::error::{Error, Res};
use ootrr_core::json::Json;

use crate::error::Hr;
use crate::pathsafe;

pub const MAX_ENTRIES: usize = 5000;

struct Entry {
    name: String,
    dir: bool,
}

fn home() -> Option<String> {
    let k: &str = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    env::var(k).ok().filter(|s| pathsafe::abs(s).is_ok())
}

fn roots() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    if let Some(h) = home() {
        out.push(("Home".to_string(), h));
    }
    if cfg!(windows) {
        for l in b'A'..=b'Z' {
            let d: String = format!("{}:\\", l as char);
            if fs::metadata(&d).is_ok() {
                out.push((format!("{}:", l as char), d));
            }
        }
    } else {
        out.push(("/".to_string(), "/".to_string()));
    }
    out
}

fn order(a: &Entry, b: &Entry) -> Ordering {
    (!a.dir, a.name.to_lowercase(), &a.name).cmp(&(!b.dir, b.name.to_lowercase(), &b.name))
}

fn read(p: &P) -> Res<(Vec<Entry>, bool)> {
    let rd: fs::ReadDir = fs::read_dir(p).map_err(|e| {
        Error::Msg(
            if p.is_file() {
                "That path is a file, not a folder."
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                "That folder cannot be opened, access is denied."
            } else {
                "That folder does not exist or cannot be read."
            }
            .to_string(),
        )
    })?;
    let mut out: Vec<Entry> = Vec::new();
    let mut cut: bool = false;
    for e in rd {
        let e: fs::DirEntry = match e {
            Ok(x) => x,
            Err(_) => continue,
        };
        let name: String = e.file_name().to_string_lossy().into_owned();
        if name.contains('\u{fffd}') || name.chars().any(|c| c.is_control()) {
            continue;
        }
        if out.len() >= MAX_ENTRIES {
            cut = true;
            break;
        }
        let dir: bool = match e.file_type() {
            Ok(t) if t.is_dir() => true,
            Ok(t) if t.is_symlink() => fs::metadata(e.path()).map(|m| m.is_dir()).unwrap_or(false),
            _ => false,
        };
        out.push(Entry { name, dir });
    }
    out.sort_by(order);
    Ok((out, cut))
}

pub fn run(path: Option<&str>) -> Hr<Json> {
    let raw: &str = path.unwrap_or("");
    if raw.is_empty() {
        let rs: Vec<Json> = roots()
            .iter()
            .map(|(l, p)| Json::obj().with("label", l.as_str().into()).with("path", p.as_str().into()))
            .collect();
        return Ok(Json::obj()
            .with("path", "".into())
            .with("parent", Json::Null)
            .with("roots", Json::Arr(rs))
            .with("entries", Json::Arr(Vec::new()))
            .with("truncated", false.into()));
    }
    let p: PB = pathsafe::abs(raw)?;
    let (es, cut): (Vec<Entry>, bool) = read(&p)?;
    let ents: Vec<Json> = es
        .iter()
        .map(|e| Json::obj().with("name", e.name.as_str().into()).with("kind", (if e.dir { "dir" } else { "file" }).into()))
        .collect();
    let parent: String = p.parent().map(pathsafe::text).unwrap_or_default();
    Ok(Json::obj()
        .with("path", pathsafe::text(&p).into())
        .with("parent", parent.into())
        .with("roots", Json::Arr(Vec::new()))
        .with("entries", Json::Arr(ents))
        .with("truncated", cut.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PB {
        let p: PB = env::temp_dir().join(format!("ootrr_web_fs_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn lists_names_and_kinds_only_with_folders_first() {
        let d: PB = tmp("list");
        fs::create_dir_all(d.join("beta")).unwrap();
        fs::create_dir_all(d.join("Alpha")).unwrap();
        fs::write(d.join("a.txt"), b"SECRET").unwrap();
        let j: Json = run(d.to_str()).unwrap();
        let t: String = j.to_string();
        assert!(!t.contains("SECRET"));
        let es: &[Json] = j.get("entries").and_then(|x| x.as_arr()).unwrap();
        let names: Vec<&str> = es.iter().filter_map(|e| e.get("name").and_then(|x| x.as_str())).collect();
        assert_eq!(names, vec!["Alpha", "beta", "a.txt"]);
        assert_eq!(es[0].get("kind").and_then(|x| x.as_str()), Some("dir"));
        assert_eq!(es[2].get("kind").and_then(|x| x.as_str()), Some("file"));
        assert_eq!(j.get("parent").and_then(|x| x.as_str()), d.parent().and_then(|x| x.to_str()));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn traversal_and_odd_paths_are_refused() {
        let d: PB = tmp("trav");
        let up: String = format!("{}{}..{}x", d.display(), std::path::MAIN_SEPARATOR, std::path::MAIN_SEPARATOR);
        for p in [up.as_str(), "relative", "..", "../..", "x\0y"] {
            assert_eq!(run(Some(p)).unwrap_err().status, 422, "{p:?}");
        }
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_file_or_missing_folder_is_an_error() {
        let d: PB = tmp("bad");
        fs::write(d.join("f"), b"x").unwrap();
        assert!(run(d.join("f").to_str()).is_err());
        assert!(run(d.join("missing").to_str()).is_err());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn no_path_lists_the_roots() {
        let j: Json = run(None).unwrap();
        let rs: &[Json] = j.get("roots").and_then(|x| x.as_arr()).unwrap();
        assert!(!rs.is_empty());
        assert_eq!(j.get("parent"), Some(&Json::Null));
        assert!(run(Some("")).is_ok());
    }

    #[test]
    fn huge_folders_are_cut() {
        let d: PB = tmp("big");
        for i in 0..(MAX_ENTRIES + 5) {
            fs::write(d.join(format!("f{i}")), b"").unwrap();
        }
        let j: Json = run(d.to_str()).unwrap();
        assert_eq!(j.get("truncated").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(j.get("entries").and_then(|x| x.as_arr()).map(|x| x.len()), Some(MAX_ENTRIES));
        let _ = fs::remove_dir_all(&d);
    }
}
