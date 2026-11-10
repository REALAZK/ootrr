use std::collections::{BTreeSet as BS, HashMap as HM};

use crate::bfevfl::cont::{Cont, Val};
use crate::bfevfl::flow::{Event, Flow, Kind, NONE};
use crate::bfevfl::rewards;
use crate::bfevfl::Bf;
use crate::error::Res;
use crate::event_chests::resolve::Row;
use crate::romfs::Romfs;

pub const GENERIC_DEMOS: [&str; 2] = ["Dm_SY_Treasurebox", "Dm_SY_TreasureboxSmall"];

pub const LIBRARY: [&str; 9] = [
    "GetPouchContent",
    "Dm_SY_Get",
    "ActionGuide",
    "Dm_SY_Treasurebox",
    "Dm_SY_TreasureboxSmall",
    "InitTalk",
    "NaviEventCommon",
    "Common_Camera_Pln",
    "NpcCommon",
];

const SHOWN: [&str; 15] = [
    "GameDataName",
    "Value",
    "Index",
    "EventName",
    "ActorName",
    "Actor",
    "Num",
    "TakenRewardActor",
    "ReceiveItem",
    "ReveiveItem",
    "FlagName",
    "EquipmentActorName",
    "ChallengeName",
    "StepName",
    "Name",
];

#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub stem: String,
    pub idx: usize,
    pub kind: &'static str,
    pub head: String,
    pub args: Vec<(String, String)>,
}

#[derive(Default, Debug)]
pub struct Reach {
    pub steps: Vec<Step>,
    pub stems: BS<String>,
    pub missing: Vec<String>,
    pub items: Vec<(String, String, String)>,
    pub demos: Vec<String>,
}

pub struct Loader<'a> {
    pub rf: &'a mut Romfs,
    pub table: &'a HM<String, Row>,
    cache: HM<String, Option<Bf>>,
}

fn show_val(v: &Val) -> String {
    match v {
        Val::Arg(s) => format!("arg:{s}"),
        Val::Int(x) => x.to_string(),
        Val::Bool(x) => (*x != 0).to_string(),
        Val::Str(s) => s.clone(),
        Val::Actor(a, _) => a.clone(),
        _ => "?".to_string(),
    }
}

fn args_of(c: &Option<Cont>) -> Vec<(String, String)> {
    let Some(c) = c else { return Vec::new() };
    c.items
        .iter()
        .filter(|(k, _)| SHOWN.contains(&k.as_str()))
        .map(|(k, x)| (k.clone(), show_val(x)))
        .collect()
}

impl<'a> Loader<'a> {
    pub fn new(rf: &'a mut Romfs, table: &'a HM<String, Row>) -> Loader<'a> {
        Loader { rf, table, cache: HM::new() }
    }

    pub fn has_110(&self, stem: &str) -> bool {
        self.rf.read(&format!("Event/EventFlow/{stem}.110.bfevfl.zs")).is_ok()
    }

    pub fn bf(&mut self, stem: &str) -> Option<&Bf> {
        if !self.cache.contains_key(stem) {
            let p: String = format!("Event/EventFlow/{stem}.bfevfl.zs");
            let b: Option<Bf> = self.rf.read_zs(&p).ok().and_then(|raw| Bf::parse(&raw).ok());
            self.cache.insert(stem.to_string(), b);
        }
        self.cache.get(stem).and_then(|b| b.as_ref())
    }
}

fn entry_of(bf: &Bf, name: &str) -> Option<(usize, u16)> {
    if name.is_empty() {
        let f: &Flow = bf.flows.first()?;
        let e = f.entries.iter().find(|e| e.name == "EntryPoint0").or(f.entries.first())?;
        return Some((0, e.main));
    }
    for (i, f) in bf.flows.iter().enumerate() {
        if let Some(e) = f.entries.iter().find(|e| e.name == name) {
            return Some((i, e.main));
        }
    }
    None
}

fn actor_text_tl(a: &[crate::bfevfl::Actor], i: u16, n: u16) -> String {
    let Some(x) = a.get(i as usize) else { return "?".to_string() };
    format!("{}.{}", x.name, x.actions.get(n as usize).map(|s| s.as_str()).unwrap_or("?"))
}

fn actor_text(f: &Flow, a: u16, n: u16, query: bool) -> String {
    let Some(x) = f.actors.get(a as usize) else { return "?".to_string() };
    let l: &Vec<String> = if query { &x.queries } else { &x.actions };
    format!("{}.{}", x.name, l.get(n as usize).map(|s| s.as_str()).unwrap_or("?"))
}

