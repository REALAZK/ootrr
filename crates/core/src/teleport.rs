use std::f32::consts::PI;
use std::fmt::Write as W;
use std::path::Path as P;

use crate::bfevfl::flow::NONE;
use crate::bfevfl::{Bf, Case, Cont, Entry, Event, Flow, Kind, Val};
use crate::byml::Byml;
use crate::error::{Error, Res};
use crate::romfs::Romfs;
use crate::rsizetable::{self, Rst};
use crate::staging::{self, MARGIN, Out};
use crate::teleport_dest::{Dest, start_name};
use crate::teleport_start;

pub const FLOW_FILE: &str = "Event/EventFlow/Dm_SY_Ocarina.bfevfl.zs";
pub const FLOW: &str = "Dm_SY_Ocarina";
pub const INDEX_FLAG: &str = "DungeonRoomStep.TestField.DungeonRoom000.Step";
pub const PREFIX: &str = "OOTRRTp";
pub const SONG_NEXT: (&str, &str) = ("Event77", "ForestMinuet");
pub const SONG_PREV: (&str, &str) = ("Event15", "FireBolero");
pub const SONG_SAME: (&str, &str) = ("Event18", "ShadowNocturne");
pub const ENTRY_NEXT: &str = "OOTRRTpNext";
pub const ENTRY_PREV: &str = "OOTRRTpPrev";
pub const ENTRY_SAME: &str = "OOTRRTpSame";
const WARP_ENTRY: &str = "OcarinaWarp";
const ABLE: &str = "Event132";
const TIMER: &str = "Event80";
const GAMBLE: &str = "Event170";
const MOVE: &str = "Event25";
const ICON: &str = "Event169";
const WARP: &str = "Event22";
const DENY: &str = "Event131";
const CLOSE: &str = "Event209";
const OK_VAL: u32 = 0x4db2_11e5;
const HOLDER: &str = "GameDataActor";
const ADD: &str = "EventTriggerAddOrSubGameDataInt";
const SET: &str = "EventTriggerSetGameDataInt";
const IS_INT: &str = "EventQueryIsGameDataInt";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Start,
    Coords,
}

