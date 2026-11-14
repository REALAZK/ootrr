use super::node::guid;
use super::Plug;
use super::tab::{INSZ, OUTSZ, PSZ};
use super::{Ainb, EL};
use crate::error::{Error, Res};

const FLAG: u32 = 0x8000_0000;
const LINKS: usize = 0x24;
const STR: usize = 3;
const PTR: usize = 5;

fn r32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

fn r16(b: &[u8], o: usize) -> i16 {
    i16::from_le_bytes([b[o], b[o + 1]])
}

fn w16(b: &mut [u8], o: usize, v: i16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}

fn pool(src: &Ainb, dst: &mut Ainb, v: u32) -> Res<u32> {
    let s: &str = src.pool.get(v & !FLAG).ok_or(Error::Format("ainb graft: string outside the pool"))?;
    Ok(dst.pool.add(s) | (v & FLAG))
}

fn slice(v: &[u8], b: u32, n: u32, sz: usize) -> Res<Vec<u8>> {
    let (b, n): (usize, usize) = (b as usize, n as usize);
    Ok(v.get(b * sz..(b + n) * sz).ok_or(Error::Format("ainb graft: entry range out of bounds"))?.to_vec())
}

pub fn graft(dst: &mut Ainb, src: &Ainb, ids: &[usize], seed: &str) -> Res<Vec<usize>> {
    let first: usize = dst.els.len();
    if first + ids.len() >= 0xffff || ids.iter().any(|&i| i >= src.els.len()) {
        return Err(Error::Format("ainb graft: bad element list"));
    }
    let map = |i: usize| ids.iter().position(|&x| x == i).map(|p| first + p);
    let mut links: usize = dst.qids.len() / 4;
    let mut fresh: Vec<usize> = Vec::new();
    for (n, &t) in ids.iter().enumerate() {
        let mut e: [u8; EL] = src.els[t];
        w16(&mut e, 2, (first + n) as i16);
        let nm: u32 = pool(src, dst, r32(&e, 8))?;
        w32(&mut e, 8, nm);
        e[0x2c..0x3c].copy_from_slice(&guid(&format!("{seed}#{n}")));
        let mut p = src.params[t].clone();
        let consumes: usize = u16::from_le_bytes([e[0x26], e[0x27]]) as usize;
        if consumes > 0 {
            w16(&mut e, LINKS, links as i16);
            links += consumes;
            for k in 0..consumes {
                dst.qids.extend_from_slice(&((links - consumes + k) as u32).to_le_bytes());
            }
        }
        for k in 0..6 {
            let mut pr: Vec<u8> = slice(&src.props[k], p.pe[k][0], p.pe[k][1], PSZ[k])?;
            for c in pr.chunks_mut(PSZ[k]) {
                let a: u32 = pool(src, dst, r32(c, 0))?;
                w32(c, 0, a);
                if k == STR {
                    let v: u32 = pool(src, dst, r32(c, 8))?;
                    w32(c, 8, v);
                }
            }
            p.pe[k][0] = (dst.props[k].len() / PSZ[k]) as u32;
            dst.props[k].extend(pr);
            let mut ins: Vec<u8> = slice(&src.ins[k], p.ioe[k][0], p.ioe[k][1], INSZ[k])?;
            for c in ins.chunks_mut(INSZ[k]) {
                let a: u32 = pool(src, dst, r32(c, 0))?;
                w32(c, 0, a);
                let at: usize = if k == PTR { 8 } else { 4 };
                let from: i16 = r16(c, at);
                if from >= 0 {
                    let to: usize = map(from as usize).ok_or(Error::Format("ainb graft: an input reads from outside the copied nodes"))?;
                    w16(c, at, to as i16);
                } else if k == STR {
                    let v: u32 = pool(src, dst, r32(c, 12))?;
                    w32(c, 12, v);
                }
                if k == PTR {
                    let v: u32 = pool(src, dst, r32(c, 4))?;
                    w32(c, 4, v);
                }
            }
            p.ioe[k][0] = (dst.ins[k].len() / INSZ[k]) as u32;
            dst.ins[k].extend(ins);
            let mut outs: Vec<u8> = slice(&src.outs[k], p.ioe[k][2], p.ioe[k][3], OUTSZ[k])?;
            for c in outs.chunks_mut(OUTSZ[k]) {
                let a: u32 = pool(src, dst, r32(c, 0))?;
                w32(c, 0, a);
                if k == PTR {
                    let v: u32 = pool(src, dst, r32(c, 4))?;
                    w32(c, 4, v);
                }
            }
            p.ioe[k][2] = (dst.outs[k].len() / OUTSZ[k]) as u32;
            dst.outs[k].extend(outs);
        }
        for g in p.plugs.iter_mut() {
            g.a = map(g.a as usize).ok_or(Error::Format("ainb graft: a plug leaves the copied nodes"))? as u32;
            g.b = pool(src, dst, g.b)?;
        }
        dst.els.push(e);
        dst.params.push(p);
        fresh.push(first + n);
    }
    Ok(fresh)
}

