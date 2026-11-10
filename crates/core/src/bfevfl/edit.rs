use super::cont::{Cont, Val};
use super::{Actor, Bf};
use crate::error::{Error, Res};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Replaced {
    pub actors: usize,
    pub values: usize,
}

impl Replaced {
    pub fn total(&self) -> usize {
        self.actors + self.values
    }
}

fn swap(s: &mut String, from: &str, to: &str) -> usize {
    if s == from {
        *s = to.to_string();
        1
    } else {
        0
    }
}

fn in_cont(c: &mut Cont, from: &str, to: &str) -> usize {
    let mut n: usize = 0;
    for (_, v) in c.items.iter_mut() {
        if let Val::Actor(a, _) = v {
            n += swap(a, from, to);
            continue;
        }
        for s in v.strings_mut() {
            n += swap(s, from, to);
        }
    }
    n
}

fn in_opt(c: &mut Option<Cont>, from: &str, to: &str) -> usize {
    c.as_mut().map_or(0, |x| in_cont(x, from, to))
}

fn in_actors(a: &mut [Actor], from: &str, to: &str, out: &mut Replaced) {
    for x in a {
        out.actors += swap(&mut x.name, from, to);
        out.values += in_opt(&mut x.params, from, to);
    }
}

pub fn replace_str(b: &mut Bf, from: &str, to: &str) -> Replaced {
    let mut out: Replaced = Replaced::default();
    for f in &mut b.flows {
        in_actors(&mut f.actors, from, to, &mut out);
        for e in &mut f.events {
            out.values += e.kind.params_mut().map_or(0, |c| in_cont(c, from, to));
        }
    }
    for t in &mut b.tls {
        in_actors(&mut t.actors, from, to, &mut out);
        out.values += in_opt(&mut t.params, from, to);
        for c in &mut t.clips {
            out.values += in_opt(&mut c.params, from, to);
        }
        for c in &mut t.shots {
            out.values += in_opt(&mut c.params, from, to);
        }
        for c in &mut t.cuts {
            out.values += in_opt(&mut c.params, from, to);
        }
    }
    out
}

pub fn replace_bytes(raw: &[u8], from: &str, to: &str) -> Res<(Vec<u8>, Replaced)> {
    let mut bf: Bf = Bf::parse(raw)?;
    let n: Replaced = replace_str(&mut bf, from, to);
    if n.total() == 0 {
        return Err(Error::Msg(format!("no occurrence of {from}")));
    }
    let out: Vec<u8> = bf.write()?;
    if Bf::parse(&out)? != bf {
        return Err(Error::Format("bfevfl: replaced file does not parse back to the edited model"));
    }
    Ok((out, n))
}
