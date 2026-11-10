mod actor;
pub mod cont;
mod dic;
pub mod edit;
pub mod flow;
mod rd;
pub mod rewards;
pub mod show;
pub mod tl;
mod wr;

use crate::error::{Error, Res};
use rd::Rd;
use wr::{n16, n32, Wr};

pub use actor::Actor;
pub use cont::{Cont, Val};
pub use edit::{replace_bytes, replace_str, Replaced};
pub use flow::{Case, Entry, Event, Flow, Kind, VarDef};
pub use tl::{Clip, Cut, Shot, Timeline, Trig};

const MAGIC: &[u8; 8] = b"BFEVFL\0\0";

#[derive(Clone, Debug, PartialEq)]
pub struct Bf {
    pub ver: [u8; 4],
    pub bom: u16,
    pub shift: u8,
    pub pad: u8,
    pub name: String,
    pub flows: Vec<Flow>,
    pub tls: Vec<Timeline>,
}

impl Bf {
    pub fn parse(b: &[u8]) -> Res<Bf> {
        let r: Rd = Rd::new(b);
        if r.bytes(0, 8)? != MAGIC {
            return Err(Error::Format("bfevfl: bad magic"));
        }
        let mut ver: [u8; 4] = [0; 4];
        ver.copy_from_slice(r.bytes(8, 4)?);
        let name_at: usize = r.u32(0x10)? as usize;
        let nf: usize = r.u16(0x20)? as usize;
        let nt: usize = r.u16(0x22)? as usize;
        let fp: usize = r.ptr(0x28)?;
        let tp: usize = r.ptr(0x38)?;
        let mut flows: Vec<Flow> = Vec::new();
        for i in 0..nf {
            flows.push(flow::read(&r, r.ptr(fp + 8 * i)?)?);
        }
        let mut tls: Vec<Timeline> = Vec::new();
        for i in 0..nt {
            tls.push(tl::read(&r, r.ptr(tp + 8 * i)?)?);
        }
        Ok(Bf {
            ver,
            bom: r.u16(12)?,
            shift: r.u8(14)?,
            pad: r.u8(15)?,
            name: r.pstr(name_at.checked_sub(2).ok_or(Error::Format("bfevfl: bad name offset"))?)?,
            flows,
            tls,
        })
    }

