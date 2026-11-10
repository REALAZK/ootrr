use std::collections::BTreeMap as Map;
use std::path::Path as P;
use std::time::Instant;

use ootrr_core::TITLE_ID;
use ootrr_core::ainb::{self, start, Ainb};
use ootrr_core::error::{Error, Res};
use ootrr_core::modpack::Pack;
use ootrr_core::murmur::murmur3_32;
use ootrr_core::romfs::Romfs;
use ootrr_core::rsizetable::{self, Rst};

const MARGIN: u32 = 0x400;
const LISTED: usize = 40;

fn load(rf: &mut Romfs, r: &str) -> Res<Vec<u8>> {
    if r.ends_with(".zs") { rf.read_zs(r) } else { rf.read(r) }
}

fn explain(a: &[u8], b: &[u8]) -> String {
    let n: usize = a.len().min(b.len());
    let at: usize = (0..n).find(|&i| a[i] != b[i]).unwrap_or(n);
    format!("sizes {} vs {}, first difference at {:#x}", a.len(), b.len(), at)
}

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = a.first().ok_or(Error::Format("missing romfs_root"))?;
    let sub: &str = a.get(1).ok_or(Error::Format("missing subfolder"))?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let files: Vec<String> = rf.list(sub)?.into_iter().filter(|p| p.ends_with(".ainb") || p.ends_with(".ainb.zs")).collect();
    let t0: Instant = Instant::now();
    let mut parsed: usize = 0;
    let mut same: usize = 0;
    let mut eq: usize = 0;
    let mut clean: usize = 0;
    let mut hashed: usize = 0;
    let mut hash_ok: usize = 0;
    let mut pushes: usize = 0;
    let mut pushes_ok: usize = 0;
    let mut kinds: Map<String, usize> = Map::new();
    let mut shown: Vec<String> = Vec::new();
    for f in &files {
        let raw: Vec<u8> = load(&mut rf, f)?;
        let ab: Ainb = match Ainb::parse(&raw) {
            Ok(x) => x,
            Err(e) => {
                *kinds.entry(format!("parse: {e}")).or_insert(0) += 1;
                shown.push(format!("{f}: parse: {e}"));
                continue;
            }
        };
        parsed += 1;
        if ainb::audit(&ab).is_empty() {
            clean += 1;
        } else {
            shown.push(format!("{f}: {}", ainb::audit(&ab)[0]));
        }
        for i in 0..ab.els.len() {
            if ab.el_type(i) == 0 {
                hashed += 1;
                if murmur3_32(ab.el_name(i).as_bytes(), 0) == ab.el_hash(i) {
                    hash_ok += 1;
                }
            }
        }
        match ab.write() {
            Ok(o) => {
                if o == raw {
                    same += 1;
                } else {
                    *kinds.entry("bytes differ".to_string()).or_insert(0) += 1;
                    shown.push(format!("{f}: {}", explain(&raw, &o)));
                }
                if Ainb::parse(&o).is_ok_and(|x| x == ab) {
                    eq += 1;
                }
            }
            Err(e) => {
                *kinds.entry(format!("write: {e}")).or_insert(0) += 1;
                shown.push(format!("{f}: write: {e}"));
            }
        }
        if !ab.find_els(start::PUSH).is_empty() {
            pushes += 1;
            let mut m: Ainb = ab.clone();
            let n: usize = start::items(&m).len();
            let ok: bool = start::add_item(&mut m, "Ootrr_Test_Item", 2)
                .and_then(|_| m.write())
                .and_then(|o| Ainb::parse(&o))
                .is_ok_and(|x| start::items(&x).len() == n + 1 && ainb::audit(&x).is_empty());
            if ok {
                pushes_ok += 1;
            }
        }
    }
    println!(
        "files: {}  parsed: {}  byte-identical rewrites: {}  differing or failed: {}  reparse structurally equal: {}",
        files.len(),
        parsed,
        same,
        files.len() - same,
        eq
    );
    println!("layout audits clean: {clean} of {parsed}");
    println!("node name hashes matching the type name: {hash_ok} of {hashed}");
    println!("add-item round trips on files with a push to pouch node: {pushes_ok} of {pushes}");
    for (k, v) in &kinds {
        println!("  {k}: {v}");
    }
    for m in shown.iter().take(12) {
        println!("  {m}");
    }
    println!("elapsed: {:.1}s", t0.elapsed().as_secs_f32());
    Ok(())
}

