use std::fs;
use std::path::Path as P;

use ootrr_core::error::{Error, Res};
use ootrr_core::hints::{labels, markup, pick, sentence, FILE};
use ootrr_core::msbt;
use ootrr_core::romfs::Romfs;
use ootrr_core::sarc;
use ootrr_core::settings::seed::SeedFile;
use ootrr_core::slice::data::Data;

pub fn show(a: &[String]) -> Res<()> {
    let seed: &str = a.first().ok_or(Error::Format("missing seed file"))?;
    let root: &str = a.get(1).ok_or(Error::Format("missing romfs_root"))?;
    let s: SeedFile = SeedFile::parse(&fs::read_to_string(seed)?)?;
    let d: Data = Data::load()?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let raw: Vec<u8> = rf.read_zs("Mals/USen.Product.110.sarc.zs")?;
    let es: Vec<sarc::Entry> = sarc::parse(&raw)?;
    let e: &sarc::Entry = sarc::get(&es, FILE).ok_or(Error::Format("no gossip stone text"))?;
    let m: msbt::Msbt = msbt::parse(&e.data)?;
    let ls: Vec<String> = labels(&m);
    let hs = pick(&d, &s, ls.len());
    for (l, h) in ls.iter().zip(hs.iter()) {
        let _ = markup(&sentence(h))?;
        println!("{l}: {}", sentence(h));
    }
    Ok(())
}
