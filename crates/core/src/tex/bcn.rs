pub fn wide(v: u16) -> [u8; 3] {
    let (r, g, b): (u32, u32, u32) = ((v >> 11) as u32, ((v >> 5) & 63) as u32, (v & 31) as u32);
    [((r * 255 + 15) / 31) as u8, ((g * 255 + 31) / 63) as u8, ((b * 255 + 15) / 31) as u8]
}

pub fn narrow(c: [u8; 3]) -> u16 {
    let r: u16 = ((c[0] as u32 * 31 + 127) / 255) as u16;
    let g: u16 = ((c[1] as u32 * 63 + 127) / 255) as u16;
    let b: u16 = ((c[2] as u32 * 31 + 127) / 255) as u16;
    (r << 11) | (g << 5) | b
}

fn mix(a: [u8; 3], b: [u8; 3], wa: u32, wb: u32) -> [u8; 4] {
    let n: u32 = wa + wb;
    let f = |i: usize| ((a[i] as u32 * wa + b[i] as u32 * wb + n / 2) / n) as u8;
    [f(0), f(1), f(2), 255]
}

fn colours(b: &[u8], four: bool, out: &mut [[u8; 4]; 16]) {
    let c0: u16 = u16::from_le_bytes([b[0], b[1]]);
    let c1: u16 = u16::from_le_bytes([b[2], b[3]]);
    let (a, z): ([u8; 3], [u8; 3]) = (wide(c0), wide(c1));
    let pal: [[u8; 4]; 4] = if four || c0 > c1 { [mix(a, z, 1, 0), mix(a, z, 0, 1), mix(a, z, 2, 1), mix(a, z, 1, 2)] } else { [mix(a, z, 1, 0), mix(a, z, 0, 1), mix(a, z, 1, 1), [0, 0, 0, 0]] };
    let idx: u32 = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
    for (i, o) in out.iter_mut().enumerate() {
        *o = pal[((idx >> (2 * i)) & 3) as usize];
    }
}

fn alphas(b: &[u8], out: &mut [[u8; 4]; 16]) {
    let (a0, a1): (u32, u32) = (b[0] as u32, b[1] as u32);
    let mut pal: [u32; 8] = [a0, a1, 0, 0, 0, 0, 0, 255];
    if a0 > a1 {
        for i in 1..7u32 {
            pal[i as usize + 1] = ((7 - i) * a0 + i * a1 + 3) / 7;
        }
    } else {
        for i in 1..5u32 {
            pal[i as usize + 1] = ((5 - i) * a0 + i * a1 + 2) / 5;
        }
    }
    let mut bits: u64 = 0;
    for i in 0..6 {
        bits |= (b[2 + i] as u64) << (8 * i);
    }
    for (i, o) in out.iter_mut().enumerate() {
        o[3] = pal[((bits >> (3 * i)) & 7) as usize] as u8;
    }
}

pub fn decode_bc1(b: &[u8], out: &mut [[u8; 4]; 16]) -> bool {
    if b.len() < 8 {
        return false;
    }
    colours(b, false, out);
    true
}

pub fn decode_bc3(b: &[u8], out: &mut [[u8; 4]; 16]) -> bool {
    if b.len() < 16 {
        return false;
    }
    colours(&b[8..16], true, out);
    alphas(b, out);
    true
}

fn ends(b: &[u8], f: &dyn Fn([u8; 3]) -> [u8; 3]) -> Option<(u16, u16, u16, u16)> {
    let c0: u16 = u16::from_le_bytes([b[0], b[1]]);
    let c1: u16 = u16::from_le_bytes([b[2], b[3]]);
    let n0: u16 = narrow(f(wide(c0)));
    let n1: u16 = narrow(f(wide(c1)));
    ((n0, n1) != (c0, c1)).then_some((c0, c1, n0, n1))
}

fn store(b: &mut [u8], c0: u16, c1: u16, idx: u32) {
    b[0..2].copy_from_slice(&c0.to_le_bytes());
    b[2..4].copy_from_slice(&c1.to_le_bytes());
    b[4..8].copy_from_slice(&idx.to_le_bytes());
}

pub fn recolor_bc1(b: &mut [u8], f: &dyn Fn([u8; 3]) -> [u8; 3]) -> bool {
    if b.len() < 8 {
        return false;
    }
    let Some((c0, c1, n0, n1)) = ends(b, f) else {
        return false;
    };
    let idx: u32 = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
    if c0 > c1 {
        if n0 > n1 {
            store(b, n0, n1, idx);
        } else if n0 < n1 {
            store(b, n1, n0, idx ^ 0x5555_5555);
        } else {
            store(b, n0, n1, 0);
        }
    } else if n0 <= n1 {
        store(b, n0, n1, idx);
    } else {
        store(b, n1, n0, idx ^ (!(idx >> 1) & 0x5555_5555));
    }
    true
}

