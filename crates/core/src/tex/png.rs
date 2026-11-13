use crate::tex::image::Img;

pub const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
const STORED_MAX: usize = 65535;

pub fn crc32(b: &[u8]) -> u32 {
    let mut c: u32 = 0xffff_ffff;
    for x in b {
        c ^= *x as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

pub fn adler32(b: &[u8]) -> u32 {
    let mut s1: u32 = 1;
    let mut s2: u32 = 0;
    for part in b.chunks(5000) {
        for x in part {
            s1 += *x as u32;
            s2 += s1;
        }
        s1 %= 65521;
        s2 %= 65521;
    }
    (s2 << 16) | s1
}

fn chunk(out: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let at: usize = out.len();
    out.extend_from_slice(tag);
    out.extend_from_slice(data);
    let c: u32 = crc32(&out[at..]);
    out.extend_from_slice(&c.to_be_bytes());
}

fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    let mut z: Vec<u8> = Vec::with_capacity(raw.len() + raw.len() / STORED_MAX * 5 + 16);
    z.extend_from_slice(&[0x78, 0x01]);
    let n: usize = raw.len().div_ceil(STORED_MAX).max(1);
    for i in 0..n {
        let part: &[u8] = &raw[i * STORED_MAX..((i + 1) * STORED_MAX).min(raw.len())];
        z.push(if i + 1 == n { 1 } else { 0 });
        let len: u16 = part.len() as u16;
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(part);
    }
    z.extend_from_slice(&adler32(raw).to_be_bytes());
    z
}

pub fn encode(im: &Img) -> Vec<u8> {
    let mut raw: Vec<u8> = Vec::with_capacity((im.w * 4 + 1) * im.h);
    for y in 0..im.h {
        raw.push(0);
        raw.extend_from_slice(&im.px[y * im.w * 4..(y + 1) * im.w * 4]);
    }
    let mut head: Vec<u8> = Vec::with_capacity(13);
    head.extend_from_slice(&(im.w as u32).to_be_bytes());
    head.extend_from_slice(&(im.h as u32).to_be_bytes());
    head.extend_from_slice(&[8, 6, 0, 0, 0]);
    let mut out: Vec<u8> = Vec::with_capacity(raw.len() + 128);
    out.extend_from_slice(&SIG);
    chunk(&mut out, b"IHDR", &head);
    chunk(&mut out, b"IDAT", &zlib_stored(&raw));
    chunk(&mut out, b"IEND", &[]);
    out
}

pub fn looks_ok(b: &[u8]) -> bool {
    b.len() > 44 && b[..8] == SIG && &b[12..16] == b"IHDR" && b[b.len() - 8..b.len() - 4] == *b"IEND"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunks(b: &[u8]) -> Vec<(String, Vec<u8>)> {
        assert_eq!(&b[..8], &SIG);
        let mut at: usize = 8;
        let mut out: Vec<(String, Vec<u8>)> = Vec::new();
        while at < b.len() {
            let n: usize = u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]) as usize;
            let tag: String = String::from_utf8_lossy(&b[at + 4..at + 8]).into_owned();
            let data: Vec<u8> = b[at + 8..at + 8 + n].to_vec();
            let c: u32 = u32::from_be_bytes([b[at + 8 + n], b[at + 9 + n], b[at + 10 + n], b[at + 11 + n]]);
            assert_eq!(c, crc32(&b[at + 4..at + 8 + n]), "{tag}");
            out.push((tag, data));
            at += 12 + n;
        }
        out
    }

    fn inflate_stored(z: &[u8]) -> Vec<u8> {
        assert_eq!(&z[..2], &[0x78, 0x01]);
        assert_eq!(u16::from_be_bytes([z[0], z[1]]) % 31, 0);
        let mut at: usize = 2;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let last: bool = z[at] == 1;
            let n: u16 = u16::from_le_bytes([z[at + 1], z[at + 2]]);
            let inv: u16 = u16::from_le_bytes([z[at + 3], z[at + 4]]);
            assert_eq!(n, !inv);
            out.extend_from_slice(&z[at + 5..at + 5 + n as usize]);
            at += 5 + n as usize;
            if last {
                break;
            }
        }
        assert_eq!(&z[at..], &adler32(&out).to_be_bytes());
        out
    }

    #[test]
    fn the_check_sums_match_known_values() {
        assert_eq!(crc32(b"IEND"), 0xae42_6082);
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
        assert_eq!(adler32(b""), 1);
    }

    #[test]
    fn a_small_image_round_trips_through_the_chunks() {
        let mut im: Img = Img::new(3, 2);
        for (i, v) in im.px.iter_mut().enumerate() {
            *v = (i * 7) as u8;
        }
        let b: Vec<u8> = encode(&im);
        assert!(looks_ok(&b));
        let cs: Vec<(String, Vec<u8>)> = chunks(&b);
        assert_eq!(cs.iter().map(|c| c.0.as_str()).collect::<Vec<&str>>(), vec!["IHDR", "IDAT", "IEND"]);
        assert_eq!(cs[0].1, vec![0, 0, 0, 3, 0, 0, 0, 2, 8, 6, 0, 0, 0]);
        let raw: Vec<u8> = inflate_stored(&cs[1].1);
        assert_eq!(raw.len(), 2 * 13);
        assert_eq!(raw[0], 0);
        assert_eq!(&raw[1..13], &im.px[..12]);
        assert_eq!(raw[13], 0);
        assert_eq!(&raw[14..], &im.px[12..]);
    }

    #[test]
    fn a_large_image_is_split_into_stored_blocks() {
        let mut im: Img = Img::new(200, 120);
        for (i, v) in im.px.iter_mut().enumerate() {
            *v = (i % 251) as u8;
        }
        let b: Vec<u8> = encode(&im);
        let cs: Vec<(String, Vec<u8>)> = chunks(&b);
        let raw: Vec<u8> = inflate_stored(&cs[1].1);
        assert_eq!(raw.len(), 120 * 801);
        assert!(raw.len() > STORED_MAX);
        assert_eq!(&raw[801 * 119 + 1..], &im.px[200 * 4 * 119..]);
    }

    #[test]
    fn an_empty_image_still_has_one_final_block() {
        let b: Vec<u8> = encode(&Img::new(0, 0));
        let cs: Vec<(String, Vec<u8>)> = chunks(&b);
        assert!(inflate_stored(&cs[1].1).is_empty());
        assert!(!looks_ok(b"nope"));
    }
}
