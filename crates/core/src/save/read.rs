use super::kind::{Kind, seg_len, word};
use super::model::{HEAD, MAGIC, Row, SENTINEL, Save};
use crate::error::{Error, Res};
use crate::flags::{FlagList, KEYED};

fn table(b: &[u8]) -> Res<(Vec<(u32, u32)>, usize, usize)> {
    let mut at: usize = HEAD;
    let mut meta: Vec<(u32, u32)> = Vec::new();
    loop {
        let h: u32 = word(b, at)? as u32;
        let v: u32 = word(b, at + 4)? as u32;
        at += 8;
        if h == SENTINEL {
            return Ok((meta, at, v as usize));
        }
        meta.push((h, v));
    }
}

fn keys_at(b: &[u8], from: usize) -> Res<(Vec<u64>, usize)> {
    let mut keys: Vec<u64> = Vec::new();
    let mut o: usize = from;
    loop {
        let s: &[u8] = b
            .get(o..o + 8)
            .ok_or(Error::Format("save: key list has no terminator"))?;
        let k: u64 = u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]);
        o += 8;
        if k == 0 {
            return Ok((keys, o));
        }
        keys.push(k);
    }
}

pub fn parse(b: &[u8], l: &FlagList) -> Res<Save> {
    if b.len() < HEAD + 8 || word(b, 0)? != MAGIC as usize {
        return Err(Error::Format("save: bad magic"));
    }
    let (meta, start, gpos): (Vec<(u32, u32)>, usize, usize) = table(b)?;
    if word(b, 8)? != start || gpos < start || gpos > b.len() {
        return Err(Error::Format(
            "save: header offsets disagree with the table",
        ));
    }
    let mut rows: Vec<Row> = Vec::with_capacity(meta.len());
    let mut pos: usize = start;
    for (h, v) in meta {
        if h == 0 {
            rows.push(Row {
                hash: 0,
                val: v,
                kind: Kind::Sep,
                seg: Vec::new(),
            });
            continue;
        }
        let f = l
            .by_hash(h as u64)
            .filter(|f| f.ty != KEYED)
            .ok_or(Error::Format("save: hash is not in the flag list"))?;
        let k: Kind = Kind::of(&f.ty).ok_or(Error::Format("save: flag type has no layout"))?;
        if k.inline() {
            rows.push(Row {
                hash: h,
                val: v,
                kind: k,
                seg: Vec::new(),
            });
            continue;
        }
        if v as usize != pos {
            return Err(Error::Format("save: data segments are not sequential"));
        }
        let n: usize = seg_len(k, &b[pos..gpos])?;
        rows.push(Row {
            hash: h,
            val: 0,
            kind: k,
            seg: b[pos..pos + n].to_vec(),
        });
        pos += n;
    }
    if pos != gpos {
        return Err(Error::Format(
            "save: data segments do not fill the data area",
        ));
    }
    let (keys, end): (Vec<u64>, usize) = keys_at(b, gpos)?;
    Ok(Save::new(b[..HEAD].to_vec(), rows, keys, b[end..].to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::fixture::{Cell, flag_list, h, raw, sample};

    #[test]
    fn parses_the_sample() {
        let s: Save = parse(&sample(), &flag_list()).unwrap();
        assert_eq!(s.rows.len(), 13);
        assert_eq!(s.keys, vec![3, 9, 27]);
        assert_eq!(s.version(), 0x2d80eb);
        assert_eq!(s.rows.iter().filter(|r| r.kind == Kind::Sep).count(), 2);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut b: Vec<u8> = sample();
        b[0] = 9;
        assert!(parse(&b, &flag_list()).is_err());
    }

    #[test]
    fn rejects_unknown_hashes() {
        let b: Vec<u8> = raw(&[(0x1234_5678, Cell::Inline(1))], &[]);
        assert!(parse(&b, &flag_list()).is_err());
    }

    #[test]
    fn rejects_scrambled_segments() {
        let mut s: Vec<u8> = raw(
            &[
                (h("Test_Big"), Cell::Seg(vec![0; 8])),
                (h("Test_Arr"), Cell::Seg(vec![0; 4])),
            ],
            &[],
        );
        let o: usize = 0x28 + 4;
        let a: u32 = u32::from_le_bytes([s[o], s[o + 1], s[o + 2], s[o + 3]]);
        s[o..o + 4].copy_from_slice(&(a + 4).to_le_bytes());
        assert!(parse(&s, &flag_list()).is_err());
    }

    #[test]
    fn rejects_truncated_files() {
        let b: Vec<u8> = sample();
        assert!(parse(&b[..b.len() - 9], &flag_list()).is_err());
        assert!(parse(&b[..20], &flag_list()).is_err());
    }
}
