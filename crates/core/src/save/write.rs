use super::model::{HEAD, SENTINEL, Save};

pub fn write(s: &Save) -> Vec<u8> {
    let start: usize = HEAD + 8 * s.rows.len() + 8;
    let mut out: Vec<u8> = Vec::with_capacity(start + (1 << 15));
    out.extend_from_slice(&s.head);
    out[8..12].copy_from_slice(&(start as u32).to_le_bytes());
    let mut pos: usize = start;
    for r in &s.rows {
        out.extend_from_slice(&r.hash.to_le_bytes());
        if r.kind.inline() {
            out.extend_from_slice(&r.val.to_le_bytes());
        } else {
            out.extend_from_slice(&(pos as u32).to_le_bytes());
            pos += r.seg.len();
        }
    }
    out.extend_from_slice(&SENTINEL.to_le_bytes());
    out.extend_from_slice(&(pos as u32).to_le_bytes());
    for r in &s.rows {
        out.extend_from_slice(&r.seg);
    }
    for k in &s.keys {
        out.extend_from_slice(&k.to_le_bytes());
    }
    out.extend_from_slice(&0u64.to_le_bytes());
    out.extend_from_slice(&s.tail);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::fixture::{flag_list, sample};
    use crate::save::read::parse;
    use crate::save::val::Val;

    #[test]
    fn rewrite_is_byte_identical() {
        let b: Vec<u8> = sample();
        let s: Save = parse(&b, &flag_list()).unwrap();
        assert_eq!(write(&s), b);
    }

    #[test]
    fn new_keys_grow_the_file_and_stay_sorted() {
        let b: Vec<u8> = sample();
        let mut s: Save = parse(&b, &flag_list()).unwrap();
        assert!(s.set_key(5, true));
        assert!(!s.set_key(5, true));
        let w: Vec<u8> = write(&s);
        assert_eq!(w.len(), b.len() + 8);
        let back: Save = parse(&w, &flag_list()).unwrap();
        assert_eq!(back.keys, vec![3, 5, 9, 27]);
    }

    #[test]
    fn edits_only_touch_their_own_bytes() {
        let b: Vec<u8> = sample();
        let mut s: Save = parse(&b, &flag_list()).unwrap();
        s.set(crate::save::fixture::h("IsGet.Test_B"), &Val::Bool(true))
            .unwrap();
        let w: Vec<u8> = write(&s);
        let d: usize = b.iter().zip(&w).filter(|(x, y)| x != y).count();
        assert_eq!(w.len(), b.len());
        assert_eq!(d, 1);
    }
}
