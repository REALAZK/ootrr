use super::{COLOR_LEVELS, Ends, Kind, Pair, block_mode, endpoints, ise, ise_bits, level, quints, rd, trits, unq_color, unq_weight};

const TRIT_BITS: [u32; 5] = [2, 2, 1, 2, 1];
const QUINT_BITS: [u32; 3] = [3, 2, 2];
const PASSES: usize = 4;
const REACH: usize = 2;

pub struct Quant {
    vals: Vec<(i32, Pair)>,
    near: Vec<usize>,
}

pub struct Tables {
    q: Vec<Quant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Done {
    Skipped,
    Same,
    Changed,
}

fn digits(k: Kind) -> u32 {
    match k {
        Kind::Bits => 1,
        Kind::Trit => 3,
        Kind::Quint => 5,
    }
}

fn quant(lv: u32) -> Quant {
    let (b, k): (u32, Kind) = level(lv);
    let mut vals: Vec<(i32, Pair)> = Vec::new();
    for d in 0..digits(k) {
        for m in 0..(1u32 << b) {
            vals.push((unq_color(d, m, lv) as i32, (d, m)));
        }
    }
    vals.sort_unstable();
    let mut near: Vec<usize> = Vec::with_capacity(256);
    for want in 0..256i32 {
        let mut best: usize = 0;
        for (i, v) in vals.iter().enumerate() {
            if (v.0 - want).abs() < (vals[best].0 - want).abs() {
                best = i;
            }
        }
        near.push(best);
    }
    Quant { vals, near }
}

impl Tables {
    pub fn new() -> Tables {
        Tables { q: COLOR_LEVELS.iter().map(|lv| quant(*lv)).collect() }
    }

    fn of(&self, lv: u32) -> Option<&Quant> {
        COLOR_LEVELS.iter().position(|l| *l == lv).map(|i| &self.q[i])
    }
}

impl Default for Tables {
    fn default() -> Tables {
        Tables::new()
    }
}

impl Quant {
    fn find(&self, p: Pair) -> usize {
        self.vals.iter().position(|v| v.1 == p).unwrap_or(0)
    }

