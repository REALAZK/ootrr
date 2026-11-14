pub mod recolor;

pub const BAD: [u8; 4] = [255, 0, 255, 255];
pub const MAX_TEXELS: usize = 144;

const WEIGHT_LEVELS: [u32; 12] = [2, 3, 4, 5, 6, 8, 10, 12, 16, 20, 24, 32];
const COLOR_LEVELS: [u32; 17] = [256, 192, 160, 128, 96, 80, 64, 48, 40, 32, 24, 20, 16, 12, 10, 8, 6];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Bits,
    Trit,
    Quint,
}

type Pair = (u32, u32);
type Ends = ([i32; 4], [i32; 4]);

fn level(n: u32) -> (u32, Kind) {
    match n {
        2 => (1, Kind::Bits),
        3 => (0, Kind::Trit),
        4 => (2, Kind::Bits),
        5 => (0, Kind::Quint),
        6 => (1, Kind::Trit),
        8 => (3, Kind::Bits),
        10 => (1, Kind::Quint),
        12 => (2, Kind::Trit),
        16 => (4, Kind::Bits),
        20 => (2, Kind::Quint),
        24 => (3, Kind::Trit),
        32 => (5, Kind::Bits),
        40 => (3, Kind::Quint),
        48 => (4, Kind::Trit),
        64 => (6, Kind::Bits),
        80 => (4, Kind::Quint),
        96 => (5, Kind::Trit),
        128 => (7, Kind::Bits),
        160 => (5, Kind::Quint),
        192 => (6, Kind::Trit),
        _ => (8, Kind::Bits),
    }
}

fn ise_bits(count: u32, lv: u32) -> u32 {
    let (b, k): (u32, Kind) = level(lv);
    count * b
        + match k {
            Kind::Bits => 0,
            Kind::Trit => (8 * count + 4) / 5,
            Kind::Quint => (7 * count + 2) / 3,
        }
}

fn rd(x: u128, pos: u32, n: u32) -> u32 {
    if n == 0 || pos >= 128 {
        return 0;
    }
    ((x >> pos) & ((1u128 << n) - 1)) as u32
}

fn trits(t: u32) -> [u32; 5] {
    let c: u32;
    let t4: u32;
    let t3: u32;
    if (t >> 2) & 7 == 7 {
        c = (((t >> 5) & 7) << 2) | (t & 3);
        t4 = 2;
        t3 = 2;
    } else {
        c = t & 0x1f;
        if (t >> 5) & 3 == 3 {
            t4 = 2;
            t3 = (t >> 7) & 1;
        } else {
            t4 = (t >> 7) & 1;
            t3 = (t >> 5) & 3;
        }
    }
    let t2: u32;
    let t1: u32;
    let t0: u32;
    if c & 3 == 3 {
        t2 = 2;
        t1 = (c >> 4) & 1;
        let c3: u32 = (c >> 3) & 1;
        let c2: u32 = (c >> 2) & 1;
        t0 = (c3 << 1) | (c2 & (1 - c3));
    } else if (c >> 2) & 3 == 3 {
        t2 = 2;
        t1 = 2;
        t0 = c & 3;
    } else {
        t2 = (c >> 4) & 1;
        t1 = (c >> 2) & 3;
        let c1: u32 = (c >> 1) & 1;
        let c0: u32 = c & 1;
        t0 = (c1 << 1) | (c0 & (1 - c1));
    }
    [t0, t1, t2, t3, t4]
}

fn quints(q: u32) -> [u32; 3] {
    let bit = |i: u32| (q >> i) & 1;
    if (q >> 1) & 3 == 3 && (q >> 5) & 3 == 0 {
        let q2: u32 = (bit(0) << 2) | ((bit(4) & (1 - bit(0))) << 1) | (bit(3) & (1 - bit(0)));
        return [4, 4, q2];
    }
    let q2: u32;
    let c: u32;
    if (q >> 1) & 3 == 3 {
        q2 = 4;
        c = (((q >> 3) & 3) << 3) | ((3 - ((q >> 5) & 3)) << 1) | bit(0);
    } else {
        q2 = (q >> 5) & 3;
        c = q & 0x1f;
    }
    if c & 7 == 5 {
        [(c >> 3) & 3, 4, q2]
    } else {
        [c & 7, (c >> 3) & 3, q2]
    }
}

