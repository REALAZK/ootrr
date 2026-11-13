use crate::error::{Error, Res};
use crate::murmur::murmur3_32;
use crate::save::model::{HEAD, MAGIC, SENTINEL};

pub const POS_HASH: u32 = 0x8773_f02d;
pub const BANC_SHORT: u32 = 0x4a6b_db7d;

#[derive(Debug, Clone, Copy)]
pub struct Lim {
    pub min: usize,
    pub max: usize,
}

impl Lim {
    pub const SAVE: Lim = Lim { min: 100 * 1024, max: 400 * 1024 };
    pub const ANY: Lim = Lim { min: 0, max: usize::MAX };
}

#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    pub map: Option<String>,
    pub child: bool,
    pub room: Option<i32>,
    pub pos: Option<[f32; 3]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Peek {
    pub rows: Vec<(u32, u32)>,
    pub keys: Vec<u64>,
    pub len: usize,
    pub place: Place,
}

fn fail(m: &'static str) -> Error {
    Error::Format(m)
}

fn w32(b: &[u8], o: usize) -> Res<u32> {
    let s: &[u8] = b.get(o..o + 4).ok_or(fail("save: the file ends inside the row table"))?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn w64(b: &[u8], o: usize) -> Option<u64> {
    let s: &[u8] = b.get(o..o + 8)?;
    Some(u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]))
}

fn table(b: &[u8]) -> Res<(Vec<(u32, u32)>, usize, usize)> {
    let mut at: usize = HEAD;
    let mut rows: Vec<(u32, u32)> = Vec::new();
    loop {
        let h: u32 = w32(b, at)?;
        let v: u32 = w32(b, at + 4)?;
        at += 8;
        if h == SENTINEL {
            return Ok((rows, at, v as usize));
        }
        if h != 0 {
            rows.push((h, v));
        }
    }
}

