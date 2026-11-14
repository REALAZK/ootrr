use std::collections::hash_map::RandomState;
use std::fs::File;
use std::hash::{BuildHasher, DefaultHasher, Hasher};
use std::io::Read;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

const ALPHABET: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";

fn os_bytes(n: usize) -> Vec<u8> {
    let mut b: Vec<u8> = vec![0; n];
    let ok: bool = File::open("/dev/urandom").map(|mut f| f.read_exact(&mut b).is_ok()).unwrap_or(false);
    if ok { b } else { Vec::new() }
}

fn mixed(i: u64) -> u64 {
    let t: u128 = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let mut h: DefaultHasher = RandomState::new().build_hasher();
    h.write_u64(i);
    h.write_u128(t);
    h.write_u32(if cfg!(target_os = "wasi") { 0 } else { process::id() });
    h.finish()
}

pub fn random_bytes(n: usize) -> Vec<u8> {
    let mut out: Vec<u8> = os_bytes(n);
    let mut i: u64 = 0;
    while out.len() < n {
        out.extend_from_slice(&mixed(i).to_le_bytes());
        i += 1;
    }
    out.truncate(n);
    out
}

pub fn token() -> String {
    random_bytes(16).iter().map(|b| format!("{b:02x}")).collect()
}

pub fn seed_text(n: usize) -> String {
    random_bytes(n)
        .iter()
        .map(|b| ALPHABET[*b as usize % ALPHABET.len()] as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_long_hex_and_differ() {
        let a: String = token();
        let b: String = token();
        assert_eq!(a.len(), 32);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    #[test]
    fn seed_text_uses_the_alphabet() {
        let s: String = seed_text(12);
        assert_eq!(s.len(), 12);
        assert!(s.bytes().all(|c| ALPHABET.contains(&c)));
        assert_ne!(seed_text(12), seed_text(12));
    }

    #[test]
    fn random_bytes_has_the_requested_length() {
        for n in [0usize, 1, 7, 8, 9, 64] {
            assert_eq!(random_bytes(n).len(), n);
        }
    }
}
