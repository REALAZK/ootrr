use std::cell::Cell;
use std::collections::{BTreeSet, HashMap};

use crate::error::{Error, Res};

const MAX_DEPTH: usize = 64;
const MAX_NODES: usize = 1 << 24;

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Null,
    Bool(bool),
    I32(i32),
    U32(u32),
    F32(f32),
    I64(i64),
    U64(u64),
    F64(f64),
    Str(String),
    Bin(Vec<u8>),
    Arr(Vec<Node>),
    Map(Vec<(String, Node)>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Byml {
    pub root: Node,
    pub version: u16,
    pub big: bool,
}

impl Node {
    pub fn get(&self, k: &str) -> Option<&Node> {
        match self {
            Node::Map(m) => m.iter().find(|e| e.0 == k).map(|e| &e.1),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, k: &str) -> Option<&mut Node> {
        match self {
            Node::Map(m) => m.iter_mut().find(|e| e.0 == k).map(|e| &mut e.1),
            _ => None,
        }
    }

    pub fn set(&mut self, k: &str, v: Node) -> bool {
        let Node::Map(m) = self else {
            return false;
        };
        if let Some(e) = m.iter_mut().find(|e| e.0 == k) {
            e.1 = v;
            return true;
        }
        let at: usize = m.iter().position(|e| e.0.as_str() > k).unwrap_or(m.len());
        m.insert(at, (k.to_string(), v));
        true
    }

    pub fn remove(&mut self, k: &str) -> bool {
        let Node::Map(m) = self else {
            return false;
        };
        match m.iter().position(|e| e.0 == k) {
            Some(i) => {
                m.remove(i);
                true
            }
            None => false,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Node::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&[Node]> {
        match self {
            Node::Arr(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_arr_mut(&mut self) -> Option<&mut Vec<Node>> {
        match self {
            Node::Arr(a) => Some(a),
            _ => None,
        }
    }
}

fn cut() -> Error {
    Error::Format("byml: truncated or out of range")
}

struct Rd<'a> {
    b: &'a [u8],
    be: bool,
    keys: Vec<String>,
    strs: Vec<String>,
    left: Cell<usize>,
}

impl<'a> Rd<'a> {
    fn bytes(&self, o: usize, n: usize) -> Res<&'a [u8]> {
        let e: usize = o.checked_add(n).ok_or_else(cut)?;
        self.b.get(o..e).ok_or_else(cut)
    }

    fn num(&self, o: usize, n: usize) -> Res<u32> {
        let s: &[u8] = self.bytes(o, n)?;
        let mut v: u32 = 0;
        if self.be {
            for x in s {
                v = (v << 8) | *x as u32;
            }
        } else {
            for x in s.iter().rev() {
                v = (v << 8) | *x as u32;
            }
        }
        Ok(v)
    }

    fn wide(&self, o: usize) -> Res<u64> {
        let a: u64 = self.num(o, 4)? as u64;
        let b: u64 = self.num(o + 4, 4)? as u64;
        Ok(if self.be { (a << 32) | b } else { (b << 32) | a })
    }

    fn table(&self, o: usize) -> Res<Vec<String>> {
        if self.bytes(o, 1)?[0] != 0xC2 {
            return Err(Error::Format("byml: bad table"));
        }
        let n: usize = self.num(o + 1, 3)? as usize;
        let mut out: Vec<String> = Vec::with_capacity(n.min(65536));
        for i in 0..n {
            let a: usize = o + self.num(o + 4 + 4 * i, 4)? as usize;
            let tail: &[u8] = self.b.get(a..).ok_or_else(cut)?;
            let z: usize = tail.iter().position(|c| *c == 0).ok_or_else(cut)?;
            let s: String = String::from_utf8(tail[..z].to_vec())
                .map_err(|_| Error::Format("byml: bad utf8"))?;
            out.push(s);
        }
        Ok(out)
    }

    fn node(&self, t: u8, v: u32, d: usize) -> Res<Node> {
        if d > MAX_DEPTH {
            return Err(Error::Format("byml: nesting too deep"));
        }
        let left: usize = self.left.get();
        if left == 0 {
            return Err(Error::Format("byml: too many nodes"));
        }
        self.left.set(left - 1);
        let o: usize = v as usize;
        match t {
            0xA0 => self
                .strs
                .get(o)
                .map(|s| Node::Str(s.clone()))
                .ok_or(Error::Format("byml: string index out of range")),
            0xA1 => {
                let n: usize = self.num(o, 4)? as usize;
                Ok(Node::Bin(self.bytes(o + 4, n)?.to_vec()))
            }
            0xC0 => {
                if self.bytes(o, 1)?[0] != 0xC0 {
                    return Err(Error::Format("byml: bad array"));
                }
                let n: usize = self.num(o + 1, 3)? as usize;
                let tys: &[u8] = self.bytes(o + 4, n)?;
                let vo: usize = (o + 4 + n + 3) & !3;
                let mut out: Vec<Node> = Vec::with_capacity(n.min(65536));
                for (i, ty) in tys.iter().enumerate() {
                    let s: u32 = self.num(vo + 4 * i, 4)?;
                    out.push(self.node(*ty, s, d + 1)?);
                }
                Ok(Node::Arr(out))
            }
            0xC1 => {
                if self.bytes(o, 1)?[0] != 0xC1 {
                    return Err(Error::Format("byml: bad map"));
                }
                let n: usize = self.num(o + 1, 3)? as usize;
                let mut out: Vec<(String, Node)> = Vec::with_capacity(n.min(65536));
                for i in 0..n {
                    let p: usize = o + 4 + 8 * i;
                    let k: usize = self.num(p, 3)? as usize;
                    let ty: u8 = self.bytes(p + 3, 1)?[0];
                    let s: u32 = self.num(p + 4, 4)?;
                    let name: &String = self
                        .keys
                        .get(k)
                        .ok_or(Error::Format("byml: key index out of range"))?;
                    out.push((name.clone(), self.node(ty, s, d + 1)?));
                }
                Ok(Node::Map(out))
            }
            0xD0 => Ok(Node::Bool(v != 0)),
            0xD1 => Ok(Node::I32(v as i32)),
            0xD2 => Ok(Node::F32(f32::from_bits(v))),
            0xD3 => Ok(Node::U32(v)),
            0xD4 => Ok(Node::I64(self.wide(o)? as i64)),
            0xD5 => Ok(Node::U64(self.wide(o)?)),
            0xD6 => Ok(Node::F64(f64::from_bits(self.wide(o)?))),
            0xFF => Ok(Node::Null),
            _ => Err(Error::Msg(format!("byml: unsupported node type 0x{t:02X}"))),
        }
    }
}

struct Wr {
    out: Vec<u8>,
    be: bool,
    keys: Vec<String>,
    strs: Vec<String>,
    wides: HashMap<(u8, u64), u32>,
    seen: HashMap<Vec<u8>, u32>,
}

fn fold(s: &str) -> Vec<(u8, char)> {
    s.chars()
        .map(|c| {
            let cls: u8 = if c.is_alphabetic() {
                2
            } else if c.is_ascii_digit() {
                1
            } else {
                0
            };
            (cls, c.to_lowercase().next().unwrap_or(c))
        })
        .collect()
}

fn order(m: &[(String, Node)]) -> Vec<usize> {
    let mut ix: Vec<usize> = (0..m.len()).collect();
    ix.sort_by(|a, b| {
        fold(&m[*a].0)
            .cmp(&fold(&m[*b].0))
            .then_with(|| m[*b].0.cmp(&m[*a].0))
    });
    ix
}

fn gather(n: &Node, out: &mut Vec<(u8, u64)>) {
    match n {
        Node::I64(x) => out.push((0xD4, *x as u64)),
        Node::U64(x) => out.push((0xD5, *x)),
        Node::F64(x) => out.push((0xD6, x.to_bits())),
        Node::Arr(a) => {
            for x in a {
                gather(x, out);
            }
        }
        Node::Map(m) => {
            for i in order(m) {
                gather(&m[i].1, out);
            }
        }
        _ => {}
    }
}

fn ident(n: &Node, out: &mut Vec<u8>) {
    match n {
        Node::Null => out.push(0xFF),
        Node::Bool(x) => out.extend_from_slice(&[0xD0, *x as u8]),
        Node::I32(x) => {
            out.push(0xD1);
            out.extend_from_slice(&x.to_le_bytes());
        }
        Node::U32(x) => {
            out.push(0xD3);
            out.extend_from_slice(&x.to_le_bytes());
        }
        Node::F32(x) => {
            out.push(0xD2);
            out.extend_from_slice(&x.to_bits().to_le_bytes());
        }
        Node::I64(x) => {
            out.push(0xD4);
            out.extend_from_slice(&x.to_le_bytes());
        }
        Node::U64(x) => {
            out.push(0xD5);
            out.extend_from_slice(&x.to_le_bytes());
        }
        Node::F64(x) => {
            out.push(0xD6);
            out.extend_from_slice(&x.to_bits().to_le_bytes());
        }
        Node::Str(s) => {
            out.push(0xA0);
            out.extend_from_slice(&(s.len() as u32).to_le_bytes());
            out.extend_from_slice(s.as_bytes());
        }
        Node::Bin(d) => {
            out.push(0xA1);
            out.extend_from_slice(&(d.len() as u32).to_le_bytes());
            out.extend_from_slice(d);
        }
        Node::Arr(a) => {
            out.push(0xC0);
            out.extend_from_slice(&(a.len() as u32).to_le_bytes());
            for x in a {
                ident(x, out);
            }
        }
        Node::Map(m) => {
            out.push(0xC1);
            out.extend_from_slice(&(m.len() as u32).to_le_bytes());
            for e in m {
                out.extend_from_slice(&(e.0.len() as u32).to_le_bytes());
                out.extend_from_slice(e.0.as_bytes());
                ident(&e.1, out);
            }
        }
    }
}

fn collect(n: &Node, keys: &mut BTreeSet<String>, strs: &mut BTreeSet<String>) {
    match n {
        Node::Str(s) => {
            strs.insert(s.clone());
        }
        Node::Arr(a) => {
            for x in a {
                collect(x, keys, strs);
            }
        }
        Node::Map(m) => {
            for (k, v) in m {
                keys.insert(k.clone());
                collect(v, keys, strs);
            }
        }
        _ => {}
    }
}

impl Wr {
    fn p32(&mut self, v: u32) {
        let b: [u8; 4] = if self.be { v.to_be_bytes() } else { v.to_le_bytes() };
        self.out.extend_from_slice(&b);
    }

    fn p24(&mut self, v: u32) {
        if self.be {
            self.out.extend_from_slice(&v.to_be_bytes()[1..4]);
        } else {
            self.out.extend_from_slice(&v.to_le_bytes()[0..3]);
        }
    }

    fn set32(&mut self, at: usize, v: u32) {
        let b: [u8; 4] = if self.be { v.to_be_bytes() } else { v.to_le_bytes() };
        self.out[at..at + 4].copy_from_slice(&b);
    }

    fn pad4(&mut self) {
        while !self.out.len().is_multiple_of(4) {
            self.out.push(0);
        }
    }

    fn put_wides(&mut self, root: &Node) {
        let mut order: Vec<(u8, u64)> = Vec::new();
        gather(root, &mut order);
        self.pad4();
        for k in order {
            if self.wides.contains_key(&k) {
                continue;
            }
            let at: u32 = self.out.len() as u32;
            let b: [u8; 8] = if self.be { k.1.to_be_bytes() } else { k.1.to_le_bytes() };
            self.out.extend_from_slice(&b);
            self.wides.insert(k, at);
        }
    }

    fn wide(&self, t: u8, v: u64) -> Res<u32> {
        self.wides
            .get(&(t, v))
            .copied()
            .ok_or(Error::Format("byml: wide value missing from block"))
    }

    fn table(&mut self, v: &[String]) {
        let at: usize = self.out.len();
        self.out.push(0xC2);
        self.p24(v.len() as u32);
        let offs: usize = self.out.len();
        for _ in 0..=v.len() {
            self.p32(0);
        }
        for (i, s) in v.iter().enumerate() {
            let rel: u32 = (self.out.len() - at) as u32;
            self.set32(offs + 4 * i, rel);
            self.out.extend_from_slice(s.as_bytes());
            self.out.push(0);
        }
        let end: u32 = (self.out.len() - at) as u32;
        self.set32(offs + 4 * v.len(), end);
        self.pad4();
    }

    fn emit(&mut self, n: &Node) -> Res<(u8, u32)> {
        match n {
            Node::Null => Ok((0xFF, 0)),
            Node::Bool(x) => Ok((0xD0, *x as u32)),
            Node::I32(x) => Ok((0xD1, *x as u32)),
            Node::U32(x) => Ok((0xD3, *x)),
            Node::F32(x) => Ok((0xD2, x.to_bits())),
            Node::I64(x) => Ok((0xD4, self.wide(0xD4, *x as u64)?)),
            Node::U64(x) => Ok((0xD5, self.wide(0xD5, *x)?)),
            Node::F64(x) => Ok((0xD6, self.wide(0xD6, x.to_bits())?)),
            Node::Str(s) => {
                let i: usize = self
                    .strs
                    .binary_search(s)
                    .map_err(|_| Error::Format("byml: string missing from table"))?;
                Ok((0xA0, i as u32))
            }
            Node::Bin(d) => {
                self.pad4();
                let at: u32 = self.out.len() as u32;
                self.p32(d.len() as u32);
                self.out.extend_from_slice(d);
                self.pad4();
                Ok((0xA1, at))
            }
            Node::Arr(a) => {
                let mut id: Vec<u8> = Vec::new();
                ident(n, &mut id);
                if let Some(at) = self.seen.get(&id) {
                    return Ok((0xC0, *at));
                }
                self.pad4();
                let at: usize = self.out.len();
                self.seen.insert(id, at as u32);
                self.out.push(0xC0);
                self.p24(a.len() as u32);
                let tys: usize = self.out.len();
                self.out.extend(std::iter::repeat_n(0u8, a.len()));
                self.pad4();
                let slots: usize = self.out.len();
                for _ in a {
                    self.p32(0);
                }
                for (i, x) in a.iter().enumerate() {
                    let (t, v): (u8, u32) = self.emit(x)?;
                    self.out[tys + i] = t;
                    self.set32(slots + 4 * i, v);
                }
                Ok((0xC0, at as u32))
            }
            Node::Map(m) => {
                let es: Vec<&(String, Node)> = m.iter().collect();
                let mut names: BTreeSet<&str> = BTreeSet::new();
                if !es.iter().all(|e| names.insert(e.0.as_str())) {
                    return Err(Error::Format("byml: duplicate map key"));
                }
                let mut id: Vec<u8> = Vec::new();
                ident(n, &mut id);
                if let Some(at) = self.seen.get(&id) {
                    return Ok((0xC1, *at));
                }
                self.pad4();
                let at: usize = self.out.len();
                self.seen.insert(id, at as u32);
                self.out.push(0xC1);
                self.p24(es.len() as u32);
                let slots: usize = self.out.len();
                for _ in &es {
                    self.p32(0);
                    self.p32(0);
                }
                for i in order(m) {
                    let e: &(String, Node) = es[i];
                    let k: usize = self
                        .keys
                        .binary_search(&e.0)
                        .map_err(|_| Error::Format("byml: key missing from table"))?;
                    let (t, v): (u8, u32) = self.emit(&e.1)?;
                    let p: usize = slots + 8 * i;
                    let kb: [u8; 4] = if self.be {
                        [(k >> 16) as u8, (k >> 8) as u8, k as u8, t]
                    } else {
                        [k as u8, (k >> 8) as u8, (k >> 16) as u8, t]
                    };
                    self.out[p..p + 4].copy_from_slice(&kb);
                    self.set32(p + 4, v);
                }
                Ok((0xC1, at as u32))
            }
        }
    }
}

impl Byml {
    pub fn parse(b: &[u8]) -> Res<Byml> {
        let be: bool = match b.get(..2) {
            Some(m) if m == b"YB" => false,
            Some(m) if m == b"BY" => true,
            _ => return Err(Error::Format("byml: bad magic")),
        };
        let mut r: Rd = Rd {
            b,
            be,
            keys: Vec::new(),
            strs: Vec::new(),
            left: Cell::new(MAX_NODES),
        };
        let version: u16 = r.num(2, 2)? as u16;
        let ko: usize = r.num(4, 4)? as usize;
        let so: usize = r.num(8, 4)? as usize;
        let ro: usize = r.num(12, 4)? as usize;
        if ko != 0 {
            r.keys = r.table(ko)?;
        }
        if so != 0 {
            r.strs = r.table(so)?;
        }
        let root: Node = if ro == 0 {
            Node::Null
        } else {
            let t: u8 = r.bytes(ro, 1)?[0];
            if t != 0xC0 && t != 0xC1 {
                return Err(Error::Format("byml: root is not a container"));
            }
            r.node(t, ro as u32, 0)?
        };
        Ok(Byml { root, version, big: be })
    }

    pub fn write(&self) -> Res<Vec<u8>> {
        let mut ks: BTreeSet<String> = BTreeSet::new();
        let mut ss: BTreeSet<String> = BTreeSet::new();
        collect(&self.root, &mut ks, &mut ss);
        let mut w: Wr = Wr {
            out: Vec::new(),
            be: self.big,
            keys: ks.into_iter().collect(),
            strs: ss.into_iter().collect(),
            wides: HashMap::new(),
            seen: HashMap::new(),
        };
        w.out.extend_from_slice(if self.big { b"BY" } else { b"YB" });
        let v: [u8; 2] = if self.big { self.version.to_be_bytes() } else { self.version.to_le_bytes() };
        w.out.extend_from_slice(&v);
        for _ in 0..3 {
            w.p32(0);
        }
        let mut ko: u32 = 0;
        let mut so: u32 = 0;
        if !w.keys.is_empty() {
            ko = w.out.len() as u32;
            let k: Vec<String> = w.keys.clone();
            w.table(&k);
        }
        if !w.strs.is_empty() {
            so = w.out.len() as u32;
            let s: Vec<String> = w.strs.clone();
            w.table(&s);
        }
        w.put_wides(&self.root);
        let ro: u32 = match &self.root {
            Node::Null => 0,
            Node::Arr(_) | Node::Map(_) => w.emit(&self.root)?.1,
            _ => return Err(Error::Format("byml: root must be a container")),
        };
        w.set32(4, ko);
        w.set32(8, so);
        w.set32(12, ro);
        Ok(w.out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> Node {
        Node::Str(x.to_string())
    }

    fn sample() -> Node {
        Node::Map(vec![
            ("Actors".into(), Node::Arr(vec![
                Node::Map(vec![
                    ("Dynamic".into(), Node::Map(vec![("Drop__DropActor".into(), s("Item_Longshot"))])),
                    ("Gyaml".into(), s("Obj_TBox_A_01")),
                    ("Hash".into(), Node::U64(0xFEDC_BA98_7654_3210)),
                    ("Translate".into(), Node::Arr(vec![Node::F32(1.5), Node::F32(-2.25), Node::F32(0.0)])),
                ]),
                Node::Map(vec![("Gyaml".into(), s("Obj_Pot_A_01")), ("Hash".into(), Node::U64(7))]),
            ])),
            ("Bin".into(), Node::Bin(vec![1, 2, 3, 4, 5])),
            ("Empty".into(), Node::Null),
            ("Flag".into(), Node::Bool(true)),
            ("Float".into(), Node::F64(1.234_567_890_123_456_7)),
            ("Int".into(), Node::I32(-42)),
            ("Long".into(), Node::I64(-9_000_000_000)),
            ("Nested".into(), Node::Arr(vec![Node::Arr(vec![]), Node::Map(vec![]), s("x")])),
            ("Uint".into(), Node::U32(4_000_000_000)),
        ])
    }

    #[test]
    fn round_trips_both_byte_orders() {
        for big in [false, true] {
            let a: Byml = Byml { root: sample(), version: 7, big };
            let b: Vec<u8> = a.write().unwrap();
            assert_eq!(&b[..2], if big { b"BY" } else { b"YB" });
            assert_eq!(Byml::parse(&b).unwrap(), a);
            assert_eq!(Byml::parse(&b).unwrap().write().unwrap(), b);
        }
    }

    #[test]
    fn map_entry_order_is_preserved() {
        let a: Byml = Byml {
            root: Node::Map(vec![("b".into(), Node::I32(2)), ("a".into(), Node::I32(1))]),
            version: 7,
            big: false,
        };
        let r: Byml = Byml::parse(&a.write().unwrap()).unwrap();
        assert_eq!(r, a);
    }

    #[test]
    fn duplicate_keys_are_rejected() {
        let a: Byml = Byml {
            root: Node::Map(vec![("a".into(), Node::I32(1)), ("a".into(), Node::I32(2))]),
            version: 7,
            big: false,
        };
        assert!(a.write().is_err());
    }

    #[test]
    fn strings_are_shared() {
        let a: Byml = Byml {
            root: Node::Arr(vec![s("hello"), s("hello"), s("hello")]),
            version: 7,
            big: false,
        };
        let b: Vec<u8> = a.write().unwrap();
        let n: usize = b.windows(6).filter(|w| *w == b"hello\0").count();
        assert_eq!(n, 1);
    }

    #[test]
    fn wide_values_are_shared() {
        let a: Byml = Byml {
            root: Node::Arr(vec![Node::U64(0xAABB_CCDD_1122_3344), Node::U64(0xAABB_CCDD_1122_3344)]),
            version: 7,
            big: false,
        };
        let b: Vec<u8> = a.write().unwrap();
        let pat: [u8; 8] = 0xAABB_CCDD_1122_3344u64.to_le_bytes();
        assert_eq!(b.windows(8).filter(|w| *w == pat).count(), 1);
        assert_eq!(Byml::parse(&b).unwrap(), a);
    }

    #[test]
    fn children_are_emitted_in_folded_order() {
        let a: Byml = Byml {
            root: Node::Map(vec![
                ("AI".into(), Node::Arr(vec![Node::I32(0x1111_1111)])),
                ("Actor".into(), Node::Arr(vec![Node::I32(0x2222_2222)])),
                ("R1".into(), Node::Arr(vec![Node::I32(0x3333_3333)])),
                ("R_x".into(), Node::Arr(vec![Node::I32(0x4444_4444)])),
            ]),
            version: 7,
            big: false,
        };
        let b: Vec<u8> = a.write().unwrap();
        let r: Byml = Byml::parse(&b).unwrap();
        assert_eq!(r, a);
        let pos = |v: i32| b.windows(4).position(|w| w == v.to_le_bytes()).unwrap();
        assert!(pos(0x2222_2222) < pos(0x1111_1111));
        assert!(pos(0x4444_4444) < pos(0x3333_3333));
    }

    #[test]
    fn identical_containers_are_shared() {
        let arr = |x: i32| Node::Arr(vec![Node::I32(1), Node::I32(2), Node::I32(x)]);
        let same: Byml = Byml { root: Node::Arr(vec![arr(3), arr(3)]), version: 7, big: false };
        let diff: Byml = Byml { root: Node::Arr(vec![arr(3), arr(4)]), version: 7, big: false };
        let (a, b): (Vec<u8>, Vec<u8>) = (same.write().unwrap(), diff.write().unwrap());
        assert_eq!(b.len() - a.len(), 20);
        assert_eq!(Byml::parse(&a).unwrap(), same);
    }

    #[test]
    fn accessors_and_set() {
        let mut m: Node = sample();
        assert_eq!(m.get("Int"), Some(&Node::I32(-42)));
        assert!(m.set("Int", Node::I32(5)));
        assert!(m.set("Aaa", Node::Bool(false)));
        assert_eq!(m.get("Int"), Some(&Node::I32(5)));
        if let Node::Map(e) = &m {
            assert_eq!(e[0].0, "Aaa");
        }
        let d: &mut Node = m.get_mut("Actors").unwrap();
        assert_eq!(d.as_arr().unwrap().len(), 2);
        d.as_arr_mut().unwrap().pop();
        assert_eq!(m.get("Actors").unwrap().as_arr().unwrap().len(), 1);
        assert!(m.remove("Aaa"));
        assert!(!m.remove("Aaa"));
        assert!(m.get("Aaa").is_none());
        assert!(Node::Null.get("x").is_none());
        assert!(!Node::Null.clone().set("x", Node::Null));
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(Byml::parse(b"nope").is_err());
        assert!(Byml::parse(b"YB").is_err());
        let a: Vec<u8> = Byml { root: sample(), version: 7, big: false }.write().unwrap();
        assert!(Byml::parse(&a[..a.len() / 2]).is_err());
    }

    #[test]
    fn self_referencing_array_hits_the_depth_limit() {
        let mut b: Vec<u8> = b"YB".to_vec();
        b.extend_from_slice(&7u16.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&[0xC0, 1, 0, 0, 0xC0, 0, 0, 0]);
        b.extend_from_slice(&16u32.to_le_bytes());
        assert!(Byml::parse(&b).is_err());
    }
}
