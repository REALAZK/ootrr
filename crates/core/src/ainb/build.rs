//! Appends whole nodes to an AI graph: properties, constant or linked inputs,
//! outputs, the query list a node reads from, and child plugs.
//!
//! Record layouts, as found in the 1.1.1 dump:
//! - element byte 6 is a role: 1 query, 2 module call, 4 graph root, 0 otherwise;
//!   composite nodes (parallel, selectors) have an empty name and a zero hash.
//! - an element's u16 pair at 0x24 is (start, count) into the query list, whose
//!   u32 entries hold the position of a source among the file's query nodes.
//! - an output record is the pool offset of its name with bit 31 set once something reads it;
//!   pointer outputs add the pool offset of the class name.
//! - a pointer input is name, class, source, output index, flags, zero.
//! - plug group 0 links each read source with the reader's input name,
//!   plug group 2 lists the children.

use super::node::guid;
use super::param::{Plug, GROUPS};
use super::tab::{INSZ, OUTSZ, PSZ, VALSZ};
use super::{Ainb, Param, EL};
use crate::error::{Error, Res};
use crate::murmur::murmur3_32;

pub const QUERY: u8 = 1;
pub const ROLE: usize = 6;
pub const PTR: usize = 5;
pub const CONST: u32 = 0x80_0000;
pub const LINKED: u32 = 0x8000_0000;

pub enum Src {
    Const(Vec<u8>),
    Link(usize, i16),
}

pub struct NIn<'a> {
    pub k: usize,
    pub name: &'a str,
    pub class: &'a str,
    pub src: Src,
}

pub struct NOut<'a> {
    pub k: usize,
    pub name: &'a str,
    pub class: &'a str,
}

