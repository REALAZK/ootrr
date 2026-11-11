use super::param::GROUPS;
use super::tab::{INSZ, OUTSZ, PSZ, VALSZ};
use super::{Ainb, Param, EL};
use crate::error::{Error, Res};
use crate::murmur::murmur3_32;

pub const BOOL: usize = 1;
pub const ONCE: &str = "ExecuteEveryFrame";

pub struct In<'a> {
    pub k: usize,
    pub name: &'a str,
    pub flags: u32,
    pub val: Vec<u8>,
}

pub fn guid(seed: &str) -> [u8; 16] {
    let mut g: [u8; 16] = [0; 16];
    for j in 0..4 {
        g[4 * j..4 * j + 4].copy_from_slice(&murmur3_32(seed.as_bytes(), j as u32 + 1).to_le_bytes());
    }
    g
}

pub fn new_node(a: &mut Ainb, ty: &str, seed: &str, ins: &[In]) -> Res<usize> {
    let id: usize = a.els.len();
    if id >= 0xffff {
        return Err(Error::Format("ainb: too many elements"));
    }
    for x in ins {
        if x.k >= 5 || x.val.len() != VALSZ[x.k] {
            return Err(Error::Format("ainb: bad input for a new node"));
        }
    }
    let mut e: [u8; EL] = [0; EL];
    e[2..4].copy_from_slice(&(id as u16).to_le_bytes());
    e[8..12].copy_from_slice(&a.pool.add(ty).to_le_bytes());
    e[12..16].copy_from_slice(&murmur3_32(ty.as_bytes(), 0).to_le_bytes());
    e[0x2c..0x3c].copy_from_slice(&guid(seed));
    let mut p: Param = Param { pe: [[0; 2]; 6], ioe: [[0; 4]; 6], pl: [[0; 2]; GROUPS], plugs: Vec::new() };
    for k in 0..6 {
        p.pe[k][0] = (a.props[k].len() / PSZ[k]) as u32;
        p.ioe[k][0] = (a.ins[k].len() / INSZ[k]) as u32;
        p.ioe[k][2] = (a.outs[k].len() / OUTSZ[k]) as u32;
    }
    let pn: u32 = a.pool.add(ONCE);
    a.props[BOOL].extend_from_slice(&pn.to_le_bytes());
    a.props[BOOL].extend_from_slice(&[0; 8]);
    p.pe[BOOL][1] = 1;
    for x in ins {
        let nm: u32 = a.pool.add(x.name);
        a.ins[x.k].extend_from_slice(&nm.to_le_bytes());
        a.ins[x.k].extend_from_slice(&(-1i16).to_le_bytes());
        a.ins[x.k].extend_from_slice(&0i16.to_le_bytes());
        a.ins[x.k].extend_from_slice(&x.flags.to_le_bytes());
        a.ins[x.k].extend_from_slice(&x.val);
        p.ioe[x.k][1] += 1;
    }
    a.els.push(e);
    a.params.push(p);
    Ok(id)
}