impl Mode {
    pub fn parse(s: &str) -> Res<Mode> {
        match s {
            "start" => Ok(Mode::Start),
            "coords" => Ok(Mode::Coords),
            _ => Err(Error::Format("teleport: mode must be start or coords")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Deg,
    Rad,
}

impl Unit {
    pub fn parse(s: &str) -> Res<Unit> {
        match s {
            "deg" => Ok(Unit::Deg),
            "rad" => Ok(Unit::Rad),
            _ => Err(Error::Format("teleport: rotation unit must be deg or rad")),
        }
    }

    pub fn conv(self, r: f32) -> f32 {
        let full: f32 = match self {
            Unit::Deg => 360.0,
            Unit::Rad => 2.0 * PI,
        };
        let v: f32 = match self {
            Unit::Deg => r.to_degrees(),
            Unit::Rad => r,
        };
        v - full * (v / full).round()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Next,
    Prev,
    Same,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub added: usize,
    pub leaves: usize,
    pub depth: usize,
    pub root: u16,
}

fn idx(n: usize) -> Res<u16> {
    if n >= NONE as usize {
        return Err(Error::Format("teleport: too many events"));
    }
    Ok(n as u16)
}

fn find(f: &Flow, name: &str) -> Res<usize> {
    let v: Vec<usize> = (0..f.events.len()).filter(|&i| f.events[i].name == name).collect();
    match v.as_slice() {
        [i] => Ok(*i),
        _ => Err(Error::Msg(format!("teleport: expected one event named {name} in {FLOW}, found {}", v.len()))),
    }
}

fn put(f: &mut Flow, kind: Kind) -> Res<u16> {
    let at: u16 = idx(f.events.len())?;
    f.events.push(Event { name: format!("{PREFIX}{at}"), kind });
    Ok(at)
}

fn set_item(c: &mut Cont, k: &str, v: Val) -> Res<()> {
    let s: &mut Val = c
        .items
        .iter_mut()
        .find(|(n, _)| n == k)
        .map(|(_, v)| v)
        .ok_or_else(|| Error::Msg(format!("teleport: the template has no {k}")))?;
    *s = v;
    Ok(())
}

fn need_floats(c: &Cont, k: &str) -> Res<()> {
    match c.get(k) {
        Some(Val::Floats(_)) => Ok(()),
        _ => Err(Error::Msg(format!("teleport: {k} is not a float list in the warp template"))),
    }
}

fn act_slot(f: &mut Flow, name: &str) -> Res<(u16, u16)> {
    let i: usize = f
        .actors
        .iter()
        .position(|a| a.name == HOLDER && a.sub.is_empty())
        .ok_or(Error::Format("teleport: the flow has no GameDataActor"))?;
    let a = &mut f.actors[i];
    let j: usize = match a.actions.iter().position(|x| x == name) {
        Some(j) => j,
        None => {
            a.actions.push(name.to_string());
            a.actions.len() - 1
        }
    };
    Ok((idx(i)?, idx(j)?))
}

fn query_slot(f: &mut Flow, name: &str) -> Res<(u16, u16)> {
    let i: usize = f
        .actors
        .iter()
        .position(|a| a.name == HOLDER && a.sub.is_empty())
        .ok_or(Error::Format("teleport: the flow has no GameDataActor"))?;
    let a = &mut f.actors[i];
    let j: usize = match a.queries.iter().position(|x| x == name) {
        Some(j) => j,
        None => {
            a.queries.push(name.to_string());
            a.queries.len() - 1
        }
    };
    Ok((idx(i)?, idx(j)?))
}

fn gd(flag: &str, v: i32) -> Cont {
    Cont {
        items: vec![
            ("GameDataName".to_string(), Val::Str(flag.to_string())),
            ("Index".to_string(), Val::Int(-1)),
            ("Value".to_string(), Val::Int(v)),
        ],
    }
}

fn act(slot: (u16, u16), flag: &str, v: i32, next: u16) -> Kind {
    Kind::Action { next, actor: slot.0, act: slot.1, params: Some(gd(flag, v)) }
}

fn cmp(slot: (u16, u16), flag: &str, v: i32, lt: u16, eq: u16, gt: u16) -> Kind {
    Kind::Switch {
        actor: slot.0,
        query: slot.1,
        cases: vec![Case { val: 0, ev: lt }, Case { val: 1, ev: eq }, Case { val: 2, ev: gt }],
        params: Some(gd(flag, v)),
    }
}

fn tree(f: &mut Flow, q: (u16, u16), flag: &str, leaves: &[u16], lo: usize, hi: usize, depth: &mut usize, at: usize) -> Res<u16> {
    if hi - lo == 1 {
        *depth = (*depth).max(at);
        return Ok(leaves[lo]);
    }
    let mid: usize = (lo + hi) / 2;
    let l: u16 = tree(f, q, flag, leaves, lo, mid, depth, at + 1)?;
    let r: u16 = tree(f, q, flag, leaves, mid, hi, depth, at + 1)?;
    let v: i32 = i32::try_from(mid + 1).map_err(|_| Error::Format("teleport: too many destinations"))?;
    put(f, cmp(q, flag, v, l, r, r))
}

fn leaf(base: &Cont, slot: (u16, u16), mode: Mode, unit: Unit, d: &Dest) -> Res<Kind> {
    let mut c: Cont = base.clone();
    set_item(&mut c, "NextBanc", Val::Str(d.banc.clone()))?;
    match mode {
        Mode::Start => set_item(&mut c, "NextStartPos", Val::Str(start_name(d)))?,
        Mode::Coords => {
            need_floats(&c, "NextPosition")?;
            need_floats(&c, "NextStartRot")?;
            set_item(&mut c, "NextStartPos", Val::Str(String::new()))?;
            set_item(&mut c, "NextPosition", Val::Floats(d.pos.to_vec()))?;
            set_item(&mut c, "NextStartRot", Val::Floats(vec![0.0, unit.conv(d.yaw), 0.0]))?;
        }
    }
    Ok(Kind::Action { next: NONE, actor: slot.0, act: slot.1, params: Some(c) })
}

fn remap(k: &Kind, ok: u16, bad: u16) -> Res<Kind> {
    let mut out: Kind = k.clone();
    match &mut out {
        Kind::Switch { cases, .. } => {
            for c in cases.iter_mut() {
                c.ev = if c.val == 0 { ok } else { bad };
            }
        }
        Kind::SubSwitch { cases, .. } => {
            for c in cases.iter_mut() {
                c.ev = if c.val == OK_VAL { ok } else { bad };
            }
        }
        _ => return Err(Error::Format("teleport: a check template is not a switch")),
    }
    Ok(out)
}

fn text<'a>(k: &'a Kind, key: &str) -> Option<&'a str> {
    match k.params()?.get(key)? {
        Val::Str(s) => Some(s.as_str()),
        _ => None,
    }
}

fn hook(f: &mut Flow, want: (&str, &str), entry: &str) -> Res<()> {
    let i: usize = find(f, want.0)?;
    if text(&f.events[i].kind, "NextStartPos") != Some(want.1) {
        return Err(Error::Msg(format!("teleport: event {} is not the {} warp", want.0, want.1)));
    }
    match &mut f.events[i].kind {
        Kind::Sub { params, entry: e, flow, .. } if e == WARP_ENTRY && flow.is_empty() => {
            *e = entry.to_string();
            *params = None;
            Ok(())
        }
        _ => Err(Error::Msg(format!("teleport: event {} is not a local OcarinaWarp sub flow", want.0))),
    }
}

struct Ctx {
    flag: String,
    n: usize,
    add: (u16, u16),
    set: (u16, u16),
    q: (u16, u16),
    tail: u16,
    able: usize,
    timer: usize,
    gamble: usize,
    deny: u16,
    close: u16,
    pad: u16,
}

fn ops(f: &mut Flow, c: &Ctx, s: Step) -> Res<u16> {
    let n: i32 = i32::try_from(c.n).map_err(|_| Error::Format("teleport: too many destinations"))?;
    match s {
        Step::Next => {
            let one: u16 = put(f, act(c.set, &c.flag, 1, c.tail))?;
            let k: u16 = put(f, cmp(c.q, &c.flag, n + 1, c.tail, one, one))?;
            put(f, act(c.add, &c.flag, 1, k))
        }
        Step::Prev => {
            let last: u16 = put(f, act(c.set, &c.flag, n, c.tail))?;
            let k: u16 = put(f, cmp(c.q, &c.flag, 1, last, c.tail, c.tail))?;
            put(f, act(c.add, &c.flag, -1, k))
        }
        Step::Same => {
            let one: u16 = put(f, act(c.set, &c.flag, 1, c.tail))?;
            put(f, cmp(c.q, &c.flag, 1, one, c.tail, c.tail))
        }
    }
}

fn entry(f: &mut Flow, c: &Ctx, s: Step, name: &str) -> Res<()> {
    let after: u16 = ops(f, c, s)?;
    let g: Kind = remap(&f.events[c.gamble].kind, after, c.close)?;
    let gi: u16 = put(f, g)?;
    let t: Kind = remap(&f.events[c.timer].kind, gi, c.close)?;
    let ti: u16 = put(f, t)?;
    let a: Kind = remap(&f.events[c.able].kind, ti, c.deny)?;
    let ai: u16 = put(f, a)?;
    f.entries.push(Entry { name: name.to_string(), subs: vec![ti, gi], vars: Vec::new(), main: ai, pad: c.pad });
    Ok(())
}

fn clone_to(f: &mut Flow, name: &str, next: u16) -> Res<u16> {
    let mut k: Kind = f.events[find(f, name)?].kind.clone();
    match &mut k {
        Kind::Action { next: n, .. } => *n = next,
        _ => return Err(Error::Format("teleport: a template event is not an action")),
    }
    put(f, k)
}

pub fn patch(b: &mut Bf, mode: Mode, unit: Unit, flag: &str, ds: &[Dest]) -> Res<Report> {
    if ds.is_empty() {
        return Err(Error::Format("teleport: no destinations"));
    }
    let f: &mut Flow = b.flows.iter_mut().find(|x| x.name == FLOW).ok_or(Error::Format("teleport: no Dm_SY_Ocarina flow"))?;
    if f.entries.iter().any(|e| e.name.starts_with(PREFIX)) {
        return Err(Error::Format("teleport: the flow is already patched"));
    }
    let base: Cont = f.events[find(f, WARP)?].kind.params().cloned().ok_or(Error::Format("teleport: the warp template has no parameters"))?;
    let (wa, wn): (u16, u16) = match &f.events[find(f, WARP)?].kind {
        Kind::Action { actor, act, .. } => (*actor, *act),
        _ => return Err(Error::Format("teleport: the warp template is not an action")),
    };
    let pad: u16 = f.entries.iter().find(|e| e.name == WARP_ENTRY).map(|e| e.pad).ok_or(Error::Format("teleport: the flow has no OcarinaWarp entry"))?;
    let deny: u16 = idx(find(f, DENY)?)?;
    let close: u16 = idx(find(f, CLOSE)?)?;
    let (able, timer, gamble): (usize, usize, usize) = (find(f, ABLE)?, find(f, TIMER)?, find(f, GAMBLE)?);
    let add: (u16, u16) = act_slot(f, ADD)?;
    let set: (u16, u16) = act_slot(f, SET)?;
    let q: (u16, u16) = query_slot(f, IS_INT)?;
    let before: usize = f.events.len();
    let mut leaves: Vec<u16> = Vec::new();
    for d in ds {
        let k: Kind = leaf(&base, (wa, wn), mode, unit, d)?;
        leaves.push(put(f, k)?);
    }
    let mut depth: usize = 0;
    let root: u16 = tree(f, q, flag, &leaves, 0, leaves.len(), &mut depth, 0)?;
    let icon: u16 = clone_to(f, ICON, root)?;
    let tail: u16 = clone_to(f, MOVE, icon)?;
    let c: Ctx = Ctx { flag: flag.to_string(), n: ds.len(), add, set, q, tail, able, timer, gamble, deny, close, pad };
    entry(f, &c, Step::Next, ENTRY_NEXT)?;
    entry(f, &c, Step::Prev, ENTRY_PREV)?;
    entry(f, &c, Step::Same, ENTRY_SAME)?;
    hook(f, SONG_NEXT, ENTRY_NEXT)?;
    hook(f, SONG_PREV, ENTRY_PREV)?;
    hook(f, SONG_SAME, ENTRY_SAME)?;
    Ok(Report { added: f.events.len() - before, leaves: leaves.len(), depth, root })
}

fn checked_bf(raw: &[u8], mode: Mode, unit: Unit, flag: &str, ds: &[Dest]) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    patch(&mut b, mode, unit, flag, ds)?;
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(Error::Format("teleport: the patched flow does not parse back to the edited model"));
    }
    Ok(out)
}

