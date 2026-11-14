use crate::error::{Error, Res};
use crate::tex::astc;
use crate::tex::image::Img;
use crate::tex::swizzle;

pub const ASTC_4X4_UNORM: u32 = 0x2d01;
pub const ASTC_4X4_SRGB: u32 = 0x2d06;
pub const MAX_SIDE: usize = 8192;
const MAX_TEX: usize = 8192;
const MAX_NAME: usize = 256;
const BLOCK_PX: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tex {
    pub name: String,
    pub fmt: u32,
    pub w: usize,
    pub h: usize,
    pub gobs: usize,
    pub off: usize,
    pub size: usize,
    pub comp: [u8; 4],
}

fn bad(m: &'static str) -> Error {
    Error::Format(m)
}

fn u16_at(b: &[u8], o: usize) -> Res<usize> {
    let s: &[u8] = b.get(o..o.saturating_add(2)).ok_or(bad("bntx: the file ends inside a field"))?;
    Ok(u16::from_le_bytes([s[0], s[1]]) as usize)
}

fn u32_at(b: &[u8], o: usize) -> Res<u32> {
    let s: &[u8] = b.get(o..o.saturating_add(4)).ok_or(bad("bntx: the file ends inside a field"))?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn ptr_at(b: &[u8], o: usize) -> Res<usize> {
    let s: &[u8] = b.get(o..o.saturating_add(8)).ok_or(bad("bntx: the file ends inside a pointer"))?;
    let v: u64 = u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]);
    usize::try_from(v).ok().filter(|p| *p < b.len()).ok_or(bad("bntx: a pointer leaves the file"))
}

fn info(b: &[u8], o: usize) -> Res<Tex> {
    if b.get(o..o.saturating_add(4)) != Some(b"BRTI".as_slice()) {
        return Err(bad("bntx: a texture entry has no BRTI tag"));
    }
    let w: usize = u32_at(b, o + 0x24)? as usize;
    let h: usize = u32_at(b, o + 0x28)? as usize;
    if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE {
        return Err(bad("bntx: a texture size is outside the supported range"));
    }
    let layout: u32 = u32_at(b, o + 0x34)?;
    let np: usize = ptr_at(b, o + 0x60)?;
    let n: usize = u16_at(b, np)?;
    if n > MAX_NAME {
        return Err(bad("bntx: a texture name is too long"));
    }
    let raw: &[u8] = b.get(np + 2..np + 2 + n).ok_or(bad("bntx: a texture name leaves the file"))?;
    let mips: usize = ptr_at(b, o + 0x70)?;
    Ok(Tex {
        name: String::from_utf8_lossy(raw).into_owned(),
        fmt: u32_at(b, o + 0x1c)?,
        w,
        h,
        gobs: 1 << (layout & 7),
        off: ptr_at(b, mips)?,
        size: u32_at(b, o + 0x50)? as usize,
        comp: u32_at(b, o + 0x58)?.to_le_bytes(),
    })
}

pub fn parse(b: &[u8]) -> Res<Vec<Tex>> {
    if b.len() < 0x40 || &b[..4] != b"BNTX" {
        return Err(bad("bntx: bad magic"));
    }
    let n: usize = u32_at(b, 0x24)? as usize;
    if n > MAX_TEX {
        return Err(bad("bntx: too many textures"));
    }
    let arr: usize = ptr_at(b, 0x28)?;
    let mut out: Vec<Tex> = Vec::with_capacity(n);
    for i in 0..n {
        out.push(info(b, ptr_at(b, arr + 8 * i)?)?);
    }
    Ok(out)
}

pub fn find<'a>(ts: &'a [Tex], name: &str) -> Option<&'a Tex> {
    ts.iter().find(|t| t.name == name)
}

pub fn supported(fmt: u32) -> bool {
    fmt == ASTC_4X4_UNORM || fmt == ASTC_4X4_SRGB
}