    fn pick(&self, want: i32) -> usize {
        self.near[want.clamp(0, 255) as usize]
    }
}

fn put(out: &mut u128, pos: &mut u32, v: u32, n: u32) {
    if n > 0 && *pos < 128 {
        *out |= ((v as u128) & ((1u128 << n) - 1)) << *pos;
    }
    *pos += n;
}

fn pack(ps: &[Pair], lv: u32) -> Option<u128> {
    let (b, k): (u32, Kind) = level(lv);
    let mut out: u128 = 0;
    let mut pos: u32 = 0;
    match k {
        Kind::Bits => {
            for p in ps {
                put(&mut out, &mut pos, p.1, b);
            }
        }
        Kind::Trit => {
            for g in ps.chunks(5) {
                let mut want: [u32; 5] = [0; 5];
                for (i, p) in g.iter().enumerate() {
                    want[i] = p.0;
                }
                let t: u32 = (0..256u32).find(|t| trits(*t) == want)?;
                let mut tb: u32 = 0;
                for (j, n) in TRIT_BITS.iter().enumerate() {
                    put(&mut out, &mut pos, g.get(j).map(|p| p.1).unwrap_or(0), b);
                    put(&mut out, &mut pos, t >> tb, *n);
                    tb += *n;
                }
            }
        }
        Kind::Quint => {
            for g in ps.chunks(3) {
                let mut want: [u32; 3] = [0; 3];
                for (i, p) in g.iter().enumerate() {
                    want[i] = p.0;
                }
                let q: u32 = (0..128u32).find(|q| quints(*q) == want)?;
                let mut qb: u32 = 0;
                for (j, n) in QUINT_BITS.iter().enumerate() {
                    put(&mut out, &mut pos, g.get(j).map(|p| p.1).unwrap_or(0), b);
                    put(&mut out, &mut pos, q >> qb, *n);
                    qb += *n;
                }
            }
        }
    }
    let n: u32 = ise_bits(ps.len() as u32, lv);
    if n < 128 && out >> n != 0 {
        return None;
    }
    Some(out)
}

fn mirror(p: Pair, lv: u32) -> Option<Pair> {
    let (b, k): (u32, Kind) = level(lv);
    let want: u32 = 64 - unq_weight(p.0, p.1, lv);
    for d in 0..digits(k) {
        for m in 0..(1u32 << b) {
            if unq_weight(d, m, lv) == want {
                return Some((d, m));
            }
        }
    }
    None
}

fn grey(c: &[i32; 4]) -> i32 {
    (c[0] + c[1] + c[2] + 1) / 3
}

fn unblue(c: &[i32; 4]) -> [i32; 4] {
    [(2 * c[0] - c[2]).clamp(0, 255), (2 * c[1] - c[2]).clamp(0, 255), c[2], c[3]]
}

fn offset(base: i32, to: i32) -> (i32, i32) {
    let off: i32 = (to - base).clamp(-32, 31);
    (((base << 1) & 0xff) | 1, (base & 0x80) | ((off & 0x3f) << 1))
}

fn guess(cem: u32, w: &Ends) -> ([[i32; 8]; 2], usize) {
    let (a, b): (&[i32; 4], &[i32; 4]) = (&w.0, &w.1);
    let mut g: [[i32; 8]; 2] = [[0; 8]; 2];
    let mut n: usize = 1;
    match cem {
        0 | 4 => g[0] = [grey(a), grey(b), a[3], b[3], 0, 0, 0, 0],
        1 => {
            let l: i32 = grey(a);
            g[0][0] = ((l & 0x3f) << 2) | 2;
            g[0][1] = (l & 0xc0) | (grey(b) - l).clamp(0, 63);
        }
        5 => {
            (g[0][0], g[0][1]) = offset(grey(a), grey(b));
            (g[0][2], g[0][3]) = offset(a[3], b[3]);
        }
        6 | 10 => {
            let dot: i32 = b[0] * b[0] + b[1] * b[1] + b[2] * b[2];
            let s: i32 = if dot > 0 { ((256 * (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]) + dot / 2) / dot).clamp(0, 255) } else { 0 };
            g[0] = [b[0], b[1], b[2], s, a[3], b[3], 0, 0];
        }
        8 | 12 => {
            let (ua, ub): ([i32; 4], [i32; 4]) = (unblue(a), unblue(b));
            g[0] = [a[0], b[0], a[1], b[1], a[2], b[2], a[3], b[3]];
            g[1] = [ub[0], ua[0], ub[1], ua[1], ub[2], ua[2], b[3], a[3]];
            n = 2;
        }
        _ => {
            let (ua, ub): ([i32; 4], [i32; 4]) = (unblue(a), unblue(b));
            for c in 0..3 {
                (g[0][2 * c], g[0][2 * c + 1]) = offset(a[c], b[c]);
                (g[1][2 * c], g[1][2 * c + 1]) = offset(ub[c], ua[c]);
            }
            (g[0][6], g[0][7]) = offset(a[3], b[3]);
            (g[1][6], g[1][7]) = offset(b[3], a[3]);
            n = 2;
        }
    }
    (g, n)
}

fn wide(cem: u32, i: usize) -> bool {
    matches!(cem, 1 | 5 | 9 | 13) && i % 2 == 1
}

fn miss(cem: u32, q: &Quant, idx: &[usize], w: &Ends) -> u32 {
    let mut v: [i32; 8] = [0; 8];
    for (i, x) in idx.iter().enumerate() {
        v[i] = q.vals[*x].0;
    }
    let Some((a, b)) = endpoints(cem, &v[..idx.len()]) else {
        return u32::MAX;
    };
    let mut e: u32 = 0;
    for c in 0..4 {
        e += ((a[c] - w.0[c]) * (a[c] - w.0[c]) + (b[c] - w.1[c]) * (b[c] - w.1[c])) as u32;
    }
    e
}

fn refine(cem: u32, q: &Quant, idx: &mut [usize], w: &Ends) -> u32 {
    let mut best: u32 = miss(cem, q, idx, w);
    for _ in 0..PASSES {
        let before: u32 = best;
        for i in 0..idx.len() {
            let (lo, hi): (usize, usize) = if wide(cem, i) { (0, q.vals.len() - 1) } else { (idx[i].saturating_sub(REACH), (idx[i] + REACH).min(q.vals.len() - 1)) };
            let keep: usize = idx[i];
            let mut pick: usize = keep;
            for x in lo..=hi {
                idx[i] = x;
                let e: u32 = miss(cem, q, idx, w);
                if e < best {
                    best = e;
                    pick = x;
                }
            }
            idx[i] = pick;
        }
        if best == before || best == 0 {
            break;
        }
    }
    best
}