fn ise(stream: u128, count: usize, lv: u32, out: &mut [Pair]) {
    let (b, k): (u32, Kind) = level(lv);
    let mut pos: u32 = 0;
    let mut i: usize = 0;
    match k {
        Kind::Bits => {
            for (j, o) in out.iter_mut().enumerate().take(count) {
                *o = (0, rd(stream, j as u32 * b, b));
            }
        }
        Kind::Trit => {
            while i < count {
                let mut ms: [u32; 5] = [0; 5];
                let mut t: u32 = 0;
                let mut tb: u32 = 0;
                for (j, tn) in [2u32, 2, 1, 2, 1].iter().enumerate() {
                    ms[j] = rd(stream, pos, b);
                    pos += b;
                    t |= rd(stream, pos, *tn) << tb;
                    pos += *tn;
                    tb += *tn;
                }
                let ds: [u32; 5] = trits(t);
                for j in 0..5.min(count - i) {
                    out[i + j] = (ds[j], ms[j]);
                }
                i += 5;
            }
        }
        Kind::Quint => {
            while i < count {
                let mut ms: [u32; 3] = [0; 3];
                let mut q: u32 = 0;
                let mut qb: u32 = 0;
                for (j, qn) in [3u32, 2, 2].iter().enumerate() {
                    ms[j] = rd(stream, pos, b);
                    pos += b;
                    q |= rd(stream, pos, *qn) << qb;
                    pos += *qn;
                    qb += *qn;
                }
                let ds: [u32; 3] = quints(q);
                for j in 0..3.min(count - i) {
                    out[i + j] = (ds[j], ms[j]);
                }
                i += 3;
            }
        }
    }
}

fn bitpat(pat: &[u8], v: u32) -> u32 {
    let mut r: u32 = 0;
    for ch in pat {
        r <<= 1;
        if *ch != b'0' {
            r |= (v >> (*ch - b'a') as u32) & 1;
        }
    }
    r
}

fn spread(v: u32, b: u32, to: u32) -> u32 {
    let mut have: u32 = 0;
    let mut out: u32 = 0;
    while have < to {
        let take: u32 = b.min(to - have);
        out = (out << take) | (v >> (b - take));
        have += take;
    }
    out
}

fn color_table(b: u32, k: Kind) -> (&'static [u8], u32) {
    match (k, b) {
        (Kind::Trit, 1) => (b"000000000", 204),
        (Kind::Trit, 2) => (b"b000b0bb0", 93),
        (Kind::Trit, 3) => (b"cb000cbcb", 44),
        (Kind::Trit, 4) => (b"dcb000dcb", 22),
        (Kind::Trit, 5) => (b"edcb000ed", 11),
        (Kind::Trit, _) => (b"fedcb000f", 5),
        (_, 1) => (b"000000000", 113),
        (_, 2) => (b"b0000bb00", 54),
        (_, 3) => (b"cb000cbc0", 26),
        (_, 4) => (b"dcb0000dc", 13),
        _ => (b"edcb0000e", 6),
    }
}

fn unq_color(d: u32, v: u32, lv: u32) -> u32 {
    let (b, k): (u32, Kind) = level(lv);
    if k == Kind::Bits {
        return spread(v, b, 8);
    }
    let (pat, c): (&[u8], u32) = color_table(b, k);
    let a: u32 = if v & 1 != 0 { 0x1ff } else { 0 };
    let t: u32 = (d * c + bitpat(pat, v)) ^ a;
    ((a & 0x80) | (t >> 2)) & 0xff
}

