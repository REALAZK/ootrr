use std::io::Read;

use ruzstd::decoding::{Dictionary, FrameDecoder, StreamingDecoder};

use crate::error::{Error, Res};

const MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];
const BLOCK_MAX: usize = 131072;

pub struct Zs {
    dec: FrameDecoder,
}

impl Zs {
    pub fn new() -> Zs {
        Zs { dec: FrameDecoder::new() }
    }

    pub fn add_dict(&mut self, raw: &[u8]) -> Res<()> {
        let d: Dictionary =
            Dictionary::decode_dict(raw).map_err(|e| Error::Zstd(format!("{e:?}")))?;
        self.dec
            .add_dict(d)
            .map_err(|e| Error::Zstd(format!("{e:?}")))
    }

    pub fn decompress(&mut self, b: &[u8]) -> Res<Vec<u8>> {
        let mut s: StreamingDecoder<&[u8], &mut FrameDecoder> =
            StreamingDecoder::new_with_decoder(b, &mut self.dec)
                .map_err(|e| Error::Zstd(format!("{e:?}")))?;
        let mut out: Vec<u8> = Vec::new();
        s.read_to_end(&mut out)
            .map_err(|e| Error::Zstd(e.to_string()))?;
        Ok(out)
    }
}

impl Default for Zs {
    fn default() -> Zs {
        Zs::new()
    }
}

pub fn compress_raw(b: &[u8], dict: Option<u32>) -> Res<Vec<u8>> {
    let n: u32 = u32::try_from(b.len())
        .map_err(|_| Error::Format("input too large for one zstd frame"))?;
    let (flag, id): (u8, Vec<u8>) = match dict {
        None => (0, Vec::new()),
        Some(i) if i <= 0xFF => (1, vec![i as u8]),
        Some(i) if i <= 0xFFFF => (2, (i as u16).to_le_bytes().to_vec()),
        Some(i) => (3, i.to_le_bytes().to_vec()),
    };
    let mut out: Vec<u8> = Vec::with_capacity(b.len() + 16 + (b.len() / BLOCK_MAX + 1) * 3);
    out.extend_from_slice(&MAGIC);
    out.push(0xA0 | flag);
    out.extend_from_slice(&id);
    out.extend_from_slice(&n.to_le_bytes());
    if b.is_empty() {
        out.extend_from_slice(&[0x01, 0x00, 0x00]);
    }
    let mut it = b.chunks(BLOCK_MAX).peekable();
    while let Some(c) = it.next() {
        let last: u32 = if it.peek().is_none() { 1 } else { 0 };
        let h: u32 = last | ((c.len() as u32) << 3);
        out.extend_from_slice(&h.to_le_bytes()[..3]);
        out.extend_from_slice(c);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(n: usize) -> Vec<u8> {
        let mut x: u32 = 12345;
        (0..n)
            .map(|_| {
                x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                (x >> 24) as u8
            })
            .collect()
    }

    #[test]
    fn raw_frames_round_trip() {
        let mut z: Zs = Zs::new();
        for n in [0usize, 1, 100, 131071, 131072, 131073, 300000] {
            let d: Vec<u8> = sample(n);
            let f: Vec<u8> = compress_raw(&d, None).unwrap();
            assert_eq!(z.decompress(&f).unwrap(), d, "size {n}");
        }
    }

    #[test]
    fn header_carries_the_dictionary_id() {
        let f: Vec<u8> = compress_raw(&[1, 2, 3], Some(3)).unwrap();
        assert_eq!(&f[..4], &MAGIC);
        assert_eq!(f[4], 0xA1);
        assert_eq!(f[5], 3);
        let g: Vec<u8> = compress_raw(&[1, 2, 3], Some(0x1_0000)).unwrap();
        assert_eq!(g[4], 0xA3);
    }

    #[test]
    fn missing_dictionary_is_an_error() {
        let f: Vec<u8> = compress_raw(&[1, 2, 3], Some(3)).unwrap();
        assert!(Zs::new().decompress(&f).is_err());
    }

    #[test]
    fn rle_block_decodes() {
        let mut f: Vec<u8> = MAGIC.to_vec();
        f.extend_from_slice(&[0x20, 0x0A, 0x53, 0x00, 0x00, 0x41]);
        assert_eq!(Zs::new().decompress(&f).unwrap(), vec![0x41; 10]);
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(Zs::new().decompress(b"not zstd at all").is_err());
        assert!(Zs::new().decompress(&[]).is_err());
    }
}
