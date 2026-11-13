use crate::bfevfl::flow::NONE;
use crate::bfevfl::{Bf, Case, Cont, Entry, Event, Flow, Kind, Val};
use crate::error::{Error, Res};
use crate::teleport::{Cfg, FLOW, Layout, Mode, PREFIX, Report};
use crate::teleport_dest::{Dest, start_name};

pub const ENTRY_GO: &str = "OOTRRTpGo";
pub const KEY_BANC: &str = "ChildBanc";
pub const KEY_START: &str = "NextStartPos";
pub const OK_VAL: u32 = 0x4db2_11e5;
pub const MELODY_QUERY: &str = "EventQueryMelodyTypeForNormal";
pub const ARRIVE_MELODIES: [u32; 3] = [8, 9, 0xb];
const WARP_ENTRY: &str = "OcarinaWarp";
const ABLE: &str = "Event132";
const TIMER: &str = "Event80";
const GAMBLE: &str = "Event170";
const MOVE: &str = "Event25";
const ICON: &str = "Event169";
const WARP: &str = "Event22";
const DENY: &str = "Event131";
const CLOSE: &str = "Event209";
const WAIT: &str = "Event8";
const SAVE: &str = "Event76";
const HOLDER: &str = "GameDataActor";
const ADD: &str = "EventTriggerAddOrSubGameDataInt";
const SET: &str = "EventTriggerSetGameDataInt";
pub const IS_INT: &str = "EventQueryIsGameDataInt";
pub const JUMP: i32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Next,
    Prev,
    Same,
    Plus,
    GNext,
    GPrev,
}

pub const STEPS: [Step; 6] = [Step::Next, Step::Prev, Step::GNext, Step::GPrev, Step::Same, Step::Plus];

impl Step {
    pub fn entry(self) -> &'static str {
        match self {
            Step::Next => "OOTRRTpNext",
            Step::Prev => "OOTRRTpPrev",
            Step::Same => "OOTRRTpSame",
            Step::Plus => "OOTRRTpPlus",
            Step::GNext => "OOTRRTpGNext",
            Step::GPrev => "OOTRRTpGPrev",
        }
    }

    pub fn song(self) -> (&'static str, &'static str) {
        match self {
            Step::Next => ("Event77", "ForestMinuet"),
            Step::Prev => ("Event15", "FireBolero"),
            Step::GNext => ("Event16", "WaterSerenade"),
            Step::GPrev => ("Event17", "SpiritRequiem"),
            Step::Same => ("Event18", "ShadowNocturne"),
            Step::Plus => ("Event20", "LightPrelude"),
        }
    }

    pub fn extra(self) -> bool {
        matches!(self, Step::GNext | Step::GPrev | Step::Plus)
    }
}

pub fn jump_for(n: usize) -> i32 {
    i32::try_from(n.saturating_sub(1)).unwrap_or(JUMP).clamp(1, JUMP)
}

pub fn groups_of(ds: &[Dest]) -> Res<Vec<(usize, usize)>> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (i, d) in ds.iter().enumerate() {
        let same: bool = out.last().is_some_and(|g| ds[g.0].banc == d.banc);
        if same {
            if let Some(g) = out.last_mut() {
                g.1 += 1;
            }
            continue;
        }
        if ds[..i].iter().any(|x| x.banc == d.banc) {
            return Err(Error::Msg(format!("teleport: the destinations of {} are not in one run", d.banc)));
        }
        out.push((i, 1));
    }
    Ok(out)
}

fn idx(n: usize) -> Res<u16> {
    if n >= NONE as usize {
        return Err(Error::Format("teleport: too many events"));
    }
    Ok(n as u16)
}

