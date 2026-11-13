use std::collections::{BTreeMap as BM, BTreeSet as BS};

use crate::bfevfl::flow::NONE;
use crate::bfevfl::{Cont, Flow, Kind, Val};
use crate::error::{Error, Res};
use crate::teleport_dest::{Dest, start_name};
use crate::teleport_flow::{self, IS_INT, OK_VAL, STEPS, Step};

const LIMIT: usize = 400;
const ABLE_QUERY: &str = "EventQueryPlayerIsAbleWarp";
const RESERVE: &str = "EventReserveBancChange";
const SET: &str = "EventTriggerSetGameDataInt";
const ADD: &str = "EventTriggerAddOrSubGameDataInt";

pub type Flags = BM<String, i32>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub banc: String,
    pub start: String,
    pub steps: usize,
}

struct Frame {
    ret: u16,
    args: BM<String, String>,
}

fn int(c: &Cont, k: &str) -> Res<i32> {
    match c.get(k) {
        Some(Val::Int(v)) => Ok(*v),
        _ => Err(Error::Msg(format!("sim: {k} is not an int"))),
    }
}

fn name(c: &Cont, k: &str) -> Res<String> {
    match c.get(k) {
        Some(Val::Str(s)) => Ok(s.clone()),
        _ => Err(Error::Msg(format!("sim: {k} is not a string"))),
    }
}

fn arg_str(c: &Cont, k: &str, args: &BM<String, String>) -> Res<String> {
    match c.get(k) {
        Some(Val::Str(s)) => Ok(s.clone()),
        Some(Val::Arg(a)) => args.get(a).cloned().ok_or_else(|| Error::Msg(format!("sim: argument {a} was not given"))),
        _ => Err(Error::Msg(format!("sim: {k} is not a string or an argument"))),
    }
}

fn strings(c: &Cont) -> BM<String, String> {
    let mut m: BM<String, String> = BM::new();
    for (k, v) in &c.items {
        if let Val::Str(s) = v {
            m.insert(k.clone(), s.clone());
        }
    }
    m
}

fn names(f: &Flow, actor: u16, i: u16, query: bool) -> &str {
    let a: &crate::bfevfl::Actor = &f.actors[actor as usize];
    let l: &Vec<String> = if query { &a.queries } else { &a.actions };
    l[i as usize].as_str()
}

pub fn run(f: &Flow, entry: &str, flags: &mut Flags) -> Res<Option<Hit>> {
    let e: &crate::bfevfl::Entry = f.entries.iter().find(|e| e.name == entry).ok_or_else(|| Error::Msg(format!("sim: no entry {entry}")))?;
    let mut at: u16 = e.main;
    let mut stack: Vec<Frame> = vec![Frame { ret: NONE, args: BM::new() }];
    for steps in 0..LIMIT {
        if at == NONE {
            let fr: Frame = stack.pop().ok_or(Error::Format("sim: empty stack"))?;
            if stack.is_empty() {
                return Ok(None);
            }
            at = fr.ret;
            continue;
        }
        let ev: &crate::bfevfl::Event = f.events.get(at as usize).ok_or(Error::Format("sim: event out of range"))?;
        let args: BM<String, String> = stack.last().map(|s| s.args.clone()).unwrap_or_default();
        match &ev.kind {
            Kind::Action { next, actor, act, params } => {
                let n: &str = names(f, *actor, *act, false);
                if n == RESERVE {
                    let c: &Cont = params.as_ref().ok_or(Error::Format("sim: reserve without parameters"))?;
                    let banc: String = arg_str(c, "NextBanc", &args)?;
                    let start: String = arg_str(c, "NextStartPos", &args)?;
                    return Ok(Some(Hit { banc, start, steps }));
                }
                if n == SET || n == ADD {
                    let c: &Cont = params.as_ref().ok_or(Error::Format("sim: write without parameters"))?;
                    let k: String = name(c, "GameDataName")?;
                    let v: i32 = int(c, "Value")?;
                    let cur: i32 = flags.get(&k).copied().unwrap_or(0);
                    flags.insert(k, if n == SET { v } else { cur + v });
                }
                at = *next;
            }
            Kind::Switch { actor, query, cases, params } => {
                let q: &str = names(f, *actor, *query, true);
                let pick: u32 = if q == IS_INT {
                    let c: &Cont = params.as_ref().ok_or(Error::Format("sim: compare without parameters"))?;
                    let k: String = name(c, "GameDataName")?;
                    let want: i32 = int(c, "Value")?;
                    let cur: i32 = flags.get(&k).copied().unwrap_or(0);
                    match cur.cmp(&want) {
                        std::cmp::Ordering::Less => 0,
                        std::cmp::Ordering::Equal => 1,
                        std::cmp::Ordering::Greater => 2,
                    }
                } else if q == ABLE_QUERY {
                    0
                } else {
                    return Err(Error::Msg(format!("sim: unsupported query {q}")));
                };
                at = cases.iter().find(|c| c.val == pick).map(|c| c.ev).unwrap_or(NONE);
            }
            Kind::SubSwitch { cases, .. } => {
                at = cases.iter().find(|c| c.val == OK_VAL).map(|c| c.ev).ok_or(Error::Format("sim: no ok case"))?;
            }
            Kind::Sub { next, params, flow, entry: callee } => {
                if !flow.is_empty() {
                    return Err(Error::Format("sim: a call into another flow"));
                }
                let t: &crate::bfevfl::Entry = f.entries.iter().find(|e| &e.name == callee).ok_or_else(|| Error::Msg(format!("sim: no entry {callee}")))?;
                let a: BM<String, String> = params.as_ref().map(strings).unwrap_or_default();
                stack.push(Frame { ret: *next, args: a });
                at = t.main;
            }
            _ => return Err(Error::Format("sim: unsupported event kind")),
        }
    }
    Err(Error::Format("sim: the walk did not end"))
}