fn fit(cem: u32, q: &Quant, cur: &[usize], w: &Ends, keep: bool) -> (u32, [usize; 8]) {
    let n: usize = cur.len();
    let mut out: [usize; 8] = [0; 8];
    let mut best: u32 = u32::MAX;
    if keep {
        out[..n].copy_from_slice(cur);
        best = refine(cem, q, &mut out[..n], w);
    }
    let (gs, m): ([[i32; 8]; 2], usize) = guess(cem, w);
    for g in gs.iter().take(m) {
        let mut idx: [usize; 8] = [0; 8];
        for i in 0..n {
            idx[i] = q.pick(g[i]);
        }
        let e: u32 = refine(cem, q, &mut idx[..n], w);
        if e < best {
            best = e;
            out = idx;
        }
    }
    (best, out)
}

fn void(data: &mut [u8; 16], f: &dyn Fn([u8; 3]) -> [u8; 3]) -> Done {
    let old: [u8; 3] = [data[9], data[11], data[13]];
    let new: [u8; 3] = f(old);
    if new == old {
        return Done::Same;
    }
    for c in 0..3 {
        data[8 + 2 * c] = new[c];
        data[9 + 2 * c] = new[c];
    }
    Done::Changed
}

fn shade(c: &[i32; 4], f: &dyn Fn([u8; 3]) -> [u8; 3]) -> [i32; 4] {
    let n: [u8; 3] = f([c[0] as u8, c[1] as u8, c[2] as u8]);
    [n[0] as i32, n[1] as i32, n[2] as i32, c[3]]
}

pub fn recolor_block(data: &mut [u8; 16], tb: &Tables, f: &dyn Fn([u8; 3]) -> [u8; 3]) -> Done {
    let x: u128 = u128::from_le_bytes(*data);
    let m: u32 = rd(x, 0, 11);
    if m & 0x1ff == 0x1fc {
        if (m >> 9) & 1 != 0 {
            return Done::Skipped;
        }
        return void(data, f);
    }
    let Some((xw, yw, dual, wlv)) = block_mode(m) else {
        return Done::Skipped;
    };
    let parts: usize = rd(x, 11, 2) as usize + 1;
    let planes: usize = if dual { 2 } else { 1 };
    let nw: usize = xw * yw * planes;
    if (dual && parts == 4) || nw > 64 {
        return Done::Skipped;
    }
    let wbits: u32 = ise_bits(nw as u32, wlv);
    if !(24..=96).contains(&wbits) {
        return Done::Skipped;
    }
    let mut below: u32 = 128 - wbits;
    if dual {
        below -= 2;
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
                return Done::Skipped;
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
        return Done::Skipped;
    }
    let Some(clv) = COLOR_LEVELS.iter().copied().find(|lv| ise_bits(nvals, *lv) <= below - start) else {
        return Done::Skipped;
    };
    let Some(q) = tb.of(clv) else {
        return Done::Skipped;
    };
    let cbits: u32 = ise_bits(nvals, clv);
    let mut craw: [Pair; 64] = [(0, 0); 64];
    ise(x >> start & ((1u128 << cbits) - 1), nvals as usize, clv, &mut craw);
    let mut cur: [usize; 18] = [0; 18];
    let mut vals: [i32; 18] = [0; 18];
    for i in 0..nvals as usize {
        cur[i] = q.find(craw[i]);
        vals[i] = q.vals[cur[i]].0;
    }
    let mut want: [Ends; 4] = [([0; 4], [0; 4]); 4];
    let mut same: bool = true;
    let mut pos: usize = 0;
    for i in 0..parts {
        let n: usize = 2 * ((cems[i] >> 2) as usize + 1);
        let Some((a, b)) = endpoints(cems[i], &vals[pos..pos + n]) else {
            return Done::Skipped;
        };
        want[i] = (shade(&a, f), shade(&b, f));
        same &= want[i] == (a, b);
        pos += n;
    }
    if same {
        return Done::Same;
    }
    let step: u32 = 256 / clv;
    let tol: u32 = parts as u32 * 6 * (step / 2 + 2) * (step / 2 + 2);
    let mut best: (u32, [usize; 18], bool) = (u32::MAX, [0; 18], false);
    for flip in [false, true] {
        let mut total: u32 = 0;
        let mut idx: [usize; 18] = [0; 18];
        let mut pos: usize = 0;
        for i in 0..parts {
            let n: usize = 2 * ((cems[i] >> 2) as usize + 1);
            let w: Ends = if flip { (want[i].1, want[i].0) } else { want[i] };
            let (e, got): (u32, [usize; 8]) = fit(cems[i], q, &cur[pos..pos + n], &w, !flip);
            total = total.saturating_add(e);
            idx[pos..pos + n].copy_from_slice(&got[..n]);
            pos += n;
        }
        if total < best.0 {
            best = (total, idx, flip);
        }
        if best.0 <= tol {
            break;
        }
    }
    let mut ps: [Pair; 18] = [(0, 0); 18];
    for i in 0..nvals as usize {
        ps[i] = q.vals[best.1[i]].1;
    }
    let Some(bits) = pack(&ps[..nvals as usize], clv) else {
        return Done::Skipped;
    };
    let mut y: u128 = x & !(((1u128 << cbits) - 1) << start) | (bits << start);
    if best.2 {
        let mut raw: [Pair; 64] = [(0, 0); 64];
        ise(x.reverse_bits() & ((1u128 << wbits) - 1), nw, wlv, &mut raw);
        for p in raw.iter_mut().take(nw) {
            let Some(n) = mirror(*p, wlv) else {
                return Done::Skipped;
            };
            *p = n;
        }
        let Some(w) = pack(&raw[..nw], wlv) else {
            return Done::Skipped;
        };
        y = (y & ((1u128 << (128 - wbits)) - 1)) | w.reverse_bits();
    }
    if y == x {
        return Done::Same;
    }
    *data = y.to_le_bytes();
    Done::Changed
}

