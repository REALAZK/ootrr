const C1: u32 = 0xCC9E_2D51;
const C2: u32 = 0x1B87_3593;

fn mix(k: u32) -> u32 {
    k.wrapping_mul(C1).rotate_left(15).wrapping_mul(C2)
}

pub fn murmur3_32(data: &[u8], seed: u32) -> u32 {
    let mut h: u32 = seed;
    let (blocks, t): (&[[u8; 4]], &[u8]) = data.as_chunks::<4>();
    for c in blocks {
        h ^= mix(u32::from_le_bytes(*c));
        h = h.rotate_left(13).wrapping_mul(5).wrapping_add(0xE654_6B64);
    }
    let mut k: u32 = 0;
    if t.len() >= 3 {
        k ^= (t[2] as u32) << 16;
    }
    if t.len() >= 2 {
        k ^= (t[1] as u32) << 8;
    }
    if !t.is_empty() {
        k ^= t[0] as u32;
        h ^= mix(k);
    }
    h ^= data.len() as u32;
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^ (h >> 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors() {
        assert_eq!(murmur3_32(b"", 0), 0);
        assert_eq!(murmur3_32(b"", 1), 0x514E_28B7);
        assert_eq!(murmur3_32(b"", 0xFFFF_FFFF), 0x81F1_6F39);
        assert_eq!(murmur3_32(b"test", 0), 0xBA6B_D213);
        assert_eq!(murmur3_32(b"Hello, world!", 0), 0xC036_3E43);
        assert_eq!(
            murmur3_32(b"The quick brown fox jumps over the lazy dog", 0),
            0x2E4F_F723
        );
    }

    #[test]
    fn every_tail_length_differs() {
        let a: u32 = murmur3_32(b"abcd", 0);
        let b: u32 = murmur3_32(b"abcde", 0);
        let c: u32 = murmur3_32(b"abcdef", 0);
        let d: u32 = murmur3_32(b"abcdefg", 0);
        assert!(a != b && b != c && c != d && a != d);
    }
}