fn unq_weight(d: u32, v: u32, lv: u32) -> u32 {
    let (b, k): (u32, Kind) = level(lv);
    let t: u32 = if k == Kind::Bits {
        spread(v, b, 6)
    } else if b == 0 {
        return d * if k == Kind::Trit { 32 } else { 16 };
    } else {
        let (pat, c): (&[u8], u32) = match (k, b) {
            (Kind::Trit, 1) => (b"0000000", 50),
            (Kind::Trit, 2) => (b"b000b0b", 23),
            (Kind::Trit, _) => (b"cb000cb", 11),
            (_, 1) => (b"0000000", 28),
            _ => (b"b0000b0", 13),
        };
        let a: u32 = if v & 1 != 0 { 0x7f } else { 0 };
        let t: u32 = (d * c + bitpat(pat, v)) ^ a;
        (a & 0x20) | (t >> 2)
    };
    if t > 32 { t + 1 } else { t }
}

fn hash52(seed: u32) -> u32 {
    let mut p: u32 = seed;
    p ^= p >> 15;
    p = p.wrapping_sub(p << 17);
    p = p.wrapping_add(p << 7);
    p = p.wrapping_add(p << 4);
    p ^= p >> 5;
    p = p.wrapping_add(p << 16);
    p ^= p >> 7;
    p ^= p >> 3;
    p ^= p << 6;
    p ^= p >> 17;
    p
}

fn partition(seed: u32, x: u32, y: u32, pc: u32, small: bool) -> usize {
    let (x, y): (u32, u32) = if small { (x << 1, y << 1) } else { (x, y) };
    let seed: u32 = seed + (pc - 1) * 1024;
    let r: u32 = hash52(seed);
    let mut s: [u32; 12] = [
        r & 0xf,
        (r >> 4) & 0xf,
        (r >> 8) & 0xf,
        (r >> 12) & 0xf,
        (r >> 16) & 0xf,
        (r >> 20) & 0xf,
        (r >> 24) & 0xf,
        (r >> 28) & 0xf,
        (r >> 18) & 0xf,
        (r >> 22) & 0xf,
        (r >> 26) & 0xf,
        ((r >> 30) | (r << 2)) & 0xf,
    ];
    for v in s.iter_mut() {
        *v *= *v;
    }
    let (sh1, sh2): (u32, u32) = if seed & 1 != 0 {
        (if seed & 2 != 0 { 4 } else { 5 }, if pc == 3 { 6 } else { 5 })
    } else {
        (if pc == 3 { 6 } else { 5 }, if seed & 2 != 0 { 4 } else { 5 })
    };
    let sh3: u32 = if seed & 0x10 != 0 { sh1 } else { sh2 };
    for (i, v) in s.iter_mut().enumerate() {
        *v >>= if i >= 8 {
            sh3
        } else if i % 2 == 0 {
            sh1
        } else {
            sh2
        };
    }
    let a: u32 = (s[0] * x + s[1] * y + (r >> 14)) & 0x3f;
    let b: u32 = (s[2] * x + s[3] * y + (r >> 10)) & 0x3f;
    let c: u32 = if pc < 3 { 0 } else { (s[4] * x + s[5] * y + (r >> 6)) & 0x3f };
    let d: u32 = if pc < 4 { 0 } else { (s[6] * x + s[7] * y + (r >> 2)) & 0x3f };
    if a >= b && a >= c && a >= d {
        0
    } else if b >= c && b >= d {
        1
    } else if c >= d {
        2
    } else {
        3
    }
}

fn clamp8(v: i32) -> i32 {
    v.clamp(0, 255)
}

fn bts(a: i32, b: i32) -> (i32, i32) {
    let b: i32 = (b >> 1) | (a & 0x80);
    let mut a: i32 = (a >> 1) & 0x3f;
    if a & 0x20 != 0 {
        a -= 0x40;
    }
    (a, b)
}

fn blue(c: [i32; 4]) -> [i32; 4] {
    [(c[0] + c[2]) >> 1, (c[1] + c[2]) >> 1, c[2], c[3]]
}

