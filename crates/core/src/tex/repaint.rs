use crate::error::{Error, Res};
use crate::tex::astc::recolor::{Done, Tables, recolor_block};
use crate::tex::bcn;
use crate::tex::bntx::{self, Tex};
use crate::tex::surface::{self, Kind};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Count {
    pub blocks: usize,
    pub changed: usize,
    pub skipped: usize,
}

pub fn repaint(b: &mut [u8], f: &dyn Fn([u8; 3]) -> [u8; 3]) -> Res<Count> {
    let ts: Vec<Tex> = bntx::parse(b)?;
    let tb: Tables = Tables::new();
    let mut n: Count = Count::default();
    for t in &ts {
        let Some(k) = surface::kind(t.fmt) else {
            return Err(Error::Msg(format!("texture {} has format {:#x}, which cannot be recoloured", t.name, t.fmt)));
        };
        let end: usize = t.off.checked_add(t.size).filter(|e| *e <= b.len()).ok_or(Error::Format("the texture data leaves the file"))?;
        for blk in b[t.off..end].chunks_exact_mut(k.bytes()) {
            n.blocks += 1;
            match k {
                Kind::Bc1 => n.changed += bcn::recolor_bc1(blk, f) as usize,
                Kind::Bc3 => n.changed += bcn::recolor_bc3(blk, f) as usize,
                Kind::Astc(..) => {
                    let mut x: [u8; 16] = [0; 16];
                    x.copy_from_slice(blk);
                    match recolor_block(&mut x, &tb, f) {
                        Done::Changed => {
                            blk.copy_from_slice(&x);
                            n.changed += 1;
                        }
                        Done::Skipped => n.skipped += 1,
                        Done::Same => {}
                    }
                }
            }
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tex::bntx::fixture::{Fake, build};
    use crate::tex::image::Img;

    fn paint(bx: usize, by: usize) -> [u8; 4] {
        [(bx * 30 + 20) as u8, (by * 40) as u8, 9, 200]
    }

    fn fake() -> Vec<u8> {
        build(&[Fake { name: "A", fmt: bntx::ASTC_4X4_SRGB, w: 20, h: 12, layout: 0, comp: [2, 3, 4, 5], rgba: paint }])
    }

    #[test]
    fn an_identity_repaint_reproduces_the_file() {
        let old: Vec<u8> = fake();
        let mut b: Vec<u8> = old.clone();
        let n: Count = repaint(&mut b, &|c| c).unwrap();
        assert_eq!(b, old);
        assert_eq!(n.changed, 0);
        assert!(n.blocks >= 15);
    }

    #[test]
    fn every_texel_follows_the_colour_map_and_the_size_stays() {
        let old: Vec<u8> = fake();
        let mut b: Vec<u8> = old.clone();
        let n: Count = repaint(&mut b, &|c| [c[2], c[0], c[1]]).unwrap();
        assert_eq!(b.len(), old.len());
        assert_eq!(n.changed, 15);
        let t: Tex = bntx::parse(&b).unwrap().remove(0);
        let (was, now): (Img, Img) = (surface::decode(&old, &t).unwrap(), surface::decode(&b, &t).unwrap());
        for y in 0..12 {
            for x in 0..20 {
                let (p, q): ([u8; 4], [u8; 4]) = (was.at(x, y), now.at(x, y));
                assert_eq!(q, [p[2], p[0], p[1], p[3]]);
            }
        }
    }

    #[test]
    fn unknown_formats_are_refused() {
        let mut b: Vec<u8> = build(&[Fake { name: "A", fmt: 0x2006, w: 8, h: 8, layout: 0, comp: [2, 3, 4, 5], rgba: paint }]);
        assert!(repaint(&mut b, &|c| c).is_err());
    }
}
