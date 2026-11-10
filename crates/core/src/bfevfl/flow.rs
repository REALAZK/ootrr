use super::actor::{self, Actor};
use super::cont::{self, Cont};
use super::dic;
use super::rd::Rd;
use super::wr::{n16, Wr};
use crate::error::{Error, Res};

pub const NONE: u16 = 0xFFFF;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Case {
    pub val: u32,
    pub ev: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Action { next: u16, actor: u16, act: u16, params: Option<Cont> },
    Switch { actor: u16, query: u16, cases: Vec<Case>, params: Option<Cont> },
    Fork { join: u16, forks: Vec<u16> },
    Join { next: u16 },
    Sub { next: u16, params: Option<Cont>, flow: String, entry: String },
    SubSwitch { cases: Vec<Case>, params: Option<Cont>, flow: String, entry: String },
    Label { hash: u32 },
}

impl Kind {
    pub fn params(&self) -> Option<&Cont> {
        match self {
            Kind::Action { params, .. }
            | Kind::Switch { params, .. }
            | Kind::Sub { params, .. }
            | Kind::SubSwitch { params, .. } => params.as_ref(),
            _ => None,
        }
    }

    pub fn params_mut(&mut self) -> Option<&mut Cont> {
        match self {
            Kind::Action { params, .. }
            | Kind::Switch { params, .. }
            | Kind::Sub { params, .. }
            | Kind::SubSwitch { params, .. } => params.as_mut(),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub name: String,
    pub kind: Kind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VarDef {
    pub name: String,
    pub raw: [u8; 16],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub name: String,
    pub subs: Vec<u16>,
    pub vars: Vec<VarDef>,
    pub main: u16,
    pub pad: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Flow {
    pub name: String,
    pub actors: Vec<Actor>,
    pub events: Vec<Event>,
    pub entries: Vec<Entry>,
}

fn opt(r: &Rd, p: usize) -> Res<Option<Cont>> {
    if p == 0 {
        return Ok(None);
    }
    Ok(Some(cont::read(r, p)?))
}

fn cases(r: &Rd, p: usize, n: usize) -> Res<Vec<Case>> {
    let mut out: Vec<Case> = Vec::new();
    if p == 0 {
        return Ok(out);
    }
    for i in 0..n {
        out.push(Case { val: r.u32(p + 8 * i)?, ev: r.u16(p + 8 * i + 4)? });
    }
    Ok(out)
}

fn read_event(r: &Rd, e: usize) -> Res<Event> {
    let t: u8 = r.u8(e + 8)?;
    let w0: u16 = r.u16(e + 0x0A)?;
    let w1: u16 = r.u16(e + 0x0C)?;
    let w2: u16 = r.u16(e + 0x0E)?;
    let p10: usize = r.ptr(e + 0x10)?;
    let p18: usize = r.ptr(e + 0x18)?;
    let kind: Kind = match t {
        0 => Kind::Action { next: w0, actor: w1, act: w2, params: opt(r, p10)? },
        1 => Kind::Switch {
            actor: w1,
            query: w2,
            cases: cases(r, p18, w0 as usize)?,
            params: opt(r, p10)?,
        },
        2 => {
            let mut forks: Vec<u16> = Vec::new();
            for i in 0..w0 as usize {
                forks.push(r.u16(p10 + 2 * i)?);
            }
            Kind::Fork { join: w1, forks }
        }
        3 => Kind::Join { next: w0 },
        4 => Kind::Sub {
            next: w0,
            params: opt(r, p10)?,
            flow: r.sref(e + 0x18)?,
            entry: r.sref(e + 0x20)?,
        },
        5 => Kind::SubSwitch {
            cases: cases(r, p18, w0 as usize)?,
            params: opt(r, p10)?,
            flow: r.sref(e + 0x20)?,
            entry: r.sref(e + 0x28)?,
        },
        6 => Kind::Label { hash: r.u32(e + 0x0C)? },
        _ => return Err(Error::Format("bfevfl: unknown event kind")),
    };
    Ok(Event { name: r.sref(e)?, kind })
}

fn read_entry(r: &Rd, e: usize, name: String) -> Res<Entry> {
    let sp: usize = r.ptr(e)?;
    let dp: usize = r.ptr(e + 8)?;
    let vp: usize = r.ptr(e + 0x10)?;
    let sc: usize = r.u16(e + 0x18)? as usize;
    let vc: usize = r.u16(e + 0x1A)? as usize;
    let mut subs: Vec<u16> = Vec::new();
    for i in 0..sc {
        subs.push(r.u16(sp + 2 * i)?);
    }
    let mut vars: Vec<VarDef> = Vec::new();
    if vc > 0 {
        let names: Vec<String> = dic::read(r, dp)?;
        if names.len() != vc {
            return Err(Error::Format("bfevfl: variable definition count mismatch"));
        }
        for (i, n) in names.into_iter().enumerate() {
            let mut raw: [u8; 16] = [0; 16];
            raw.copy_from_slice(r.bytes(vp + 16 * i, 16)?);
            vars.push(VarDef { name: n, raw });
        }
    }
    Ok(Entry { name, subs, vars, main: r.u16(e + 0x1C)?, pad: r.u16(e + 0x1E)? })
}

pub fn read(r: &Rd, o: usize) -> Res<Flow> {
    if r.bytes(o, 4)? != b"EVFL" {
        return Err(Error::Format("bfevfl: missing EVFL magic"));
    }
    let na: usize = r.u16(o + 0x10)? as usize;
    let ne: usize = r.u16(o + 0x16)? as usize;
    let nep: usize = r.u16(o + 0x18)? as usize;
    let ap: usize = r.ptr(o + 0x28)?;
    let ep: usize = r.ptr(o + 0x30)?;
    let dp: usize = r.ptr(o + 0x38)?;
    let xp: usize = r.ptr(o + 0x40)?;
    let mut actors: Vec<Actor> = Vec::new();
    for i in 0..na {
        actors.push(actor::read(r, ap + 0x38 * i)?);
    }
    let mut events: Vec<Event> = Vec::new();
    for i in 0..ne {
        events.push(read_event(r, ep + 0x30 * i)?);
    }
    let names: Vec<String> = if dp == 0 { Vec::new() } else { dic::read(r, dp)? };
    if names.len() != nep {
        return Err(Error::Format("bfevfl: entry point count mismatch"));
    }
    let mut entries: Vec<Entry> = Vec::new();
    for (i, n) in names.into_iter().enumerate() {
        entries.push(read_entry(r, xp + 0x20 * i, n)?);
    }
    Ok(Flow { name: r.sref(o + 0x20)?, actors, events, entries })
}

fn write_event(w: &mut Wr, v: &Event) -> Res<[usize; 4]> {
    w.sref(&v.name);
    let at: usize = w.pos();
    let s: [usize; 4] = [at + 8, at + 16, at + 24, at + 32];
    match &v.kind {
        Kind::Action { next, actor, act, params } => {
            w.u8(0);
            w.u8(0);
            w.u16(*next);
            w.u16(*actor);
            w.u16(*act);
            w.ptr(params.is_some());
            w.ptr(false);
            w.ptr(false);
            w.ptr(false);
        }
        Kind::Switch { actor, query, cases, params } => {
            w.u8(1);
            w.u8(0);
            w.u16(n16(cases.len())?);
            w.u16(*actor);
            w.u16(*query);
            w.ptr(params.is_some());
            w.ptr(true);
            w.ptr(false);
            w.ptr(false);
        }
        Kind::Fork { join, forks } => {
            w.u8(2);
            w.u8(0);
            w.u16(n16(forks.len())?);
            w.u16(*join);
            w.u16(0);
            w.ptr(true);
            w.ptr(false);
            w.ptr(false);
            w.ptr(false);
        }
        Kind::Join { next } => {
            w.u8(3);
            w.u8(0);
            w.u16(*next);
            w.u16(0);
            w.u16(0);
            w.ptr(false);
            w.ptr(false);
            w.ptr(false);
            w.ptr(false);
        }
        Kind::Sub { next, params, flow, entry } => {
            w.u8(4);
            w.u8(0);
            w.u16(*next);
            w.u16(0);
            w.u16(0);
            w.ptr(params.is_some());
            w.sref(flow);
            w.sref(entry);
            w.ptr(false);
        }
        Kind::SubSwitch { cases, params, flow, entry } => {
            w.u8(5);
            w.u8(0);
            w.u16(n16(cases.len())?);
            w.u16(0);
            w.u16(0);
            w.ptr(params.is_some());
            w.ptr(true);
            w.sref(flow);
            w.sref(entry);
        }
        Kind::Label { hash } => {
            w.u8(6);
            w.u8(0);
            w.u16(0);
            w.u32(*hash);
            w.ptr(false);
            w.ptr(false);
            w.ptr(false);
            w.ptr(false);
        }
    }
    Ok(s)
}

fn write_cases(w: &mut Wr, slot: usize, c: &[Case]) {
    if c.is_empty() {
        return;
    }
    w.align(8);
    w.here(slot);
    for x in c {
        w.u32(x.val);
        w.u16(x.ev);
        w.u16(0);
    }
}

fn write_event_extra(w: &mut Wr, v: &Event, s: [usize; 4]) -> Res<()> {
    let params: &Option<Cont> = match &v.kind {
        Kind::Fork { forks, .. } => {
            w.align(8);
            w.here(s[0]);
            for f in forks {
                w.u16(*f);
            }
            return Ok(());
        }
        Kind::Switch { cases, params, .. } | Kind::SubSwitch { cases, params, .. } => {
            write_cases(w, s[1], cases);
            params
        }
        Kind::Action { params, .. } | Kind::Sub { params, .. } => params,
        _ => return Ok(()),
    };
    if let Some(c) = params {
        w.align(8);
        w.here(s[0]);
        cont::write(w, c)?;
    }
    Ok(())
}

fn write_entry_head(w: &mut Wr, e: &Entry) -> Res<[usize; 3]> {
    let a: usize = w.ptr(true);
    let b: usize = w.ptr(!e.vars.is_empty());
    let c: usize = w.ptr(true);
    w.u16(n16(e.subs.len())?);
    w.u16(n16(e.vars.len())?);
    w.u16(e.main);
    w.u16(e.pad);
    Ok([a, b, c])
}

fn write_entry_extra(w: &mut Wr, e: &Entry, s: [usize; 3]) -> Res<()> {
    if !e.subs.is_empty() {
        w.align(8);
        w.here(s[0]);
        for x in &e.subs {
            w.u16(*x);
        }
    }
    w.align(8);
    if e.vars.is_empty() {
        w.zeros(24);
        return Ok(());
    }
    w.here(s[1]);
    let names: Vec<String> = e.vars.iter().map(|v| v.name.clone()).collect();
    dic::write(w, &names)?;
    w.here(s[2]);
    for v in &e.vars {
        w.raw(&v.raw);
    }
    Ok(())
}

pub fn write(w: &mut Wr, f: &Flow) -> Res<usize> {
    w.align(8);
    let o: usize = w.pos();
    w.raw(b"EVFL");
    w.u32(0);
    w.u64(0);
    w.u16(n16(f.actors.len())?);
    w.u16(n16(f.actors.iter().map(|a| a.actions.len()).sum())?);
    w.u16(n16(f.actors.iter().map(|a| a.queries.len()).sum())?);
    w.u16(n16(f.events.len())?);
    w.u16(n16(f.entries.len())?);
    w.u16(0);
    w.u16(0);
    w.u16(0);
    w.sref(&f.name);
    let hs: [usize; 4] = [w.ptr(true), w.ptr(true), w.ptr(true), w.ptr(true)];
    if !f.actors.is_empty() {
        w.align(8);
        w.here(hs[0]);
    }
    let mut ah: Vec<[usize; 3]> = Vec::new();
    for a in &f.actors {
        ah.push(actor::write_head(w, a)?);
    }
    if !f.events.is_empty() {
        w.align(8);
        w.here(hs[1]);
    }
    let mut eh: Vec<[usize; 4]> = Vec::new();
    for v in &f.events {
        eh.push(write_event(w, v)?);
    }
    w.align(8);
    w.here(hs[2]);
    let names: Vec<String> = f.entries.iter().map(|e| e.name.clone()).collect();
    dic::write(w, &names)?;
    if !f.entries.is_empty() {
        w.align(8);
        w.here(hs[3]);
    }
    let mut xh: Vec<[usize; 3]> = Vec::new();
    for e in &f.entries {
        xh.push(write_entry_head(w, e)?);
    }
    for (v, s) in f.events.iter().zip(eh) {
        write_event_extra(w, v, s)?;
    }
    for (a, s) in f.actors.iter().zip(ah) {
        let off: [usize; 3] = actor::write_extra(w, a)?;
        actor::patch(w, s, off);
    }
    for (e, s) in f.entries.iter().zip(xh) {
        write_entry_extra(w, e, s)?;
    }
    Ok(o)
}
