mod audit;
mod grow;
mod node;
mod param;
mod pool;
mod rd;
pub mod show;
pub mod start;
mod tab;
mod view;

use std::vec::IntoIter;

use crate::error::{Error, Res};
use rd::Rd;
use tab::{INSZ, OUTSZ, PSZ};

pub use param::{Param, Plug};
pub use audit::audit;
pub use pool::Pool;
pub use view::Input;

pub const VERSION: u32 = 0x408;
pub const HDR: usize = 0x74;
pub const EL: usize = 0x3c;

#[derive(Clone, Debug, PartialEq)]
pub struct Ainb {
    pub hdr: [u8; HDR],
    pub cmds: Vec<u8>,
    pub els: Vec<[u8; EL]>,
    pub params: Vec<Param>,
    pub bb: Vec<u8>,
    pub props: [Vec<u8>; 6],
    pub ins: [Vec<u8>; 6],
    pub outs: [Vec<u8>; 6],
    pub jump_first: bool,
    pub jump: Vec<u8>,
    pub multi: Vec<u8>,
    pub h50: Vec<u8>,
    pub qids: Vec<u8>,
    pub exb: Option<Vec<u8>>,
    pub modc: Vec<u8>,
    pub ext: Vec<u8>,
    pub mcl: Vec<u8>,
    pub repl: Vec<u8>,
    pub h6c: Vec<u8>,
    pub enm: Vec<u8>,
    pub pool: Pool,
}

fn nx(it: &mut IntoIter<Vec<u8>>) -> Res<Vec<u8>> {
    it.next().ok_or(Error::Format("ainb: missing section"))
}

fn arr<const N: usize>(b: &[u8]) -> Res<[u8; N]> {
    b.try_into().map_err(|_| Error::Format("ainb: bad record size"))
}

fn put(out: &mut [u8], o: usize, v: usize) {
    out[o..o + 4].copy_from_slice(&(v as u32).to_le_bytes());
}

