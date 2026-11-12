use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap as BMap, BTreeSet as BSet, HashMap as Map, HashSet as Set};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::bfevfl::flow::NONE;
use crate::bfevfl::{Actor, Case, Cont, Entry, Flow, Kind, Val};
use crate::error::{Error, Res};
use crate::flow_sim::db::Db;
use crate::flow_sim::load::Src;
use crate::flow_sim::state::State;
use crate::flow_sim::vocab::{self, Class};

pub type Env = BMap<String, Val>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RK {
    Ev,
    Ask,
    Set,
    Grant,
    Chal,
    Msg,
    Trans,
    Scene,
    Warn,
}

pub struct Rec {
    pub k: RK,
    pub t: String,
    pub prev: Option<Rc<Rec>>,
}

#[derive(Clone)]
pub enum Ret {
    Sub { flow: usize, next: u16, env: Rc<(u32, Env)> },
    Sw { flow: usize, cases: Vec<Case>, env: Rc<(u32, Env)> },
    Fork { flow: usize, rest: Vec<u16>, join: u16, env: Rc<(u32, Env)> },
}

#[derive(Clone)]
pub struct Mach {
    pub flow: usize,
    pub pc: u16,
    pub env: Rc<(u32, Env)>,
    pub stack: Vec<Ret>,
    pub st: State,
    pub eff: BSet<(usize, u16)>,
    pub log: Option<Rc<Rec>>,
    pub steps: usize,
    pub seen: Map<u64, usize>,
    pub nd: usize,
    pub chosen: usize,
    pub viol: Vec<String>,
    pub writes: BMap<String, String>,
    pub gr: BMap<String, i64>,
    pub reqs: Vec<String>,
    pub trans: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum End {
    Done,
    Menu(String),
    Loop(String),
    Limit,
    NoCase(String),
    Fail(String),
}

pub struct Fin {
    pub end: End,
    pub m: Mach,
}

#[derive(Clone, Debug)]
pub struct Opts {
    pub max_paths: usize,
    pub max_steps: usize,
    pub choose: Vec<u32>,
    pub hour: Option<u8>,
    pub verbose: bool,
    pub follow: bool,
    pub answers: BMap<String, u32>,
}

impl Default for Opts {
    fn default() -> Opts {
        Opts { max_paths: 4000, max_steps: 30000, choose: Vec::new(), hour: None, verbose: false, follow: false, answers: BMap::new() }
    }
}

pub struct Sim {
    pub db: Db,
    pub src: Box<dyn Src>,
    pub opts: Opts,
    pub flows: Vec<Rc<Flow>>,
    pub names: Vec<String>,
    pub fid: Map<String, Option<usize>>,
    pub envs: Map<String, Rc<(u32, Env)>>,
    pub inv: Vec<(String, Vec<(bool, u32)>)>,
    pub merged: Set<u64>,
    pub unknown: BSet<String>,
    pub dropped: usize,
    pub assumed: BMap<String, usize>,
}

pub fn push(m: &mut Mach, k: RK, t: String) {
    let prev: Option<Rc<Rec>> = m.log.take();
    m.log = Some(Rc::new(Rec { k, t, prev }));
}

pub fn lines(l: &Option<Rc<Rec>>) -> Vec<(RK, String)> {
    let mut out: Vec<(RK, String)> = Vec::new();
    let mut c: &Option<Rc<Rec>> = l;
    while let Some(r) = c {
        out.push((r.k, r.t.clone()));
        c = &r.prev;
    }
    out.reverse();
    out
}

fn effectful(c: Class) -> bool {
    matches!(c, Class::DataSet | Class::Grant | Class::Bottle | Class::Challenge | Class::Leave | Class::TimeSet | Class::NpcFlag | Class::WorldBool)
}

impl Sim {
    pub fn new(db: Db, src: Box<dyn Src>, opts: Opts) -> Sim {
        Sim {
            db,
            src,
            opts,
            flows: Vec::new(),
            names: Vec::new(),
            fid: Map::new(),
            envs: Map::new(),
            inv: Vec::new(),
            merged: Set::new(),
            unknown: BSet::new(),
            dropped: 0,
            assumed: BMap::new(),
        }
    }

