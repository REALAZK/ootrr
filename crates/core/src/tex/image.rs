#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Img {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

impl Img {
    pub fn new(w: usize, h: usize) -> Img {
        Img { w, h, px: vec![0; w * h * 4] }
    }

    pub fn at(&self, x: usize, y: usize) -> [u8; 4] {
        let i: usize = (y * self.w + x) * 4;
        [self.px[i], self.px[i + 1], self.px[i + 2], self.px[i + 3]]
    }

    pub fn put(&mut self, x: usize, y: usize, c: [u8; 4]) {
        let i: usize = (y * self.w + x) * 4;
        self.px[i..i + 4].copy_from_slice(&c);
    }

    pub fn pick(&mut self, sel: [u8; 4]) {
        if sel == [2, 3, 4, 5] {
            return;
        }
        for p in self.px.chunks_exact_mut(4) {
            let src: [u8; 4] = [p[0], p[1], p[2], p[3]];
            for (i, s) in sel.iter().enumerate() {
                p[i] = match s {
                    0 => 0,
                    1 => 255,
                    2..=5 => src[(*s - 2) as usize],
                    _ => src[i],
                };
            }
        }
    }

    pub fn tint(&mut self, c: [u8; 3]) {
        for p in self.px.chunks_exact_mut(4) {
            for i in 0..3 {
                p[i] = ((p[i] as u32 * c[i] as u32 + 127) / 255) as u8;
            }
        }
    }

    pub fn shrink(&self, max: usize) -> Img {
        let side: usize = self.w.max(self.h);
        if max == 0 || side <= max {
            return self.clone();
        }
        let f: usize = side.div_ceil(max);
        let w: usize = self.w.div_ceil(f);
        let h: usize = self.h.div_ceil(f);
        let mut out: Img = Img::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let mut acc: [u64; 4] = [0; 4];
                let mut n: u64 = 0;
                for sy in y * f..((y + 1) * f).min(self.h) {
                    for sx in x * f..((x + 1) * f).min(self.w) {
                        let c: [u8; 4] = self.at(sx, sy);
                        let a: u64 = c[3] as u64;
                        acc[0] += c[0] as u64 * a;
                        acc[1] += c[1] as u64 * a;
                        acc[2] += c[2] as u64 * a;
                        acc[3] += a;
                        n += 1;
                    }
                }
                if n == 0 || acc[3] == 0 {
                    continue;
                }
                let a: u64 = acc[3];
                out.put(x, y, [((acc[0] + a / 2) / a) as u8, ((acc[1] + a / 2) / a) as u8, ((acc[2] + a / 2) / a) as u8, ((a + n / 2) / n) as u8]);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shrink_averages_with_alpha_weights() {
        let mut a: Img = Img::new(4, 2);
        a.put(0, 0, [200, 0, 0, 255]);
        a.put(1, 0, [100, 0, 0, 255]);
        a.put(0, 1, [0, 90, 0, 0]);
        a.put(1, 1, [0, 90, 0, 0]);
        let b: Img = a.shrink(2);
        assert_eq!((b.w, b.h), (2, 1));
        assert_eq!(b.at(0, 0), [150, 0, 0, 128]);
        assert_eq!(b.at(1, 0), [0, 0, 0, 0]);
        assert_eq!(a.shrink(4), a);
        assert_eq!(a.shrink(0), a);
    }

    #[test]
    fn shrink_keeps_an_odd_edge() {
        let mut a: Img = Img::new(5, 5);
        for y in 0..5 {
            for x in 0..5 {
                a.put(x, y, [10, 20, 30, 255]);
            }
        }
        let b: Img = a.shrink(2);
        assert_eq!((b.w, b.h), (2, 2));
        assert_eq!(b.at(1, 1), [10, 20, 30, 255]);
        let c: Img = a.shrink(3);
        assert_eq!((c.w, c.h), (3, 3));
        assert_eq!(c.at(2, 2), [10, 20, 30, 255]);
    }

    #[test]
    fn pick_and_tint_change_channels() {
        let mut a: Img = Img::new(1, 1);
        a.put(0, 0, [40, 80, 120, 200]);
        let mut b: Img = a.clone();
        b.pick([2, 2, 2, 5]);
        assert_eq!(b.at(0, 0), [40, 40, 40, 200]);
        let mut c: Img = a.clone();
        c.pick([1, 0, 4, 2]);
        assert_eq!(c.at(0, 0), [255, 0, 120, 40]);
        a.tint([255, 128, 0]);
        assert_eq!(a.at(0, 0), [40, 40, 0, 200]);
    }
}