pub struct Spec<'a> {
    pub ty: u16,
    pub name: &'a str,
    pub role: u8,
    pub seed: &'a str,
    pub props: Vec<(usize, &'a str, Vec<u8>)>,
    pub ins: Vec<NIn<'a>>,
    pub outs: Vec<NOut<'a>>,
    pub link_extra: usize,
    pub kids: Vec<(usize, &'a str)>,
}

impl<'a> Spec<'a> {
    pub fn node(name: &'a str, seed: &'a str) -> Spec<'a> {
        Spec { ty: 0, name, role: 0, seed, props: Vec::new(), ins: Vec::new(), outs: Vec::new(), link_extra: 0, kids: Vec::new() }
    }

    pub fn query(name: &'a str, seed: &'a str) -> Spec<'a> {
        let mut s: Spec = Spec::node(name, seed);
        s.role = QUERY;
        s.props.push((1, "OutputOnlyOnEnter", vec![0; 4]));
        s
    }

    pub fn composite(ty: u16, seed: &'a str) -> Spec<'a> {
        let mut s: Spec = Spec::node("", seed);
        s.ty = ty;
        s
    }

    pub fn linked(mut self, k: usize, name: &'a str, from: usize, at: i16) -> Spec<'a> {
        self.ins.push(NIn { k, name, class: "", src: Src::Link(from, at) });
        self
    }

    pub fn linked_ptr(mut self, name: &'a str, class: &'a str, from: usize) -> Spec<'a> {
        self.ins.push(NIn { k: PTR, name, class, src: Src::Link(from, 0) });
        self
    }

    pub fn constant(mut self, k: usize, name: &'a str, v: Vec<u8>) -> Spec<'a> {
        self.ins.push(NIn { k, name, class: "", src: Src::Const(v) });
        self
    }

    pub fn out(mut self, k: usize, name: &'a str) -> Spec<'a> {
        self.outs.push(NOut { k, name, class: "" });
        self
    }

    pub fn out_ptr(mut self, name: &'a str, class: &'a str) -> Spec<'a> {
        self.outs.push(NOut { k: PTR, name, class });
        self
    }

    pub fn prop(mut self, k: usize, name: &'a str, v: Vec<u8>) -> Spec<'a> {
        self.props.push((k, name, v));
        self
    }

    pub fn kid(mut self, el: usize, name: &'a str) -> Spec<'a> {
        self.kids.push((el, name));
        self
    }
}

fn sorted<T>(xs: &[T], k: impl Fn(&T) -> usize, name: impl Fn(&T) -> String) -> bool {
    (0..6).all(|t| {
        let ns: Vec<String> = xs.iter().filter(|x| k(x) == t).map(&name).collect();
        ns.windows(2).all(|w| w[0] < w[1])
    })
}

impl Ainb {
    pub fn role(&self, i: usize) -> u8 {
        self.els[i][ROLE]
    }

    pub fn query_pos(&self, i: usize) -> Option<usize> {
        if i >= self.els.len() || self.role(i) & QUERY == 0 {
            return None;
        }
        Some((0..i).filter(|&j| self.role(j) & QUERY != 0).count())
    }

    pub fn add_node(&mut self, s: &Spec) -> Res<usize> {
        let id: usize = self.els.len();
        if id >= 0xffff {
            return Err(Error::Format("ainb: too many elements"));
        }
        if !sorted(&s.ins, |x| x.k, |x| x.name.to_lowercase()) || !sorted(&s.outs, |x| x.k, |x| x.name.to_lowercase()) || !sorted(&s.props, |x| x.0, |x| x.1.to_lowercase()) {
            return Err(Error::Format("ainb: node fields must be listed in name order"));
        }
        let mut srcs: Vec<usize> = Vec::new();
        for x in &s.ins {
            match &x.src {
                Src::Const(v) => {
                    if x.k >= PTR || v.len() != VALSZ[x.k] {
                        return Err(Error::Format("ainb: bad constant input"));
                    }
                }
                Src::Link(from, _) => {
                    self.query_pos(*from).ok_or(Error::Format("ainb: a linked input must read a query node"))?;
                    if !srcs.contains(from) {
                        srcs.push(*from);
                    }
                }
            }
        }
        for (k, _, v) in &s.props {
            if *k >= 4 || v.len() != PSZ[*k] - 8 {
                return Err(Error::Format("ainb: bad property"));
            }
        }
        for (el, _) in &s.kids {
            if *el >= id {
                return Err(Error::Format("ainb: a child must already exist"));
            }
        }
        let mut e: [u8; EL] = [0; EL];
        e[0..2].copy_from_slice(&s.ty.to_le_bytes());
        e[2..4].copy_from_slice(&(id as u16).to_le_bytes());
        e[ROLE] = s.role;
        e[8..12].copy_from_slice(&self.pool.add(s.name).to_le_bytes());
        let hash: u32 = if s.name.is_empty() { 0 } else { murmur3_32(s.name.as_bytes(), 0) };
        e[12..16].copy_from_slice(&hash.to_le_bytes());
        if !srcs.is_empty() {
            let base: usize = self.qids.len() / 4;
            if base > 0xffff {
                return Err(Error::Format("ainb: query list too long"));
            }
            for &q in &srcs {
                let pos: usize = self.query_pos(q).unwrap_or(0);
                self.qids.extend_from_slice(&(pos as u16).to_le_bytes());
                self.qids.extend_from_slice(&0u16.to_le_bytes());
            }
            e[0x24..0x26].copy_from_slice(&(base as u16).to_le_bytes());
            e[0x26..0x28].copy_from_slice(&(srcs.len() as u16).to_le_bytes());
        }
        e[0x2c..0x3c].copy_from_slice(&guid(s.seed));
        let mut p: Param = Param { pe: [[0; 2]; 6], ioe: [[0; 4]; 6], pl: [[0; 2]; GROUPS], plugs: Vec::new() };
        for k in 0..6 {
            p.pe[k][0] = (self.props[k].len() / PSZ[k]) as u32;
            p.ioe[k][0] = (self.ins[k].len() / INSZ[k]) as u32;
            p.ioe[k][2] = (self.outs[k].len() / OUTSZ[k]) as u32;
        }
        for (k, name, v) in &s.props {
            let nm: u32 = self.pool.add(name);
            self.props[*k].extend_from_slice(&nm.to_le_bytes());
            self.props[*k].extend_from_slice(&0u32.to_le_bytes());
            self.props[*k].extend_from_slice(v);
            p.pe[*k][1] += 1;
        }
        let mut links: Vec<Plug> = Vec::new();
        for x in &s.ins {
            let nm: u32 = self.pool.add(x.name);
            let (src, at, flags, val): (i16, i16, u32, Vec<u8>) = match &x.src {
                Src::Const(v) => (-1, 0, CONST, v.clone()),
                Src::Link(from, at) => {
                    links.push(Plug { a: *from as u32, b: nm, extra: vec![0; s.link_extra] });
                    (*from as i16, *at, 0, vec![0; VALSZ[x.k]])
                }
            };
            let r: &mut Vec<u8> = &mut self.ins[x.k];
            r.extend_from_slice(&nm.to_le_bytes());
            if x.k == PTR {
                let cl: u32 = self.pool.add(x.class);
                let r: &mut Vec<u8> = &mut self.ins[x.k];
                r.extend_from_slice(&cl.to_le_bytes());
                r.extend_from_slice(&src.to_le_bytes());
                r.extend_from_slice(&at.to_le_bytes());
                r.extend_from_slice(&flags.to_le_bytes());
                r.extend_from_slice(&0u32.to_le_bytes());
            } else {
                r.extend_from_slice(&src.to_le_bytes());
                r.extend_from_slice(&at.to_le_bytes());
                r.extend_from_slice(&flags.to_le_bytes());
                r.extend_from_slice(&val);
            }
            p.ioe[x.k][1] += 1;
        }
        for x in &s.outs {
            let nm: u32 = self.pool.add(x.name) | LINKED;
            self.outs[x.k].extend_from_slice(&nm.to_le_bytes());
            if x.k == PTR {
                let cl: u32 = self.pool.add(x.class);
                self.outs[x.k].extend_from_slice(&cl.to_le_bytes());
            }
            p.ioe[x.k][3] += 1;
        }
        let n0: usize = links.len();
        let n2: usize = s.kids.len();
        if n0 + n2 > 255 {
            return Err(Error::Format("ainb: too many plugs"));
        }
        p.plugs = links;
        for (el, name) in &s.kids {
            let nm: u32 = self.pool.add(name);
            p.plugs.push(Plug { a: *el as u32, b: nm, extra: Vec::new() });
        }
        p.pl[0] = [n0 as u8, 0];
        p.pl[1] = [0, n0 as u8];
        p.pl[2] = [n2 as u8, n0 as u8];
        for g in 3..GROUPS {
            p.pl[g] = [0, (n0 + n2) as u8];
        }
        self.els.push(e);
        self.params.push(p);
        Ok(id)
    }

    /// The parent and plug position of the one child plug that points at `child`.
    pub fn parent_of(&self, child: usize) -> Res<(usize, usize)> {
        let mut hits: Vec<(usize, usize)> = Vec::new();
        for (i, p) in self.params.iter().enumerate() {
            let b: usize = p.pl[2][1] as usize;
            for (j, g) in p.group(2).iter().enumerate() {
                if g.a as usize == child {
                    hits.push((i, b + j));
                }
            }
        }
        match hits.as_slice() {
            [one] => Ok(*one),
            [] => Err(Error::Format("ainb: the node has no parent")),
            _ => Err(Error::Format("ainb: the node has more than one parent")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::synth;
    use super::super::audit;
    use super::*;

    #[test]
    fn a_query_chain_round_trips_and_keeps_the_arrays_contiguous() {
        let mut a: Ainb = synth();
        let q: usize = a.add_node(&Spec::query("QueryPhysicsCctGetForwardSpeedMps", "t#1").out(2, "SpeedMps")).unwrap();
        let m: usize = a
            .add_node(&Spec::query("QueryMathMulF32", "t#2").linked(2, "InputA", q, 0).constant(2, "InputB", 1.5f32.to_le_bytes().to_vec()).out(2, "Result"))
            .unwrap();
        let ptr: usize = a.add_node(&Spec::query("QueryEquipmentUserGetStaticEquipmentGameActorLink", "t#3").constant(0, "Slot", vec![0; 4]).out_ptr("Actor", "game::actor::ActorLinkBase")).unwrap();
        let chk: usize = a
            .add_node(&Spec::query("QueryCheckActorFileName", "t#4").linked_ptr("Actor", "game::actor::ActorLinkBase", ptr).out(1, "Bool"))
            .unwrap();
        let x: usize = a.add_node(&Spec::node("ExecutePhysicsCctSetForwardSpeedMps", "t#5").linked(2, "SpeedMps", m, 0)).unwrap();
        let mut sel: Spec = Spec::composite(7, "t#6").linked(1, "Input", chk, 0).kid(x, "True").kid(1, "False");
        sel.link_extra = 8;
        let s: usize = a.add_node(&sel).unwrap();
        assert!(audit(&a).is_empty(), "{:?}", audit(&a));
        assert_eq!(a.query_pos(m), Some(1));
        assert_eq!(a.query_pos(x), None);
        assert_eq!(a.parent_of(x).unwrap(), (s, 1));
        let b: Vec<u8> = a.write().unwrap();
        let c: Ainb = Ainb::parse(&b).unwrap();
        assert_eq!((&c.params, &c.ins, &c.outs, &c.props, &c.qids, &c.pool), (&a.params, &a.ins, &a.outs, &a.props, &a.qids, &a.pool));
        assert_eq!(c.write().unwrap(), b);
        assert_eq!(u32::from_le_bytes([b[0x14], b[0x15], b[0x16], b[0x17]]), 4, "the header counts the query nodes");
        let i =c.input(PTR, c.find_in(chk, PTR, "Actor").unwrap()).unwrap();
        assert_eq!((i.src, i.plug, i.flags), (ptr as i16, 0, 0));
        let k = c.input(2, c.find_in(m, 2, "InputB").unwrap()).unwrap();
        assert_eq!((k.src, k.flags, k.val), (-1, CONST, 1.5f32.to_le_bytes().to_vec()));
        let shown: String = super::super::show::show(&c);
        assert!(shown.contains("out f32 Result flags 0x2"));
        assert!(shown.contains(&format!("plug 1 -> element {x} \"True\"")));
    }

    #[test]
    fn bad_specs_are_refused() {
        let mut a: Ainb = synth();
        assert!(a.add_node(&Spec::node("X", "b#1").linked(2, "A", 1, 0)).is_err());
        assert!(a.add_node(&Spec::node("X", "b#2").constant(2, "B", vec![0; 4]).constant(2, "A", vec![0; 4])).is_err());
        assert!(a.add_node(&Spec::composite(3, "b#3").kid(99, "")).is_err());
        assert!(audit(&a).is_empty());
    }
}