#[cfg(test)]
mod tests {
    use super::super::decode_block;
    use super::*;

    fn set(x: &mut u128, pos: u32, n: u32, v: u32) {
        *x |= ((v as u128) & ((1u128 << n) - 1)) << pos;
    }

    fn rgb(vals: [u32; 6], weights: &[(usize, u32)]) -> [u8; 16] {
        let mut x: u128 = 0;
        set(&mut x, 0, 11, 0x42);
        set(&mut x, 13, 4, 8);
        for (i, v) in vals.iter().enumerate() {
            set(&mut x, 17 + 8 * i as u32, 8, *v);
        }
        for (i, w) in weights {
            set(&mut x, 127 - 2 * *i as u32, 1, w & 1);
            set(&mut x, 126 - 2 * *i as u32, 1, (w >> 1) & 1);
        }
        x.to_le_bytes()
    }

    fn px(b: &[u8; 16]) -> Vec<[u8; 4]> {
        let mut out: [[u8; 4]; 16] = [[0; 4]; 16];
        assert!(decode_block(b, 4, 4, false, &mut out));
        out.to_vec()
    }

    fn swap(c: [u8; 3]) -> [u8; 3] {
        [c[1], c[2], c[0]]
    }

    fn close(a: &[[u8; 4]], b: &[[u8; 4]], by: i32) -> bool {
        a.iter().zip(b).all(|(p, q)| (0..4).all(|c| (p[c] as i32 - q[c] as i32).abs() <= by))
    }

    #[test]
    fn an_unchanged_colour_map_leaves_the_bytes_alone() {
        let tb: Tables = Tables::new();
        let mut b: [u8; 16] = rgb([10, 200, 20, 210, 30, 220], &[(1, 3), (4, 1), (9, 2)]);
        let old: [u8; 16] = b;
        assert_eq!(recolor_block(&mut b, &tb, &|c| c), Done::Same);
        assert_eq!(b, old);
    }

    #[test]
    fn a_channel_swap_moves_every_texel_of_a_direct_block() {
        let tb: Tables = Tables::new();
        let mut b: [u8; 16] = rgb([10, 200, 20, 210, 30, 220], &[(1, 3), (4, 1), (9, 2), (15, 3)]);
        let old: Vec<[u8; 4]> = px(&b);
        assert_eq!(recolor_block(&mut b, &tb, &swap), Done::Changed);
        let want: Vec<[u8; 4]> = old.iter().map(|p| [p[1], p[2], p[0], p[3]]).collect();
        assert_eq!(px(&b), want);
    }

