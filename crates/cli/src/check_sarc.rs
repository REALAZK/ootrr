use std::collections::BTreeMap as Map;
use std::path::Path as P;
use std::time::Instant;

use ootrr_core::error::{Error, Res};
use ootrr_core::romfs::Romfs;
use ootrr_core::sarc;
use ootrr_core::zs;

fn gist(s: &str) -> String {
    if let Some(i) = s.find(" magic ") {
        let t: &str = &s[i + 7..];
        return format!("file magic {}", t.split(' ').next().unwrap_or(""));
    }
    s.split(' ').take(2).collect::<Vec<&str>>().join(" ")
}

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = a.first().ok_or(Error::Format("missing romfs_root"))?;
    let sub: &str = a.get(1).ok_or(Error::Format("missing subfolder"))?;
    let suf: &str = a.get(2).ok_or(Error::Format("missing file suffix"))?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let files: Vec<String> = rf.list(sub)?.into_iter().filter(|p| p.ends_with(suf)).collect();
    let t0: Instant = Instant::now();
    let mut same: usize = 0;
    let mut zsok: usize = 0;
    let mut skipped: usize = 0;
    let mut kinds: Map<String, usize> = Map::new();
    let mut shown: Vec<String> = Vec::new();
    let mut bad: Vec<String> = Vec::new();
    for f in &files {
        let got: Res<Vec<u8>> = if f.ends_with(".zs") { rf.read_zs(f) } else { rf.read(f) };
        let raw: Vec<u8> = match got {
            Ok(b) => b,
            Err(e) => {
                bad.push(format!("{f}: {e}"));
                continue;
            }
        };
        if raw.get(..4) != Some(b"SARC".as_slice()) {
            skipped += 1;
            continue;
        }
        let out: Res<Vec<u8>> = sarc::parse(&raw).and_then(|e| sarc::write(&e));
        match out {
            Ok(o) => {
                if o == raw {
                    same += 1;
                } else {
                    let why: String = sarc::explain(&raw, &o);
                    *kinds.entry(gist(&why)).or_insert(0) += 1;
                    if shown.len() < 12 {
                        shown.push(format!("{f}: {why}"));
                    }
                }
                let packed: Vec<u8> = zs::compress_raw(&o, Some(3))?;
                if rf.zs.decompress(&packed)? == o && sarc::parse(&o).is_ok() {
                    zsok += 1;
                }
            }
            Err(e) => bad.push(format!("{f}: {e}")),
        }
    }
    println!(
        "files: {}  not sarc: {}  byte-identical rewrites: {}  differing: {}  failures: {}  zstd round trips: {}",
        files.len(),
        skipped,
        same,
        files.len() - skipped - same - bad.len(),
        bad.len(),
        zsok
    );
    for (k, v) in &kinds {
        println!("  pattern {k}: {v}");
    }
    for m in shown.iter().chain(bad.iter().take(8)) {
        println!("  {m}");
    }
    println!("elapsed: {:.1}s", t0.elapsed().as_secs_f32());
    Ok(())
}