pub fn decode(b: &[u8], t: &Tex) -> Res<(Img, usize)> {
    if !supported(t.fmt) {
        return Err(Error::Msg(format!("bntx: texture {} has format {:#x}, only ASTC 4x4 is read", t.name, t.fmt)));
    }
    let srgb: bool = t.fmt == ASTC_4X4_SRGB;
    let wb: usize = t.w.div_ceil(BLOCK_PX);
    let hb: usize = t.h.div_ceil(BLOCK_PX);
    let data: &[u8] = b.get(t.off..).ok_or(bad("bntx: the texture data starts outside the file"))?;
    let mut im: Img = Img::new(t.w, t.h);
    let mut px: [[u8; 4]; 16] = [[0; 4]; 16];
    let mut failed: usize = 0;
    for by in 0..hb {
        for bx in 0..wb {
            let a: usize = swizzle::addr(bx, by, wb, t.gobs);
            let s: &[u8] = data.get(a..a + swizzle::BLOCK).ok_or(bad("bntx: the texture data leaves the file"))?;
            let mut blk: [u8; 16] = [0; 16];
            blk.copy_from_slice(s);
            if !astc::decode_block(&blk, BLOCK_PX, BLOCK_PX, srgb, &mut px) {
                px = [astc::BAD; 16];
                failed += 1;
            }
            for (i, c) in px.iter().enumerate() {
                let x: usize = bx * BLOCK_PX + i % BLOCK_PX;
                let y: usize = by * BLOCK_PX + i / BLOCK_PX;
                if x < t.w && y < t.h {
                    im.put(x, y, *c);
                }
            }
        }
    }
    im.pick(t.comp);
    Ok((im, failed))
}

#[cfg(test)]
pub mod fixture {
    use crate::tex::swizzle;

    pub struct Fake {
        pub name: &'static str,
        pub fmt: u32,
        pub w: usize,
        pub h: usize,
        pub layout: u32,
        pub comp: [u8; 4],
        pub rgba: fn(usize, usize) -> [u8; 4],
    }

    fn void(c: [u8; 4]) -> [u8; 16] {
        let mut x: u128 = 0xdfc | 0xffff_ffff_ffff_f000u128;
        for (i, v) in c.iter().enumerate() {
            x |= ((*v as u128) << 8 | *v as u128) << (64 + 16 * i);
        }
        x.to_le_bytes()
    }

    fn le64(out: &mut [u8], o: usize, v: usize) {
        out[o..o + 8].copy_from_slice(&(v as u64).to_le_bytes());
    }

