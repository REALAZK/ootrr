use super::cont::{self, Cont};
use super::rd::Rd;
use super::wr::{n16, Wr};
use crate::error::Res;

#[derive(Clone, Debug, PartialEq)]
pub struct Actor {
    pub name: String,
    pub sub: String,
    pub arg: String,
    pub actions: Vec<String>,
    pub queries: Vec<String>,
    pub params: Option<Cont>,
    pub ep: u16,
    pub cut: u16,
}

fn names(r: &Rd, p: usize, n: usize) -> Res<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for i in 0..n {
        out.push(r.sref(p + 8 * i)?);
    }
    Ok(out)
}

pub fn read(r: &Rd, o: usize) -> Res<Actor> {
    let ap: usize = r.ptr(o + 0x18)?;
    let qp: usize = r.ptr(o + 0x20)?;
    let pp: usize = r.ptr(o + 0x28)?;
    let ac: usize = r.u16(o + 0x30)? as usize;
    let qc: usize = r.u16(o + 0x32)? as usize;
    Ok(Actor {
        name: r.sref(o)?,
        sub: r.sref(o + 8)?,
        arg: r.sref(o + 16)?,
        actions: names(r, ap, ac)?,
        queries: names(r, qp, qc)?,
        params: if pp == 0 { None } else { Some(cont::read(r, pp)?) },
        ep: r.u16(o + 0x34)?,
        cut: r.u16(o + 0x36)?,
    })
}

pub fn write_head(w: &mut Wr, a: &Actor) -> Res<[usize; 3]> {
    w.sref(&a.name);
    w.sref(&a.sub);
    w.sref(&a.arg);
    let pa: usize = w.ptr(true);
    let pq: usize = w.ptr(true);
    let pp: usize = w.ptr(a.params.is_some());
    w.u16(n16(a.actions.len())?);
    w.u16(n16(a.queries.len())?);
    w.u16(a.ep);
    w.u16(a.cut);
    Ok([pa, pq, pp])
}

pub fn write_extra(w: &mut Wr, a: &Actor) -> Res<[usize; 3]> {
    let mut o: [usize; 3] = [0; 3];
    if let Some(c) = &a.params {
        w.align(8);
        o[2] = w.pos();
        cont::write(w, c)?;
    }
    for (v, k) in [(&a.actions, 0), (&a.queries, 1)] {
        if !v.is_empty() {
            w.align(8);
            o[k] = w.pos();
            for x in v {
                w.sref(x);
            }
        }
    }
    Ok(o)
}

pub fn patch(w: &mut Wr, s: [usize; 3], o: [usize; 3]) {
    for k in 0..3 {
        if o[k] != 0 {
            w.set_ptr(s[k], o[k]);
        }
    }
}
