use std::fs;
use std::path::{Component, Path as P, PathBuf as PB};

use crate::error::{Error, Res};

pub struct Pack {
    pub root: PB,
}

fn is_title(s: &str) -> bool {
    s.len() == 16 && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_name(s: &str) -> bool {
    let ok_chars: bool = s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-' | '!' | '.'));
    !s.is_empty() && s.len() <= 64 && ok_chars && !s.contains("..") && s.trim() == s
}

pub(crate) fn rel(r: &str) -> Res<PB> {
    let mut out: PB = PB::new();
    for c in P::new(r).components() {
        match c {
            Component::Normal(n) if !n.to_string_lossy().contains(':') => out.push(n),
            _ => return Err(Error::BadPath(r.to_string())),
        }
    }
    if out.as_os_str().is_empty() {
        return Err(Error::BadPath(r.to_string()));
    }
    Ok(out)
}

impl Pack {
    pub fn create(load: &P, title: &str, name: &str) -> Res<Pack> {
        if !is_title(title) {
            return Err(Error::BadPath(title.to_string()));
        }
        if !is_name(name) {
            return Err(Error::BadPath(name.to_string()));
        }
        let root: PB = load.join(title.to_ascii_uppercase()).join(name);
        fs::create_dir_all(root.join("romfs"))?;
        fs::create_dir_all(root.join("exefs"))?;
        Ok(Pack { root })
    }

    pub fn clean(load: &P, title: &str, name: &str) -> Res<()> {
        if !is_title(title) {
            return Err(Error::BadPath(title.to_string()));
        }
        if !is_name(name) {
            return Err(Error::BadPath(name.to_string()));
        }
        let root: PB = load.join(title.to_ascii_uppercase()).join(name);
        for d in ["romfs", "exefs"] {
            let p: PB = root.join(d);
            if p.is_dir() {
                fs::remove_dir_all(&p)?;
            }
        }
        Ok(())
    }

    pub fn write_exefs(&self, r: &str, data: &[u8]) -> Res<PB> {
        self.put("exefs", r, data)
    }

    pub fn write_romfs(&self, r: &str, data: &[u8]) -> Res<PB> {
        self.put("romfs", r, data)
    }

    fn put(&self, top: &str, r: &str, data: &[u8]) -> Res<PB> {
        let p: PB = self.root.join(top).join(rel(r)?);
        if let Some(d) = p.parent() {
            fs::create_dir_all(d)?;
        }
        fs::write(&p, data)?;
        Ok(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PB {
        let pb: PB = std::env::temp_dir().join(format!("ootrr_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&pb);
        pb
    }

    #[test]
    fn clean_empties_the_two_mod_folders_and_nothing_else() {
        let load: PB = tmp("clean");
        let pk: Pack = Pack::create(&load, "0400486024dec000", "Test Mod").unwrap();
        pk.write_romfs("A/b.bin", b"x").unwrap();
        fs::write(pk.root.join("notes.txt"), b"keep").unwrap();
        Pack::clean(&load, "0400486024dec000", "Test Mod").unwrap();
        assert!(!pk.root.join("romfs").exists());
        assert!(pk.root.join("notes.txt").is_file());
        assert!(Pack::clean(&load, "0400486024dec000", "..").is_err());
        let _ = fs::remove_dir_all(&load);
    }

    #[test]
    fn creates_layout() {
        let load: PB = tmp("layout");
        let pk: Pack = Pack::create(&load, "0400486024dec000", "Test Mod").unwrap();
        assert!(pk.root.join("romfs").is_dir());
        assert!(pk.root.join("exefs").is_dir());
        assert!(pk.root.ends_with(P::new("0400486024DEC000").join("Test Mod")));
        let _ = fs::remove_dir_all(&load);
    }

    #[test]
    fn writes_nested_files() {
        let load: PB = tmp("nested");
        let pk: Pack = Pack::create(&load, "0400486024DEC000", "Nested").unwrap();
        let p: PB = pk.write_romfs("a/b/c.bin", &[1, 2, 3]).unwrap();
        assert_eq!(fs::read(&p).unwrap(), vec![1, 2, 3]);
        let _ = fs::remove_dir_all(&load);
    }

    #[test]
    fn rejects_bad_names_and_paths() {
        let load: PB = tmp("bad");
        assert!(Pack::create(&load, "xyz", "ok").is_err());
        assert!(Pack::create(&load, "0400486024DEC000", "..").is_err());
        assert!(Pack::create(&load, "0400486024DEC000", "a/b").is_err());
        assert!(Pack::create(&load, "0400486024DEC000", "a:b").is_err());
        let pk: Pack = Pack::create(&load, "0400486024DEC000", "Safe").unwrap();
        assert!(pk.write_exefs("../escape.bin", &[0]).is_err());
        assert!(pk.write_exefs("/abs.bin", &[0]).is_err());
        assert!(pk.write_exefs("C:\\abs.bin", &[0]).is_err());
        assert!(pk.write_exefs("f:stream", &[0]).is_err());
        assert!(pk.write_exefs("", &[0]).is_err());
        let _ = fs::remove_dir_all(&load);
    }
}