    pub fn flow_id(&mut self, n: &str) -> Res<Option<usize>> {
        if let Some(i) = self.fid.get(n) {
            return Ok(*i);
        }
        let r: Option<usize> = match self.src.flow(n)? {
            Some(f) => {
                self.flows.push(Rc::new(f));
                self.names.push(n.to_string());
                Some(self.flows.len() - 1)
            }
            None => None,
        };
        self.fid.insert(n.to_string(), r);
        Ok(r)
    }

    pub fn intern(&mut self, e: Env) -> Rc<(u32, Env)> {
        let k: String = format!("{e:?}");
        let n: u32 = self.envs.len() as u32;
        self.envs.entry(k).or_insert_with(|| Rc::new((n, e))).clone()
    }

    pub fn start(&mut self, flow: &str, entry: &str, st: State, args: Env) -> Res<Mach> {
        let fi: usize = self.flow_id(flow)?.ok_or_else(|| Error::Msg(format!("no flow {flow}")))?;
        let f: Rc<Flow> = self.flows[fi].clone();
        let e: &Entry = f
            .entries
            .iter()
            .find(|e| e.name == entry)
            .ok_or_else(|| Error::Msg(format!("flow {flow} has no entry {entry}. entries: {}", f.entries.iter().map(|e| e.name.as_str()).collect::<Vec<&str>>().join(", "))))?;
        let env: Rc<(u32, Env)> = self.intern(args);
        let mut m: Mach = Mach {
            flow: fi,
            pc: e.main,
            env,
            stack: Vec::new(),
            st,
            eff: BSet::new(),
            log: None,
            steps: 0,
            seen: Map::new(),
            nd: 0,
            chosen: 0,
            viol: Vec::new(),
            writes: BMap::new(),
            gr: BMap::new(),
            reqs: Vec::new(),
            trans: Vec::new(),
        };
        push(&mut m, RK::Ev, format!("enter {flow}:{entry} at event {}", e.main));
        Ok(m)
    }

    fn key(&self, m: &Mach, with_eff: bool) -> u64 {
        let mut h: DefaultHasher = DefaultHasher::new();
        m.flow.hash(&mut h);
        m.pc.hash(&mut h);
        m.env.0.hash(&mut h);
        for r in &m.stack {
            match r {
                Ret::Sub { flow, next, env } => (1u8, *flow, *next, env.0).hash(&mut h),
                Ret::Sw { flow, cases, env } => {
                    (2u8, *flow, env.0).hash(&mut h);
                    for c in cases {
                        (c.val, c.ev).hash(&mut h);
                    }
                }
                Ret::Fork { flow, rest, join, env } => (3u8, *flow, rest.clone(), *join, env.0).hash(&mut h),
            }
        }
        m.st.hash(&mut h);
        if with_eff {
            m.eff.hash(&mut h);
        }
        h.finish()
    }

    pub fn explore(&mut self, first: Mach) -> Vec<Fin> {
        let mut work: Vec<Mach> = vec![first];
        let mut out: Vec<Fin> = Vec::new();
        self.merged.clear();
        self.dropped = 0;
        let mut paths: usize = 0;
        while let Some(mut m) = work.pop() {
            paths += 1;
            if paths > self.opts.max_paths {
                out.push(Fin { end: End::Limit, m });
                continue;
            }
            let mut alts: Vec<Mach> = Vec::new();
            let end: Option<End> = loop {
                if m.pc == NONE {
                    match self.unwind(&mut m, None) {
                        Some(e) => break Some(e),
                        None => continue,
                    }
                }
                m.steps += 1;
                if m.steps > self.opts.max_steps {
                    break Some(End::Limit);
                }
                let k: u64 = self.key(&m, false);
                match m.seen.get(&k) {
                    Some(n0) => {
                        let w: String = format!("{} event {}", self.names[m.flow], m.pc);
                        if m.nd > *n0 {
                            break Some(End::Menu(w));
                        }
                        break Some(End::Loop(w));
                    }
                    None => {
                        m.seen.insert(k, m.nd);
                    }
                }
                match self.exec(&mut m, &mut alts) {
                    Some(e) => break Some(e),
                    None => {
                        if !alts.is_empty() {
                            break None;
                        }
                    }
                }
            };
            match end {
                Some(e) => out.push(Fin { end: e, m }),
                None => {
                    for a in alts.into_iter().rev() {
                        let k: u64 = self.key(&a, true);
                        if self.merged.insert(k) {
                            work.push(a);
                        } else {
                            self.dropped += 1;
                        }
                    }
                }
            }
        }
        out
    }