pub fn walk(l: &mut Loader, stem: &str, entry: &str) -> Res<Reach> {
    let mut out: Reach = Reach::default();
    let mut seen: BS<(String, usize, usize)> = BS::new();
    let mut todo: Vec<(String, String, usize)> = vec![(stem.to_string(), entry.to_string(), 0)];
    let mut called: BS<(String, String)> = BS::new();
    while let Some((s, e, depth)) = todo.pop() {
        if !called.insert((s.clone(), e.clone())) {
            continue;
        }
        out.stems.insert(s.clone());
        let Some(bf) = l.bf(&s).cloned() else {
            out.missing.push(format!("{s}:{e}"));
            continue;
        };
        if depth > 0 {
            for t in &bf.tls {
                for c in &t.clips {
                    let h: String = actor_text_tl(&t.actors, c.actor, c.act);
                    out.steps.push(Step { stem: s.clone(), idx: 0, kind: "clip", head: h, args: args_of(&c.params) });
                }
                for c in &t.shots {
                    let h: String = actor_text_tl(&t.actors, c.actor, c.act);
                    out.steps.push(Step { stem: s.clone(), idx: 0, kind: "shot", head: h, args: args_of(&c.params) });
                }
            }
        }
        let Some((fi, main)) = entry_of(&bf, &e) else {
            if !bf.tls.is_empty() && e.is_empty() {
                continue;
            }
            out.missing.push(format!("{s}:{e}"));
            continue;
        };
        let f: &Flow = &bf.flows[fi];
        let hits: Vec<rewards::Hit> = rewards::hits(&bf).into_iter().nth(fi).map(|x| x.1).unwrap_or_default();
        let mut stack: Vec<u16> = vec![main];
        while let Some(i) = stack.pop() {
            if i == NONE || (i as usize) >= f.events.len() || !seen.insert((s.clone(), fi, i as usize)) {
                continue;
            }
            let ev: &Event = &f.events[i as usize];
            for h in hits.iter().filter(|h| h.index == i as usize) {
                out.items.push((h.role.to_string(), h.item.clone(), h.via.clone()));
            }
            let (kind, head, args): (&'static str, String, Vec<(String, String)>) = match &ev.kind {
                Kind::Action { next, actor, act, params } => {
                    stack.push(*next);
                    let h: String = actor_text(f, *actor, *act, false);
                    let a: Vec<(String, String)> = args_of(params);
                    if h == "EventSystemActor.EventTransition"
                        && let Some((_, n)) = a.iter().find(|(k, _)| k == "EventName")
                    {
                        out.demos.push(n.clone());
                        if !GENERIC_DEMOS.contains(&n.as_str()) && depth < 3 {
                            if let Some(r) = l.table.get(n) {
                                todo.push((r.stem.clone(), r.entry.clone(), depth + 1));
                            } else {
                                out.missing.push(format!("demo {n}"));
                            }
                        }
                    }
                    ("action", h, a)
                }
                Kind::Switch { actor, query, cases, params } => {
                    for c in cases {
                        stack.push(c.ev);
                    }
                    ("switch", actor_text(f, *actor, *query, true), args_of(params))
                }
                Kind::Fork { join, forks } => {
                    stack.push(*join);
                    for x in forks {
                        stack.push(*x);
                    }
                    ("fork", String::new(), Vec::new())
                }
                Kind::Join { next } => {
                    stack.push(*next);
                    ("join", String::new(), Vec::new())
                }
                Kind::Sub { next, params, flow, entry } => {
                    stack.push(*next);
                    let t: String = if flow.is_empty() { s.clone() } else { flow.clone() };
                    if t == s || !LIBRARY.contains(&t.as_str()) {
                        todo.push((t.clone(), entry.clone(), depth));
                    }
                    ("sub", format!("{t}:{entry}"), args_of(params))
                }
                Kind::SubSwitch { cases, params, flow, entry } => {
                    for c in cases {
                        stack.push(c.ev);
                    }
                    let t: String = if flow.is_empty() { s.clone() } else { flow.clone() };
                    if t == s || !LIBRARY.contains(&t.as_str()) {
                        todo.push((t.clone(), entry.clone(), depth));
                    }
                    ("subswitch", format!("{t}:{entry}"), args_of(params))
                }
                Kind::Label { .. } => ("label", String::new(), Vec::new()),
            };
            out.steps.push(Step { stem: s.clone(), idx: i as usize, kind, head, args });
        }
    }
    Ok(out)
}





