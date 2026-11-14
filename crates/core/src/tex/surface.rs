use crate::error::{Error, Res};
use crate::tex::astc;
use crate::tex::bcn;
use crate::tex::bntx::Tex;
use crate::tex::image::Img;
use crate::tex::swizzle;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Bc1,
    Bc3,
    Astc(usize, usize),
}

const ASTC: [(usize, usize); 14] = [(4, 4), (5, 4), (5, 5), (6, 5), (6, 6), (8, 5), (8, 6), (8, 8), (10, 5), (10, 6), (10, 8), (10, 10), (12, 10), (12, 12)];

pub fn kind(fmt: u32) -> Option<Kind> {
    match fmt >> 8 {
        0x1a => Some(Kind::Bc1),
        0x1c => Some(Kind::Bc3),
        k @ 0x2d..=0x3a => {
            let (w, h): (usize, usize) = ASTC[(k - 0x2d) as usize];
            Some(Kind::Astc(w, h))
        }
        _ => None,
    }
}

impl Kind {
    pub fn cell(self) -> (usize, usize) {
        match self {
            Kind::Astc(w, h) => (w, h),
            _ => (4, 4),
        }
    }

    pub fn bytes(self) -> usize {
        if self == Kind::Bc1 { 8 } else { 16 }
    }
}

pub fn decode(b: &[u8], t: &Tex) -> Res<Img> {
    let Some(k) = kind(t.fmt) else {
        return Err(Error::Msg(format!("texture {} has format {:#x}, which is not read", t.name, t.fmt)));
    };
    let (cw, ch): (usize, usize) = k.cell();
    let n: usize = k.bytes();
    let (wb, hb): (usize, usize) = (t.w.div_ceil(cw), t.h.div_ceil(ch));
    let per: usize = swizzle::BLOCK / n;
    let wide: usize = wb.div_ceil(per);
    let data: &[u8] = b.get(t.off..).ok_or(Error::Format("the texture data starts outside the file"))?;
    let srgb: bool = t.fmt & 0xff == 6;
    let mut im: Img = Img::new(t.w, t.h);
    let mut px: [[u8; 4]; astc::MAX_TEXELS] = [[0; 4]; astc::MAX_TEXELS];
    for by in 0..hb {
        for bx in 0..wb {
            let a: usize = swizzle::addr(bx / per, by, wide, t.gobs) + (bx % per) * n;
            let s: &[u8] = data.get(a..a + n).ok_or(Error::Format("the texture data leaves the file"))?;
            let ok: bool = match k {
                Kind::Bc1 => {
                    let mut o: [[u8; 4]; 16] = [[0; 4]; 16];
                    let r: bool = bcn::decode_bc1(s, &mut o);
                    px[..16].copy_from_slice(&o);
                    r
                }
                Kind::Bc3 => {
                    let mut o: [[u8; 4]; 16] = [[0; 4]; 16];
                    let r: bool = bcn::decode_bc3(s, &mut o);
                    px[..16].copy_from_slice(&o);
                    r
                }
                Kind::Astc(..) => {
                    let mut blk: [u8; 16] = [0; 16];
                    blk.copy_from_slice(s);
                    astc::decode_block(&blk, cw, ch, srgb, &mut px)
                }
            };
            if !ok {
                px[..cw * ch].fill(astc::BAD);
            }
            for (i, c) in px.iter().enumerate().take(cw * ch) {
                let (x, y): (usize, usize) = (bx * cw + i % cw, by * ch + i / cw);
                if x < t.w && y < t.h {
                    im.put(x, y, *c);
                }
            }
        }
    }
    Ok(im)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tex::bntx::{self, fixture::{Fake, build}};

    fn flat(bx: usize, by: usize) -> [u8; 4] {
        [(bx * 30) as u8, (by * 40) as u8, 9, 255]
    }

    #[test]
    fn formats_map_to_their_block_shapes() {
        assert_eq!(kind(0x1a06), Some(Kind::Bc1));
        assert_eq!(kind(0x1c06), Some(Kind::Bc3));
        assert_eq!(kind(0x3306), Some(Kind::Astc(8, 6)));
        assert_eq!(kind(0x3a01), Some(Kind::Astc(12, 12)));
        assert_eq!(kind(0x2006), None);
        assert_eq!((Kind::Bc1.bytes(), Kind::Bc3.bytes(), Kind::Astc(8, 6).cell()), (8, 16, (8, 6)));
    }

    #[test]
    fn a_four_by_four_surface_matches_the_older_reader() {
        let b: Vec<u8> = build(&[Fake { name: "A", fmt: bntx::ASTC_4X4_UNORM, w: 21, h: 37, layout: 1, comp: [2, 3, 4, 5], rgba: flat }]);
        let ts: Vec<Tex> = bntx::parse(&b).unwrap();
        let old: Img = bntx::decode(&b, &ts[0]).unwrap().0;
        assert_eq!(decode(&b, &ts[0]).unwrap(), old);
    }
}