fn endpoints(cem: u32, v: &[i32]) -> Option<Ends> {
    Some(match cem {
        0 => ([v[0], v[0], v[0], 255], [v[1], v[1], v[1], 255]),
        1 => {
            let l0: i32 = (v[0] >> 2) | (v[1] & 0xc0);
            let l1: i32 = clamp8(l0 + (v[1] & 0x3f));
            ([l0, l0, l0, 255], [l1, l1, l1, 255])
        }
        4 => ([v[0], v[0], v[0], v[2]], [v[1], v[1], v[1], v[3]]),
        5 => {
            let (a1, a0): (i32, i32) = bts(v[1], v[0]);
            let (a3, a2): (i32, i32) = bts(v[3], v[2]);
            let l1: i32 = clamp8(a0 + a1);
            ([a0, a0, a0, a2], [l1, l1, l1, clamp8(a2 + a3)])
        }
        6 | 10 => {
            let (al0, al1): (i32, i32) = if cem == 10 { (v[4], v[5]) } else { (255, 255) };
            ([(v[0] * v[3]) >> 8, (v[1] * v[3]) >> 8, (v[2] * v[3]) >> 8, al0], [v[0], v[1], v[2], al1])
        }
        8 | 12 => {
            let (al0, al1): (i32, i32) = if cem == 12 { (v[6], v[7]) } else { (255, 255) };
            if v[1] + v[3] + v[5] >= v[0] + v[2] + v[4] {
                ([v[0], v[2], v[4], al0], [v[1], v[3], v[5], al1])
            } else {
                (blue([v[1], v[3], v[5], al1]), blue([v[0], v[2], v[4], al0]))
            }
        }
        9 | 13 => {
            let (o0, b0): (i32, i32) = bts(v[1], v[0]);
            let (o1, b1): (i32, i32) = bts(v[3], v[2]);
            let (o2, b2): (i32, i32) = bts(v[5], v[4]);
            let (oa, ba): (i32, i32) = if cem == 13 { bts(v[7], v[6]) } else { (0, 255) };
            let e0: [i32; 4] = [b0, b1, b2, ba];
            let e1: [i32; 4] = [clamp8(b0 + o0), clamp8(b1 + o1), clamp8(b2 + o2), clamp8(ba + oa)];
            if o0 + o1 + o2 < 0 { (blue(e1), blue(e0)) } else { (e0, e1) }
        }
        _ => return None,
    })
}

fn block_mode(m: u32) -> Option<(usize, usize, bool, u32)> {
    let mut base: u32 = (m >> 4) & 1;
    let mut high: u32 = (m >> 9) & 1;
    let mut dual: bool = (m >> 10) & 1 != 0;
    let a: u32 = (m >> 5) & 3;
    let (xw, yw): (u32, u32);
    if m & 3 != 0 {
        base |= (m & 3) << 1;
        let b: u32 = (m >> 7) & 3;
        (xw, yw) = match (m >> 2) & 3 {
            0 => (b + 4, a + 2),
            1 => (b + 8, a + 2),
            2 => (a + 2, b + 8),
            _ => {
                if m & 0x100 != 0 {
                    ((b & 1) + 2, a + 2)
                } else {
                    (a + 2, (b & 1) + 6)
                }
            }
        };
    } else {
        base |= ((m >> 2) & 3) << 1;
        if (m >> 2) & 3 == 0 {
            return None;
        }
        let b: u32 = (m >> 9) & 3;
        (xw, yw) = match (m >> 7) & 3 {
            0 => (12, a + 2),
            1 => (a + 2, 12),
            2 => {
                dual = false;
                high = 0;
                (a + 6, b + 6)
            }
            _ => match a {
                0 => (6, 10),
                1 => (10, 6),
                _ => return None,
            },
        };
    }
    if base < 2 {
        return None;
    }
    let lv: u32 = WEIGHT_LEVELS[((base - 2) + 6 * high) as usize];
    Some((xw as usize, yw as usize, dual, lv))
}

