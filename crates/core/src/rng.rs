pub struct Rng {
    s: [u64; 4],
}

fn splitmix(x: &mut u64) -> u64 {
    *x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z: u64 = *x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn fnv(text: &str) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for b in text.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

impl Rng {
    pub fn from_seed(seed: u64) -> Rng {
        let mut x: u64 = seed;
        let a: u64 = splitmix(&mut x);
        let b: u64 = splitmix(&mut x);
        let c: u64 = splitmix(&mut x);
        let d: u64 = splitmix(&mut x);
        Rng { s: [a, b, c, d] }
    }

    pub fn from_text(text: &str) -> Rng {
        Rng::from_seed(fnv(text))
    }

    pub fn next_u64(&mut self) -> u64 {
        let r: u64 = self.s[0]
            .wrapping_add(self.s[3])
            .rotate_left(23)
            .wrapping_add(self.s[0]);
        let t: u64 = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        r
    }

    pub fn below(&mut self, n: u64) -> u64 {
        if n <= 1 {
            return 0;
        }
        let zone: u64 = u64::MAX - (u64::MAX % n);
        loop {
            let r: u64 = self.next_u64();
            if r < zone {
                return r % n;
            }
        }
    }

    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        let mut i: usize = v.len();
        while i > 1 {
            let j: usize = self.below(i as u64) as usize;
            i -= 1;
            v.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_stream() {
        let mut a: Rng = Rng::from_text("abc");
        let mut b: Rng = Rng::from_text("abc");
        for _ in 0..64 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn seeds_are_stable() {
        let mut a: Vec<usize> = (0..12).collect();
        Rng::from_text("ootrr").shuffle(&mut a);
        assert_eq!(a, vec![7, 10, 1, 8, 4, 2, 6, 5, 0, 11, 9, 3]);
        let mut b: Vec<usize> = (0..12).collect();
        Rng::from_text("Seed With Spaces").shuffle(&mut b);
        assert_eq!(b, vec![1, 4, 6, 0, 7, 2, 10, 3, 9, 5, 8, 11]);
    }

    #[test]
    fn different_seed_differs() {
        let mut a: Rng = Rng::from_text("abc");
        let mut b: Rng = Rng::from_text("abd");
        let same: bool = (0..8).all(|_| a.next_u64() == b.next_u64());
        assert!(!same);
    }

    #[test]
    fn below_stays_in_range() {
        let mut r: Rng = Rng::from_seed(7);
        for n in [1u64, 2, 3, 7, 100, 1 << 40, u64::MAX] {
            for _ in 0..200 {
                assert!(r.below(n) < n.max(1));
            }
        }
    }

    #[test]
    fn shuffle_keeps_every_item() {
        let mut r: Rng = Rng::from_seed(99);
        let mut v: Vec<u32> = (0..500).collect();
        r.shuffle(&mut v);
        let mut s: Vec<u32> = v.clone();
        s.sort();
        assert_eq!(s, (0..500).collect::<Vec<u32>>());
        assert_ne!(v, (0..500).collect::<Vec<u32>>());
    }
}