#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::cont::Cont;
    use crate::bfevfl::flow::{Case, Entry, Event};
    use crate::bfevfl::{Actor, Bf};
    use crate::zs::Zs;

    fn actor(n: &str, acts: &[&str]) -> Actor {
        Actor {
            name: n.to_string(),
            sub: String::new(),
            arg: String::new(),
            actions: acts.iter().map(|x| x.to_string()).collect(),
            queries: vec!["Q".to_string()],
            params: None,
            ep: 0xFFFF,
            cut: 1,
        }
    }

    fn cont(k: &str, v: Val) -> Option<Cont> {
        Some(Cont { items: vec![(k.to_string(), v)] })
    }

    fn bf(entries: &[(&str, u16)], events: Vec<Event>) -> Bf {
        let f: Flow = Flow {
            name: "F".to_string(),
            actors: vec![actor("EventSystemActor", &["EventTransition"]), actor("GameDataActor", &["EventTriggerSetGameDataBool"])],
            events,
            entries: entries.iter().map(|(n, m)| Entry { name: n.to_string(), subs: Vec::new(), vars: Vec::new(), main: *m, pad: 0 }).collect(),
        };
        Bf { ver: [0, 4, 0, 0], bom: 0xFEFF, shift: 3, pad: 0, name: "F".to_string(), flows: vec![f], tls: Vec::new() }
    }

    fn ev(n: &str, kind: Kind) -> Event {
        Event { name: n.to_string(), kind }
    }

    fn loader<'a>(rf: &'a mut Romfs, t: &'a HM<String, Row>, files: Vec<(&str, Bf)>) -> Loader<'a> {
        let mut l: Loader = Loader::new(rf, t);
        for (k, b) in files {
            l.cache.insert(k.to_string(), Some(b));
        }
        l
    }

    #[test]
    fn follows_transitions_subs_and_loops_once() {
        let main: Bf = bf(
            &[("E", 0)],
            vec![
                ev("A", Kind::Action { next: 1, actor: 0, act: 0, params: cont("EventName", Val::Str("Dm_X".to_string())) }),
                ev("B", Kind::Action { next: 2, actor: 1, act: 0, params: cont("GameDataName", Val::Str("Flag".to_string())) }),
                ev("C", Kind::Sub { next: 0, params: None, flow: String::new(), entry: "Local".to_string() }),
                ev("D", Kind::Switch { actor: 1, query: 0, cases: vec![Case { val: 0, ev: 1 }], params: None }),
            ],
        );
        let main: Bf = Bf { flows: vec![Flow { entries: vec![Entry { name: "E".to_string(), subs: Vec::new(), vars: Vec::new(), main: 0, pad: 0 }, Entry { name: "Local".to_string(), subs: Vec::new(), vars: Vec::new(), main: 3, pad: 0 }], ..main.flows[0].clone() }], ..main };
        let demo: Bf = bf(&[("", 0)], vec![ev("X", Kind::Action { next: 0xFFFF, actor: 1, act: 0, params: cont("GameDataName", Val::Str("InDemo".to_string())) })]);
        let mut t: HM<String, Row> = HM::new();
        t.insert("Dm_X".to_string(), Row { stem: "Demo".to_string(), entry: String::new(), comp: String::new() });
        let mut rf: Romfs = Romfs { root: std::path::PathBuf::from("."), zs: Zs::new() };
        let mut l: Loader = loader(&mut rf, &t, vec![("Main", main), ("Demo", demo)]);
        let r: Reach = walk(&mut l, "Main", "E").unwrap();
        assert_eq!(r.demos, vec!["Dm_X".to_string()]);
        let heads: Vec<&str> = r.steps.iter().map(|s| s.head.as_str()).collect();
        assert!(heads.contains(&"GameDataActor.EventTriggerSetGameDataBool"));
        assert_eq!(r.steps.iter().filter(|s| s.args.iter().any(|a| a.1 == "InDemo")).count(), 1);
        assert_eq!(r.steps.iter().filter(|s| s.args.iter().any(|a| a.1 == "Flag")).count(), 1);
        assert!(r.missing.is_empty());
        assert!(r.stems.contains("Demo"));
    }

    #[test]
    fn missing_flows_and_entries_are_reported() {
        let main: Bf = bf(&[("E", 0)], vec![ev("A", Kind::Sub { next: 0xFFFF, params: None, flow: String::new(), entry: "Nope".to_string() })]);
        let t: HM<String, Row> = HM::new();
        let mut rf: Romfs = Romfs { root: std::path::PathBuf::from("."), zs: Zs::new() };
        let mut l: Loader = loader(&mut rf, &t, vec![("Main", main)]);
        let r: Reach = walk(&mut l, "Main", "E").unwrap();
        assert_eq!(r.missing, vec!["Main:Nope".to_string()]);
        let r2: Reach = walk(&mut l, "Absent", "E").unwrap();
        assert_eq!(r2.missing, vec!["Absent:E".to_string()]);
    }
}