fn infill(wts: &[u32], plane: usize, planes: usize, xw: usize, yw: usize, bw: usize, bh: usize, out: &mut [u32]) {
    let ds: usize = (1024 + bw / 2) / (bw - 1).max(1);
    let dt: usize = (1024 + bh / 2) / (bh - 1).max(1);
    let n: usize = xw * yw;
    let g = |i: usize| wts[i.min(n - 1) * planes + plane];
    for t in 0..bh {
        for s in 0..bw {
            let gs: usize = (ds * s * (xw - 1) + 32) >> 6;
            let gt: usize = (dt * t * (yw - 1) + 32) >> 6;
            let (js, fs): (usize, u32) = (gs >> 4, (gs & 15) as u32);
            let (jt, ft): (usize, u32) = (gt >> 4, (gt & 15) as u32);
            let v0: usize = js + jt * xw;
            let w11: u32 = (fs * ft + 8) >> 4;
            let w10: u32 = ft - w11;
            let w01: u32 = fs - w11;
            let w00: u32 = 16 + w11 - fs - ft;
            let right: bool = js + 1 < xw;
            let down: bool = jt + 1 < yw;
            let p00: u32 = g(v0);
            let p01: u32 = if right { g(v0 + 1) } else { p00 };
            let p10: u32 = if down { g(v0 + xw) } else { p00 };
            let p11: u32 = if right && down {
                g(v0 + xw + 1)
            } else if right {
                p01
            } else {
                p10
            };
            out[t * bw + s] = (p00 * w00 + p01 * w01 + p10 * w10 + p11 * w11 + 8) >> 4;
        }
    }
}

