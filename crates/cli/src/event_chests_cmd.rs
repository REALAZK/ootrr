use std::collections::HashMap as HM;
use std::fs;
use std::path::Path as P;

use ootrr_core::error::{Error, Res};
use ootrr_core::event_chests::resolve::{self, Row};
use ootrr_core::event_chests::testmod::{self, Pick};
use ootrr_core::event_chests::{self, catalog, out, walk};
use ootrr_core::modpack::Pack;
use ootrr_core::placement::Edit;
use ootrr_core::rsizetable::{self, Rst};
use ootrr_core::{zs, TITLE_ID};
use ootrr_core::romfs::Romfs;

const SIZE_MARGIN: u32 = 0x400;

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(Error::Format(what))
}

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let loc: &str = arg(a, 1, "missing locations.json")?;
    let sl: &str = arg(a, 2, "missing slice.json")?;
    let o: &str = arg(a, 3, "missing out file")?;
    let snap: catalog::Snap = catalog::parse(&fs::read(loc)?, &fs::read(sl)?)?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let r: event_chests::Report = event_chests::analyze(&mut rf, snap)?;
    let mut t: String = out::to_json(&r).pretty();
    t.push('\n');
    fs::write(o, t)?;
    print!("{}", out::summary(&r));
    Ok(())
}

pub fn show(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let t: HM<String, Row> = resolve::table(&mut rf)?;
    let mut l: walk::Loader = walk::Loader::new(&mut rf, &t);
    for n in &a[1..] {
        let Some(r) = t.get(n) else {
            println!("{n}: no EventSetting row");
            continue;
        };
        println!("{n} -> {}:{} [{}]", r.stem, r.entry, r.comp);
        let w: walk::Reach = walk::walk(&mut l, &r.stem, &r.entry)?;
        for s in &w.steps {
            if matches!(s.kind, "fork" | "join" | "label") {
                continue;
            }
            println!("  {} #{} {} {} {:?}", s.stem, s.idx, s.kind, s.head, s.args);
        }
        for m in &w.missing {
            println!("  missing {m}");
        }
    }
    Ok(())
}




pub fn testmod(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let src: &str = arg(a, 1, "missing event_chests.json")?;
    let load: &str = arg(a, 2, "missing load_dir")?;
    let name: &str = arg(a, 3, "missing mod_name")?;
    let list: Vec<Pick> = testmod::picks(&fs::read_to_string(src)?)?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let edits: Vec<Edit> = testmod::edits(&mut rf, &list)?;
    let pk: Pack = Pack::create(P::new(load), TITLE_ID, name)?;
    let mut rst: Option<Rst> = None;
    for e in &edits {
        let z: Vec<u8> = zs::compress_raw(&e.bytes, Some(3))?;
        let at: std::path::PathBuf = pk.write_romfs(&e.file, &z)?;
        println!("{}: {} -> {} bytes decompressed", e.file, e.old_len, e.bytes.len());
        println!("  wrote {}", at.display());
        if e.bytes.len() > e.old_len {
            if rst.is_none() {
                rst = Some(rsizetable::load(&mut rf)?);
            }
            if let Some(t) = rst.as_mut() {
                t.grow(&e.file, e.old_len as u64, e.bytes.len() as u64, SIZE_MARGIN)?;
            }
        }
    }
    if let Some(t) = &rst {
        pk.write_romfs(rsizetable::TABLE_PATH, &t.to_zs()?)?;
        println!("resource size table updated");
    }
    println!("{} chest(s) in {} file(s)", list.len(), edits.len());
    Ok(())
}



