use std::path::{Path as P, PathBuf as PB};

use crate::error::{Error, Res};
use crate::romfs::Romfs;
use crate::zs::{self, Zs};

pub const MAGIC: [u8; 6] = *b"RESTBL";
pub const HEAD: usize = 22;
pub const TABLE_PATH: &str =
    "System/Resource/ResourceSizeTable.Product.111.Nin_Ounce_NVN2.rsizetable.zs";
pub const ALIGN: u64 = 32;

const fn make_crc() -> [u32; 256] {
    let mut t: [u32; 256] = [0; 256];
    let mut i: usize = 0;
    while i < 256 {
        let mut c: u32 = i as u32;
        let mut j: u32 = 0;
        while j < 8 {
            c = if c & 1 == 1 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            j += 1;
        }
        t[i] = c;
        i += 1;
    }
    t
}

const CRC: [u32; 256] = make_crc();

pub fn hash(p: &str) -> u32 {
    let mut c: u32 = 0xFFFF_FFFF;
    for b in p.bytes() {
        c = CRC[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    !c
}

pub fn key(p: &str) -> &str {
    let t: &str = p.trim_start_matches('/');
    t.strip_suffix(".zs").unwrap_or(t)
}

pub fn align(n: u64) -> u64 {
    n.div_ceil(ALIGN) * ALIGN
}

fn pad(k: &str, n: usize) -> Option<Vec<u8>> {
    if k.len() > n {
        return None;
    }
    let mut out: Vec<u8> = k.as_bytes().to_vec();
    out.resize(n, 0);
    Some(out)
}

fn u32_at(b: &[u8], i: usize) -> Res<u32> {
    let s: &[u8] = b
        .get(i..i.saturating_add(4))
        .ok_or(Error::Format("rsizetable truncated"))?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rst {
    pub version: u32,
    pub name_len: usize,
    pub crc: Vec<(u32, u32)>,
    pub names: Vec<(Vec<u8>, u32)>,
}

impl Rst {
    pub fn parse(b: &[u8]) -> Res<Rst> {
        if b.len() < HEAD || b[..6] != MAGIC {
            return Err(Error::Format("not a RESTBL table"));
        }
        let version: u32 = u32_at(b, 6)?;
        let nl: usize = u32_at(b, 10)? as usize;
        let nc: usize = u32_at(b, 14)? as usize;
        let nn: usize = u32_at(b, 18)? as usize;
        let ent: usize = nl
            .checked_add(4)
            .ok_or(Error::Format("rsizetable name length overflow"))?;
        let need: usize = nc
            .checked_mul(8)
            .and_then(|x| nn.checked_mul(ent).and_then(|y| x.checked_add(y)))
            .and_then(|x| x.checked_add(HEAD))
            .ok_or(Error::Format("rsizetable counts overflow"))?;
        if need != b.len() {
            return Err(Error::Format("rsizetable size does not match its counts"));
        }
        let mut crc: Vec<(u32, u32)> = Vec::with_capacity(nc);
        for i in 0..nc {
            let o: usize = HEAD + i * 8;
            crc.push((u32_at(b, o)?, u32_at(b, o + 4)?));
        }
        let base: usize = HEAD + nc * 8;
        let mut names: Vec<(Vec<u8>, u32)> = Vec::with_capacity(nn);
        for i in 0..nn {
            let o: usize = base + i * ent;
            let n: &[u8] = b
                .get(o..o + nl)
                .ok_or(Error::Format("rsizetable truncated"))?;
            names.push((n.to_vec(), u32_at(b, o + nl)?));
        }
        Ok(Rst { version, name_len: nl, crc, names })
    }

    pub fn write(&self) -> Res<Vec<u8>> {
        let nl: u32 = u32::try_from(self.name_len)
            .map_err(|_| Error::Format("rsizetable name length too large"))?;
        let nc: u32 = u32::try_from(self.crc.len())
            .map_err(|_| Error::Format("rsizetable too many hashes"))?;
        let nn: u32 = u32::try_from(self.names.len())
            .map_err(|_| Error::Format("rsizetable too many names"))?;
        let mut out: Vec<u8> = Vec::with_capacity(
            HEAD + self.crc.len() * 8 + self.names.len() * (self.name_len + 4),
        );
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&self.version.to_le_bytes());
        out.extend_from_slice(&nl.to_le_bytes());
        out.extend_from_slice(&nc.to_le_bytes());
        out.extend_from_slice(&nn.to_le_bytes());
        for (h, v) in &self.crc {
            out.extend_from_slice(&h.to_le_bytes());
            out.extend_from_slice(&v.to_le_bytes());
        }
        for (n, v) in &self.names {
            if n.len() != self.name_len {
                return Err(Error::Format("rsizetable name has the wrong width"));
            }
            out.extend_from_slice(n);
            out.extend_from_slice(&v.to_le_bytes());
        }
        Ok(out)
    }

    pub fn to_zs(&self) -> Res<Vec<u8>> {
        zs::compress_raw(&self.write()?, None)
    }

    pub fn sorted(&self) -> (bool, bool) {
        let c: bool = self.crc.windows(2).all(|w| w[0].0 < w[1].0);
        let n: bool = self.names.windows(2).all(|w| w[0].0 < w[1].0);
        (c, n)
    }

    fn name_at(&self, k: &str) -> Option<usize> {
        let nb: Vec<u8> = pad(k, self.name_len)?;
        self.names.binary_search_by(|e| e.0.cmp(&nb)).ok()
    }

    pub fn get(&self, path: &str) -> Option<u32> {
        let k: &str = key(path);
        if let Some(i) = self.name_at(k) {
            return Some(self.names[i].1);
        }
        let h: u32 = hash(k);
        let i: usize = self.crc.binary_search_by_key(&h, |e| e.0).ok()?;
        Some(self.crc[i].1)
    }

    pub fn set_size(&mut self, path: &str, size: u32) -> Res<Option<u32>> {
        let k: &str = key(path);
        if k.is_empty() {
            return Err(Error::BadPath(path.to_string()));
        }
        if let Some(i) = self.name_at(k) {
            let old: u32 = self.names[i].1;
            self.names[i].1 = size;
            return Ok(Some(old));
        }
        let h: u32 = hash(k);
        match self.crc.binary_search_by_key(&h, |e| e.0) {
            Ok(i) => {
                let old: u32 = self.crc[i].1;
                self.crc[i].1 = size;
                Ok(Some(old))
            }
            Err(i) => {
                self.crc.insert(i, (h, size));
                Ok(None)
            }
        }
    }

    pub fn absorb(&mut self, other: &Rst) -> usize {
        let mut n: usize = 0;
        for &(h, sz) in &other.crc {
            match self.crc.binary_search_by_key(&h, |e| e.0) {
                Ok(i) => {
                    if self.crc[i].1 < sz {
                        self.crc[i].1 = sz;
                        n += 1;
                    }
                }
                Err(i) => {
                    self.crc.insert(i, (h, sz));
                    n += 1;
                }
            }
        }
        if other.name_len == self.name_len {
            for (name, sz) in &other.names {
                match self.names.binary_search_by(|e| e.0.cmp(name)) {
                    Ok(i) => {
                        if self.names[i].1 < *sz {
                            self.names[i].1 = *sz;
                            n += 1;
                        }
                    }
                    Err(i) => {
                        self.names.insert(i, (name.clone(), *sz));
                        n += 1;
                    }
                }
            }
        }
        n
    }

    pub fn grow(&mut self, path: &str, old_real: u64, new_real: u64, margin: u32) -> Res<Option<u32>> {
        let cur: u32 = match self.get(path) {
            Some(c) => c,
            None => return Ok(None),
        };
        if new_real <= old_real {
            return Ok(Some(cur));
        }
        let add: u64 = align(new_real) - align(old_real) + align(margin as u64);
        let n: u32 = u32::try_from(cur as u64 + add)
            .map_err(|_| Error::Format("resource size does not fit in 32 bits"))?;
        self.set_size(path, n)?;
        Ok(Some(n))
    }
}

pub fn load(rf: &mut Romfs) -> Res<Rst> {
    Rst::parse(&rf.read_zs(TABLE_PATH)?)
}

pub struct Fold {
    pub grew: usize,
    /// Mods whose size table is there but could not be read.
    pub unread: Vec<String>,
    /// Mods with a size table whose name sorts after the skipped mod, so the game serves their table instead.
    pub later: Vec<String>,
}

/// The enabled mods under `dir`, as Hookshot and Longshot find them: a folder holding romfs or exefs is a mod, other
/// folders are searched up to four levels down, and a name starting with `_`, `-` or `.` is off.
fn enabled_mods(dir: &P, name: &str, depth: usize, out: &mut Vec<(String, PB)>) {
    if depth > 0 && (dir.join("romfs").is_dir() || dir.join("exefs").is_dir()) {
        out.push((name.to_string(), dir.to_path_buf()));
        return;
    }
    if depth >= 4 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let n: String = e.file_name().to_string_lossy().into_owned();
        if n.starts_with(['_', '-', '.']) || !e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let sub: String = if name.is_empty() { n } else { format!("{name}/{n}") };
        enabled_mods(&e.path(), &sub, depth + 1, out);
    }
}

pub fn fold_mods(table: &mut Rst, mods: &P, skip: &str) -> Res<Fold> {
    let mut found: Vec<(String, PB)> = Vec::new();
    enabled_mods(mods, "", 0, &mut found);
    let own: String = skip.to_ascii_lowercase();
    let mut f: Fold = Fold { grew: 0, unread: Vec::new(), later: Vec::new() };
    for (name, dir) in found {
        if name == skip {
            continue;
        }
        let raw: Vec<u8> = match std::fs::read(dir.join("romfs").join(TABLE_PATH)) {
            Ok(b) if !b.is_empty() => b,
            _ => continue,
        };
        if name.to_ascii_lowercase() > own {
            f.later.push(name.clone());
        }
        match Zs::new().decompress(&raw).and_then(|b| Rst::parse(&b)) {
            Ok(t) => f.grew += table.absorb(&t),
            Err(_) => f.unread.push(name),
        }
    }
    f.unread.sort();
    f.later.sort();
    Ok(f)
}

fn pick(v: &[String], n: usize) -> Vec<&String> {
    let m: usize = v.len();
    if m == 0 || n == 0 {
        return Vec::new();
    }
    let c: usize = n.min(m);
    (0..c).map(|i| &v[i * m / c]).collect()
}

pub fn check(root: &P) -> Res<Vec<String>> {
    let mut rf: Romfs = Romfs::open(root)?;
    let raw: Vec<u8> = rf.read_zs(TABLE_PATH)?;
    let t: Rst = Rst::parse(&raw)?;
    let out: Vec<u8> = t.write()?;
    let (sc, sn): (bool, bool) = t.sorted();
    let packed: Vec<u8> = t.to_zs()?;
    let back: Vec<u8> = rf.zs.decompress(&packed)?;
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "table: version {}  name width {}  hash entries {}  name entries {}",
        t.version,
        t.name_len,
        t.crc.len(),
        t.names.len()
    ));
    lines.push(format!("decompressed bytes: {}", raw.len()));
    lines.push(format!("hash entries sorted: {sc}  name entries sorted: {sn}"));
    lines.push(format!("byte-identical rewrite: {}", out == raw));
    lines.push(format!("reparse structurally equal: {}", Rst::parse(&out)? == t));
    lines.push(format!("zstd repack round trip identical: {}", back == out));
    let files: Vec<String> = rf
        .list("Banc")?
        .into_iter()
        .filter(|p| p.ends_with(".bcett.byml.zs"))
        .collect();
    if let Some(f) = files.first() {
        let mut m: Rst = t.clone();
        let old: Option<u32> = m.set_size(f, 0x1234_5678)?;
        let got: Option<u32> = Rst::parse(&m.write()?)?.get(f);
        let edit_ok: bool = got == Some(0x1234_5678) && m.crc.len() == t.crc.len();
        if let Some(o) = old {
            m.set_size(f, o)?;
        }
        lines.push(format!("set_size edit visible after reparse: {edit_ok}"));
        lines.push(format!("set_size revert byte-identical: {}", m.write()? == raw));
        let mut g: Rst = t.clone();
        let ins: Option<u32> = g.set_size("Banc/Ootrr_Not_A_Real_File.bcett.byml", 7)?;
        let (gc, _): (bool, bool) = g.sorted();
        lines.push(format!(
            "insert of a new path: previous {:?}  count {} -> {}  still sorted: {gc}",
            ins,
            t.crc.len(),
            g.crc.len()
        ));
    }
    Ok(lines)
}

