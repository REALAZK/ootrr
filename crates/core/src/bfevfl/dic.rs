use super::rd::Rd;
use super::wr::{n16, n32, Wr};
use crate::error::{Error, Res};

struct Node {
    key: Vec<u8>,
    bit: i64,
    ch: [usize; 2],
}

pub fn read(r: &Rd, o: usize) -> Res<Vec<String>> {
    if r.bytes(o, 4)? != b"DIC " {
        return Err(Error::Format("bfevfl: missing DIC magic"));
    }
    let n: usize = r.u32(o + 4)? as usize;
    let mut out: Vec<String> = Vec::new();
    for i in 1..=n {
        out.push(r.sref(o + 8 + 16 * i + 8)?);
    }
    Ok(out)
}

fn byte_from_end(k: &[u8], j: usize) -> u8 {
    if j < k.len() { k[k.len() - 1 - j] } else { 0 }
}

fn bit(k: &[u8], i: i64) -> usize {
    if i < 0 {
        return 0;
    }
    ((byte_from_end(k, (i >> 3) as usize) >> (i & 7)) & 1) as usize
}

fn first_diff(a: &[u8], b: &[u8]) -> Option<i64> {
    for j in 0..a.len().max(b.len()) {
        let x: u8 = byte_from_end(a, j) ^ byte_from_end(b, j);
        if x != 0 {
            return Some(j as i64 * 8 + x.trailing_zeros() as i64);
        }
    }
    None
}

pub fn tree(keys: &[String]) -> Res<Vec<(u32, usize, usize)>> {
    let mut v: Vec<Node> = vec![Node { key: Vec::new(), bit: -1, ch: [0, 0] }];
    for k in keys {
        let kb: Vec<u8> = k.as_bytes().to_vec();
        let mut p: usize = 0;
        let mut x: usize = v[0].ch[0];
        while v[p].bit < v[x].bit {
            p = x;
            x = v[x].ch[bit(&kb, v[x].bit)];
        }
        let b: i64 = first_diff(&kb, &v[x].key)
            .ok_or(Error::Format("bfevfl: duplicate dictionary key"))?;
        p = 0;
        x = v[0].ch[0];
        while v[p].bit < v[x].bit && v[x].bit < b {
            p = x;
            x = v[x].ch[bit(&kb, v[x].bit)];
        }
        let id: usize = v.len();
        let d: usize = bit(&kb, b);
        let mut ch: [usize; 2] = [0, 0];
        ch[d] = id;
        ch[1 - d] = x;
        let dir: usize = bit(&kb, v[p].bit);
        v.push(Node { key: kb, bit: b, ch });
        v[p].ch[dir] = id;
    }
    Ok(v.iter()
        .map(|n| (if n.bit < 0 { 0xFFFF_FFFF } else { n.bit as u32 }, n.ch[0], n.ch[1]))
        .collect())
}

pub fn write(w: &mut Wr, keys: &[String]) -> Res<()> {
    let t: Vec<(u32, usize, usize)> = tree(keys)?;
    w.raw(b"DIC ");
    w.u32(n32(keys.len())?);
    for (i, (b, l, r)) in t.iter().enumerate() {
        w.u32(*b);
        w.u16(n16(*l)?);
        w.u16(n16(*r)?);
        w.sref(if i == 0 { "" } else { &keys[i - 1] });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn empty_dictionary_is_only_the_root() {
        assert_eq!(tree(&[]).unwrap(), vec![(0xFFFF_FFFF, 0, 0)]);
    }

    #[test]
    fn single_key_matches_the_game_encoding() {
        assert_eq!(tree(&keys(&["Talk"])).unwrap(), vec![(0xFFFF_FFFF, 1, 0), (0, 0, 1)]);
    }

    #[test]
    fn two_keys_match_the_game_encoding() {
        let t: Vec<(u32, usize, usize)> = tree(&keys(&["Animal_Cow", "ActorCawName"])).unwrap();
        assert_eq!(t, vec![(0xFFFF_FFFF, 1, 0), (0, 0, 2), (1, 2, 1)]);
    }

    #[test]
    fn duplicate_keys_are_rejected() {
        assert!(tree(&keys(&["a", "a"])).is_err());
    }
}