pub fn dump(a: &[String]) -> Res<()> {
    let root: &str = a.first().ok_or(Error::Format("missing romfs_root"))?;
    let r: &str = a.get(1).ok_or(Error::Format("missing relative path"))?;
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let raw: Vec<u8> = load(&mut rf, r)?;
    print!("{}", ainb::show::show(&Ainb::parse(&raw)?));
    Ok(())
}

fn state(a: &Ainb) -> String {
    let h: String = start::hearts(a).map_or("unknown".to_string(), |n| n.to_string());
    let life: String = start::game_int(a, "Life").map_or("unknown".to_string(), |n| n.to_string());
    let mut s: String = format!("hearts {h} (Life {life})\n");
    for it in start::items(a) {
        s.push_str(&format!("  item {} x{} (element {})\n", it.actor, it.num, it.el));
    }
    s
}

fn diff(a: &[u8], b: &[u8]) {
    if a.len() != b.len() {
        println!("size {} -> {} bytes ({:+})", a.len(), b.len(), b.len() as i64 - a.len() as i64);
        println!("{}", explain(a, b));
        return;
    }
    let d: Vec<usize> = (0..a.len()).filter(|&i| a[i] != b[i]).collect();
    println!("size unchanged at {} bytes, {} byte(s) differ", a.len(), d.len());
    for i in d.iter().take(LISTED) {
        println!("  {:#06x}: {:02x} -> {:02x}", i, a[*i], b[*i]);
    }
}

pub fn start_cmd(a: &[String]) -> Res<()> {
    let root: &str = a.first().ok_or(Error::Format("missing romfs_root"))?;
    let load_dir: &str = a.get(1).ok_or(Error::Format("missing load_dir or -"))?;
    let name: &str = a.get(2).ok_or(Error::Format("missing mod_name"))?;
    let mut hearts: Option<i32> = None;
    let mut add: Vec<(String, i32)> = Vec::new();
    let mut i: usize = 3;
    while i < a.len() {
        let v: &str = a.get(i + 1).map(|s| s.as_str()).ok_or(Error::Format("flag needs a value"))?;
        match a[i].as_str() {
            "--hearts" => hearts = Some(v.parse().map_err(|_| Error::Format("bad heart count"))?),
            "--item" => {
                let (n, c): (&str, &str) = v.split_once(':').unwrap_or((v, "1"));
                add.push((n.to_string(), c.parse().map_err(|_| Error::Format("bad item count"))?));
            }
            _ => return Err(Error::Format("unknown flag, use --hearts N or --item NAME[:COUNT]")),
        }
        i += 2;
    }
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let raw: Vec<u8> = rf.read(start::MODULE)?;
    let mut ab: Ainb = Ainb::parse(&raw)?;
    print!("before: {}", state(&ab));
    start::patch(&mut ab, hearts, &add)?;
    let out: Vec<u8> = ab.write()?;
    let back: Ainb = Ainb::parse(&out)?;
    print!("after: {}", state(&back));
    println!("patched file rewrites byte-identically: {}  layout audit problems: {}", back.write()? == out, ainb::audit(&back).len());
    diff(&raw, &out);
    if load_dir == "-" {
        println!("dry run, nothing written");
        return Ok(());
    }
    let pk: Pack = Pack::create(P::new(load_dir), TITLE_ID, name)?;
    println!("wrote {}", pk.write_romfs(start::MODULE, &out)?.display());
    if out.len() > raw.len() {
        let mut t: Rst = rsizetable::load(&mut rf)?;
        match t.grow(start::MODULE, raw.len() as u64, out.len() as u64, MARGIN)? {
            Some(n) => {
                println!("resource size table entry grown to {n}");
                println!("wrote {}", pk.write_romfs(rsizetable::TABLE_PATH, &t.to_zs()?)?.display());
            }
            None => println!("module has no resource size table entry, table left alone"),
        }
    }
    Ok(())
}
