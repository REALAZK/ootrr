use crate::error::{Error, Res};

const HEAD: &[u8; 5] = b"IPS32";
const TAIL: &[u8; 4] = b"EEOF";
const MAX_REC: usize = 0xFFFF;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rec {
    pub off: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ips {
    pub recs: Vec<Rec>,
}

impl Ips {
    pub fn new() -> Ips {
        Ips::default()
    }

    pub fn add(&mut self, off: u32, data: &[u8]) -> Res<()> {
        for (i, c) in data.chunks(MAX_REC).enumerate() {
            let o: u32 = off
                .checked_add((i * MAX_REC) as u32)
                .ok_or(Error::Format("offset overflow"))?;
            if &o.to_be_bytes() == TAIL {
                return Err(Error::Format("offset collides with end marker"));
            }
            self.recs.push(Rec { off: o, data: c.to_vec() });
        }
        Ok(())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(HEAD);
        for r in &self.recs {
            out.extend_from_slice(&r.off.to_be_bytes());
            out.extend_from_slice(&(r.data.len() as u16).to_be_bytes());
            out.extend_from_slice(&r.data);
        }
        out.extend_from_slice(TAIL);
        out
    }

    pub fn from_bytes(b: &[u8]) -> Res<Ips> {
        if b.len() < HEAD.len() + TAIL.len() || &b[..HEAD.len()] != HEAD {
            return Err(Error::Format("missing IPS32 header"));
        }
        let mut p: usize = HEAD.len();
        let mut ips: Ips = Ips::new();
        loop {
            let t: &[u8] = b.get(p..p + 4).ok_or(Error::Format("truncated patch"))?;
            if t == TAIL {
                return Ok(ips);
            }
            let off: u32 = u32::from_be_bytes([t[0], t[1], t[2], t[3]]);
            p += 4;
            let s: &[u8] = b.get(p..p + 2).ok_or(Error::Format("truncated patch"))?;
            let n: usize = u16::from_be_bytes([s[0], s[1]]) as usize;
            p += 2;
            if n == 0 {
                let r: &[u8] = b.get(p..p + 3).ok_or(Error::Format("truncated patch"))?;
                let cnt: usize = u16::from_be_bytes([r[0], r[1]]) as usize;
                p += 3;
                ips.recs.push(Rec { off, data: vec![r[2]; cnt] });
            } else {
                let d: &[u8] = b.get(p..p + n).ok_or(Error::Format("truncated patch"))?;
                p += n;
                ips.recs.push(Rec { off, data: d.to_vec() });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut a: Ips = Ips::new();
        a.add(0x1000, &[1, 2, 3]).unwrap();
        a.add(0x2000, &[9]).unwrap();
        let b: Ips = Ips::from_bytes(&a.to_bytes()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn big_data_splits() {
        let mut a: Ips = Ips::new();
        a.add(0x10, &vec![5u8; 70_000]).unwrap();
        assert_eq!(a.recs.len(), 2);
        assert_eq!(a.recs[1].off, 0x10 + 0xFFFF);
        let b: Ips = Ips::from_bytes(&a.to_bytes()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn end_marker_offset_rejected() {
        let mut a: Ips = Ips::new();
        assert!(a.add(0x4545_4F46, &[1]).is_err());
    }

    #[test]
    fn rle_record_reads() {
        let mut b: Vec<u8> = b"IPS32".to_vec();
        b.extend_from_slice(&0x20u32.to_be_bytes());
        b.extend_from_slice(&0u16.to_be_bytes());
        b.extend_from_slice(&4u16.to_be_bytes());
        b.push(0xAB);
        b.extend_from_slice(b"EEOF");
        let a: Ips = Ips::from_bytes(&b).unwrap();
        assert_eq!(a.recs[0], Rec { off: 0x20, data: vec![0xAB; 4] });
    }

    #[test]
    fn truncated_is_error() {
        assert!(Ips::from_bytes(b"IPS32\x00\x00").is_err());
        assert!(Ips::from_bytes(b"nope").is_err());
    }
}
