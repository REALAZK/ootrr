use super::tab::{INSZ, PSZ, VALSZ};
use super::Ainb;
use crate::error::{Error, Res};

#[derive(Clone, Debug, PartialEq)]
pub struct Input {
    pub name: u32,
    pub src: i16,
    pub plug: i16,
    pub flags: u32,
    pub val: Vec<u8>,
}

fn rd32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn rd16(b: &[u8], o: usize) -> i16 {
    i16::from_le_bytes([b[o], b[o + 1]])
}

impl Ainb {
    pub fn s(&self, o: u32) -> &str {
        self.pool.get(o).unwrap_or("?")
    }

    pub fn el_type(&self, i: usize) -> u16 {
        u16::from_le_bytes([self.els[i][0], self.els[i][1]])
    }

    pub fn el_name(&self, i: usize) -> &str {
        self.s(rd32(&self.els[i], 8))
    }

    pub fn el_hash(&self, i: usize) -> u32 {
        rd32(&self.els[i], 0xc)
    }

    pub fn find_els(&self, name: &str) -> Vec<usize> {
        (0..self.els.len()).filter(|&i| self.el_type(i) == 0 && self.el_name(i) == name).collect()
    }

    pub fn input_ids(&self, i: usize, k: usize) -> Vec<usize> {
        let b: usize = self.params[i].ioe[k][0] as usize;
        let n: usize = self.params[i].ioe[k][1] as usize;
        (b..b + n).collect()
    }

    pub fn input(&self, k: usize, g: usize) -> Option<Input> {
        let e: &[u8] = self.ins[k].get(g * INSZ[k]..(g + 1) * INSZ[k])?;
        if k == 5 {
            return Some(Input { name: rd32(e, 0), src: rd16(e, 8), plug: rd16(e, 10), flags: rd32(e, 12), val: Vec::new() });
        }
        Some(Input { name: rd32(e, 0), src: rd16(e, 4), plug: rd16(e, 6), flags: rd32(e, 8), val: e[12..12 + VALSZ[k]].to_vec() })
    }

    pub fn find_in(&self, i: usize, k: usize, name: &str) -> Option<usize> {
        self.input_ids(i, k).into_iter().find(|&g| self.input(k, g).is_some_and(|x| self.s(x.name) == name))
    }

    pub fn in_s32(&self, i: usize, name: &str) -> Option<i32> {
        let x: Input = self.input(0, self.find_in(i, 0, name)?)?;
        Some(i32::from_le_bytes([x.val[0], x.val[1], x.val[2], x.val[3]]))
    }

    pub fn in_str(&self, i: usize, name: &str) -> Option<&str> {
        let x: Input = self.input(3, self.find_in(i, 3, name)?)?;
        Some(self.s(rd32(&x.val, 0)))
    }

    pub fn set_in_val(&mut self, k: usize, g: usize, v: &[u8]) -> Res<()> {
        if k >= 5 || v.len() != VALSZ[k] {
            return Err(Error::Format("ainb: bad input value"));
        }
        let o: usize = g * INSZ[k] + 12;
        let d: &mut [u8] = self.ins[k].get_mut(o..o + v.len()).ok_or(Error::Format("ainb: input out of range"))?;
        d.copy_from_slice(v);
        Ok(())
    }

    pub fn prop_name(&self, k: usize, g: usize) -> Option<&str> {
        let e: &[u8] = self.props[k].get(g * PSZ[k]..(g + 1) * PSZ[k])?;
        Some(self.s(rd32(e, 0)))
    }
}
