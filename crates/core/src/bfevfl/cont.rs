use super::dic;
use super::rd::Rd;
use super::wr::{n16, Wr};
use crate::error::{Error, Res};

#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Arg(String),
    Int(i32),
    Bool(u32),
    Float(f32),
    Str(String),
    Ints(Vec<i32>),
    Bools(Vec<u32>),
    Floats(Vec<f32>),
    Strs(Vec<String>),
    Actor(String, String),
    Key(String, String),
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Cont {
    pub items: Vec<(String, Val)>,
}

impl Val {
    pub fn kind(&self) -> u8 {
        match self {
            Val::Arg(_) => 0,
            Val::Int(_) => 2,
            Val::Bool(_) => 3,
            Val::Float(_) => 4,
            Val::Str(_) => 5,
            Val::Ints(_) => 7,
            Val::Bools(_) => 8,
            Val::Floats(_) => 9,
            Val::Strs(_) => 10,
            Val::Actor(..) => 12,
            Val::Key(..) => 13,
        }
    }

    pub fn strings_mut(&mut self) -> Vec<&mut String> {
        match self {
            Val::Arg(s) | Val::Str(s) => vec![s],
            Val::Strs(v) => v.iter_mut().collect(),
            _ => Vec::new(),
        }
    }

    pub fn strings(&self) -> Vec<&str> {
        match self {
            Val::Arg(s) | Val::Str(s) => vec![s.as_str()],
            Val::Strs(v) => v.iter().map(|s| s.as_str()).collect(),
            _ => Vec::new(),
        }
    }
}

impl Cont {
    pub fn get(&self, k: &str) -> Option<&Val> {
        self.items.iter().find(|(n, _)| n == k).map(|(_, v)| v)
    }
}

fn items_of<T>(n: usize, mut f: impl FnMut(usize) -> Res<T>) -> Res<Vec<T>> {
    let mut out: Vec<T> = Vec::new();
    for i in 0..n {
        out.push(f(i)?);
    }
    Ok(out)
}

fn item(r: &Rd, o: usize) -> Res<Val> {
    let t: u8 = r.u8(o)?;
    let n: usize = r.u16(o + 2)? as usize;
    let d: usize = o + 0x10;
    Ok(match t {
        0 => Val::Arg(r.sref(d)?),
        2 => Val::Int(r.u32(d)? as i32),
        3 => Val::Bool(r.u32(d)?),
        4 => Val::Float(r.f32(d)?),
        5 => Val::Str(r.sref(d)?),
        7 => Val::Ints(items_of(n, |i| Ok(r.u32(d + 4 * i)? as i32))?),
        8 => Val::Bools(items_of(n, |i| r.u32(d + 4 * i))?),
        9 => Val::Floats(items_of(n, |i| r.f32(d + 4 * i))?),
        10 => Val::Strs(items_of(n, |i| r.sref(d + 8 * i))?),
        12 => Val::Actor(r.sref(d)?, r.sref(d + 8)?),
        13 => Val::Key(r.sref(d)?, r.sref(d + 8)?),
        _ => return Err(Error::Format("bfevfl: unsupported container item type")),
    })
}

pub fn read(r: &Rd, o: usize) -> Res<Cont> {
    if r.u8(o)? != 1 {
        return Err(Error::Format("bfevfl: params block is not a container"));
    }
    let n: usize = r.u16(o + 2)? as usize;
    let keys: Vec<String> = dic::read(r, r.ptr(o + 8)?)?;
    if keys.len() != n {
        return Err(Error::Format("bfevfl: container key count mismatch"));
    }
    let mut items: Vec<(String, Val)> = Vec::new();
    for (i, k) in keys.into_iter().enumerate() {
        items.push((k, item(r, r.ptr(o + 0x10 + 8 * i)?)?));
    }
    Ok(Cont { items })
}

fn write_item(w: &mut Wr, v: &Val) -> Res<()> {
    let n: usize = match v {
        Val::Ints(a) => a.len(),
        Val::Bools(a) => a.len(),
        Val::Floats(a) => a.len(),
        Val::Strs(a) => a.len(),
        Val::Actor(..) | Val::Key(..) => 2,
        _ => 1,
    };
    w.u8(v.kind());
    w.u8(0);
    w.u16(n16(n)?);
    w.u32(0);
    w.u64(0);
    match v {
        Val::Int(x) => w.u32(*x as u32),
        Val::Bool(x) => w.u32(*x),
        Val::Float(x) => w.f32(*x),
        Val::Arg(s) | Val::Str(s) => {
            let at: usize = w.ptr(true);
            w.here(at);
            w.pstr(s)?;
        }
        Val::Ints(a) => a.iter().for_each(|x| w.u32(*x as u32)),
        Val::Bools(a) => a.iter().for_each(|x| w.u32(*x)),
        Val::Floats(a) => a.iter().for_each(|x| w.f32(*x)),
        Val::Strs(a) => {
            let at: Vec<usize> = a.iter().map(|_| w.ptr(true)).collect();
            for (s, p) in a.iter().zip(at) {
                w.align(8);
                w.here(p);
                w.pstr(s)?;
            }
        }
        Val::Actor(a, b) | Val::Key(a, b) => {
            let p0: usize = w.ptr(true);
            let p1: usize = w.ptr(true);
            w.here(p0);
            w.pstr(a)?;
            w.here(p1);
            w.pstr(b)?;
        }
    }
    Ok(())
}

pub fn write(w: &mut Wr, c: &Cont) -> Res<()> {
    w.align(8);
    w.u8(1);
    w.u8(0);
    w.u16(n16(c.items.len())?);
    w.u32(0);
    let dp: usize = w.ptr(true);
    let ips: Vec<usize> = c.items.iter().map(|_| w.ptr(true)).collect();
    w.here(dp);
    let keys: Vec<String> = c.items.iter().map(|(k, _)| k.clone()).collect();
    dic::write(w, &keys)?;
    for ((_, v), p) in c.items.iter().zip(ips) {
        w.align(8);
        w.here(p);
        write_item(w, v)?;
    }
    Ok(())
}
