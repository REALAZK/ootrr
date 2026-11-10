use std::collections::HashMap as Map;

use super::kind::Kind;

pub const MAGIC: u32 = 0x0102_0304;
pub const HEAD: usize = 0x28;
pub const SENTINEL: u32 = 0xa3db_7114;

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub hash: u32,
    pub val: u32,
    pub kind: Kind,
    pub seg: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Save {
    pub head: Vec<u8>,
    pub rows: Vec<Row>,
    pub keys: Vec<u64>,
    pub tail: Vec<u8>,
    by_hash: Map<u32, usize>,
}

impl Save {
    pub fn new(head: Vec<u8>, rows: Vec<Row>, keys: Vec<u64>, tail: Vec<u8>) -> Save {
        let mut by_hash: Map<u32, usize> = Map::with_capacity(rows.len());
        for (i, r) in rows.iter().enumerate() {
            if r.kind != Kind::Sep {
                by_hash.entry(r.hash).or_insert(i);
            }
        }
        Save {
            head,
            rows,
            keys,
            tail,
            by_hash,
        }
    }

    pub fn version(&self) -> u32 {
        u32::from_le_bytes([self.head[4], self.head[5], self.head[6], self.head[7]])
    }

    pub fn index_of(&self, h: u32) -> Option<usize> {
        self.by_hash.get(&h).copied()
    }

    pub fn row(&self, h: u32) -> Option<&Row> {
        self.index_of(h).map(|i| &self.rows[i])
    }

    pub fn row_mut(&mut self, h: u32) -> Option<&mut Row> {
        let i: usize = self.index_of(h)?;
        Some(&mut self.rows[i])
    }

    pub fn has_key(&self, k: u64) -> bool {
        self.keys.binary_search(&k).is_ok()
    }

    pub fn set_key(&mut self, k: u64, on: bool) -> bool {
        match (self.keys.binary_search(&k), on) {
            (Err(i), true) => {
                self.keys.insert(i, k);
                true
            }
            (Ok(i), false) => {
                self.keys.remove(i);
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_stay_sorted_and_unique() {
        let mut s: Save = Save::new(vec![0; HEAD], Vec::new(), vec![2, 8], Vec::new());
        assert!(s.set_key(5, true));
        assert!(!s.set_key(5, true));
        assert_eq!(s.keys, vec![2, 5, 8]);
        assert!(s.set_key(2, false));
        assert!(!s.set_key(2, false));
        assert!(s.has_key(8) && !s.has_key(2));
    }
}
