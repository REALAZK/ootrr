use std::path::Path as P;

use ootrr_core::error::{Error, Res};
use ootrr_core::rsizetable;

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = a.first().ok_or(Error::Format("missing romfs_root"))?;
    for l in rsizetable::check(P::new(root))? {
        println!("{l}");
    }
    if let Some(n) = a.get(1) {
        let per: usize = n.parse().map_err(|_| Error::Format("bad sample count"))?;
        for l in rsizetable::sample(P::new(root), per)? {
            println!("{l}");
        }
    }
    Ok(())
}