    pub fn unwind(&mut self, m: &mut Mach, label: Option<u32>) -> Option<End> {
        let mut lab: Option<u32> = label;
        loop {
            let Some(top) = m.stack.pop() else {
                m.pc = NONE;
                return Some(End::Done);
            };
            match top {
                Ret::Fork { flow, mut rest, join, env } => {
                    if lab.is_some() {
                        continue;
                    }
                    m.flow = flow;
                    m.env = env.clone();
                    if rest.is_empty() {
                        let f: Rc<Flow> = self.flows[flow].clone();
                        m.pc = match f.events.get(join as usize).map(|e| &e.kind) {
                            Some(Kind::Join { next }) => *next,
                            _ => join,
                        };
                        if m.pc == NONE {
                            continue;
                        }
                        return None;
                    }
                    let nx: u16 = rest.remove(0);
                    m.stack.push(Ret::Fork { flow, rest, join, env });
                    m.pc = nx;
                    return None;
                }
                Ret::Sub { flow, next, env } => {
                    if lab.is_some() {
                        continue;
                    }
                    m.flow = flow;
                    m.env = env;
                    m.pc = next;
                    if next == NONE {
                        continue;
                    }
                    return None;
                }
                Ret::Sw { flow, cases, env } => {
                    m.flow = flow;
                    m.env = env;
                    let hit: Option<Case> = lab.and_then(|h| cases.iter().find(|c| c.val == h).copied());
                    match (lab, hit) {
                        (_, Some(c)) => {
                            push(m, RK::Ev, format!("sub switch returns label {:#x} -> event {}", c.val, c.ev));
                            lab = None;
                            m.pc = c.ev;
                            if c.ev == NONE {
                                continue;
                            }
                            return None;
                        }
                        (Some(h), None) => {
                            push(m, RK::Warn, format!("sub switch label {h:#x} has no case, the chain ends here"));
                            lab = None;
                            continue;
                        }
                        (None, None) => {
                            push(m, RK::Warn, "sub switch callee ended without a label, the chain ends here".to_string());
                            continue;
                        }
                    }
                }
            }
        }
    }

    pub fn resolve(&self, c: &Option<Cont>, env: &Env) -> Env {
        let mut out: Env = Env::new();
        if let Some(c) = c {
            for (k, v) in &c.items {
                let r: Option<Val> = match v {
                    Val::Arg(n) => env.get(n).cloned(),
                    x => Some(x.clone()),
                };
                if let Some(r) = r {
                    out.insert(k.clone(), r);
                }
            }
        }
        out
    }

    fn call(&mut self, m: &mut Mach, flow: &str, entry: &str, params: &Option<Cont>, ret: Ret) -> Option<End> {
        let fi: usize = if flow.is_empty() {
            m.flow
        } else {
            match self.flow_id(flow) {
                Ok(Some(i)) => i,
                Ok(None) => return Some(End::Fail(format!("sub flow {flow} is not in the game"))),
                Err(e) => return Some(End::Fail(format!("sub flow {flow}: {e}"))),
            }
        };
        let f: Rc<Flow> = self.flows[fi].clone();
        let Some(e) = f.entries.iter().find(|e| e.name == entry) else {
            return Some(End::Fail(format!("flow {} has no entry {entry}", self.names[fi])));
        };
        let env: Env = self.resolve(params, &m.env.1);
        let ne: Rc<(u32, Env)> = self.intern(env);
        m.stack.push(ret);
        m.flow = fi;
        m.env = ne;
        m.pc = e.main;
        None
    }

