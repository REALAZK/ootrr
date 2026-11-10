use super::actor::{self, Actor};
use super::cont::{self, Cont};
use super::rd::Rd;
use super::wr::{n16, Wr};
use crate::error::{Error, Res};

#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    pub start: f32,
    pub dur: f32,
    pub actor: u16,
    pub act: u16,
    pub flags: u32,
    pub params: Option<Cont>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shot {
    pub time: f32,
    pub actor: u16,
    pub act: u16,
    pub params: Option<Cont>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trig {
    pub clip: u16,
    pub kind: u8,
    pub pad: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cut {
    pub start: f32,
    pub x4: u32,
    pub name: String,
    pub params: Option<Cont>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Timeline {
    pub name: String,
    pub dur: f32,
    pub actors: Vec<Actor>,
    pub clips: Vec<Clip>,
    pub shots: Vec<Shot>,
    pub trigs: Vec<Trig>,
    pub subs: Vec<String>,
    pub cuts: Vec<Cut>,
    pub params: Option<Cont>,
}

fn opt(r: &Rd, p: usize) -> Res<Option<Cont>> {
    if p == 0 {
        return Ok(None);
    }
    Ok(Some(cont::read(r, p)?))
}

pub fn read(r: &Rd, o: usize) -> Res<Timeline> {
    if r.bytes(o, 4)? != b"TLIN" {
        return Err(Error::Format("bfevfl: missing TLIN magic"));
    }
    let na: usize = r.u16(o + 0x14)? as usize;
    let ncl: usize = r.u16(o + 0x18)? as usize;
    let nos: usize = r.u16(o + 0x1A)? as usize;
    let nsub: usize = r.u16(o + 0x1C)? as usize;
    let ncut: usize = r.u16(o + 0x1E)? as usize;
    let ap: usize = r.ptr(o + 0x28)?;
    let clp: usize = r.ptr(o + 0x30)?;
    let osp: usize = r.ptr(o + 0x38)?;
    let trp: usize = r.ptr(o + 0x40)?;
    let sp: usize = r.ptr(o + 0x48)?;
    let cp: usize = r.ptr(o + 0x50)?;
    let mut actors: Vec<Actor> = Vec::new();
    for i in 0..na {
        actors.push(actor::read(r, ap + 0x38 * i)?);
    }
    let mut clips: Vec<Clip> = Vec::new();
    for i in 0..ncl {
        let c: usize = clp + 0x18 * i;
        clips.push(Clip {
            start: r.f32(c)?,
            dur: r.f32(c + 4)?,
            actor: r.u16(c + 8)?,
            act: r.u16(c + 10)?,
            flags: r.u32(c + 12)?,
            params: opt(r, r.ptr(c + 0x10)?)?,
        });
    }
    let mut shots: Vec<Shot> = Vec::new();
    for i in 0..nos {
        let c: usize = osp + 0x18 * i;
        shots.push(Shot {
            time: r.f32(c)?,
            actor: r.u16(c + 4)?,
            act: r.u16(c + 6)?,
            params: opt(r, r.ptr(c + 0x10)?)?,
        });
    }
    let mut trigs: Vec<Trig> = Vec::new();
    if trp != 0 {
        for i in 0..ncl * 2 {
            let c: usize = trp + 4 * i;
            trigs.push(Trig { clip: r.u16(c)?, kind: r.u8(c + 2)?, pad: r.u8(c + 3)? });
        }
    }
    let mut subs: Vec<String> = Vec::new();
    for i in 0..nsub {
        subs.push(r.sref(sp + 8 * i)?);
    }
    let mut cuts: Vec<Cut> = Vec::new();
    for i in 0..ncut {
        let c: usize = cp + 0x18 * i;
        cuts.push(Cut {
            start: r.f32(c)?,
            x4: r.u32(c + 4)?,
            name: r.sref(c + 8)?,
            params: opt(r, r.ptr(c + 0x10)?)?,
        });
    }
    Ok(Timeline {
        name: r.sref(o + 0x20)?,
        dur: r.f32(o + 0x10)?,
        actors,
        clips,
        shots,
        trigs,
        subs,
        cuts,
        params: opt(r, r.ptr(o + 0x58)?)?,
    })
}

fn extras(w: &mut Wr, slots: &[usize], ps: &[&Option<Cont>]) -> Res<()> {
    for (s, p) in slots.iter().zip(ps) {
        if let Some(c) = p {
            w.align(8);
            w.here(*s);
            cont::write(w, c)?;
        }
    }
    Ok(())
}

pub fn write(w: &mut Wr, t: &Timeline) -> Res<usize> {
    let mut offs: Vec<[usize; 3]> = Vec::new();
    for a in &t.actors {
        offs.push(actor::write_extra(w, a)?);
    }
    let mut po: usize = 0;
    if let Some(c) = &t.params {
        w.align(8);
        po = w.pos();
        cont::write(w, c)?;
    }
    w.align(8);
    let o: usize = w.pos();
    w.raw(b"TLIN");
    w.u32(0);
    w.u64(0);
    w.f32(t.dur);
    w.u16(n16(t.actors.len())?);
    w.u16(n16(t.actors.iter().map(|a| a.actions.len()).sum())?);
    w.u16(n16(t.clips.len())?);
    w.u16(n16(t.shots.len())?);
    w.u16(n16(t.subs.len())?);
    w.u16(n16(t.cuts.len())?);
    w.sref(&t.name);
    let hs: [usize; 7] = [
        w.ptr(true),
        w.ptr(true),
        w.ptr(true),
        w.ptr(true),
        w.ptr(true),
        w.ptr(true),
        w.ptr(t.params.is_some()),
    ];
    if po != 0 {
        w.set_ptr(hs[6], po);
    }
    if !t.actors.is_empty() {
        w.align(8);
        w.here(hs[0]);
    }
    let mut ah: Vec<[usize; 3]> = Vec::new();
    for a in &t.actors {
        ah.push(actor::write_head(w, a)?);
    }
    for (s, f) in ah.into_iter().zip(offs) {
        actor::patch(w, s, f);
    }
    let mut cs: Vec<usize> = Vec::new();
    if !t.clips.is_empty() {
        w.align(8);
        w.here(hs[1]);
    }
    for c in &t.clips {
        w.f32(c.start);
        w.f32(c.dur);
        w.u16(c.actor);
        w.u16(c.act);
        w.u32(c.flags);
        cs.push(w.ptr(c.params.is_some()));
    }
    let mut os: Vec<usize> = Vec::new();
    if !t.shots.is_empty() {
        w.align(8);
        w.here(hs[2]);
    }
    for c in &t.shots {
        w.f32(c.time);
        w.u16(c.actor);
        w.u16(c.act);
        w.u64(0);
        os.push(w.ptr(c.params.is_some()));
    }
    if !t.subs.is_empty() {
        w.align(8);
        w.here(hs[4]);
        for s in &t.subs {
            w.sref(s);
        }
    }
    if !t.trigs.is_empty() {
        w.align(8);
        w.here(hs[3]);
        for x in &t.trigs {
            w.u16(x.clip);
            w.u8(x.kind);
            w.u8(x.pad);
        }
    }
    let mut ks: Vec<usize> = Vec::new();
    if !t.cuts.is_empty() {
        w.align(8);
        w.here(hs[5]);
    }
    for c in &t.cuts {
        w.f32(c.start);
        w.u32(c.x4);
        w.sref(&c.name);
        ks.push(w.ptr(c.params.is_some()));
    }
    let cp: Vec<&Option<Cont>> = t.clips.iter().map(|c| &c.params).collect();
    extras(w, &cs, &cp)?;
    let sp: Vec<&Option<Cont>> = t.shots.iter().map(|c| &c.params).collect();
    extras(w, &os, &sp)?;
    let kp: Vec<&Option<Cont>> = t.cuts.iter().map(|c| &c.params).collect();
    extras(w, &ks, &kp)?;
    Ok(o)
}
