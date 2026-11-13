use crate::logic_items::ItemId;
use crate::world::{LocId, World};

pub struct Grid {
    pub kinds: Vec<ItemId>,
    pub kind_of: Vec<usize>,
    pub ok: Vec<Vec<bool>>,
}

impl Grid {
    pub fn new(pool: &[ItemId], open: &[LocId], allow: &dyn Fn(ItemId, LocId) -> bool) -> Grid {
        let mut kinds: Vec<ItemId> = Vec::new();
        let mut kind_of: Vec<usize> = Vec::with_capacity(pool.len());
        for i in pool {
            let k: usize = match kinds.iter().position(|x| x == i) {
                Some(k) => k,
                None => {
                    kinds.push(*i);
                    kinds.len() - 1
                }
            };
            kind_of.push(k);
        }
        let ok: Vec<Vec<bool>> = kinds.iter().map(|i| open.iter().map(|l| allow(*i, *l)).collect()).collect();
        Grid { kinds, kind_of, ok }
    }

    pub fn fits(&self, p: usize, j: usize) -> bool {
        self.ok[self.kind_of[p]][j]
    }

    pub fn room(&self, p: usize) -> usize {
        self.ok[self.kind_of[p]].iter().filter(|x| **x).count()
    }
}

#[derive(Clone)]
pub struct Fit<'a> {
    g: &'a Grid,
    item_at: Vec<Option<usize>>,
    loc_has: Vec<Option<usize>>,
    gone_item: Vec<bool>,
    gone_loc: Vec<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Short {
    pub items: Vec<ItemId>,
    pub spots: usize,
    pub kinds: Vec<String>,
}

impl<'a> Fit<'a> {
    pub fn new(g: &'a Grid) -> Fit<'a> {
        let np: usize = g.kind_of.len();
        let nl: usize = g.ok.first().map(|r| r.len()).unwrap_or(0);
        let mut f: Fit<'a> = Fit { g, item_at: vec![None; np], loc_has: vec![None; nl], gone_item: vec![false; np], gone_loc: vec![false; nl] };
        let mut order: Vec<usize> = (0..np).collect();
        order.sort_by_key(|p| g.room(*p));
        for p in order {
            let mut seen: Vec<bool> = vec![false; nl];
            f.augment(p, &mut seen);
        }
        f
    }

    fn augment(&mut self, p: usize, seen: &mut Vec<bool>) -> bool {
        let nl: usize = self.loc_has.len();
        for j in 0..nl {
            if seen[j] || self.gone_loc[j] || !self.g.fits(p, j) {
                continue;
            }
            seen[j] = true;
            let free: bool = match self.loc_has[j] {
                None => true,
                Some(q) => self.augment(q, seen),
            };
            if free {
                self.loc_has[j] = Some(p);
                self.item_at[p] = Some(j);
                return true;
            }
        }
        false
    }

    pub fn full(&self) -> bool {
        (0..self.item_at.len()).all(|p| self.gone_item[p] || self.item_at[p].is_some())
    }

    pub fn take(&mut self, p: usize, j: usize) -> bool {
        if self.gone_item[p] || self.gone_loc[j] || !self.g.fits(p, j) {
            return false;
        }
        let keep: Fit<'a> = self.clone();
        let was: Option<usize> = self.item_at[p];
        let held: Option<usize> = self.loc_has[j];
        self.gone_item[p] = true;
        self.gone_loc[j] = true;
        self.item_at[p] = Some(j);
        self.loc_has[j] = Some(p);
        if let Some(m) = was
            && m != j
        {
            self.loc_has[m] = None;
        }
        if let Some(q) = held
            && q != p
        {
            self.item_at[q] = None;
            let mut seen: Vec<bool> = vec![false; self.loc_has.len()];
            if !self.augment(q, &mut seen) {
                *self = keep;
                return false;
            }
        }
        true
    }

    pub fn short(&self, w: &World) -> Option<Short> {
        let np: usize = self.item_at.len();
        let nl: usize = self.loc_has.len();
        let mut in_s: Vec<bool> = vec![false; np];
        let mut in_n: Vec<bool> = vec![false; nl];
        let mut stack: Vec<usize> = (0..np).filter(|p| !self.gone_item[*p] && self.item_at[*p].is_none()).collect();
        if stack.is_empty() {
            return None;
        }
        for p in &stack {
            in_s[*p] = true;
        }
        while let Some(p) = stack.pop() {
            for j in 0..nl {
                if in_n[j] || self.gone_loc[j] || !self.g.fits(p, j) {
                    continue;
                }
                in_n[j] = true;
                if let Some(q) = self.loc_has[j]
                    && !in_s[q]
                {
                    in_s[q] = true;
                    stack.push(q);
                }
            }
        }
        let items: Vec<ItemId> = (0..np).filter(|p| in_s[*p]).map(|p| self.g.kinds[self.g.kind_of[p]]).collect();
        let mut kinds: Vec<String> = Vec::new();
        for i in &items {
            let n: &String = &w.items.names[*i];
            if !kinds.contains(n) {
                kinds.push(n.clone());
            }
        }
        Some(Short { items, spots: in_n.iter().filter(|x| **x).count(), kinds })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(rows: &[&[bool]]) -> Grid {
        let kinds: Vec<ItemId> = (0..rows.len()).collect();
        Grid { kinds: kinds.clone(), kind_of: kinds, ok: rows.iter().map(|r| r.to_vec()).collect() }
    }

    #[test]
    fn a_perfect_matching_is_found_when_greedy_would_miss_it() {
        let g: Grid = grid(&[&[true, true], &[true, false]]);
        let f: Fit = Fit::new(&g);
        assert!(f.full());
    }

    #[test]
    fn a_take_that_would_strand_an_item_is_refused_and_undone() {
        let g: Grid = grid(&[&[true, true], &[true, false]]);
        let mut f: Fit = Fit::new(&g);
        assert!(!f.take(0, 0));
        assert!(f.full());
        assert!(f.take(0, 1));
        assert!(f.take(1, 0));
        assert!(f.full());
    }

    #[test]
    fn hall_shortfall_names_the_crowded_items() {
        let g: Grid = grid(&[&[true, false, false], &[true, false, false], &[false, true, true]]);
        let f: Fit = Fit::new(&g);
        assert!(!f.full());
        let np: usize = f.item_at.len();
        let left: Vec<usize> = (0..np).filter(|p| f.item_at[*p].is_none()).collect();
        assert_eq!(left.len(), 1);
        assert!(left[0] < 2);
    }
}