pub fn decode_block(data: &[u8; 16], bw: usize, bh: usize, srgb: bool, out: &mut [[u8; 4]]) -> bool {
    let texels: usize = bw * bh;
    if bw < 2 || bh < 2 || texels > MAX_TEXELS || out.len() < texels {
        return false;
    }
    let x: u128 = u128::from_le_bytes(*data);
    let m: u32 = rd(x, 0, 11);
    if m & 0x1ff == 0x1fc {
        if (m >> 9) & 1 != 0 {
            return false;
        }
        let c: [u8; 4] = [(rd(x, 64, 16) >> 8) as u8, (rd(x, 80, 16) >> 8) as u8, (rd(x, 96, 16) >> 8) as u8, (rd(x, 112, 16) >> 8) as u8];
        out[..texels].fill(c);
        return true;
    }
    let Some((xw, yw, dual, wlv)) = block_mode(m) else {
        return false;
    };
    let parts: usize = rd(x, 11, 2) as usize + 1;
    let planes: usize = if dual { 2 } else { 1 };
    let nw: usize = xw * yw * planes;
    if (dual && parts == 4) || nw > 64 || xw > bw || yw > bh {
        return false;
    }
    let wbits: u32 = ise_bits(nw as u32, wlv);
    if !(24..=96).contains(&wbits) {
        return false;
    }
    let mut raw: [Pair; 64] = [(0, 0); 64];
    ise(x.reverse_bits() & ((1u128 << wbits) - 1), nw, wlv, &mut raw);
    let mut wts: [u32; 64] = [0; 64];
    for i in 0..nw {
        wts[i] = unq_weight(raw[i].0, raw[i].1, wlv);
    }
    let mut below: u32 = 128 - wbits;
    let mut ccs: usize = 0;
    if dual {
        below -= 2;
        ccs = rd(x, below, 2) as usize;
    }
    let mut cems: [u32; 4] = [0; 4];
    let start: u32;
    if parts == 1 {
        cems[0] = rd(x, 13, 4);
        start = 17;
    } else {
        start = 29;
        let mut enc: u32 = rd(x, 23, 6);
        let base: u32 = enc & 3;
        if base == 0 {
            cems[..parts].fill((enc >> 2) & 0xf);
        } else {
            let extra: u32 = 3 * parts as u32 - 4;
            if below < extra {
                return false;
            }
            below -= extra;
            enc |= rd(x, below, extra) << 6;
            for (i, c) in cems.iter_mut().enumerate().take(parts) {
                let cs: u32 = (enc >> (2 + i as u32)) & 1;
                let mm: u32 = (enc >> (2 + parts as u32 + 2 * i as u32)) & 3;
                *c = ((base - 1 + cs) << 2) | mm;
            }
        }
    }
    let nvals: u32 = cems[..parts].iter().map(|c| 2 * ((c >> 2) + 1)).sum();
    if nvals > 18 || below < start {
        return false;
    }
    let avail: u32 = below - start;
    let Some(clv) = COLOR_LEVELS.iter().copied().find(|lv| ise_bits(nvals, *lv) <= avail) else {
        return false;
    };
    let mut craw: [Pair; 64] = [(0, 0); 64];
    ise(x >> start & ((1u128 << ise_bits(nvals, clv)) - 1), nvals as usize, clv, &mut craw);
    let mut cvals: [i32; 18] = [0; 18];
    for i in 0..nvals as usize {
        cvals[i] = unq_color(craw[i].0, craw[i].1, clv) as i32;
    }
    let mut eps: [Ends; 4] = [([0; 4], [0; 4]); 4];
    let mut pos: usize = 0;
    for i in 0..parts {
        let n: usize = 2 * ((cems[i] >> 2) as usize + 1);
        let Some(e) = endpoints(cems[i], &cvals[pos..pos + n]) else {
            return false;
        };
        eps[i] = e;
        pos += n;
    }
    let mut w0: [u32; MAX_TEXELS] = [0; MAX_TEXELS];
    let mut w1: [u32; MAX_TEXELS] = [0; MAX_TEXELS];
    infill(&wts, 0, planes, xw, yw, bw, bh, &mut w0);
    if dual {
        infill(&wts, 1, planes, xw, yw, bw, bh, &mut w1);
    }
    let seed: u32 = rd(x, 13, 10);
    for i in 0..texels {
        let p: usize = if parts > 1 { partition(seed, (i % bw) as u32, (i / bw) as u32, parts as u32, texels < 31) } else { 0 };
        let (e0, e1): Ends = eps[p];
        let mut px: [u8; 4] = [0; 4];
        for ch in 0..4 {
            let w: u32 = if dual && ch == ccs { w1[i] } else { w0[i] };
            let wide = |c: i32| if srgb && ch < 3 { ((c as u32) << 8) | 0x80 } else { c as u32 * 257 };
            px[ch] = (((wide(e0[ch]) * (64 - w) + wide(e1[ch]) * w + 32) >> 6) >> 8) as u8;
        }
        out[i] = px;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(x: &mut u128, pos: u32, n: u32, v: u32) {
        *x |= ((v as u128) & ((1u128 << n) - 1)) << pos;
    }

    fn run(x: u128, srgb: bool) -> Option<Vec<[u8; 4]>> {
        let mut out: [[u8; 4]; 16] = [[0; 4]; 16];
        decode_block(&x.to_le_bytes(), 4, 4, srgb, &mut out).then(|| out.to_vec())
    }

    #[test]
    fn a_void_extent_block_is_one_flat_colour() {
        let mut x: u128 = 0;
        put(&mut x, 0, 9, 0x1fc);
        put(&mut x, 12, 32, 0xffff_ffff);
        put(&mut x, 44, 20, 0xfffff);
        for (i, v) in [0x1234u32, 0x8080, 0xfe00, 0x7fff].iter().enumerate() {
            put(&mut x, 64 + 16 * i as u32, 16, *v);
        }
        let px: Vec<[u8; 4]> = run(x, false).unwrap();
        assert!(px.iter().all(|p| *p == [0x12, 0x80, 0xfe, 0x7f]));
        put(&mut x, 9, 1, 1);
        assert!(run(x, false).is_none());
    }

    #[test]
    fn an_rgb_block_follows_its_weight_bits() {
        let mut x: u128 = 0;
        put(&mut x, 0, 11, 0x42);
        put(&mut x, 13, 4, 8);
        for (i, v) in [10u32, 200, 20, 210, 30, 220].iter().enumerate() {
            put(&mut x, 17 + 8 * i as u32, 8, *v);
        }
        let ones: [usize; 6] = [1, 2, 5, 8, 14, 15];
        for i in ones {
            put(&mut x, 126 - 2 * i as u32, 2, 3);
        }
        let px: Vec<[u8; 4]> = run(x, false).unwrap();
        for (i, p) in px.iter().enumerate() {
            let want: [u8; 4] = if ones.contains(&i) { [200, 210, 220, 255] } else { [10, 20, 30, 255] };
            assert_eq!(*p, want, "{i}");
        }
        assert_eq!(run(x, true).unwrap(), px);
    }

    #[test]
    fn swapped_endpoints_use_the_blue_contraction() {
        let mut x: u128 = 0;
        put(&mut x, 0, 11, 0x42);
        put(&mut x, 13, 4, 8);
        for (i, v) in [200u32, 10, 210, 20, 220, 30].iter().enumerate() {
            put(&mut x, 17 + 8 * i as u32, 8, *v);
        }
        let px: Vec<[u8; 4]> = run(x, false).unwrap();
        assert_eq!(px[0], [20, 25, 30, 255]);
    }

    #[test]
    fn a_four_level_luminance_alpha_block_interpolates() {
        let mut x: u128 = 0;
        put(&mut x, 0, 11, 0x42);
        put(&mut x, 13, 4, 4);
        for (i, v) in [0u32, 255, 0, 255].iter().enumerate() {
            put(&mut x, 17 + 8 * i as u32, 8, *v);
        }
        for (i, w) in [0u32, 1, 2, 3].iter().enumerate() {
            put(&mut x, 127 - 2 * i as u32, 1, w & 1);
            put(&mut x, 126 - 2 * i as u32, 1, (w >> 1) & 1);
        }
        let px: Vec<[u8; 4]> = run(x, false).unwrap();
        assert_eq!(px[0], [0, 0, 0, 0]);
        assert_eq!(px[1], [84, 84, 84, 84]);
        assert_eq!(px[2], [171, 171, 171, 171]);
        assert_eq!(px[3], [255, 255, 255, 255]);
    }

    #[test]
    fn reserved_modes_and_wrong_sizes_are_refused() {
        assert!(run(0, false).is_none());
        let mut out: [[u8; 4]; 4] = [[0; 4]; 4];
        assert!(!decode_block(&[0; 16], 4, 4, false, &mut out));
        let mut big: Vec<[u8; 4]> = vec![[0; 4]; 200];
        assert!(!decode_block(&[0; 16], 14, 14, false, &mut big));
        assert!(!decode_block(&[0; 16], 1, 4, false, &mut big));
    }

    #[test]
    fn the_unquantised_tables_cover_their_full_range() {
        for lv in COLOR_LEVELS {
            let (b, k): (u32, Kind) = level(lv);
            let ds: u32 = match k {
                Kind::Bits => 1,
                Kind::Trit => 3,
                Kind::Quint => 5,
            };
            let mut vals: Vec<u32> = Vec::new();
            for d in 0..ds {
                for v in 0..(1u32 << b) {
                    vals.push(unq_color(d, v, lv));
                }
            }
            vals.sort_unstable();
            vals.dedup();
            assert_eq!(vals.len() as u32, lv, "{lv}");
            assert_eq!((vals[0], vals[vals.len() - 1]), (0, 255), "{lv}");
        }
        for lv in WEIGHT_LEVELS {
            let (b, k): (u32, Kind) = level(lv);
            let ds: u32 = match k {
                Kind::Bits => 1,
                Kind::Trit => 3,
                Kind::Quint => 5,
            };
            let mut vals: Vec<u32> = Vec::new();
            for d in 0..ds {
                for v in 0..(1u32 << b) {
                    vals.push(unq_weight(d, v, lv));
                }
            }
            vals.sort_unstable();
            vals.dedup();
            assert_eq!(vals.len() as u32, lv, "{lv}");
            assert_eq!((vals[0], vals[vals.len() - 1]), (0, 64), "{lv}");
        }
    }

    #[test]
    fn trits_and_quints_decode_every_valid_group() {
        let mut seen: std::collections::HashSet<[u32; 5]> = std::collections::HashSet::new();
        for t in 0..256 {
            let d: [u32; 5] = trits(t);
            assert!(d.iter().all(|v| *v < 3), "{t}");
            seen.insert(d);
        }
        assert_eq!(seen.len(), 243);
        let mut qs: std::collections::HashSet<[u32; 3]> = std::collections::HashSet::new();
        for q in 0..128 {
            let d: [u32; 3] = quints(q);
            assert!(d.iter().all(|v| *v < 5), "{q}");
            qs.insert(d);
        }
        assert_eq!(qs.len(), 125);
    }
}
