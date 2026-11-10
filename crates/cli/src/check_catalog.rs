use std::path::Path as P;
use std::time::Instant;

use ootrr_core::catalog::{self, Catalog, Seen};
use ootrr_core::error::{Error, Res};
use ootrr_core::romfs::Romfs;

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = a.first().ok_or(Error::Format("missing romfs_root"))?;
    let src: &str = a.get(1).ok_or(Error::Format("missing catalog_json"))?;
    let c: Catalog = Catalog::load(P::new(src))?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let t0: Instant = Instant::now();
    let seen: Vec<Seen> = catalog::scan(&mut rf)?;
    let bad: Vec<String> = catalog::verify(&c, &seen);
    print!("{}", catalog::coverage(&c));
    println!("actors scanned: {}  issues: {}", seen.len(), bad.len());
    for m in bad.iter().take(20) {
        println!("  {m}");
    }
    println!("elapsed: {:.1}s", t0.elapsed().as_secs_f32());
    if bad.is_empty() {
        Ok(())
    } else {
        Err(Error::Format("catalog verification failed"))
    }
}