    pub fn write(&self) -> Res<Vec<u8>> {
        let mut w: Wr = Wr::new();
        w.raw(MAGIC);
        w.raw(&self.ver);
        w.u16(self.bom);
        w.u8(self.shift);
        w.u8(self.pad);
        w.href(&self.name);
        w.u16(0);
        w.u16(0);
        w.u32(0);
        w.u32(0);
        w.u16(n16(self.flows.len())?);
        w.u16(n16(self.tls.len())?);
        w.u32(0);
        let hs: [usize; 4] = [w.ptr(true), w.ptr(true), w.ptr(true), w.ptr(true)];
        let mut fs: Vec<usize> = Vec::new();
        if !self.flows.is_empty() {
            w.align(8);
            w.here(hs[0]);
            fs = self.flows.iter().map(|_| w.ptr(true)).collect();
        }
        w.align(8);
        w.here(hs[1]);
        let fnames: Vec<String> = self.flows.iter().map(|f| f.name.clone()).collect();
        dic::write(&mut w, &fnames)?;
        let mut ts: Vec<usize> = Vec::new();
        if !self.tls.is_empty() {
            w.align(8);
            w.here(hs[2]);
            ts = self.tls.iter().map(|_| w.ptr(true)).collect();
        }
        w.align(8);
        w.here(hs[3]);
        let tnames: Vec<String> = self.tls.iter().map(|t| t.name.clone()).collect();
        dic::write(&mut w, &tnames)?;
        let mut blocks: Vec<usize> = Vec::new();
        for (t, s) in self.tls.iter().zip(ts) {
            let o: usize = tl::write(&mut w, t)?;
            w.set_ptr(s, o);
            blocks.push(o);
        }
        for (f, s) in self.flows.iter().zip(fs) {
            let o: usize = flow::write(&mut w, f)?;
            w.set_ptr(s, o);
            blocks.push(o);
        }
        w.align(8);
        let pool: usize = w.pool()?;
        for o in &blocks {
            w.set32(*o + 4, n32(pool - *o)?);
        }
        let end: usize = w.pos();
        let rel: usize = w.reloc(end)?;
        let first: usize = blocks.first().copied().unwrap_or(0);
        w.set16(0x16, first as u16);
        w.set32(0x18, n32(rel)?);
        let n: usize = w.pos();
        w.set32(0x1C, n32(n)?);
        Ok(w.b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> Cont {
        Cont {
            items: vec![
                ("ActorName".to_string(), Val::Str("Item_Test_A".to_string())),
                ("Num".to_string(), Val::Int(-1)),
                ("On".to_string(), Val::Bool(0x8000_0001)),
                ("Scale".to_string(), Val::Float(1.5)),
                ("Pos".to_string(), Val::Floats(vec![1.0, 2.0, 3.0])),
                ("Who".to_string(), Val::Actor("Npc_Test".to_string(), String::new())),
                ("Arg".to_string(), Val::Arg("Value".to_string())),
                ("Many".to_string(), Val::Strs(vec!["x".to_string(), "Item_Test_A".to_string()])),
            ],
        }
    }

    fn actor(n: &str, p: Option<Cont>) -> Actor {
        Actor {
            name: n.to_string(),
            sub: String::new(),
            arg: String::new(),
            actions: vec!["Act".to_string()],
            queries: vec!["Qry".to_string()],
            params: p,
            ep: 0xFFFF,
            cut: 1,
        }
    }

    fn ev(n: &str, kind: Kind) -> Event {
        Event { name: n.to_string(), kind }
    }

    fn sample() -> Bf {
        let f: Flow = Flow {
            name: "Test_Flow".to_string(),
            actors: vec![actor("Item_Test_A", Some(params())), actor("Plain", None)],
            events: vec![
                ev("Event0", Kind::Action { next: 1, actor: 0, act: 0, params: Some(params()) }),
                ev(
                    "Event1",
                    Kind::Switch {
                        actor: 1,
                        query: 0,
                        cases: vec![Case { val: 0, ev: 2 }, Case { val: 1, ev: 2 }],
                        params: None,
                    },
                ),
                ev("Event2", Kind::Fork { join: 3, forks: vec![0, 1] }),
                ev("Event3", Kind::Join { next: 0xFFFF }),
                ev(
                    "Event4",
                    Kind::Sub { next: 0xFFFF, params: Some(params()), flow: "Other".to_string(), entry: "Go".to_string() },
                ),
                ev(
                    "Event5",
                    Kind::SubSwitch {
                        cases: vec![Case { val: 5, ev: 0 }],
                        params: None,
                        flow: String::new(),
                        entry: "Check".to_string(),
                    },
                ),
                ev("Event6", Kind::Label { hash: 0x1234_5678 }),
            ],
            entries: vec![
                Entry { name: "Start".to_string(), subs: vec![4], vars: Vec::new(), main: 0, pad: 0 },
                Entry {
                    name: "Second".to_string(),
                    subs: Vec::new(),
                    vars: vec![VarDef { name: "v".to_string(), raw: [7; 16] }],
                    main: 1,
                    pad: 0,
                },
            ],
        };
        let t: Timeline = Timeline {
            name: "Test_Tl".to_string(),
            dur: 10.0,
            actors: vec![actor("Item_Test_A", None)],
            clips: vec![Clip { start: 0.0, dur: 5.0, actor: 0, act: 0, flags: 1, params: Some(params()) }],
            shots: vec![Shot { time: 2.0, actor: 0, act: 0, params: None }],
            trigs: vec![Trig { clip: 0, kind: 1, pad: 0 }, Trig { clip: 0, kind: 2, pad: 0 }],
            subs: vec!["Sub1".to_string()],
            cuts: vec![Cut { start: 0.0, x4: 0xFFFF_FFFF, name: "cut1".to_string(), params: Some(params()) }],
            params: Some(params()),
        };
        Bf {
            ver: [0, 4, 0, 0],
            bom: 0xFEFF,
            shift: 3,
            pad: 0,
            name: "Test_Flow".to_string(),
            flows: vec![f],
            tls: vec![t],
        }
    }

    #[test]
    fn model_round_trips_through_bytes() {
        let b: Bf = sample();
        let raw: Vec<u8> = b.write().unwrap();
        let c: Bf = Bf::parse(&raw).unwrap();
        assert_eq!(b, c);
        assert_eq!(c.write().unwrap(), raw);
    }

    #[test]
    fn header_fields_are_consistent() {
        let raw: Vec<u8> = sample().write().unwrap();
        assert_eq!(&raw[..8], MAGIC);
        let size: usize = u32::from_le_bytes([raw[0x1C], raw[0x1D], raw[0x1E], raw[0x1F]]) as usize;
        assert_eq!(size, raw.len());
        let rel: usize = u32::from_le_bytes([raw[0x18], raw[0x19], raw[0x1A], raw[0x1B]]) as usize;
        assert_eq!(&raw[rel..rel + 4], b"RELT");
        assert_eq!(rel % 8, 0);
    }

    #[test]
    fn replacement_edits_names_and_values_and_restores() {
        let raw: Vec<u8> = sample().write().unwrap();
        let (out, n) = replace_bytes(&raw, "Item_Test_A", "Item_Other_Longer").unwrap();
        assert_eq!(n.actors, 2);
        assert!(n.values >= 6);
        let c: Bf = Bf::parse(&out).unwrap();
        assert_eq!(c.flows[0].actors[0].name, "Item_Other_Longer");
        assert!(out.len() > raw.len());
        let (back, m) = replace_bytes(&out, "Item_Other_Longer", "Item_Test_A").unwrap();
        assert_eq!(m, n);
        assert_eq!(back, raw);
    }

    #[test]
    fn replacement_of_a_missing_name_is_an_error() {
        let raw: Vec<u8> = sample().write().unwrap();
        assert!(replace_bytes(&raw, "Item_Nothing", "Item_Other").is_err());
    }

    #[test]
    fn bad_input_is_an_error() {
        assert!(Bf::parse(b"not a flow file").is_err());
        let raw: Vec<u8> = sample().write().unwrap();
        assert!(Bf::parse(&raw[..raw.len() / 2]).is_err());
    }

    #[test]
    fn reward_catalog_reads_grants() {
        let mut b: Bf = sample();
        let f: &mut Flow = &mut b.flows[0];
        f.actors[1].name = "PouchSystemActor".to_string();
        f.actors[1].actions = vec!["EventTriggerAddOrRemovePouchContent".to_string()];
        f.events[0].kind = Kind::Action {
            next: 1,
            actor: 1,
            act: 0,
            params: Some(Cont {
                items: vec![
                    ("Actor".to_string(), Val::Str("Item_Test_A".to_string())),
                    ("Num".to_string(), Val::Int(1)),
                ],
            }),
        };
        let e: rewards::Entry = rewards::entry("Test_Flow.bfevfl", &b).unwrap();
        assert_eq!(e.hits.len(), 1);
        assert_eq!(e.hits[0].role, "give");
        assert_eq!(e.hits[0].item, "Item_Test_A");
        assert_eq!(e.actors, vec!["Item_Test_A".to_string()]);
        assert!(rewards::to_json("x", &[e]).contains("\"item\": \"Item_Test_A\""));
    }
}