pub fn sample(root: &P, per: usize) -> Res<Vec<String>> {
    let mut rf: Romfs = Romfs::open(root)?;
    let t: Rst = load(&mut rf)?;
    let groups: [(&str, &str); 3] = [
        ("Banc", ".bcett.byml.zs"),
        ("Pack", ".pack.zs"),
        ("Mals", ".sarc.zs"),
    ];
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "{:<76} {:>9} {:>9} {:>7} {:>7}",
        "path", "entry", "real", "diff", "diff32"
    ));
    for (dir, suf) in groups {
        let files: Vec<String> = rf
            .list(dir)?
            .into_iter()
            .filter(|p| p.ends_with(suf))
            .collect();
        let have: usize = files.iter().filter(|p| t.get(p).is_some()).count();
        let mut ks: Vec<i64> = Vec::new();
        for f in pick(&files, per) {
            let real: u64 = rf.read_zs(f)?.len() as u64;
            let e: Option<u32> = t.get(f);
            match e {
                Some(e) => {
                    let d: i64 = e as i64 - real as i64;
                    let k: i64 = e as i64 - align(real) as i64;
                    ks.push(k);
                    lines.push(format!("{:<76} {:>9} {:>9} {:>7} {:>7}", key(f), e, real, d, k));
                }
                None => lines.push(format!("{:<76} {:>9} {:>9}", key(f), "none", real)),
            }
        }
        let lo: i64 = ks.iter().copied().min().unwrap_or(0);
        let hi: i64 = ks.iter().copied().max().unwrap_or(0);
        lines.push(format!(
            "group {dir}: {} files with suffix {suf}  {} have an entry  sampled diff32 range {lo}..{hi}",
            files.len(),
            have
        ));
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(s: &str) -> Vec<u8> {
        pad(s, 16).unwrap_or_default()
    }

    fn table() -> Rst {
        let mut c: Vec<(u32, u32)> = vec![(hash("a.bin"), 10), (hash("b.bin"), 20), (hash("c.bin"), 30)];
        c.sort();
        Rst {
            version: 1,
            name_len: 16,
            crc: c,
            names: vec![(name("x/one.bin"), 5), (name("x/two.bin"), 6)],
        }
    }

    #[test]
    fn crc32_check_value() {
        assert_eq!(hash("123456789"), 0xCBF4_3926);
        assert_eq!(hash(""), 0);
    }

    #[test]
    fn write_then_parse_is_identity() {
        let t: Rst = table();
        let b: Vec<u8> = t.write().unwrap();
        assert_eq!(b.len(), HEAD + 3 * 8 + 2 * 20);
        assert_eq!(Rst::parse(&b).unwrap(), t);
        assert_eq!(Rst::parse(&b).unwrap().write().unwrap(), b);
    }

    #[test]
    fn lookups_strip_zs_and_prefer_names() {
        let t: Rst = table();
        assert_eq!(t.get("a.bin"), Some(10));
        assert_eq!(t.get("/a.bin.zs"), Some(10));
        assert_eq!(t.get("x/two.bin.zs"), Some(6));
        assert_eq!(t.get("missing.bin"), None);
    }

    #[test]
    fn set_size_updates_and_inserts_in_order() {
        let mut t: Rst = table();
        assert_eq!(t.set_size("b.bin", 99).unwrap(), Some(20));
        assert_eq!(t.get("b.bin"), Some(99));
        assert_eq!(t.set_size("x/one.bin", 77).unwrap(), Some(5));
        assert_eq!(t.set_size("d.bin.zs", 1).unwrap(), None);
        assert_eq!(t.get("d.bin"), Some(1));
        assert!(t.sorted().0);
        assert_eq!(t.crc.len(), 4);
        assert!(t.set_size("", 1).is_err());
    }

    #[test]
    fn grow_adds_aligned_growth_and_margin() {
        let mut t: Rst = table();
        assert_eq!(t.grow("a.bin", 100, 100, 64).unwrap(), Some(10));
        assert_eq!(t.grow("a.bin", 100, 90, 64).unwrap(), Some(10));
        assert_eq!(t.grow("a.bin", 100, 200, 64).unwrap(), Some(10 + 224 - 128 + 64));
        assert_eq!(t.grow("nope.bin", 1, 2, 0).unwrap(), None);
    }

    #[test]
    fn absorb_keeps_the_larger_size_and_inserts_a_missing_hash() {
        let mut a: Rst = table();
        let mut b: Rst = table();
        let had: u32 = a.get("a.bin").unwrap();
        b.set_size("a.bin", had + 50).unwrap();
        b.set_size("fresh.bin", 7).unwrap();
        assert_eq!(a.absorb(&b), 2);
        assert_eq!(a.get("a.bin"), Some(had + 50));
        assert_eq!(a.get("fresh.bin"), Some(7));
        let (sorted, _): (bool, bool) = a.sorted();
        assert!(sorted);
        assert_eq!(a.absorb(&b), 0);
    }

    #[test]
    fn fold_reads_nested_enabled_mods_and_names_a_table_that_would_win() {
        let d: PB = std::env::temp_dir().join(format!("ootrr_fold_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let put = |dir: &str, t: &Rst| {
            let p: PB = d.join(dir).join("romfs").join(TABLE_PATH);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, t.to_zs().unwrap()).unwrap();
        };
        let mut a: Rst = table();
        a.set_size("from_a.bin", 11).unwrap();
        put("00-coop", &a);
        let mut b: Rst = table();
        b.set_size("from_b.bin", 12).unwrap();
        put("contents/0400486024DEC000/hd-textures", &b);
        let mut z: Rst = table();
        z.set_size("from_z.bin", 14).unwrap();
        put("zz-late", &z);
        let mut off: Rst = table();
        off.set_size("from_off.bin", 13).unwrap();
        put("_old", &off);
        put("ootrr-seed", &off);
        std::fs::create_dir_all(d.join("broken").join("romfs").join("System").join("Resource")).unwrap();
        std::fs::write(d.join("broken").join("romfs").join(TABLE_PATH), b"not zstd").unwrap();
        let mut t: Rst = table();
        let f: Fold = fold_mods(&mut t, &d, "ootrr-seed").unwrap();
        assert_eq!(t.get("from_a.bin"), Some(11));
        assert_eq!(t.get("from_b.bin"), Some(12));
        assert_eq!(t.get("from_z.bin"), Some(14));
        assert_eq!(t.get("from_off.bin"), None);
        assert_eq!(f.grew, 3);
        assert_eq!(f.unread, vec!["broken".to_string()]);
        assert_eq!(f.later, vec!["zz-late".to_string()]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn bad_input_is_an_error() {
        assert!(Rst::parse(b"").is_err());
        assert!(Rst::parse(b"RESTBX000000000000000000").is_err());
        let mut b: Vec<u8> = table().write().unwrap();
        b.push(0);
        assert!(Rst::parse(&b).is_err());
        b.truncate(b.len() - 5);
        assert!(Rst::parse(&b).is_err());
    }

    #[test]
    fn align_rounds_up_to_32() {
        assert_eq!(align(0), 0);
        assert_eq!(align(1), 32);
        assert_eq!(align(32), 32);
        assert_eq!(align(33), 64);
    }
}