pub const MAKE: &str = "QueryGenericMakeOverwriteParam";
pub const ADD: &str = "QueryBlackboardOverwriteParamAddNoRespawnGmdHash";
pub const SET: &str = "OneShotDropSetOverWriteParam";
const CHILD: usize = 2;

pub fn root(a: &Ainb) -> Option<usize> {
    let c: &[u8] = a.cmds.get(0x14..0x18)?;
    let i: usize = u32::from_le_bytes([c[0], c[1], c[2], c[3]]) as usize;
    (i < a.els.len()).then_some(i)
}

pub fn hand_hash(pot: &mut Ainb, skull: &Ainb) -> Res<Vec<usize>> {
    let top: usize = root(pot).ok_or(Error::Format("ainb graft: the graph has no root"))?;
    let one = |n: &str| skull.find_els(n).first().copied().ok_or(Error::Format("ainb graft: the source graph lacks the hash chain"));
    let ids: [usize; 3] = [one(MAKE)?, one(ADD)?, one(SET)?];
    if !pot.find_els(SET).is_empty() {
        return Err(Error::Format("ainb graft: the graph already hands its hash to its drops"));
    }
    let out: Vec<usize> = graft(pot, skull, &ids, "hand_hash")?;
    let name: u32 = pot.pool.add("");
    pot.params[top].add_plug(CHILD, 0, Plug { a: out[2] as u32, b: name, extra: Vec::new() })?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::super::audit;
    use super::*;

    fn pair() -> (Ainb, Ainb) {
        (super::super::tests::synth(), super::super::tests::synth())
    }

    #[test]
    fn a_copied_node_lands_at_the_end_and_the_graph_stays_whole() {
        let (mut d, s) = pair();
        let before: usize = d.els.len();
        let ids: Vec<usize> = vec![1, 2];
        let out: Vec<usize> = graft(&mut d, &s, &ids, "t").unwrap();
        assert_eq!(out, vec![before, before + 1]);
        assert!(audit(&d).is_empty());
        let b: Vec<u8> = d.write().unwrap();
        let c: Ainb = Ainb::parse(&b).unwrap();
        assert_eq!(c.els.len(), before + 2);
        assert_eq!(c.el_name(before), d.el_name(before));
        assert!(audit(&c).is_empty());
    }

    #[test]
    fn a_node_that_reads_from_a_left_out_node_is_refused() {
        let (mut d, mut s) = pair();
        let x: usize = super::super::tests::node(&mut s, "Reader", 0, &[(0, "In", 1i32.to_le_bytes().to_vec())]);
        let g: usize = s.find_in(x, 0, "In").unwrap();
        s.ins[0][g * INSZ[0] + 4..g * INSZ[0] + 6].copy_from_slice(&1i16.to_le_bytes());
        assert!(graft(&mut d, &s, &[x], "t").is_err());
    }
}