fn num(n: usize) -> Res<i32> {
    i32::try_from(n).map_err(|_| Error::Format("teleport: too many destinations"))
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
    f.events.push(Event { name: format!("Tp{at}"), kind });
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

fn holder(f: &mut Flow) -> Res<usize> {
    f.actors
        .iter()
        .position(|a| a.name == HOLDER && a.sub.is_empty())
        .ok_or(Error::Format("teleport: the flow has no GameDataActor"))
}

fn slot(list: &mut Vec<String>, name: &str) -> usize {
    match list.iter().position(|x| x == name) {
        Some(j) => j,
        None => {
            list.push(name.to_string());
            list.len() - 1
        }
    }
}

fn act_slot(f: &mut Flow, name: &str) -> Res<(u16, u16)> {
    let i: usize = holder(f)?;
    let j: usize = slot(&mut f.actors[i].actions, name);
    Ok((idx(i)?, idx(j)?))
}

fn query_slot(f: &mut Flow, name: &str) -> Res<(u16, u16)> {
    let i: usize = holder(f)?;
    let j: usize = slot(&mut f.actors[i].queries, name);
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

fn act(s: (u16, u16), flag: &str, v: i32, next: u16) -> Kind {
    Kind::Action { next, actor: s.0, act: s.1, params: Some(gd(flag, v)) }
}

fn cmp(s: (u16, u16), flag: &str, v: i32, lt: u16, eq: u16, gt: u16) -> Kind {
    Kind::Switch {
        actor: s.0,
        query: s.1,
        cases: vec![Case { val: 0, ev: lt }, Case { val: 1, ev: eq }, Case { val: 2, ev: gt }],
        params: Some(gd(flag, v)),
    }
}

struct Tree {
    q: (u16, u16),
    flag: String,
    depth: usize,
}

impl Tree {
    fn build(&mut self, f: &mut Flow, items: &[u16], keys: &[i32], at: usize) -> Res<u16> {
        if items.len() == 1 {
            self.depth = self.depth.max(at);
            return Ok(items[0]);
        }
        let mid: usize = items.len() / 2;
        let l: u16 = self.build(f, &items[..mid], &keys[..mid], at + 1)?;
        let r: u16 = self.build(f, &items[mid..], &keys[mid..], at + 1)?;
        put(f, cmp(self.q, &self.flag, keys[mid], l, r, r))
    }
}

fn direct(base: &Cont, s: (u16, u16), cfg: &Cfg, d: &Dest, next: u16) -> Res<Kind> {
    let mut c: Cont = base.clone();
    set_item(&mut c, "NextBanc", Val::Str(d.banc.clone()))?;
    match cfg.mode {
        Mode::Start => set_item(&mut c, "NextStartPos", Val::Str(start_name(d)))?,
        Mode::Coords => {
            need_floats(&c, "NextPosition")?;
            need_floats(&c, "NextStartRot")?;
            set_item(&mut c, "NextStartPos", Val::Str(String::new()))?;
            set_item(&mut c, "NextPosition", Val::Floats(d.pos.to_vec()))?;
            set_item(&mut c, "NextStartRot", Val::Floats(vec![0.0, cfg.unit.conv(d.yaw), 0.0]))?;
        }
    }
    Ok(Kind::Action { next, actor: s.0, act: s.1, params: Some(c) })
}

fn shared(d: &Dest) -> Kind {
    let c: Cont = Cont {
        items: vec![
            (KEY_BANC.to_string(), Val::Str(d.banc.clone())),
            (KEY_START.to_string(), Val::Str(start_name(d))),
        ],
    };
    Kind::Sub { next: NONE, params: Some(c), flow: String::new(), entry: ENTRY_GO.to_string() }
}

fn next_of(k: &Kind) -> Vec<u16> {
    match k {
        Kind::Action { next, .. } | Kind::Join { next } | Kind::Sub { next, .. } => vec![*next],
        Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => cases.iter().map(|c| c.ev).collect(),
        Kind::Fork { join, forks } => forks.iter().copied().chain(std::iter::once(*join)).collect(),
        Kind::Label { .. } => Vec::new(),
    }
}

pub fn calls_from(f: &Flow, main: u16) -> Vec<u16> {
    let mut seen: Vec<bool> = vec![false; f.events.len()];
    let mut out: Vec<u16> = Vec::new();
    let mut todo: Vec<u16> = vec![main];
    while let Some(i) = todo.pop() {
        if i == NONE || i as usize >= f.events.len() || seen[i as usize] {
            continue;
        }
        seen[i as usize] = true;
        let k: &Kind = &f.events[i as usize].kind;
        if matches!(k, Kind::Sub { .. } | Kind::SubSwitch { .. }) {
            out.push(i);
        }
        todo.extend(next_of(k).into_iter().rev());
    }
    out
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

fn hook(f: &mut Flow, s: Step) -> Res<()> {
    let (name, want): (&str, &str) = s.song();
    let i: usize = find(f, name)?;
    if text(&f.events[i].kind, KEY_START) != Some(want) {
        return Err(Error::Msg(format!("teleport: event {name} is not the {want} warp")));
    }
    match &mut f.events[i].kind {
        Kind::Sub { params, entry: e, flow, .. } if e == WARP_ENTRY && flow.is_empty() => {
            *e = s.entry().to_string();
            *params = None;
            Ok(())
        }
        _ => Err(Error::Msg(format!("teleport: event {name} is not a local OcarinaWarp sub flow"))),
    }
}

fn clone_to(f: &mut Flow, name: &str, next: u16) -> Res<u16> {
    let mut k: Kind = f.events[find(f, name)?].kind.clone();
    match &mut k {
        Kind::Action { next: n, .. } => *n = next,
        _ => return Err(Error::Format("teleport: a template event is not an action")),
    }
    put(f, k)
}

fn arrival_switch(f: &Flow, i: usize) -> bool {
    let Kind::Switch { actor, query, cases, .. } = &f.events[i].kind else {
        return false;
    };
    let named: bool = f.actors.get(*actor as usize).and_then(|a| a.queries.get(*query as usize)).is_some_and(|q| q == MELODY_QUERY);
    let mut v: Vec<u32> = cases.iter().map(|c| c.val).collect();
    v.sort_unstable();
    named && v == ARRIVE_MELODIES
}

fn quiet(f: &mut Flow) -> Res<()> {
    let at: Vec<usize> = (0..f.events.len()).filter(|&i| arrival_switch(f, i)).collect();
    let [sw] = at.as_slice() else {
        return Err(Error::Msg(format!("teleport: expected one arrival switch, found {}", at.len())));
    };
    let mut k: Kind = f.events[find(f, WAIT)?].kind.clone();
    match &mut k {
        Kind::Action { next, params: Some(c), .. } => {
            *next = NONE;
            set_item(c, "Frames", Val::Int(1))?;
        }
        _ => return Err(Error::Format("teleport: the wait template is not an action with parameters")),
    }
    let w: u16 = put(f, k)?;
    if let Kind::Switch { cases, .. } = &mut f.events[*sw].kind {
        for c in cases.iter_mut() {
            c.ev = w;
        }
    }
    Ok(())
}

struct Ctx {
    flag: String,
    mflag: String,
    groups: bool,
    n: usize,
    add: (u16, u16),
    set: (u16, u16),
    q: (u16, u16),
    tail: u16,
    clamp: u16,
    able: usize,
    timer: usize,
    gamble: usize,
    deny: u16,
    close: u16,
    pad: u16,
}

fn mode_set(f: &mut Flow, c: &Ctx, v: i32, next: u16) -> Res<u16> {
    if !c.groups {
        return Ok(next);
    }
    put(f, act(c.set, &c.mflag, v, next))
}

fn clamp(f: &mut Flow, c: &Ctx, n: i32) -> Res<u16> {
    let one: u16 = put(f, act(c.set, &c.flag, 1, c.tail))?;
    let lo: u16 = put(f, cmp(c.q, &c.flag, 1, one, c.tail, c.tail))?;
    if !c.groups {
        return Ok(lo);
    }
    put(f, cmp(c.q, &c.flag, n + 1, lo, one, one))
}

fn ops(f: &mut Flow, c: &Ctx, s: Step) -> Res<u16> {
    let n: i32 = num(c.n)?;
    match s {
        Step::Next => {
            let one: u16 = put(f, act(c.set, &c.flag, 1, c.tail))?;
            let k: u16 = put(f, cmp(c.q, &c.flag, n + 1, c.tail, one, one))?;
            let a: u16 = put(f, act(c.add, &c.flag, 1, k))?;
            mode_set(f, c, 0, a)
        }
        Step::Prev => {
            let last: u16 = put(f, act(c.set, &c.flag, n, c.tail))?;
            let k: u16 = put(f, cmp(c.q, &c.flag, 1, last, c.tail, c.tail))?;
            let a: u16 = put(f, act(c.add, &c.flag, -1, k))?;
            mode_set(f, c, 0, a)
        }
        Step::Plus => {
            let wrap: u16 = put(f, act(c.add, &c.flag, -n, c.tail))?;
            let k: u16 = put(f, cmp(c.q, &c.flag, n + 1, c.tail, wrap, wrap))?;
            let a: u16 = put(f, act(c.add, &c.flag, jump_for(c.n), k))?;
            mode_set(f, c, 0, a)
        }
        Step::Same => mode_set(f, c, 0, c.clamp),
        Step::GNext => mode_set(f, c, 1, c.clamp),
        Step::GPrev => mode_set(f, c, 2, c.clamp),
    }
}

fn entry(f: &mut Flow, c: &Ctx, s: Step) -> Res<()> {
    let after: u16 = ops(f, c, s)?;
    let g: Kind = remap(&f.events[c.gamble].kind, after, c.close)?;
    let gi: u16 = put(f, g)?;
    let t: Kind = remap(&f.events[c.timer].kind, gi, c.close)?;
    let ti: u16 = put(f, t)?;
    let a: Kind = remap(&f.events[c.able].kind, ti, c.deny)?;
    let ai: u16 = put(f, a)?;
    let subs: Vec<u16> = calls_from(f, ai);
    f.entries.push(Entry { name: s.entry().to_string(), subs, vars: Vec::new(), main: ai, pad: c.pad });
    Ok(())
}

fn go_entry(f: &mut Flow, base: &Cont, s: (u16, u16), pad: u16, next: u16) -> Res<()> {
    let mut c: Cont = base.clone();
    set_item(&mut c, "NextBanc", Val::Arg(KEY_BANC.to_string()))?;
    set_item(&mut c, "NextStartPos", Val::Arg(KEY_START.to_string()))?;
    let main: u16 = put(f, Kind::Action { next, actor: s.0, act: s.1, params: Some(c) })?;
    let subs: Vec<u16> = calls_from(f, main);
    f.entries.push(Entry { name: ENTRY_GO.to_string(), subs, vars: Vec::new(), main, pad });
    Ok(())
}

pub fn patch(b: &mut Bf, cfg: &Cfg, ds: &[Dest]) -> Res<Report> {
    if ds.is_empty() {
        return Err(Error::Format("teleport: no destinations"));
    }
    let gs: Vec<(usize, usize)> = groups_of(ds)?;
    let f: &mut Flow = b.flows.iter_mut().find(|x| x.name == FLOW).ok_or(Error::Format("teleport: no Dm_SY_Ocarina flow"))?;
    if f.entries.iter().any(|e| e.name.starts_with(PREFIX)) {
        return Err(Error::Format("teleport: the flow is already patched"));
    }
    let wi: usize = find(f, WARP)?;
    let base: Cont = f.events[wi].kind.params().cloned().ok_or(Error::Format("teleport: the warp template has no parameters"))?;
    let (wa, wn): (u16, u16) = match &f.events[wi].kind {
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
    let after_go: u16 = if cfg.autosave { clone_to(f, SAVE, NONE)? } else { NONE };
    if cfg.layout == Layout::Shared {
        go_entry(f, &base, (wa, wn), pad, after_go)?;
    }
    let mut leaves: Vec<u16> = Vec::new();
    for d in ds {
        let k: Kind = match cfg.layout {
            Layout::Shared => shared(d),
            Layout::Direct => direct(&base, (wa, wn), cfg, d, after_go)?,
        };
        leaves.push(put(f, k)?);
    }
    let mut t: Tree = Tree { q, flag: cfg.flag.clone(), depth: 0 };
    let mut heads: Vec<u16> = Vec::new();
    let mut firsts: Vec<i32> = Vec::new();
    for (g, &(s, len)) in gs.iter().enumerate() {
        let keys: Vec<i32> = (0..len).map(|i| num(s + i + 1)).collect::<Res<Vec<i32>>>()?;
        let r: u16 = t.build(f, &leaves[s..s + len], &keys, 0)?;
        let d: u16 = if cfg.groups {
            let nx: usize = gs[(g + 1) % gs.len()].0;
            let pv: usize = gs[(g + gs.len() - 1) % gs.len()].0;
            let a: u16 = put(f, act(set, &cfg.flag, num(nx + 1)?, leaves[nx]))?;
            let b: u16 = put(f, act(set, &cfg.flag, num(pv + 1)?, leaves[pv]))?;
            put(f, cmp(q, &cfg.mflag, 1, r, a, b))?
        } else {
            r
        };
        heads.push(d);
        firsts.push(num(s + 1)?);
    }
    let root: u16 = t.build(f, &heads, &firsts, 0)?;
    let icon: u16 = clone_to(f, ICON, root)?;
    let tail: u16 = clone_to(f, MOVE, icon)?;
    let mut c: Ctx = Ctx { flag: cfg.flag.clone(), mflag: cfg.mflag.clone(), groups: cfg.groups, n: ds.len(), add, set, q, tail, clamp: 0, able, timer, gamble, deny, close, pad };
    c.clamp = clamp(f, &c, num(ds.len())?)?;
    let steps: Vec<Step> = STEPS.iter().copied().filter(|s| cfg.groups || !s.extra()).collect();
    for s in &steps {
        entry(f, &c, *s)?;
    }
    for s in &steps {
        hook(f, *s)?;
    }
    if cfg.groups {
        quiet(f)?;
    }
    Ok(Report { added: f.events.len() - before, leaves: leaves.len(), groups: gs.len(), depth: t.depth, root })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::Actor;
    use crate::teleport::{Cfg, Layout, Mode, Unit};
    use crate::teleport_sim;

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
        let c: Cont = Cont { items: vec![(KEY_START.to_string(), Val::Str(start.to_string()))] };
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
        let wait: Cont = Cont { items: vec![("IsWaitEnd".to_string(), Val::Bool(1)), ("Frames".to_string(), Val::Int(30))] };
        let ss = |a: u16, b: u16| Kind::SubSwitch {
            cases: vec![Case { val: OK_VAL, ev: a }, Case { val: 0xb069958, ev: b }],
            params: None,
            flow: "Other".to_string(),
            entry: "Check".to_string(),
        };
        let flag: Cont = Cont { items: vec![("GameDataName".to_string(), Val::Str("HidingHole_AFK_ON".to_string())), ("Value".to_string(), Val::Bool(1))] };
        let events: Vec<Event> = vec![
            named(ABLE, Kind::Switch { actor: 2, query: 0, cases: vec![Case { val: 0, ev: 1 }, Case { val: 1, ev: 6 }], params: None }),
            named(TIMER, ss(2, 5)),
            named(GAMBLE, ss(3, 5)),
            named(MOVE, Kind::Action { next: 4, actor: 0, act: 0, params: None }),
            named(ICON, Kind::Action { next: NONE, actor: 3, act: 0, params: None }),
            named(CLOSE, Kind::Action { next: NONE, actor: 1, act: 0, params: None }),
            named(DENY, Kind::Action { next: 5, actor: 1, act: 0, params: None }),
            named(WARP, Kind::Action { next: NONE, actor: 1, act: 1, params: Some(warp) }),
            song("Event77", "ForestMinuet"),
            song("Event15", "FireBolero"),
            song("Event16", "WaterSerenade"),
            song("Event17", "SpiritRequiem"),
            song("Event18", "ShadowNocturne"),
            song("Event20", "LightPrelude"),
            named(WAIT, Kind::Action { next: NONE, actor: 0, act: 1, params: Some(wait) }),
            named("Event147", Kind::Switch { actor: 1, query: 0, cases: vec![Case { val: 8, ev: 16 }, Case { val: 9, ev: 16 }, Case { val: 0xb, ev: 16 }], params: None }),
            named("Event151", Kind::Action { next: NONE, actor: 3, act: 0, params: Some(flag) }),
            named(SAVE, Kind::Action { next: NONE, actor: 1, act: 2, params: None }),
        ];
        let f: Flow = Flow {
            name: FLOW.to_string(),
            actors: vec![
                actor("EventSystemActor", &["EventTransition", "EventWait"], &[]),
                actor("GameSystemActor", &["EventTriggerFadeOcarinaMelodyHandle", "EventReserveBancChange", "EventTriggerRequestAutoSave"], &[MELODY_QUERY]),
                actor("Player_Event", &[], &["EventQueryPlayerIsAbleWarp"]),
                actor(HOLDER, &["EventTriggerSetGameDataBool"], &["EventQueryIsGameDataBool"]),
            ],
            events,
            entries: vec![
                Entry { name: WARP_ENTRY.to_string(), subs: vec![1, 2], vars: Vec::new(), main: 0, pad: 0x40 },
                Entry { name: "OcarinaWarp_Arrive".to_string(), subs: Vec::new(), vars: Vec::new(), main: 15, pad: 0x40 },
            ],
        };
        Bf { ver: [0, 4, 0, 0], bom: 0xFEFF, shift: 3, pad: 0, name: FLOW.to_string(), flows: vec![f], tls: Vec::new() }
    }

    fn dests(sizes: &[usize]) -> Vec<Dest> {
        let mut out: Vec<Dest> = Vec::new();
        for (g, n) in sizes.iter().enumerate() {
            for _ in 0..*n {
                let id: usize = out.len() + 1;
                out.push(Dest { id: id.to_string(), banc: format!("Scene{g}"), pos: [id as f32, 1.0, 2.0], yaw: 0.5, label: String::new(), kind: "chest".to_string() });
            }
        }
        out
    }

    fn run(cfg: &Cfg, sizes: &[usize]) -> (Bf, Vec<Dest>, Report) {
        let mut b: Bf = sample();
        let ds: Vec<Dest> = dests(sizes);
        let r: Report = patch(&mut b, cfg, &ds).unwrap();
        (b, ds, r)
    }

    fn sweep(cfg: &Cfg, sizes: &[usize]) -> teleport_sim::Sweep {
        let (b, ds, _) = run(cfg, sizes);
        teleport_sim::sweep(&b.flows[0], &ds, &cfg.flag, &cfg.mflag, cfg.groups).unwrap()
    }

    #[test]
    fn shared_layout_with_groups_walks_every_song_correctly() {
        for sizes in [vec![1usize], vec![1, 1], vec![3, 1, 4], vec![5, 2, 7, 1, 3], vec![23, 1, 1, 9, 2, 2, 6], vec![12]] {
            let s: teleport_sim::Sweep = sweep(&Cfg::new(), &sizes);
            assert!(s.runs > 0);
            assert!(s.bad.is_empty(), "{sizes:?}: {:?}", s.bad);
        }
    }

    #[test]
    fn direct_layout_and_flat_layouts_walk_correctly() {
        for (layout, groups) in [(Layout::Direct, true), (Layout::Direct, false), (Layout::Shared, false)] {
            let mut c: Cfg = Cfg::new();
            c.layout = layout;
            c.groups = groups;
            let s: teleport_sim::Sweep = sweep(&c, &[4, 3, 6]);
            assert!(s.bad.is_empty(), "{:?}", s.bad);
        }
    }

    #[test]
    fn songs_are_redirected_and_the_rest_stay() {
        let (b, _, _) = run(&Cfg::new(), &[3, 2]);
        let f: &Flow = &b.flows[0];
        for s in STEPS {
            let e: &Event = f.events.iter().find(|e| e.name == s.song().0).unwrap();
            assert!(matches!(&e.kind, Kind::Sub { entry, params: None, .. } if entry == s.entry()));
        }
        let mut c: Cfg = Cfg::new();
        c.groups = false;
        let (b2, _, _) = run(&c, &[3, 2]);
        let e: &Event = b2.flows[0].events.iter().find(|e| e.name == "Event16").unwrap();
        assert!(matches!(&e.kind, Kind::Sub { entry, .. } if entry == WARP_ENTRY));
        assert!(b2.flows[0].entries.iter().all(|e| e.name != "OOTRRTpGNext"));
    }

    #[test]
    fn arrival_event_stops_setting_the_song_flags() {
        let b0: Bf = sample();
        assert!(!teleport_sim::arrival_clean(&b0.flows[0], 15).unwrap());
        let (b, _, _) = run(&Cfg::new(), &[3, 2]);
        assert!(teleport_sim::arrival_clean(&b.flows[0], 15).unwrap());
        let mut c: Cfg = Cfg::new();
        c.groups = false;
        let (b2, _, _) = run(&c, &[3, 2]);
        assert!(!teleport_sim::arrival_clean(&b2.flows[0], 15).unwrap());
    }

    #[test]
    fn every_added_event_is_reachable_and_in_range() {
        let (b, _, r) = run(&Cfg::new(), &[3, 5, 2, 1]);
        let f: &Flow = &b.flows[0];
        let mut seen: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
        for e in &f.entries {
            seen.extend(teleport_sim::reach(f, e.main));
        }
        assert!((18..f.events.len()).all(|i| seen.contains(&i)));
        assert_eq!(r.leaves, 11);
        assert_eq!(r.groups, 4);
    }

    #[test]
    fn written_flow_parses_back_and_double_patch_is_refused() {
        let (b, ds, _) = run(&Cfg::new(), &[3, 4]);
        let w: Vec<u8> = b.write().unwrap();
        assert_eq!(Bf::parse(&w).unwrap(), b);
        let mut again: Bf = b.clone();
        assert!(patch(&mut again, &Cfg::new(), &ds).is_err());
    }

    #[test]
    fn broken_groups_and_modes_are_refused() {
        let mut ds: Vec<Dest> = dests(&[2, 2]);
        ds[1].banc = "Scene1".to_string();
        ds[2].banc = "Scene0".to_string();
        assert!(groups_of(&ds).is_err());
        assert!(patch(&mut Bf::parse(&sample().write().unwrap()).unwrap(), &Cfg::new(), &[]).is_err());
    }

    #[test]
    fn coords_leaf_carries_the_position_and_the_rotation() {
        let mut c: Cfg = Cfg::new();
        c.mode = Mode::Coords;
        c.layout = Layout::Direct;
        c.unit = Unit::Rad;
        let (b, _, _) = run(&c, &[2]);
        let l: &Event = b.flows[0].events.iter().find(|e| matches!(&e.kind, Kind::Action { params: Some(p), .. } if p.get("NextBanc") == Some(&Val::Str("Scene0".to_string())))).unwrap();
        let Kind::Action { params: Some(p), .. } = &l.kind else {
            panic!()
        };
        assert_eq!(p.get("NextStartPos"), Some(&Val::Str(String::new())));
        assert_eq!(p.get("NextStartRot"), Some(&Val::Floats(vec![0.0, 0.5, 0.0])));
    }

    #[test]
    fn autosave_follows_the_warp_when_asked() {
        let mut c: Cfg = Cfg::new();
        c.autosave = true;
        let (b, _, _) = run(&c, &[2]);
        let f: &Flow = &b.flows[0];
        let go: &Entry = f.entries.iter().find(|e| e.name == ENTRY_GO).unwrap();
        let Kind::Action { next, .. } = &f.events[go.main as usize].kind else {
            panic!()
        };
        assert!(matches!(&f.events[*next as usize].kind, Kind::Action { actor: 1, act: 2, .. }));
    }

    #[test]
    fn every_entry_lists_exactly_the_calls_it_reaches() {
        for (layout, groups) in [(Layout::Shared, true), (Layout::Shared, false), (Layout::Direct, true), (Layout::Direct, false)] {
            let mut c: Cfg = Cfg::new();
            c.layout = layout;
            c.groups = groups;
            let (b, _, _) = run(&c, &[3, 2, 4]);
            let f: &Flow = &b.flows[0];
            assert!(teleport_sim::subs_wrong(f).is_empty(), "{:?}", teleport_sim::subs_wrong(f));
            let e: &Entry = f.entries.iter().find(|e| e.name == Step::Next.entry()).unwrap();
            let want: usize = if layout == Layout::Shared { 2 + 9 } else { 2 };
            assert_eq!(e.subs.len(), want);
        }
    }

    #[test]
    fn jump_is_ten_or_less() {
        assert_eq!(jump_for(1), 1);
        assert_eq!(jump_for(5), 4);
        assert_eq!(jump_for(267), 10);
    }
}