    #[test]
    fn a_blue_contracted_block_is_rebuilt_with_mirrored_weights() {
        let tb: Tables = Tables::new();
        let mut b: [u8; 16] = rgb([250, 10, 120, 20, 60, 30], &[(0, 3), (5, 1), (10, 2)]);
        let old: Vec<[u8; 4]> = px(&b);
        let red = |c: [u8; 3]| [c[0].saturating_add(c[1] / 2), c[1] / 4, c[2] / 4];
        assert_eq!(recolor_block(&mut b, &tb, &red), Done::Changed);
        let want: Vec<[u8; 4]> = old.iter().map(|p| { let n: [u8; 3] = red([p[0], p[1], p[2]]); [n[0], n[1], n[2], p[3]] }).collect();
        assert!(close(&px(&b), &want, 3), "{:?} {:?}", px(&b), want);
    }

    #[test]
    fn a_void_block_takes_the_new_colour_and_keeps_its_alpha() {
        let tb: Tables = Tables::new();
        let mut x: u128 = 0;
        set(&mut x, 0, 9, 0x1fc);
        set(&mut x, 12, 32, 0xffff_ffff);
        set(&mut x, 44, 20, 0xfffff);
        for (i, v) in [0x1234u32, 0x8080, 0xfe00, 0x7fff].iter().enumerate() {
            set(&mut x, 64 + 16 * i as u32, 16, *v);
        }
        let mut b: [u8; 16] = x.to_le_bytes();
        assert_eq!(recolor_block(&mut b, &tb, &swap), Done::Changed);
        assert!(px(&b).iter().all(|p| *p == [0x80, 0xfe, 0x12, 0x7f]));
        let mut z: [u8; 16] = [0; 16];
        assert_eq!(recolor_block(&mut z, &tb, &swap), Done::Skipped);
    }

    #[test]
    fn packing_undoes_the_integer_sequence_reader_at_every_level() {
        for lv in COLOR_LEVELS {
            let (b, k): (u32, Kind) = level(lv);
            for n in (1..=18usize).filter(|n| ise_bits(*n as u32, lv) < 128) {
                let ps: Vec<Pair> = (0..n).map(|i| ((i as u32 * 7 + lv) % digits(k), (i as u32 * 13 + 5) % (1 << b))).collect();
                let bits: u128 = pack(&ps, lv).unwrap();
                assert!(bits >> ise_bits(n as u32, lv) == 0, "{lv} {n}");
                let mut back: [Pair; 64] = [(0, 0); 64];
                ise(bits, n, lv, &mut back);
                assert_eq!(&back[..n], &ps[..], "{lv} {n}");
            }
        }
    }

    #[test]
    fn every_weight_has_a_mirror_at_its_own_level() {
        for lv in super::super::WEIGHT_LEVELS {
            let (b, k): (u32, Kind) = level(lv);
            for d in 0..digits(k) {
                for m in 0..(1u32 << b) {
                    let p: Pair = mirror((d, m), lv).unwrap();
                    assert_eq!(unq_weight(p.0, p.1, lv), 64 - unq_weight(d, m, lv));
                }
            }
        }
    }

    #[test]
    fn offset_and_scale_modes_reach_a_rotated_target() {
        let tb: Tables = Tables::new();
        let q: &Quant = tb.of(256).unwrap();
        for cem in [6u32, 9, 13, 10, 12, 0, 1, 4, 5] {
            let n: usize = 2 * ((cem >> 2) as usize + 1);
            let seedv: [i32; 8] = [120, 140, 90, 200, 60, 70, 255, 255];
            let cur: Vec<usize> = (0..n).map(|i| q.pick(seedv[i])).collect();
            let (a, b): Ends = endpoints(cem, &seedv[..n]).unwrap();
            let w: Ends = ([a[1], a[2], a[0], a[3]], [b[1], b[2], b[0], b[3]]);
            let direct: u32 = fit(cem, q, &cur, &w, true).0;
            let turned: u32 = fit(cem, q, &cur, &(w.1, w.0), false).0;
            assert!(direct.min(turned) <= 12, "{cem} {direct} {turned}");
        }
    }
}