fn checked_byml(raw: &[u8], ds: &[Dest]) -> Res<Vec<u8>> {
    let mut b: Byml = Byml::parse(raw)?;
    teleport_start::add(&mut b, ds)?;
    let out: Vec<u8> = b.write()?;
    if Byml::parse(&out)? != b {
        return Err(Error::Format("teleport: the new StartPos file does not parse back to the edited model"));
    }
    Ok(out)
}

pub fn stage(rf: &mut Romfs, mode: Mode, unit: Unit, flag: &str, ds: &[Dest]) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    if mode == Mode::Start {
        outs.push(staging::rewrite(rf, teleport_start::FILE, |raw| checked_byml(raw, ds))?);
    }
    outs.push(staging::rewrite(rf, FLOW_FILE, |raw| checked_bf(raw, mode, unit, flag, ds))?);
    Ok(outs)
}

pub fn build(romfs: &P, load: &P, name: &str, mode: Mode, unit: Unit, flag: &str, ds: &[Dest]) -> Res<String> {
    let mut rf: Romfs = Romfs::open(romfs)?;
    let outs: Vec<Out> = stage(&mut rf, mode, unit, flag, ds)?;
    let rst: Rst = rsizetable::load(&mut rf)?;
    let mut note: String = String::new();
    for x in &outs {
        let was: Option<u32> = rst.get(&x.path);
        let mut r: Rst = rst.clone();
        let now: Option<u32> = r.grow(&x.path, x.old as u64, x.new as u64, MARGIN)?;
        let _ = writeln!(note, "size table {}: {:?} -> {:?}", x.path, was, now);
    }
    let c: staging::Committed = staging::commit(&mut rf, load, name, &outs, None)?;
    Ok(format!("{}{note}", c.report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::Actor;
    use std::collections::BTreeSet as BS;

    fn actor(n: &str, acts: &[&str], qs: &[&str]) -> Actor {
        Actor {
            name: n.to_string(),
            sub: String::new(),
            arg: String::new(),
            actions: acts.iter().map(|s| s.to_string()).collect(),
            queries: qs.iter().map(|s| s.to_string()).collect(),
            params: Some(Cont { items: vec![("EventMemberCreateMethod".to_string(), Val::Int(0))] }),
            ep: 0xFFFF,
            cut: 1,
        }
    }

    fn named(n: &str, kind: Kind) -> Event {
        Event { name: n.to_string(), kind }
    }

    fn song(n: &str, start: &str) -> Event {
        let c: Cont = Cont { items: vec![("NextStartPos".to_string(), Val::Str(start.to_string()))] };
        named(n, Kind::Sub { next: 0, params: Some(c), flow: String::new(), entry: WARP_ENTRY.to_string() })
    }

    fn sample() -> Bf {
        let warp: Cont = Cont {
            items: vec![
                ("NextBanc".to_string(), Val::Arg("ChildBanc".to_string())),
                ("NextStartPos".to_string(), Val::Arg("NextStartPos".to_string())),
                ("NextPosition".to_string(), Val::Floats(vec![0.0, 0.0, 0.0])),
                ("NextStartRot".to_string(), Val::Floats(vec![0.0, 0.0, 0.0])),
                ("IsItemWarp".to_string(), Val::Bool(1)),
            ],
        };
        let sw = |cases: Vec<Case>| Kind::Switch { actor: 2, query: 0, cases, params: None };
        let ss = |a: u32, b: u32| Kind::SubSwitch {
            cases: vec![Case { val: OK_VAL, ev: a as u16 }, Case { val: 0xb069958, ev: b as u16 }],
            params: None,
            flow: "Other".to_string(),
            entry: "Check".to_string(),
        };
        let events: Vec<Event> = vec![
            named(ABLE, sw(vec![Case { val: 0, ev: 9 }, Case { val: 1, ev: 6 }])),
            named(TIMER, ss(2, 5)),
            named(GAMBLE, ss(3, 5)),
            named(MOVE, Kind::Action { next: 4, actor: 0, act: 1, params: None }),
            named(ICON, Kind::Action { next: NONE, actor: 2, act: 0, params: None }),
            named(CLOSE, Kind::Action { next: NONE, actor: 1, act: 0, params: None }),
            named(DENY, Kind::Action { next: 5, actor: 1, act: 1, params: None }),
            named(WARP, Kind::Action { next: NONE, actor: 1, act: 2, params: Some(warp) }),
            song(SONG_NEXT.0, SONG_NEXT.1),
            song(SONG_PREV.0, SONG_PREV.1),
            song(SONG_SAME.0, SONG_SAME.1),
        ];
        let f: Flow = Flow {
            name: FLOW.to_string(),
            actors: vec![
                actor("EventSystemActor", &["EventTransition"], &[]),
                actor("GameSystemActor", &["EventWait", "EventTriggerFadeOcarinaMelodyHandle", "EventReserveBancChange"], &[]),
                actor(HOLDER, &["EventTriggerSetGameDataBool"], &["EventQueryIsGameDataBool"]),
            ],
            events,
            entries: vec![Entry { name: WARP_ENTRY.to_string(), subs: Vec::new(), vars: Vec::new(), main: 0, pad: 0x40 }],
        };
        Bf { ver: [0, 4, 0, 0], bom: 0xFEFF, shift: 3, pad: 0, name: FLOW.to_string(), flows: vec![f], tls: Vec::new() }
    }

    fn dests(n: usize) -> Vec<Dest> {
        (0..n).map(|i| Dest { id: format!("T{i}"), banc: "FireTemple".to_string(), pos: [i as f32, 1.0, 2.0], yaw: 0.5, label: String::new() }).collect()
    }

    fn targets(e: &Event) -> Vec<u16> {
        match &e.kind {
            Kind::Action { next, .. } | Kind::Join { next } | Kind::Sub { next, .. } => vec![*next],
            Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => cases.iter().map(|c| c.ev).collect(),
            Kind::Fork { join, forks } => forks.iter().copied().chain(std::iter::once(*join)).collect(),
            Kind::Label { .. } => Vec::new(),
        }
    }

    fn reach(f: &Flow, from: u16) -> BS<usize> {
        let mut seen: BS<usize> = BS::new();
        let mut todo: Vec<usize> = vec![from as usize];
        while let Some(i) = todo.pop() {
            if i >= f.events.len() || !seen.insert(i) {
                continue;
            }
            todo.extend(targets(&f.events[i]).into_iter().filter(|t| *t != NONE).map(|t| t as usize));
        }
        seen
    }

    fn run(mode: Mode, n: usize) -> (Bf, Report) {
        let mut b: Bf = sample();
        let r: Report = patch(&mut b, mode, Unit::Rad, INDEX_FLAG, &dests(n)).unwrap();
        (b, r)
    }

    #[test]
    fn every_target_is_in_range() {
        let (b, _) = run(Mode::Start, 12);
        let f: &Flow = &b.flows[0];
        for e in &f.events {
            for t in targets(e) {
                assert!(t == NONE || (t as usize) < f.events.len());
            }
        }
    }

    #[test]
    fn every_leaf_is_reached_from_every_entry() {
        for n in [1usize, 2, 3, 12, 13, 64, 200] {
            let (b, r) = run(Mode::Coords, n);
            let f: &Flow = &b.flows[0];
            assert_eq!(r.leaves, n);
            for name in [ENTRY_NEXT, ENTRY_PREV, ENTRY_SAME] {
                let e: &Entry = f.entries.iter().find(|e| e.name == name).unwrap();
                let seen: BS<usize> = reach(f, e.main);
                let hit: usize = f.events.iter().enumerate().filter(|(i, v)| v.name.starts_with(PREFIX) && matches!(&v.kind, Kind::Action { params: Some(c), .. } if c.get("NextBanc").is_some()) && seen.contains(i)).count();
                assert_eq!(hit, n);
                for s in &e.subs {
                    assert!(matches!(f.events[*s as usize].kind, Kind::SubSwitch { .. }));
                    assert!(seen.contains(&(*s as usize)));
                }
            }
        }
    }

    #[test]
    fn tree_selects_the_right_leaf() {
        for n in [1usize, 2, 5, 37, 100] {
            let (b, r) = run(Mode::Coords, n);
            let f: &Flow = &b.flows[0];
            for want in 1..=n {
                let mut at: usize = r.root as usize;
                let mut steps: usize = 0;
                loop {
                    steps += 1;
                    assert!(steps <= r.depth + 1);
                    match &f.events[at].kind {
                        Kind::Switch { cases, params: Some(c), .. } => {
                            let Some(Val::Int(v)) = c.get("Value") else {
                                panic!("no value")
                            };
                            at = if (want as i32) < *v { cases[0].ev as usize } else { cases[1].ev as usize };
                        }
                        Kind::Action { params: Some(c), .. } => {
                            let Some(Val::Floats(p)) = c.get("NextPosition") else {
                                panic!("no position")
                            };
                            assert_eq!(p[0], (want - 1) as f32);
                            break;
                        }
                        _ => panic!("unexpected node"),
                    }
                }
            }
        }
    }

    #[test]
    fn songs_are_redirected_and_vanilla_events_stay() {
        let (b, _) = run(Mode::Start, 5);
        let f: &Flow = &b.flows[0];
        for (n, want) in [(SONG_NEXT.0, ENTRY_NEXT), (SONG_PREV.0, ENTRY_PREV), (SONG_SAME.0, ENTRY_SAME)] {
            let e: &Event = f.events.iter().find(|e| e.name == n).unwrap();
            assert!(matches!(&e.kind, Kind::Sub { entry, params: None, .. } if entry == want));
        }
        let o: &Entry = f.entries.iter().find(|e| e.name == WARP_ENTRY).unwrap();
        assert_eq!(o.main, 0);
    }

    #[test]
    fn modes_differ_only_in_the_leaf_parameters() {
        let (a, _) = run(Mode::Start, 3);
        let (c, _) = run(Mode::Coords, 3);
        assert_eq!(a.flows[0].events.len(), c.flows[0].events.len());
        let la: &Event = a.flows[0].events.iter().find(|e| matches!(&e.kind, Kind::Action { params: Some(p), .. } if p.get("NextBanc").is_some() && e.name.starts_with(PREFIX))).unwrap();
        let Kind::Action { params: Some(p), .. } = &la.kind else { panic!() };
        assert_eq!(p.get("NextStartPos"), Some(&Val::Str("TP_T0".to_string())));
        let lc: &Event = c.flows[0].events.iter().find(|e| matches!(&e.kind, Kind::Action { params: Some(p), .. } if p.get("NextBanc").is_some() && e.name.starts_with(PREFIX))).unwrap();
        let Kind::Action { params: Some(p), .. } = &lc.kind else { panic!() };
        assert_eq!(p.get("NextStartPos"), Some(&Val::Str(String::new())));
        assert_eq!(p.get("NextStartRot"), Some(&Val::Floats(vec![0.0, 0.5, 0.0])));
    }

    #[test]
    fn written_flow_parses_back_and_double_patch_is_refused() {
        let (b, _) = run(Mode::Start, 9);
        let w: Vec<u8> = b.write().unwrap();
        assert_eq!(Bf::parse(&w).unwrap(), b);
        let mut again: Bf = b.clone();
        assert!(patch(&mut again, Mode::Start, Unit::Rad, INDEX_FLAG, &dests(2)).is_err());
    }

    #[test]
    fn index_ops_use_the_added_game_data_slots() {
        let (b, _) = run(Mode::Start, 4);
        let a: &crate::bfevfl::Actor = b.flows[0].actors.iter().find(|a| a.name == HOLDER).unwrap();
        assert!(a.actions.iter().any(|x| x == ADD) && a.actions.iter().any(|x| x == SET));
        assert!(a.queries.iter().any(|x| x == IS_INT));
    }

    #[test]
    fn rotation_units_convert_and_wrap() {
        assert!((Unit::Deg.conv(PI) - 180.0).abs() < 1e-3 || (Unit::Deg.conv(PI) + 180.0).abs() < 1e-3);
        assert!((Unit::Deg.conv(3.0 * PI / 2.0) + 90.0).abs() < 1e-3);
        assert!((Unit::Rad.conv(3.0 * PI / 2.0) + PI / 2.0).abs() < 1e-4);
        assert!((Unit::Deg.conv(0.5) - 28.6479).abs() < 1e-3);
        assert!(Unit::parse("deg").is_ok() && Unit::parse("rad").is_ok() && Unit::parse("turns").is_err());
    }

    #[test]
    fn mode_names_parse() {
        assert_eq!(Mode::parse("start").unwrap(), Mode::Start);
        assert_eq!(Mode::parse("coords").unwrap(), Mode::Coords);
        assert!(Mode::parse("x").is_err());
    }
}
