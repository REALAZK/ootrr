use std::collections::BTreeMap as Map;

use crate::error::{Error, Res};

pub struct Wr {
    pub b: Vec<u8>,
    pub slots: Vec<usize>,
    refs: Vec<(usize, String, bool)>,
}

pub fn n32(n: usize) -> Res<u32> {
    u32::try_from(n).map_err(|_| Error::Format("bfevfl: value does not fit in 32 bits"))
}

pub fn n16(n: usize) -> Res<u16> {
    u16::try_from(n).map_err(|_| Error::Format("bfevfl: count does not fit in 16 bits"))
}

fn skey(s: &str) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::with_capacity(s.len() * 8);
    for c in s.as_bytes().iter().rev() {
        for i in 0..8 {
            v.push((c >> i) & 1);
        }
    }
    while v.last() == Some(&0) {
        v.pop();
    }
    if v.is_empty() {
        v.push(0);
    }
    v
}

impl Wr {
    pub fn new() -> Wr {
        Wr { b: Vec::new(), slots: Vec::new(), refs: Vec::new() }
    }

    pub fn pos(&self) -> usize {
        self.b.len()
    }

    pub fn align(&mut self, n: usize) {
        while !self.b.len().is_multiple_of(n) {
            self.b.push(0);
        }
    }

    pub fn zeros(&mut self, n: usize) {
        self.b.resize(self.b.len() + n, 0);
    }

    pub fn raw(&mut self, d: &[u8]) {
        self.b.extend_from_slice(d);
    }

    pub fn u8(&mut self, v: u8) {
        self.b.push(v);
    }

    pub fn u16(&mut self, v: u16) {
        self.b.extend_from_slice(&v.to_le_bytes());
    }

    pub fn u32(&mut self, v: u32) {
        self.b.extend_from_slice(&v.to_le_bytes());
    }

    pub fn u64(&mut self, v: u64) {
        self.b.extend_from_slice(&v.to_le_bytes());
    }

    pub fn f32(&mut self, v: f32) {
        self.u32(v.to_bits());
    }

    pub fn set16(&mut self, at: usize, v: u16) {
        self.b[at..at + 2].copy_from_slice(&v.to_le_bytes());
    }

    pub fn set32(&mut self, at: usize, v: u32) {
        self.b[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }

    pub fn ptr(&mut self, reg: bool) -> usize {
        let s: usize = self.pos();
        self.u64(0);
        if reg {
            self.slots.push(s);
        }
        s
    }

    pub fn set_ptr(&mut self, slot: usize, target: usize) {
        self.b[slot..slot + 8].copy_from_slice(&(target as u64).to_le_bytes());
    }

    pub fn here(&mut self, slot: usize) {
        let p: usize = self.pos();
        self.set_ptr(slot, p);
    }

    pub fn sref(&mut self, s: &str) {
        let at: usize = self.ptr(true);
        self.refs.push((at, s.to_string(), false));
    }

    pub fn href(&mut self, s: &str) {
        let at: usize = self.pos();
        self.u32(0);
        self.refs.push((at, s.to_string(), true));
    }

    pub fn pstr(&mut self, s: &str) -> Res<()> {
        let n: u16 = n16(s.len())?;
        self.u16(n);
        self.raw(s.as_bytes());
        self.u8(0);
        self.align(2);
        Ok(())
    }

    pub fn pool(&mut self) -> Res<usize> {
        type Slots = Map<(Vec<u8>, String), Vec<(usize, bool)>>;
        let mut m: Slots = Map::new();
        m.entry((skey(""), String::new())).or_default();
        for (at, s, h) in &self.refs {
            m.entry((skey(s), s.clone())).or_default().push((*at, *h));
        }
        let start: usize = self.pos();
        self.raw(b"STR ");
        self.u32(0);
        self.u64(0);
        self.u32(n32(m.len() - 1)?);
        for ((_, s), v) in &m {
            let off: usize = self.pos();
            for (at, h) in v {
                if *h {
                    self.set32(*at, n32(off + 2)?);
                } else {
                    self.b[*at..*at + 8].copy_from_slice(&(off as u64).to_le_bytes());
                }
            }
            self.pstr(s)?;
        }
        Ok(start)
    }

    pub fn reloc(&mut self, data_end: usize) -> Res<usize> {
        self.align(8);
        let t: usize = self.pos();
        let mut s: Vec<usize> = self.slots.clone();
        s.sort_unstable();
        s.dedup();
        let mut e: Vec<(u32, u32)> = Vec::new();
        let mut i: usize = 0;
        while i < s.len() {
            let base: usize = s[i];
            let mut bits: u32 = 0;
            while i < s.len() && s[i] < base + 256 {
                if !(s[i] - base).is_multiple_of(8) {
                    return Err(Error::Format("bfevfl: unaligned pointer slot"));
                }
                bits |= 1 << ((s[i] - base) / 8);
                i += 1;
            }
            e.push((n32(base)?, bits));
        }
        self.raw(b"RELT");
        self.u32(n32(t)?);
        self.u32(1);
        self.u32(0);
        self.u64(0);
        self.u32(0);
        self.u32(n32(data_end)?);
        self.u32(0);
        self.u32(n32(e.len())?);
        for (o, bits) in e {
            self.u32(o);
            self.u32(bits);
        }
        Ok(t)
    }
}

impl Default for Wr {
    fn default() -> Wr {
        Wr::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(b: &[u8], at: usize) -> u32 {
        u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
    }

    #[test]
    fn string_pool_sorts_by_reversed_bits() {
        assert!(skey("") < skey("a"));
        let mut v: Vec<&str> = vec!["Talk", "Event0", "IsWorld"];
        v.sort_by_key(|s| skey(s));
        assert_eq!(v[0], "Event0");
    }

    #[test]
    fn pascal_strings_pad_to_two_bytes() {
        let mut w: Wr = Wr::new();
        w.pstr("abc").unwrap();
        assert_eq!(w.b, vec![3, 0, b'a', b'b', b'c', 0]);
        w.pstr("").unwrap();
        assert_eq!(w.b.len(), 10);
    }

    #[test]
    fn relocation_entries_group_slots_in_windows_of_32() {
        let mut w: Wr = Wr::new();
        w.zeros(0x400);
        w.slots = vec![0x28, 0x30, 0x128, 0x230];
        let t: usize = w.reloc(0x400).unwrap();
        assert_eq!(word(&w.b, t + 0x28), 0x28);
        assert_eq!(word(&w.b, t + 0x2C), 0b11);
        assert_eq!(word(&w.b, t + 0x30), 0x128);
        assert_eq!(word(&w.b, t + 0x38), 0x230);
    }
}