    fn le32(out: &mut [u8], o: usize, v: u32) {
        out[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }

    pub fn build(fs: &[Fake]) -> Vec<u8> {
        let arr: usize = 0x40;
        let mut heads: usize = arr + 8 * fs.len();
        let mut out: Vec<u8> = vec![0; heads + fs.len() * 0x100];
        out[..4].copy_from_slice(b"BNTX");
        out[0x20..0x24].copy_from_slice(b"Fake");
        le32(&mut out, 0x24, fs.len() as u32);
        le64(&mut out, 0x28, arr);
        for (i, f) in fs.iter().enumerate() {
            let o: usize = heads;
            heads += 0x100;
            le64(&mut out, arr + 8 * i, o);
            out[o..o + 4].copy_from_slice(b"BRTI");
            le32(&mut out, o + 0x1c, f.fmt);
            le32(&mut out, o + 0x24, f.w as u32);
            le32(&mut out, o + 0x28, f.h as u32);
            le32(&mut out, o + 0x34, f.layout);
            out[o + 0x58..o + 0x5c].copy_from_slice(&f.comp);
            le64(&mut out, o + 0x60, o + 0x80);
            out[o + 0x80..o + 0x82].copy_from_slice(&(f.name.len() as u16).to_le_bytes());
            out[o + 0x82..o + 0x82 + f.name.len()].copy_from_slice(f.name.as_bytes());
            le64(&mut out, o + 0x70, o + 0xf0);
            let (wb, hb): (usize, usize) = (f.w.div_ceil(4), f.h.div_ceil(4));
            let gobs: usize = 1 << (f.layout & 7);
            let span: usize = swizzle::span(wb, hb, gobs);
            let off: usize = out.len();
            le64(&mut out, o + 0xf0, off);
            le32(&mut out, o + 0x50, span as u32);
            out.resize(off + span, 0);
            for by in 0..hb {
                for bx in 0..wb {
                    let a: usize = off + swizzle::addr(bx, by, wb, gobs);
                    out[a..a + 16].copy_from_slice(&void((f.rgba)(bx, by)));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::{Fake, build};
    use super::*;

    fn checker(bx: usize, by: usize) -> [u8; 4] {
        [(bx * 20) as u8, (by * 30) as u8, 7, if (bx + by) % 2 == 0 { 255 } else { 64 }]
    }

    fn grey(bx: usize, _: usize) -> [u8; 4] {
        [(bx * 50) as u8, 1, 2, 200]
    }

    fn two() -> Vec<u8> {
        build(&[
            Fake { name: "First_00", fmt: ASTC_4X4_SRGB, w: 21, h: 37, layout: 1, comp: [2, 3, 4, 5], rgba: checker },
            Fake { name: "Second", fmt: ASTC_4X4_UNORM, w: 8, h: 4, layout: 0, comp: [2, 2, 2, 5], rgba: grey },
        ])
    }

    #[test]
    fn a_fake_container_lists_its_textures() {
        let b: Vec<u8> = two();
        let ts: Vec<Tex> = parse(&b).unwrap();
        assert_eq!(ts.len(), 2);
        assert_eq!((ts[0].name.as_str(), ts[0].w, ts[0].h, ts[0].gobs, ts[0].fmt), ("First_00", 21, 37, 2, ASTC_4X4_SRGB));
        assert_eq!((ts[1].name.as_str(), ts[1].comp), ("Second", [2, 2, 2, 5]));
        assert!(find(&ts, "Second").is_some());
        assert!(find(&ts, "second").is_none());
    }

    #[test]
    fn decode_places_every_block_and_crops_the_edge() {
        let b: Vec<u8> = two();
        let ts: Vec<Tex> = parse(&b).unwrap();
        let (im, failed): (Img, usize) = decode(&b, &ts[0]).unwrap();
        assert_eq!((im.w, im.h, failed), (21, 37, 0));
        for (x, y) in [(0, 0), (3, 3), (4, 0), (20, 36), (17, 9), (9, 33)] {
            assert_eq!(im.at(x, y), checker(x / 4, y / 4), "{x} {y}");
        }
        let (g, _): (Img, usize) = decode(&b, &ts[1]).unwrap();
        assert_eq!(g.at(5, 2), [50, 50, 50, 200]);
    }

    #[test]
    fn broken_blocks_are_counted_and_painted() {
        let mut b: Vec<u8> = two();
        let ts: Vec<Tex> = parse(&b).unwrap();
        let a: usize = ts[1].off + swizzle::addr(1, 0, 2, 1);
        b[a..a + 16].fill(0);
        let (g, failed): (Img, usize) = decode(&b, &ts[1]).unwrap();
        assert_eq!(failed, 1);
        assert_eq!(g.at(4, 0), [255, 255, 255, 255]);
        assert_eq!(g.at(0, 0), [0, 0, 0, 200]);
    }

    #[test]
    fn damaged_files_are_refused() {
        let b: Vec<u8> = two();
        assert!(parse(&b[..0x30]).is_err());
        assert!(parse(b"nope").is_err());
        let mut c: Vec<u8> = b.clone();
        c[0] = b'X';
        assert!(parse(&c).is_err());
        let mut d: Vec<u8> = b.clone();
        d[0x28..0x30].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(parse(&d).is_err());
        let mut e: Vec<u8> = b.clone();
        e[0x24..0x28].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse(&e).is_err());
        let ts: Vec<Tex> = parse(&b).unwrap();
        let cut: usize = ts[0].off + 64;
        assert!(decode(&b[..cut], &ts[0]).is_err());
        let mut f: Tex = ts[0].clone();
        f.fmt = 0x1d01;
        assert!(decode(&b, &f).is_err());
        let mut g: Vec<u8> = b.clone();
        let o: usize = 0x40 + 16;
        g[o + 0x24..o + 0x28].copy_from_slice(&0u32.to_le_bytes());
        assert!(parse(&g).is_err());
    }
}
