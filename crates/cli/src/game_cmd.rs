use std::path::Path as P;

use ootrr_core::error::{Error, Res};
use ootrr_core::required::{list, missing};
use ootrr_core::slice::data::Data;

pub fn check(a: &[String]) -> Res<()> {
    let root: &str = a.first().ok_or(Error::Format("missing romfs_root"))?;
    let d: Data = Data::load()?;
    let total: usize = list(&d).len();
    let gone: Vec<String> = missing(P::new(root), &d);
    println!("{} of {} files the build reads are present", total - gone.len(), total);
    for g in &gone {
        println!("missing {g}");
    }
    if gone.is_empty() {
        Ok(())
    } else {
        Err(Error::Msg(format!("{} files are missing from {root}", gone.len())))
    }
}
