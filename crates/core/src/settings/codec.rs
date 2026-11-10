use crate::error::{Error, Res};

const ALPHA: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

pub fn put_varint(out: &mut Vec<u8>, mut n: u64) {
    loop {
        let b: u8 = (n & 0x7F) as u8;
        n >>= 7;
        if n == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

pub fn zig(n: i64) -> u64 {
    ((n << 1) ^ (n >> 63)) as u64
}

pub fn unzig(n: u64) -> i64 {
    ((n >> 1) as i64) ^ -((n & 1) as i64)
}

pub struct Rd<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Rd<'a> {
    pub fn new(b: &'a [u8]) -> Rd<'a> {
        Rd { b, i: 0 }
    }

    pub fn left(&self) -> usize {
        self.b.len() - self.i
    }

    pub fn done(&self) -> bool {
        self.i >= self.b.len()
    }

    pub fn varint(&mut self) -> Res<u64> {
        let mut n: u64 = 0;
        for k in 0..10 {
            let c: u8 = *self.b.get(self.i).ok_or(Error::Format("settings string is truncated"))?;
            self.i += 1;
            if k == 9 && c > 1 {
                return Err(Error::Format("settings string has an oversized number"));
            }
            n |= ((c & 0x7F) as u64) << (7 * k);
            if c & 0x80 == 0 {
                return Ok(n);
            }
        }
        Err(Error::Format("settings string has an oversized number"))
    }

    pub fn take(&mut self, n: usize) -> Res<&'a [u8]> {
        if n > self.left() {
            return Err(Error::Format("settings string is truncated"));
        }
        let s: &[u8] = &self.b[self.i..self.i + n];
        self.i += n;
        Ok(s)
    }
}

pub fn b32_encode(b: &[u8]) -> String {
    let mut out: String = String::with_capacity(b.len() * 8 / 5 + 1);
    let mut acc: u32 = 0;
    let mut n: u32 = 0;
    for x in b {
        acc = (acc << 8) | *x as u32;
        n += 8;
        while n >= 5 {
            n -= 5;
            out.push(ALPHA[((acc >> n) & 31) as usize] as char);
        }
        acc &= (1 << n) - 1;
    }
    if n > 0 {
        out.push(ALPHA[((acc << (5 - n)) & 31) as usize] as char);
    }
    out
}

pub fn b32_decode(t: &str) -> Res<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(t.len() * 5 / 8);
    let mut acc: u32 = 0;
    let mut n: u32 = 0;
    for c in t.chars() {
        if c == '-' || c.is_whitespace() {
            continue;
        }
        let u: char = c.to_ascii_uppercase();
        let v: usize = ALPHA
            .iter()
            .position(|a| *a as char == u)
            .ok_or(Error::Format("settings string has a character outside its alphabet"))?;
        acc = (acc << 5) | v as u32;
        n += 5;
        if n >= 8 {
            n -= 8;
            out.push(((acc >> n) & 0xFF) as u8);
            acc &= (1 << n) - 1;
        }
    }
    if n >= 5 || acc != 0 {
        return Err(Error::Format("settings string has a bad length or stray bits"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base32_known_vectors() {
        assert_eq!(b32_encode(b""), "");
        assert_eq!(b32_encode(b"f"), "MY");
        assert_eq!(b32_encode(b"fo"), "MZXQ");
        assert_eq!(b32_encode(b"foo"), "MZXW6");
        assert_eq!(b32_encode(b"foob"), "MZXW6YQ");
        assert_eq!(b32_encode(b"fooba"), "MZXW6YTB");
        assert_eq!(b32_encode(b"foobar"), "MZXW6YTBOI");
    }

    #[test]
    fn base32_round_trip_every_length() {
        for n in 0..40usize {
            let b: Vec<u8> = (0..n).map(|i| (i * 37 + 11) as u8).collect();
            assert_eq!(b32_decode(&b32_encode(&b)).unwrap(), b);
        }
    }

    #[test]
    fn base32_decoder_is_forgiving_about_case_and_spacing() {
        assert_eq!(b32_decode("mzxw6-ytb oi").unwrap(), b"foobar");
    }

    #[test]
    fn base32_rejects_bad_input() {
        for t in ["0", "1", "M", "MZX", "MZXW6Y", "MZ=", "MZ!Q", "MZ\u{e9}Q", "MX"] {
            assert!(b32_decode(t).is_err(), "accepted {t:?}");
        }
    }

    #[test]
    fn varint_round_trip() {
        for n in [0u64, 1, 127, 128, 300, 16383, 16384, u32::MAX as u64, u64::MAX] {
            let mut b: Vec<u8> = Vec::new();
            put_varint(&mut b, n);
            let mut r: Rd = Rd::new(&b);
            assert_eq!(r.varint().unwrap(), n);
            assert!(r.done());
        }
    }

    #[test]
    fn varint_rejects_bad_input() {
        assert!(Rd::new(&[0x80]).varint().is_err());
        assert!(Rd::new(&[0xFF; 10]).varint().is_err());
        assert!(Rd::new(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x02]).varint().is_err());
        assert!(Rd::new(&[]).varint().is_err());
    }

    #[test]
    fn zigzag_round_trip() {
        for n in [0i64, 1, -1, 2, -2, 1000, -1000, i64::MAX, i64::MIN] {
            assert_eq!(unzig(zig(n)), n);
        }
        assert_eq!(zig(-1), 1);
        assert_eq!(zig(1), 2);
    }

    #[test]
    fn reader_take_is_bounds_checked() {
        let mut r: Rd = Rd::new(&[1, 2, 3]);
        assert_eq!(r.take(2).unwrap(), &[1, 2]);
        assert!(r.take(2).is_err());
        assert_eq!(r.left(), 1);
    }
}