pub fn recolor_bc3(b: &mut [u8], f: &dyn Fn([u8; 3]) -> [u8; 3]) -> bool {
    if b.len() < 16 {
        return false;
    }
    let Some((_, _, n0, n1)) = ends(&b[8..16], f) else {
        return false;
    };
    b[8..10].copy_from_slice(&n0.to_le_bytes());
    b[10..12].copy_from_slice(&n1.to_le_bytes());
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(c0: [u8; 3], c1: [u8; 3], idx: u32) -> [u8; 8] {
        let mut b: [u8; 8] = [0; 8];
        store(&mut b, narrow(c0), narrow(c1), idx);
        b
    }

    fn px(b: &[u8]) -> [[u8; 4]; 16] {
        let mut out: [[u8; 4]; 16] = [[0; 4]; 16];
        assert!(decode_bc1(b, &mut out));
        out
    }

    fn swap(c: [u8; 3]) -> [u8; 3] {
        [c[2], c[0], c[1]]
    }

    fn turned(p: &[[u8; 4]; 16]) -> Vec<[u8; 4]> {
        p.iter().map(|c| if c[3] == 0 { *c } else { [c[2], c[0], c[1], c[3]] }).collect()
    }

    fn close(a: &[[u8; 4]], b: &[[u8; 4]]) -> bool {
        a.iter().zip(b).all(|(p, q)| (0..4).all(|c| (p[c] as i32 - q[c] as i32).abs() <= 5))
    }

    #[test]
    fn five_six_five_round_trips_through_eight_bits() {
        for v in [0u16, 1, 0x1f, 0x7e0, 0xf800, 0xffff, 0x1234, 0xabcd] {
            assert_eq!(narrow(wide(v)), v);
        }
    }

    #[test]
    fn four_colour_blocks_keep_their_texels_when_the_order_flips() {
        let idx: u32 = 0b11_10_01_00_11_01_10_00_00_01_10_11_00_11_10_01;
        for (a, z) in [([248, 16, 8], [8, 200, 16]), ([8, 16, 248], [200, 60, 16]), ([40, 250, 40], [16, 16, 120])] {
            let mut b: [u8; 8] = block(a, z, idx);
            if u16::from_le_bytes([b[0], b[1]]) <= u16::from_le_bytes([b[2], b[3]]) {
                b = block(z, a, idx);
            }
            let want: Vec<[u8; 4]> = turned(&px(&b));
            assert!(recolor_bc1(&mut b, &swap));
            assert!(u16::from_le_bytes([b[0], b[1]]) > u16::from_le_bytes([b[2], b[3]]));
            assert!(close(&px(&b), &want), "{:?}", px(&b));
        }
    }

    #[test]
    fn three_colour_blocks_keep_the_clear_texels_clear() {
        let idx: u32 = 0b11_10_01_00_11_01_10_00_00_01_10_11_00_11_10_01;
        let mut b: [u8; 8] = block([8, 16, 248], [248, 16, 8], idx);
        assert!(u16::from_le_bytes([b[0], b[1]]) <= u16::from_le_bytes([b[2], b[3]]));
        let old: [[u8; 4]; 16] = px(&b);
        assert!(recolor_bc1(&mut b, &swap));
        let new: [[u8; 4]; 16] = px(&b);
        assert!(u16::from_le_bytes([b[0], b[1]]) <= u16::from_le_bytes([b[2], b[3]]));
        assert!(close(&new, &turned(&old)), "{new:?}");
        assert_eq!(new.iter().filter(|p| p[3] == 0).count(), 4);
    }

    #[test]
    fn the_same_colours_leave_the_block_untouched() {
        let mut b: [u8; 8] = block([200, 100, 50], [10, 20, 30], 0x1234_5678);
        let old: [u8; 8] = b;
        assert!(!recolor_bc1(&mut b, &|c| c));
        assert_eq!(b, old);
    }

    #[test]
    fn the_alpha_half_of_a_bc3_block_is_kept() {
        let mut b: [u8; 16] = [0; 16];
        b[..8].copy_from_slice(&[200, 40, 0x53, 0x21, 0x77, 0x9a, 0xbc, 0x0e]);
        b[8..].copy_from_slice(&block([10, 200, 30], [250, 20, 90], 0x0f0f_a5a5));
        let mut old: [[u8; 4]; 16] = [[0; 4]; 16];
        assert!(decode_bc3(&b, &mut old));
        let head: [u8; 8] = [b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]];
        assert!(recolor_bc3(&mut b, &swap));
        assert_eq!(b[..8], head);
        let mut new: [[u8; 4]; 16] = [[0; 4]; 16];
        assert!(decode_bc3(&b, &mut new));
        let want: Vec<[u8; 4]> = old.iter().map(|c| [c[2], c[0], c[1], c[3]]).collect();
        assert!(close(&new, &want), "{new:?}");
        assert!(old.iter().any(|c| c[3] != 255));
    }
}
