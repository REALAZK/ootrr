use std::path::{Path as P, PathBuf as PB};

use ootrr_core::byml::Byml;
use ootrr_core::error::{Error, Res};
use ootrr_core::modpack::Pack;
use ootrr_core::placement::banc_files;
use ootrr_core::romfs::Romfs;
use ootrr_core::rsizetable::{self, Rst};
use ootrr_core::slice::treasure::test_edit;
use ootrr_core::{zs, TITLE_ID};

const SIZE_MARGIN: u32 = 0x400;

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(Error::Format(what))
}

pub fn testmod(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let load: &str = arg(a, 1, "missing load_dir")?;
    let name: &str = arg(a, 2, "missing mod_name")?;
    let actor: &str = arg(a, 3, "missing item actor")?;
    let view: bool = !a.get(4).is_some_and(|s| s == "keep-view");
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let pk: Pack = Pack::create(P::new(load), TITLE_ID, name)?;
    let mut rst: Option<Rst> = None;
    let mut n: usize = 0;
    for f in banc_files(&rf)? {
        let raw: Vec<u8> = rf.read_zs(&f)?;
        let mut y: Byml = Byml::parse(&raw)?;
        if test_edit(&mut y, actor, view)? == 0 {
            continue;
        }
        n += 1;
        let b: Vec<u8> = y.write()?;
        let at: PB = pk.write_romfs(&f, &zs::compress_raw(&b, Some(3))?)?;
        println!("{f}: {} -> {} bytes decompressed", raw.len(), b.len());
        println!("  wrote {}", at.display());
        if b.len() > raw.len() {
            if rst.is_none() {
                rst = Some(rsizetable::load(&mut rf)?);
            }
            if let Some(t) = rst.as_mut() {
                t.grow(&f, raw.len() as u64, b.len() as u64, SIZE_MARGIN)?;
            }
        }
    }
    if n == 0 {
        return Err(Error::Format("the Treasure Chest Game reward chest was not found"));
    }
    if let Some(t) = &rst {
        pk.write_romfs(rsizetable::TABLE_PATH, &t.to_zs()?)?;
        println!("resource size table updated");
    }
    println!("first win chest now holds {actor}, preview {}", if view { "changed" } else { "kept" });
    Ok(())
}