pub fn expect(ds: &[Dest], gs: &[(usize, usize)], s: Step, n0: i32) -> Option<(i32, usize)> {
    let n: i32 = ds.len() as i32;
    let cur: i32 = if n0 < 1 || n0 > n { 1 } else { n0 };
    let g: usize = gs.iter().position(|&(a, l)| (cur as usize) > a && (cur as usize) <= a + l)?;
    let nn: i32 = match s {
        Step::Next => cur % n + 1,
        Step::Prev => (cur + n - 2) % n + 1,
        Step::Same => cur,
        Step::Plus => (cur - 1 + teleport_flow::jump_for(ds.len())) % n + 1,
        Step::GNext => gs[(g + 1) % gs.len()].0 as i32 + 1,
        Step::GPrev => gs[(g + gs.len() - 1) % gs.len()].0 as i32 + 1,
    };
    Some((nn, nn as usize - 1))
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sweep {
    pub runs: usize,
    pub bad: Vec<String>,
    pub deepest: usize,
}

pub fn sweep(f: &Flow, ds: &[Dest], flag: &str, mflag: &str, groups: bool) -> Res<Sweep> {
    let gs: Vec<(usize, usize)> = teleport_flow::groups_of(ds)?;
    let mut out: Sweep = Sweep::default();
    let n: i32 = ds.len() as i32;
    for s in STEPS.iter().filter(|s| groups || !s.extra()) {
        for n0 in (0..=n + 1).chain([n + 40, -3]) {
            let inside: bool = n0 >= 1 && n0 <= n;
            if !inside && (!matches!(s, Step::Same | Step::GNext | Step::GPrev) || (!groups && n0 > n)) {
                continue;
            }
            for m0 in [0, 1, 2, 9] {
                let mut fl: Flags = Flags::new();
                fl.insert(flag.to_string(), n0);
                fl.insert(mflag.to_string(), m0);
                let hit: Option<Hit> = run(f, s.entry(), &mut fl)?;
                out.runs += 1;
                let Some((nn, di)) = expect(ds, &gs, *s, n0) else {
                    out.bad.push(format!("{:?} from {n0}: no group", s));
                    continue;
                };
                let got: Option<i32> = fl.get(flag).copied();
                match hit {
                    Some(h) => {
                        out.deepest = out.deepest.max(h.steps);
                        let want_banc: &str = &ds[di].banc;
                        let want_start: String = start_name(&ds[di]);
                        if h.banc != want_banc || h.start != want_start || got != Some(nn) {
                            out.bad.push(format!("{:?} from {n0} mode {m0}: got {}/{} flag {:?}, want {}/{} flag {}", s, h.banc, h.start, got, want_banc, want_start, nn));
                        }
                    }
                    None => out.bad.push(format!("{:?} from {n0} mode {m0}: no destination", s)),
                }
            }
        }
    }
    Ok(out)
}

pub fn reach(f: &Flow, from: u16) -> BS<usize> {
    let mut seen: BS<usize> = BS::new();
    let mut todo: Vec<usize> = vec![from as usize];
    while let Some(i) = todo.pop() {
        if i >= f.events.len() || !seen.insert(i) {
            continue;
        }
        let t: Vec<u16> = match &f.events[i].kind {
            Kind::Action { next, .. } | Kind::Join { next } => vec![*next],
            Kind::Sub { next, entry, .. } => {
                let mut v: Vec<u16> = vec![*next];
                if let Some(e) = f.entries.iter().find(|e| &e.name == entry) {
                    v.push(e.main);
                }
                v
            }
            Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => cases.iter().map(|c| c.ev).collect(),
            Kind::Fork { join, forks } => forks.iter().copied().chain(std::iter::once(*join)).collect(),
            Kind::Label { .. } => Vec::new(),
        };
        todo.extend(t.into_iter().filter(|x| *x != NONE).map(|x| x as usize));
    }
    seen
}

pub fn arrival_clean(f: &Flow, from: u16) -> Res<bool> {
    let seen: BS<usize> = reach(f, from);
    for i in seen {
        if let Kind::Action { params: Some(c), .. } = &f.events[i].kind {
            if let Some(Val::Str(s)) = c.get("GameDataName") {
                if s == "HidingHole_AFK_ON" || s == "IsSettingMapPlayerIconAnchor" {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

pub fn subs_wrong(f: &Flow) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for e in &f.entries {
        let a: BS<u16> = teleport_flow::calls_from(f, e.main).into_iter().collect();
        let b: BS<u16> = e.subs.iter().copied().collect();
        if a != b || e.subs.len() != b.len() {
            out.push(e.name.clone());
        }
    }
    out
}

pub fn indexes_wrong(f: &Flow) -> usize {
    let mut bad: usize = 0;
    for e in &f.events {
        match &e.kind {
            Kind::Action { actor, act, .. } => {
                let ok: bool = f.actors.get(*actor as usize).is_some_and(|a| (*act as usize) < a.actions.len());
                bad += usize::from(!ok);
            }
            Kind::Switch { actor, query, cases, .. } => {
                let ok: bool = f.actors.get(*actor as usize).is_some_and(|a| (*query as usize) < a.queries.len());
                bad += usize::from(!ok || cases.is_empty());
            }
            _ => {}
        }
    }
    bad
}