    fn follow(&mut self, m: &mut Mach, flow: &str, entry: &str, ret: Ret) -> Option<End> {
        if !entry.is_empty() {
            return self.call(m, flow, entry, &None, ret);
        }
        let fi: usize = match self.flow_id(flow) {
            Ok(Some(i)) => i,
            _ => return None,
        };
        let f: Rc<Flow> = self.flows[fi].clone();
        let mut order: Vec<String> = vec!["EntryPoint0".to_string()];
        order.extend(self.src.cuts(flow));
        let mut mains: Vec<u16> = Vec::new();
        for n in order {
            if let Some(e) = f.entries.iter().find(|e| e.name == n) {
                mains.push(e.main);
            }
        }
        if mains.is_empty() {
            push(m, RK::Trans, format!("event {flow} is a timeline with no flow entries"));
            return None;
        }
        push(m, RK::Ev, format!("play {flow}: {} entries in timeline order", mains.len()));
        let env: Rc<(u32, Env)> = self.intern(Env::new());
        m.stack.push(ret);
        let first: u16 = mains.remove(0);
        m.stack.push(Ret::Fork { flow: fi, rest: mains, join: NONE, env: env.clone() });
        m.flow = fi;
        m.env = env;
        m.pc = first;
        None
    }

    pub fn exec(&mut self, m: &mut Mach, alts: &mut Vec<Mach>) -> Option<End> {
        let f: Rc<Flow> = self.flows[m.flow].clone();
        let Some(ev) = f.events.get(m.pc as usize) else {
            return Some(End::Fail(format!("event {} is out of range in {}", m.pc, self.names[m.flow])));
        };
        let at: (usize, u16) = (m.flow, m.pc);
        let env: Rc<(u32, Env)> = m.env.clone();
        match &ev.kind {
            Kind::Action { next, actor, act, params } => {
                let a: Option<&Actor> = f.actors.get(*actor as usize);
                let (an, nm): (String, String) = match a {
                    Some(a) => (a.name.clone(), a.actions.get(*act as usize).cloned().unwrap_or_default()),
                    None => (String::new(), String::new()),
                };
                let cls: Class = vocab::action(&nm);
                if nm.starts_with("EventWait") || nm == "EventTriggerSetEventBool" {
                    m.nd += 1;
                }
                let cont: Option<Cont> = params.clone();
                let r: Option<(String, String)> = self.act(m, &an, &nm, &cont, &env.1, cls, &ev.name);
                if effectful(cls) {
                    m.eff.insert(at);
                }
                m.pc = *next;
                if let Some((fl, en)) = r {
                    let ret: Ret = Ret::Sub { flow: m.flow, next: *next, env: env.clone() };
                    return self.follow(m, &fl, &en, ret);
                }
                self.tick(m);
                self.check(m);
                None
            }
            Kind::Switch { actor, query, cases, params } => {
                let a: Option<&Actor> = f.actors.get(*actor as usize);
                let (an, qn): (String, String) = match a {
                    Some(a) => (a.name.clone(), a.queries.get(*query as usize).cloned().unwrap_or_default()),
                    None => (String::new(), String::new()),
                };
                let cont: Option<Cont> = params.clone();
                let vals: Vec<u32> = cases.iter().map(|c| c.val).collect();
                let picks: Vec<(u32, String, bool)> = self.pick(m, &an, &qn, &cont, &env.1, &vals);
                m.eff.insert(at);
                if picks.is_empty() {
                    push(m, RK::Ask, format!("{} switch {an}.{qn} has no case, chain ends", ev.name));
                    m.pc = NONE;
                    return None;
                }
                let multi: bool = picks.len() > 1;
                if multi || !matches!(vocab::query(&qn), Class::QData | Class::QPouch) {
                    m.nd += 1;
                }
                for (v, why, quiet) in picks {
                    let t: Option<u16> = cases.iter().find(|c| c.val == v).map(|c| c.ev);
                    let to: String = match t {
                        Some(e) => format!("event {e}"),
                        None => "no case, chain ends".to_string(),
                    };
                    let line: String = format!("{} switch {an}.{qn} {why} -> {to}", ev.name);
                    let k: RK = if quiet { RK::Ev } else { RK::Ask };
                    if multi {
                        let mut x: Mach = m.clone();
                        push(&mut x, k, line);
                        x.pc = t.unwrap_or(NONE);
                        alts.push(x);
                    } else {
                        push(m, k, line);
                        m.pc = t.unwrap_or(NONE);
                    }
                }
                None
            }
            Kind::Fork { join, forks } => {
                if self.opts.verbose {
                    push(m, RK::Ev, format!("{} fork {} branches", ev.name, forks.len()));
                }
                if forks.is_empty() {
                    m.pc = match f.events.get(*join as usize).map(|e| &e.kind) {
                        Some(Kind::Join { next }) => *next,
                        _ => *join,
                    };
                    return None;
                }
                m.stack.push(Ret::Fork { flow: m.flow, rest: forks[1..].to_vec(), join: *join, env });
                m.pc = forks[0];
                None
            }
            Kind::Join { next } => {
                let ends: bool = matches!(m.stack.last(), Some(Ret::Fork { flow, join, .. }) if *flow == m.flow && *join == m.pc);
                if ends {
                    m.pc = NONE;
                } else {
                    m.pc = *next;
                }
                None
            }
            Kind::Sub { next, params, flow, entry } => {
                push(m, RK::Ev, format!("{} call {}:{entry}", ev.name, if flow.is_empty() { &self.names[m.flow] } else { flow }));
                let ret: Ret = Ret::Sub { flow: m.flow, next: *next, env };
                self.call(m, flow, entry, params, ret)
            }
            Kind::SubSwitch { cases, params, flow, entry } => {
                push(m, RK::Ev, format!("{} call {}:{entry} (label switch)", ev.name, if flow.is_empty() { &self.names[m.flow] } else { flow }));
                let ret: Ret = Ret::Sw { flow: m.flow, cases: cases.clone(), env };
                self.call(m, flow, entry, params, ret)
            }
            Kind::Label { hash } => {
                push(m, RK::Ev, format!("{} label {hash:#x}", ev.name));
                self.unwind(m, Some(*hash))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::{Actor, Bf, Entry, Event};
    use crate::door_fix;
    use crate::flags::hash_name;
    use crate::flow_sim::db::{Def, Tok, Ty};
    use crate::flow_sim::load::Mem;
    use crate::flow_sim::state::V;

    fn actor(n: &str, acts: &[&str], qs: &[&str]) -> Actor {
        Actor {
            name: n.to_string(),
            sub: String::new(),
            arg: String::new(),
            actions: acts.iter().map(|s| s.to_string()).collect(),
            queries: qs.iter().map(|s| s.to_string()).collect(),
            params: None,
            ep: 0xFFFF,
            cut: 1,
        }
    }

    fn cont(v: Vec<(&str, Val)>) -> Option<Cont> {
        Some(Cont { items: v.into_iter().map(|(k, x)| (k.to_string(), x)).collect() })
    }

    fn s(x: &str) -> Val {
        Val::Str(x.to_string())
    }

    fn en(n: &[&str]) -> Def {
        Def { ty: Ty::Enum, dv: V::E(hash_name(n[0])), vals: n.iter().map(|x| x.to_string()).collect(), exp: Vec::new() }
    }

    fn bl() -> Def {
        Def { ty: Ty::Bool, dv: V::B(false), vals: Vec::new(), exp: Vec::new() }
    }

    fn eq(flag: &str, v: &str) -> Def {
        Def {
            ty: Ty::Exp,
            dv: V::B(false),
            vals: Vec::new(),
            exp: vec![Tok { op: 1, a: hash_name(flag), b: 0 }, Tok { op: 2, a: hash_name(v), b: 0 }],
        }
    }

    fn door_db() -> Db {
        let steps: [&str; 12] = ["NotReady", "Ready", "Step01", "Step02", "Step2_1", "Srep2_1_1", "Step2_2", "Step03", "Step04", "Step05", "Step06", "Complete"];
        let q: [&str; 4] = ["NotReady", "Ready", "KakarikoVillageNpc", "Complete"];
        let mut d: Db = Db::from_defs(vec![
            ("Step_DoorOfTime", en(&steps)),
            ("Step_Get_StoneOfFire", en(&q)),
            ("Step_Get_StoneOfWater", en(&q)),
            ("Get_StoneOfFire_IsCompleted_Exp", eq("Step_Get_StoneOfFire", "Complete")),
            ("Get_StoneOfWater_IsCompleted_Exp", eq("Step_Get_StoneOfWater", "Complete")),
            (door_fix::OPENED, bl()),
            ("HavePlayedEvent.Dm_MelodyGet_Time", bl()),
            ("HavePlayedEvent.HyruleField_DoorOfTime_Escape", bl()),
        ]);
        let st: u32 = d.reg("Step_DoorOfTime");
        d.chals.push(crate::flow_sim::db::Chal { name: "DoorOfTime".to_string(), step: st, dep: None });
        d
    }

    fn step(n: &str, who: &str, st: &str, next: u16) -> Event {
        Event {
            name: n.to_string(),
            kind: Kind::Action {
                next,
                actor: 2,
                act: 0,
                params: cont(vec![("ChallengeName", s(who)), ("StepName", s(st)), ("IsIgnoreCompletedChallenge", Val::Bool(0))]),
            },
        }
    }

    fn sw(n: &str, q: u16, name: &str, a: u16, b: u16) -> Event {
        Event {
            name: n.to_string(),
            kind: Kind::Switch {
                actor: 3,
                query: q,
                cases: vec![Case { val: 0, ev: a }, Case { val: 1, ev: b }],
                params: cont(vec![("GameDataName", s(name))]),
            },
        }
    }

    fn door_flow() -> Flow {
        let events: Vec<Event> = vec![
            step("Event7", "DoorOfTime", "Step02", 1),
            step(door_fix::SEND_OFF, "Get_StoneOfFire", "KakarikoVillageNpc", NONE),
            sw(door_fix::FIRE_HEAD, 1, "Get_StoneOfFire_IsCompleted_Exp", 3, 4),
            sw("Event39", 1, "Get_StoneOfWater_IsCompleted_Exp", 5, 6),
            sw("Event56", 1, "Get_StoneOfWater_IsCompleted_Exp", 7, 8),
            step(door_fix::STEP03, "DoorOfTime", "Step03", NONE),
            step("Event147", "DoorOfTime", "Step2_1", NONE),
            step("Event148", "DoorOfTime", "Step2_2", NONE),
            step("Event6", "DoorOfTime", "Step2_0", NONE),
            step("Event77", "DoorOfTime", "Step03", NONE),
            step(door_fix::STEP06, "DoorOfTime", "Step06", NONE),
        ];
        Flow {
            name: door_fix::FLOW.to_string(),
            actors: vec![
                actor("EventSystemActor", &["EventTransition"], &[]),
                actor("GameSystemActor", &["EventWait"], &[]),
                actor("ChallengeSystemActor", &["EventExecuteProgressChallengeStep"], &[]),
                actor("GameDataActor", &[], &["EventQueryHavePlayedEvent", "EventQueryIsGameDataBoolExp", "EventQueryIsGameDataBool"]),
            ],
            events,
            entries: vec![
                Entry { name: "SendOff".to_string(), subs: Vec::new(), vars: Vec::new(), main: 0, pad: 0 },
                Entry { name: "Check".to_string(), subs: Vec::new(), vars: Vec::new(), main: 2, pad: 0 },
            ],
        }
    }

    fn run_flow(f: Flow, d: Db, entry: &str, pre: &[(&str, &str)]) -> (Sim, Vec<Fin>) {
        let mut mem: Mem = Mem::default();
        let name: String = f.name.clone();
        mem.flows.insert(name.clone(), f);
        let mut sim: Sim = Sim::new(d, Box::new(mem), Opts::default());
        let mut st: State = State::new();
        for (k, v) in pre {
            let h: u32 = sim.db.reg(k);
            let val: V = sim.db.parse(h, v).unwrap();
            st.v.insert(h, val);
        }
        sim.settle(&mut st);
        let m: Mach = sim.start(&name, entry, st, Env::new()).unwrap();
        let fins: Vec<Fin> = sim.explore(m);
        (sim, fins)
    }

    fn step_of(sim: &Sim, f: &Fin) -> String {
        crate::flow_sim::report::step_of(&sim.db, &f.m.st, "DoorOfTime")
    }

    fn patched() -> Flow {
        let mut b: Bf = Bf { ver: [0, 4, 0, 0], bom: 0xFEFF, shift: 3, pad: 0, name: door_fix::FLOW.to_string(), flows: vec![door_flow()], tls: Vec::new() };
        door_fix::patch_flow(&mut b).unwrap();
        b.flows.remove(0)
    }

    const DONE: [(&str, &str); 3] = [("Step_Get_StoneOfFire", "Complete"), ("Step_Get_StoneOfWater", "Complete"), ("Step_DoorOfTime", "Step01")];

    #[test]
    fn vanilla_send_off_stops_at_step02_and_the_patched_flow_reaches_step03() {
        let (s0, a) = run_flow(door_flow(), door_db(), "SendOff", &DONE);
        assert_eq!(a.len(), 1);
        assert_eq!(step_of(&s0, &a[0]), "Step02");
        let (s1, c) = run_flow(patched(), door_db(), "SendOff", &DONE);
        assert_eq!(c.len(), 1);
        assert_eq!(step_of(&s1, &c[0]), "Step03");
    }

    #[test]
    fn the_ladder_keeps_later_phases_and_never_goes_backwards() {
        for (flag, want) in [(door_fix::OPENED, "Step06"), ("HavePlayedEvent.Dm_MelodyGet_Time", "Step05"), ("HavePlayedEvent.HyruleField_DoorOfTime_Escape", "Step04")] {
            let mut pre: Vec<(&str, &str)> = DONE.to_vec();
            pre.push((flag, "true"));
            let (s, c) = run_flow(patched(), door_db(), "SendOff", &pre);
            assert_eq!(step_of(&s, &c[0]), want, "with {flag}");
        }
    }

    #[test]
    fn a_request_is_dropped_while_the_challenge_is_not_ready_and_a_dependency_wakes_it() {
        let mut d: Db = door_db();
        let flag: u32 = d.reg("Zelda_Met");
        d.defs.insert(flag, bl());
        d.chals[0].dep = Some(flag);
        let (s, a) = run_flow(door_flow(), d.clone(), "Check", &[("Step_Get_StoneOfFire", "Complete"), ("Step_Get_StoneOfWater", "Complete")]);
        assert_eq!(step_of(&s, &a[0]), "NotReady");
        assert!(a[0].m.reqs.iter().any(|r| r.contains("dropped")));
        let (s2, b) = run_flow(door_flow(), d, "Check", &[("Step_Get_StoneOfFire", "Complete"), ("Step_Get_StoneOfWater", "Complete"), ("Zelda_Met", "true")]);
        assert_eq!(step_of(&s2, &b[0]), "Step03");
    }

    fn mini(events: Vec<Event>, actors: Vec<Actor>, entries: Vec<(&str, u16)>) -> Flow {
        Flow {
            name: "T".to_string(),
            actors,
            events,
            entries: entries.into_iter().map(|(n, m)| Entry { name: n.to_string(), subs: Vec::new(), vars: Vec::new(), main: m, pad: 0 }).collect(),
        }
    }

    fn set_bool(n: &str, flag: &str, v: bool, next: u16) -> Event {
        Event {
            name: n.to_string(),
            kind: Kind::Action { next, actor: 0, act: 0, params: cont(vec![("GameDataName", s(flag)), ("Value", Val::Bool(v as u32)), ("Index", Val::Int(-1))]) },
        }
    }

    fn data_db() -> Db {
        Db::from_defs(vec![("A", bl()), ("B", bl())])
    }

    #[test]
    fn a_player_choice_branches_and_a_scripted_choice_picks_one_branch() {
        let acts: Vec<Actor> = vec![actor("GameDataActor", &["EventTriggerSetGameDataBool"], &[]), actor("GameSystemActor", &[], &["EventQueryTalkChoice2"])];
        let ev: Vec<Event> = vec![
            Event { name: "E0".to_string(), kind: Kind::Switch { actor: 1, query: 0, cases: vec![Case { val: 0, ev: 1 }, Case { val: 1, ev: 2 }], params: None } },
            set_bool("E1", "A", true, NONE),
            set_bool("E2", "B", true, NONE),
        ];
        let (_, all) = run_flow(mini(ev.clone(), acts.clone(), vec![("Go", 0)]), data_db(), "Go", &[]);
        assert_eq!(all.len(), 2);
        let mut mem: Mem = Mem::default();
        mem.flows.insert("T".to_string(), mini(ev, acts, vec![("Go", 0)]));
        let mut o: Opts = Opts::default();
        o.choose = vec![1];
        let mut sim: Sim = Sim::new(data_db(), Box::new(mem), o);
        let m: Mach = sim.start("T", "Go", State::new(), Env::new()).unwrap();
        let fins: Vec<Fin> = sim.explore(m);
        assert_eq!(fins.len(), 1);
        assert_eq!(fins[0].m.writes.keys().collect::<Vec<&String>>(), vec!["B"]);
    }

    #[test]
    fn fork_branches_run_before_the_join_continues_and_labels_return_to_sub_switches() {
        let acts: Vec<Actor> = vec![actor("GameDataActor", &["EventTriggerSetGameDataBool"], &[])];
        let ev: Vec<Event> = vec![
            Event { name: "F".to_string(), kind: Kind::Fork { join: 3, forks: vec![1, 2] } },
            set_bool("X", "A", true, NONE),
            set_bool("Y", "B", true, NONE),
            Event { name: "J".to_string(), kind: Kind::Join { next: 4 } },
            Event { name: "S".to_string(), kind: Kind::SubSwitch { cases: vec![Case { val: 7, ev: 6 }], params: None, flow: String::new(), entry: "Callee".to_string() } },
            Event { name: "L".to_string(), kind: Kind::Label { hash: 7 } },
            set_bool("After", "A", false, NONE),
        ];
        let (sim, f) = run_flow(mini(ev, acts, vec![("Go", 0), ("Callee", 5)]), data_db(), "Go", &[]);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].end, End::Done);
        let h: u32 = hash_name("A");
        assert_eq!(sim.db.get(&f[0].m.st, h), Some(V::B(false)));
        assert_eq!(f[0].m.writes.get("B").map(|x| x.as_str()), Some("true"));
    }

    #[test]
    fn a_loop_without_a_decision_is_an_error_and_with_a_choice_it_is_a_menu() {
        let acts: Vec<Actor> = vec![actor("GameDataActor", &["EventTriggerSetGameDataBool"], &[]), actor("GameSystemActor", &[], &["EventQueryTalkChoice2"])];
        let spin: Vec<Event> = vec![set_bool("A0", "A", true, 1), set_bool("A1", "B", true, 0)];
        let (_, a) = run_flow(mini(spin, acts.clone(), vec![("Go", 0)]), data_db(), "Go", &[]);
        assert!(matches!(a[0].end, End::Loop(_)));
        let menu: Vec<Event> = vec![
            Event { name: "M0".to_string(), kind: Kind::Switch { actor: 1, query: 0, cases: vec![Case { val: 0, ev: 0 }, Case { val: 1, ev: 1 }], params: None } },
            set_bool("Out", "A", true, NONE),
        ];
        let (_, b) = run_flow(mini(menu, acts, vec![("Go", 0)]), data_db(), "Go", &[]);
        assert!(b.iter().any(|f| matches!(f.end, End::Menu(_))));
        assert!(b.iter().any(|f| f.end == End::Done));
    }

    #[test]
    fn a_grant_adds_to_the_totals_and_the_pouch_stock() {
        let acts: Vec<Actor> = vec![actor("PouchSystemActor", &["EventTriggerAddOrRemovePouchContent"], &[])];
        let ev: Vec<Event> = vec![Event {
            name: "G".to_string(),
            kind: Kind::Action { next: NONE, actor: 0, act: 0, params: cont(vec![("Actor", s("Item_X")), ("Num", Val::Int(1)), ("IsGetEvent", Val::Bool(0))]) },
        }];
        let d: Db = Db::from_defs(vec![("PouchData.Item_X.StockNum", Def { ty: Ty::Int, dv: V::I(0), vals: Vec::new(), exp: Vec::new() })]);
        let (sim, f) = run_flow(mini(ev, acts, vec![("Go", 0)]), d, "Go", &[]);
        assert_eq!(f[0].m.gr.get("Item_X"), Some(&1));
        let h: u32 = hash_name("PouchData.Item_X.StockNum");
        assert_eq!(sim.db.int(&f[0].m.st, h), Some(1));
    }
}