fn key_list(b: &[u8], from: usize) -> Res<Vec<u64>> {
    let mut o: usize = from;
    let mut out: Vec<u64> = Vec::new();
    loop {
        let k: u64 = w64(b, o).ok_or(fail("save: the key list has no end"))?;
        o += 8;
        if k == 0 {
            break;
        }
        out.push(k);
    }
    if o != b.len() {
        return Err(fail("save: bytes follow the key list"));
    }
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

fn sort_rows(mut rows: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    let n: usize = rows.len();
    let mut tagged: Vec<(u32, usize, u32)> = rows.drain(..).enumerate().map(|(i, (h, v))| (h, i, v)).collect();
    tagged.sort_unstable_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
    let mut out: Vec<(u32, u32)> = Vec::with_capacity(n);
    for (h, _, v) in tagged {
        match out.last_mut() {
            Some(l) if l.0 == h => l.1 = v,
            _ => out.push((h, v)),
        }
    }
    out
}

fn text_at(b: &[u8], o: usize) -> Option<String> {
    let s: &[u8] = b.get(o..)?;
    let e: usize = s.iter().position(|c| *c == 0).unwrap_or(s.len());
    let t: String = String::from_utf8_lossy(&s[..e]).into_owned();
    if t.is_empty() { None } else { Some(t) }
}

fn vec3_at(b: &[u8], o: usize) -> Option<[f32; 3]> {
    let s: &[u8] = b.get(o..o + 12)?;
    let f = |i: usize| f32::from_le_bytes([s[i], s[i + 1], s[i + 2], s[i + 3]]);
    Some([f(0), f(4), f(8)])
}

impl Peek {
    pub fn val(&self, h: u32) -> Option<u32> {
        self.rows.binary_search_by(|r| r.0.cmp(&h)).ok().map(|i| self.rows[i].1)
    }

    pub fn on(&self, h: u32) -> bool {
        self.val(h).unwrap_or(0) != 0
    }

    pub fn has_key(&self, k: u64) -> bool {
        self.keys.binary_search(&k).is_ok()
    }
}

fn place(b: &[u8], p: &Peek) -> Place {
    let h = |n: &str| murmur3_32(n.as_bytes(), 0);
    let map: Option<String> = p
        .val(h("Sequence_CurrentBanc"))
        .and_then(|o| text_at(b, o as usize))
        .or_else(|| p.val(BANC_SHORT).and_then(|o| text_at(b, o as usize)));
    Place {
        map,
        child: p.on(h("Sequence_IsChildhood")),
        room: p.val(h("DungeonRoom")).map(|v| v as i32),
        pos: p.val(POS_HASH).and_then(|o| vec3_at(b, o as usize)),
    }
}

pub fn peek_with(b: &[u8], lim: &Lim) -> Res<Peek> {
    if b.len() < lim.min || b.len() > lim.max {
        return Err(fail("save: the file size is outside the expected range"));
    }
    if b.len() < HEAD + 8 || w32(b, 0)? != MAGIC {
        return Err(fail("save: bad magic"));
    }
    let (rows, end, keys_at): (Vec<(u32, u32)>, usize, usize) = table(b)?;
    if w32(b, 8)? as usize != end {
        return Err(fail("save: the data offset is not the end of the row table"));
    }
    if keys_at < end || keys_at > b.len() {
        return Err(fail("save: the key list offset is outside the file"));
    }
    let keys: Vec<u64> = key_list(b, keys_at)?;
    let mut p: Peek = Peek { rows: sort_rows(rows), keys, len: b.len(), place: Place { map: None, child: false, room: None, pos: None } };
    p.place = place(b, &p);
    Ok(p)
}

pub fn peek(b: &[u8]) -> Res<Peek> {
    peek_with(b, &Lim::SAVE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::fixture::{Cell, h, raw, sample};

    #[test]
    fn reads_the_sample_rows_keys_and_place() {
        let p: Peek = peek_with(&sample(), &Lim::ANY).unwrap();
        assert_eq!(p.keys, vec![3, 9, 27]);
        assert!(p.on(h("IsGet.Test_A")));
        assert!(!p.on(h("IsGet.Test_B")));
        assert_eq!(p.val(h("PouchData.Test_A.StockNum")), Some(7));
        assert_eq!(p.place.map.as_deref(), Some("KokiriForest_Child"));
        assert!(p.rows.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(p.rows.iter().all(|r| r.0 != 0));
    }

    #[test]
    fn the_real_size_limit_refuses_tiny_files() {
        assert!(peek(&sample()).is_err());
    }

    #[test]
    fn a_truncated_file_is_refused() {
        let b: Vec<u8> = sample();
        for cut in [b.len() - 1, b.len() - 8, b.len() - 9, b.len() / 2, 40, 12, 3, 0] {
            assert!(peek_with(&b[..cut], &Lim::ANY).is_err(), "{cut}");
        }
    }

    #[test]
    fn a_file_without_the_sentinel_is_refused() {
        let mut b: Vec<u8> = sample();
        let s: [u8; 4] = SENTINEL.to_le_bytes();
        let at: usize = b.windows(4).position(|w| w == s).unwrap();
        b[at] ^= 0xff;
        assert!(peek_with(&b, &Lim::ANY).is_err());
    }

    #[test]
    fn a_key_list_without_its_end_is_refused() {
        let mut b: Vec<u8> = raw(&[(h("IsGet.Test_A"), Cell::Inline(1))], &[5, 6]);
        let n: usize = b.len();
        b.truncate(n - 8);
        assert!(peek_with(&b, &Lim::ANY).is_err());
        let mut c: Vec<u8> = raw(&[(h("IsGet.Test_A"), Cell::Inline(1))], &[5, 6]);
        c.extend([1u8, 0, 0, 0]);
        assert!(peek_with(&c, &Lim::ANY).is_err());
    }

    #[test]
    fn a_wrong_data_offset_is_refused() {
        let mut b: Vec<u8> = sample();
        b[8] = b[8].wrapping_add(8);
        assert!(peek_with(&b, &Lim::ANY).is_err());
    }

    #[test]
    fn duplicate_rows_keep_the_last_value() {
        let b: Vec<u8> = raw(&[(7, Cell::Inline(1)), (7, Cell::Inline(0)), (3, Cell::Inline(2))], &[]);
        let p: Peek = peek_with(&b, &Lim::ANY).unwrap();
        assert_eq!(p.rows, vec![(3, 2), (7, 0)]);
    }
}