impl Ainb {
    pub fn parse(b: &[u8]) -> Res<Ainb> {
        let r: Rd = Rd::new(b);
        if r.bytes(0, 4)? != b"AIB " {
            return Err(Error::Format("ainb: bad magic"));
        }
        if r.u32(4)? != VERSION {
            return Err(Error::Format("ainb: unsupported version"));
        }
        if r.u32(0x18)? != 0 {
            return Err(Error::Format("ainb: attachments are not supported"));
        }
        let hdr: [u8; HDR] = arr(r.bytes(0, HDR)?)?;
        let nc: usize = r.ptr(0xc)?;
        let ne: usize = r.ptr(0x10)?;
        let cmds: Vec<u8> = r.bytes(HDR, 0x18 * nc)?.to_vec();
        let eb: usize = HDR + 0x18 * nc;
        let mut els: Vec<[u8; EL]> = Vec::new();
        let mut pp: Vec<usize> = Vec::new();
        for i in 0..ne {
            let o: usize = eb + EL * i;
            els.push(arr(r.bytes(o, EL)?)?);
            pp.push(r.ptr(o + 0x14)?);
        }
        let bbp: usize = r.ptr(0x20)?;
        if bbp != eb + EL * ne {
            return Err(Error::Format("ainb: blackboard does not follow the elements"));
        }
        let prop: usize = r.ptr(0x2c)?;
        let io: usize = r.ptr(0x34)?;
        if r.ptr(0x3c)? != prop || r.ptr(0x40)? != prop {
            return Err(Error::Format("ainb: unexpected attachment table position"));
        }
        let bb: Vec<u8> = r.span(bbp, pp.first().copied().unwrap_or(prop))?.to_vec();
        let mut params: Vec<Param> = Vec::new();
        for i in 0..ne {
            params.push(Param::parse(&r, pp[i], pp.get(i + 1).copied().unwrap_or(prop))?);
        }
        let mut pt: [usize; 6] = [0; 6];
        for k in 0..6 {
            pt[k] = r.ptr(prop + 4 * k)?;
        }
        if pt[0] != prop + 0x18 {
            return Err(Error::Format("ainb: property arrays do not follow the table"));
        }
        let mut props: [Vec<u8>; 6] = std::array::from_fn(|_| Vec::new());
        for k in 0..6 {
            let e: usize = if k < 5 { pt[k + 1] } else { io };
            props[k] = r.span(pt[k], e)?.to_vec();
            if props[k].len() % PSZ[k] != 0 {
                return Err(Error::Format("ainb: property array size mismatch"));
            }
        }
        let jp: usize = r.ptr(0x30)?;
        let mp: usize = r.ptr(0x38)?;
        let sm: usize = els.iter().map(|e| u16::from_le_bytes([e[0x1c], e[0x1d]]) as usize).sum();
        let jump_first: bool = jp < mp || (jp == mp && sm > 0);
        let ex: usize = r.ptr(0x44)?;
        let mut ord: Vec<usize> = if jump_first { vec![jp, mp] } else { vec![mp, jp] };
        ord.push(r.ptr(0x50)?);
        ord.push(r.ptr(0x4c)?);
        if ex != 0 {
            ord.push(ex);
        }
        for o in [0x5c, 0x68, 0x70, 0x48, 0x6c, 0x28, 0x24] {
            ord.push(r.ptr(o)?);
        }
        ord.push(r.len());
        if ord[0] < io {
            return Err(Error::Format("ainb: sections overlap the input table"));
        }
        let mut ip: [usize; 12] = [0; 12];
        for j in 0..12 {
            ip[j] = r.ptr(io + 4 * j)?;
        }
        if ip[0] != io + 0x30 {
            return Err(Error::Format("ainb: input arrays do not follow the table"));
        }
        let mut ins: [Vec<u8>; 6] = std::array::from_fn(|_| Vec::new());
        let mut outs: [Vec<u8>; 6] = std::array::from_fn(|_| Vec::new());
        for j in 0..12 {
            let e: usize = if j < 11 { ip[j + 1] } else { ord[0] };
            let d: Vec<u8> = r.span(ip[j], e)?.to_vec();
            let k: usize = j / 2;
            if j % 2 == 0 {
                if d.len() % INSZ[k] != 0 {
                    return Err(Error::Format("ainb: input array size mismatch"));
                }
                ins[k] = d;
            } else {
                if d.len() % OUTSZ[k] != 0 {
                    return Err(Error::Format("ainb: output array size mismatch"));
                }
                outs[k] = d;
            }
        }
        let mut parts: Vec<Vec<u8>> = Vec::new();
        for w in ord.windows(2) {
            parts.push(r.span(w[0], w[1])?.to_vec());
        }
        let mut it: IntoIter<Vec<u8>> = parts.into_iter();
        let first: Vec<u8> = nx(&mut it)?;
        let second: Vec<u8> = nx(&mut it)?;
        let h50: Vec<u8> = nx(&mut it)?;
        let qids: Vec<u8> = nx(&mut it)?;
        let exb: Option<Vec<u8>> = if ex != 0 { Some(nx(&mut it)?) } else { None };
        let modc: Vec<u8> = nx(&mut it)?;
        let ext: Vec<u8> = nx(&mut it)?;
        let mcl: Vec<u8> = nx(&mut it)?;
        let repl: Vec<u8> = nx(&mut it)?;
        let h6c: Vec<u8> = nx(&mut it)?;
        let enm: Vec<u8> = nx(&mut it)?;
        let pool: Vec<u8> = nx(&mut it)?;
        let (jump, multi): (Vec<u8>, Vec<u8>) = if jump_first { (first, second) } else { (second, first) };
        Ok(Ainb {
            hdr,
            cmds,
            els,
            params,
            bb,
            props,
            ins,
            outs,
            jump_first,
            jump,
            multi,
            h50,
            qids,
            exb,
            modc,
            ext,
            mcl,
            repl,
            h6c,
            enm,
            pool: Pool { b: pool },
        })
    }

