use std::fs;
use std::path::{Path as P, PathBuf as PB};

use crate::byml::Byml;
use crate::error::{Error, Res};
use crate::modpack::rel;
use crate::sarc;
use crate::zs::Zs;

pub struct Romfs {
    pub root: PB,
    pub zs: Zs,
}

impl Romfs {
    pub fn open(root: &P) -> Res<Romfs> {
        let mut zs: Zs = Zs::new();
        let raw: Vec<u8> = fs::read(root.join("Pack").join("ZsDic.pack.zs"))?;
        let pack: Vec<u8> = zs.decompress(&raw)?;
        for e in sarc::parse(&pack)? {
            if e.name.ends_with(".zsdic") {
                zs.add_dict(&e.data)?;
            }
        }
        Ok(Romfs { root: root.to_path_buf(), zs })
    }

    pub fn read(&self, r: &str) -> Res<Vec<u8>> {
        Ok(fs::read(self.root.join(rel(r)?))?)
    }

    pub fn read_zs(&mut self, r: &str) -> Res<Vec<u8>> {
        let b: Vec<u8> = self.read(r)?;
        self.zs.decompress(&b)
    }

    pub fn read_byml(&mut self, r: &str) -> Res<Byml> {
        let b: Vec<u8> = if r.ends_with(".zs") { self.read_zs(r)? } else { self.read(r)? };
        Byml::parse(&b)
    }

    pub fn list(&self, sub: &str) -> Res<Vec<String>> {
        let top: PB = self.root.join(rel(sub)?);
        let mut stack: Vec<PB> = vec![top];
        let mut out: Vec<String> = Vec::new();
        while let Some(d) = stack.pop() {
            for e in fs::read_dir(&d)? {
                let e: fs::DirEntry = e?;
                let p: PB = e.path();
                if e.file_type()?.is_dir() {
                    stack.push(p);
                } else {
                    let r: &P = p
                        .strip_prefix(&self.root)
                        .map_err(|_| Error::BadPath(p.display().to_string()))?;
                    out.push(r.to_string_lossy().replace('\\', "/"));
                }
            }
        }
        out.sort();
        Ok(out)
    }
}