    pub fn write(&self) -> Res<Vec<u8>> {
        if self.els.len() != self.params.len() {
            return Err(Error::Format("ainb: element and parameter counts differ"));
        }
        let eb: usize = HDR + self.cmds.len();
        let bbp: usize = eb + EL * self.els.len();
        let mut pos: usize = bbp + self.bb.len();
        let mut pps: Vec<usize> = Vec::new();
        for p in &self.params {
            pps.push(pos);
            pos += p.size();
        }
        let prop: usize = pos;
        let io: usize = prop + 0x18 + self.props.iter().map(|v| v.len()).sum::<usize>();
        let ios: usize = self.ins.iter().map(|v| v.len()).sum::<usize>() + self.outs.iter().map(|v| v.len()).sum::<usize>();
        let after: usize = io + 0x30 + ios;
        let (a, c): (&Vec<u8>, &Vec<u8>) = if self.jump_first { (&self.jump, &self.multi) } else { (&self.multi, &self.jump) };
        let mut blobs: Vec<&Vec<u8>> = vec![a, c, &self.h50, &self.qids];
        if let Some(x) = &self.exb {
            blobs.push(x);
        }
        for x in [&self.modc, &self.ext, &self.mcl, &self.repl, &self.h6c, &self.enm, &self.pool.b] {
            blobs.push(x);
        }
        let mut at: Vec<usize> = Vec::new();
        let mut q: usize = after;
        for x in &blobs {
            at.push(q);
            q += x.len();
        }
        let has_exb: bool = self.exb.is_some();
        let w: usize = if has_exb { 5 } else { 4 };
        let mut h: [u8; HDR] = self.hdr;
        put(&mut h, 0x10, self.els.len());
        put(&mut h, 0x20, bbp);
        put(&mut h, 0x24, at[w + 6]);
        put(&mut h, 0x28, at[w + 5]);
        put(&mut h, 0x2c, prop);
        put(&mut h, 0x30, if self.jump_first { at[0] } else { at[1] });
        put(&mut h, 0x34, io);
        put(&mut h, 0x38, if self.jump_first { at[1] } else { at[0] });
        put(&mut h, 0x3c, prop);
        put(&mut h, 0x40, prop);
        put(&mut h, 0x44, if has_exb { at[4] } else { 0 });
        put(&mut h, 0x48, at[w + 3]);
        put(&mut h, 0x4c, at[3]);
        put(&mut h, 0x50, at[2]);
        put(&mut h, 0x5c, at[w]);
        put(&mut h, 0x68, at[w + 1]);
        put(&mut h, 0x6c, at[w + 4]);
        put(&mut h, 0x70, at[w + 2]);
        let mut out: Vec<u8> = Vec::with_capacity(q);
        out.extend_from_slice(&h);
        out.extend_from_slice(&self.cmds);
        for (i, e) in self.els.iter().enumerate() {
            let mut c2: [u8; EL] = *e;
            put(&mut c2, 0x14, pps[i]);
            out.extend_from_slice(&c2);
        }
        out.extend_from_slice(&self.bb);
        for p in &self.params {
            p.write(&mut out);
        }
        let mut q2: usize = prop + 0x18;
        for k in 0..6 {
            out.extend_from_slice(&(q2 as u32).to_le_bytes());
            q2 += self.props[k].len();
        }
        for k in 0..6 {
            out.extend_from_slice(&self.props[k]);
        }
        let mut q3: usize = io + 0x30;
        for k in 0..6 {
            out.extend_from_slice(&(q3 as u32).to_le_bytes());
            q3 += self.ins[k].len();
            out.extend_from_slice(&(q3 as u32).to_le_bytes());
            q3 += self.outs[k].len();
        }
        for k in 0..6 {
            out.extend_from_slice(&self.ins[k]);
            out.extend_from_slice(&self.outs[k]);
        }
        if out.len() != after {
            return Err(Error::Format("ainb: layout mismatch"));
        }
        for x in &blobs {
            out.extend_from_slice(x);
        }
        Ok(out)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::tab::{INSZ, PSZ};
    use super::*;

    fn p32(v: &mut Vec<u8>, x: u32) {
        v.extend_from_slice(&x.to_le_bytes());
    }

    fn blank() -> Param {
        Param { pe: [[0; 2]; 6], ioe: [[0; 4]; 6], pl: [[0; 2]; 10], plugs: Vec::new() }
    }

    pub(crate) fn node(a: &mut Ainb, tn: &str, ty: u16, ins: &[(usize, &str, Vec<u8>)]) -> usize {
        let id: usize = a.els.len();
        let mut e: [u8; EL] = [0; EL];
        e[0..2].copy_from_slice(&ty.to_le_bytes());
        e[2..4].copy_from_slice(&(id as u16).to_le_bytes());
        let nm: u32 = a.pool.add(tn);
        e[8..12].copy_from_slice(&nm.to_le_bytes());
        let mut p: Param = blank();
        if ty == 0 {
            let pn: u32 = a.pool.add("ExecuteEveryFrame");
            p.pe[1] = [(a.props[1].len() / PSZ[1]) as u32, 1];
            p32(&mut a.props[1], pn);
            p32(&mut a.props[1], 0);
            p32(&mut a.props[1], 0);
        }
        for k in 0..6 {
            p.ioe[k][0] = (a.ins[k].len() / INSZ[k]) as u32;
            for (kk, n, v) in ins {
                if *kk == k {
                    let o: u32 = a.pool.add(n);
                    p32(&mut a.ins[k], o);
                    a.ins[k].extend_from_slice(&(-1i16).to_le_bytes());
                    a.ins[k].extend_from_slice(&0i16.to_le_bytes());
                    p32(&mut a.ins[k], 0x80_0000);
                    a.ins[k].extend_from_slice(v);
                    p.ioe[k][1] += 1;
                }
            }
        }
        a.els.push(e);
        a.params.push(p);
        id
    }

    pub(crate) fn synth() -> Ainb {
        let mut h: [u8; HDR] = [0; HDR];
        h[0..4].copy_from_slice(b"AIB ");
        h[4..8].copy_from_slice(&VERSION.to_le_bytes());
        h[0xc..0x10].copy_from_slice(&1u32.to_le_bytes());
        let mut a: Ainb = Ainb {
            hdr: h,
            cmds: vec![0; 0x18],
            els: Vec::new(),
            params: Vec::new(),
            bb: vec![0; 0x38],
            props: std::array::from_fn(|_| Vec::new()),
            ins: std::array::from_fn(|_| Vec::new()),
            outs: std::array::from_fn(|_| Vec::new()),
            jump_first: false,
            jump: Vec::new(),
            multi: Vec::new(),
            h50: Vec::new(),
            qids: Vec::new(),
            exb: None,
            modc: vec![0; 4],
            ext: vec![0; 4],
            mcl: vec![0; 8],
            repl: vec![0, 0, 0, 0, 255, 255, 255, 255],
            h6c: vec![0; 4],
            enm: vec![0; 4],
            pool: Pool::default(),
        };
        a.pool.add("synthetic");
        let empty: u32 = a.pool.add("");
        let root: usize = node(&mut a, "", 3, &[]);
        let w: Vec<u8> = a.pool.add("Work/Actor/Item_Synthetic_A.engine__actor__ActorParam.gyml").to_le_bytes().to_vec();
        let one: Vec<u8> = 1i32.to_le_bytes().to_vec();
        let x: usize = node(&mut a, super::start::PUSH, 0, &[(0, "Num", one), (3, "ActorName", w)]);
        let life: Vec<u8> = 12i32.to_le_bytes().to_vec();
        let ln: Vec<u8> = a.pool.add("Life").to_le_bytes().to_vec();
        let mn: Vec<u8> = a.pool.add("MaxLife").to_le_bytes().to_vec();
        node(&mut a, super::start::SET_INT, 0, &[(0, "Value", life.clone()), (3, "GameDataName", ln)]);
        node(&mut a, super::start::SET_INT, 0, &[(0, "Value", life), (3, "GameDataName", mn)]);
        a.params[root].plugs.push(Plug { a: x as u32, b: empty, extra: Vec::new() });
        a.params[root].plugs.push(Plug { a: 2, b: empty, extra: Vec::new() });
        a.params[root].plugs.push(Plug { a: 3, b: empty, extra: Vec::new() });
        a.params[root].pl[2] = [3, 0];
        for j in 3..10 {
            a.params[root].pl[j] = [0, 3];
        }
        a
    }

    #[test]
    fn synthetic_round_trip() {
        let a: Ainb = synth();
        let b: Vec<u8> = a.write().unwrap();
        let c: Ainb = Ainb::parse(&b).unwrap();
        assert_eq!(c.els.len(), a.els.len());
        assert_eq!(c.params, a.params);
        assert_eq!(c.pool, a.pool);
        assert_eq!(b, c.write().unwrap());
        assert_eq!(Ainb::parse(&c.write().unwrap()).unwrap(), c);
        assert!(audit(&c).is_empty());
    }

    #[test]
    fn rejects_bad_input() {
        assert!(Ainb::parse(b"AIB ").is_err());
        assert!(Ainb::parse(&[0; 200]).is_err());
        let mut b: Vec<u8> = synth().write().unwrap();
        b[4] = 7;
        assert!(Ainb::parse(&b).is_err());
        let ok: Vec<u8> = synth().write().unwrap();
        for n in [ok.len() / 2, HDR, 8] {
            assert!(Ainb::parse(&ok[..n]).is_err());
        }
    }

    #[test]
    fn jump_and_multi_order_survive() {
        let mut a: Ainb = synth();
        a.jump = vec![1, 2, 3, 4];
        let b: Vec<u8> = a.write().unwrap();
        assert_eq!(Ainb::parse(&b).unwrap().write().unwrap(), b);
        a.jump_first = true;
        let c: Vec<u8> = a.write().unwrap();
        assert_eq!(Ainb::parse(&c).unwrap().write().unwrap(), c);
    }

    #[test]
    fn pool_dedupes_and_appends() {
        let mut p: Pool = Pool::default();
        let a: u32 = p.add("alpha");
        let b: u32 = p.add("beta");
        assert_eq!(p.add("alpha"), a);
        assert_eq!(p.get(b), Some("beta"));
        assert_eq!(p.find("gamma"), None);
        assert_eq!(p.add(""), p.b.len() as u32 - 1);
    }
}
