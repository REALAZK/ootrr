use crate::bfevfl::flow::NONE;
use crate::bfevfl::{Bf, Case, Cont, Event, Flow, Kind, Val};
use crate::error::{Error, Res};

pub const STEP_ACT: &str = "EventExecuteProgressChallengeStep";
pub const HOLDER: &str = "GameDataActor";
pub const EXP: &str = "EventQueryIsGameDataBoolExp";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Run {
    WhenTrue,
    WhenFalse,
}

pub fn text<'a>(k: &'a Kind, key: &str) -> Option<&'a str> {
    match k.params()?.get(key)? {
        Val::Str(s) => Some(s.as_str()),
        _ => None,
    }
}

pub fn idx(n: usize) -> Res<u16> {
    if n >= NONE as usize {
        return Err(Error::Format("open world: too many events"));
    }
    Ok(n as u16)
}

pub fn flow_mut<'a>(b: &'a mut Bf, name: &str) -> Res<&'a mut Flow> {
    b.flows.iter_mut().find(|x| x.name == name).ok_or_else(|| Error::Msg(format!("open world: no flow named {name}")))
}

pub fn query_slot(f: &mut Flow, query: &str) -> Res<(u16, u16)> {
    let i: usize = f
        .actors
        .iter()
        .position(|a| a.name == HOLDER && a.sub.is_empty())
        .ok_or_else(|| Error::Msg(format!("open world: the flow {} has no GameDataActor", f.name)))?;
    let a = &mut f.actors[i];
    let j: usize = match a.queries.iter().position(|q| q == query) {
        Some(j) => j,
        None => {
            a.queries.push(query.to_string());
            a.queries.len() - 1
        }
    };
    Ok((idx(i)?, idx(j)?))
}

pub fn step_of<'a>(f: &'a Flow, e: &'a Event, challenge: &str) -> Option<&'a str> {
    let Kind::Action { actor, act, .. } = &e.kind else {
        return None;
    };
    let a: Option<&str> = f.actors.get(*actor as usize).and_then(|x| x.actions.get(*act as usize)).map(|s| s.as_str());
    if a != Some(STEP_ACT) || text(&e.kind, "ChallengeName") != Some(challenge) {
        return None;
    }
    text(&e.kind, "StepName")
}

pub fn retarget(f: &mut Flow, from: u16, to: u16) -> usize {
    let mut n: usize = 0;
    for e in f.events.iter_mut() {
        match &mut e.kind {
            Kind::Action { next, .. } | Kind::Join { next } | Kind::Sub { next, .. } => {
                if *next == from {
                    *next = to;
                    n += 1;
                }
            }
            Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => {
                for c in cases.iter_mut().filter(|c| c.ev == from) {
                    c.ev = to;
                    n += 1;
                }
            }
            Kind::Fork { join, forks } => {
                if *join == from {
                    *join = to;
                    n += 1;
                }
                for x in forks.iter_mut().filter(|x| **x == from) {
                    *x = to;
                    n += 1;
                }
            }
            Kind::Label { .. } => {}
        }
    }
    for e in f.entries.iter_mut() {
        if e.main == from {
            e.main = to;
            n += 1;
        }
        for s in e.subs.iter_mut().filter(|s| **s == from) {
            *s = to;
            n += 1;
        }
    }
    n
}

pub fn guard(f: &mut Flow, targets: &[usize], exp: &str, run: Run, prefix: &str) -> Res<Vec<String>> {
    if f.events.iter().any(|e| e.name.starts_with(prefix)) {
        return Err(Error::Msg(format!("open world: the flow {} is already guarded", f.name)));
    }
    let slot: (u16, u16) = query_slot(f, EXP)?;
    let mut out: Vec<String> = Vec::new();
    for (n, t) in targets.iter().enumerate() {
        let next: u16 = match &f.events.get(*t).map(|e| &e.kind) {
            Some(Kind::Action { next, .. }) => *next,
            _ => return Err(Error::Msg(format!("open world: event {t} of {} is not an action", f.name))),
        };
        let g: u16 = idx(f.events.len())?;
        let ti: u16 = idx(*t)?;
        retarget(f, ti, g);
        let (yes, no): (u16, u16) = match run {
            Run::WhenTrue => (ti, next),
            Run::WhenFalse => (next, ti),
        };
        let cases: Vec<Case> = [(0, yes), (1, no)].into_iter().filter(|(_, ev)| *ev != NONE).map(|(val, ev)| Case { val, ev }).collect();
        let params: Cont = Cont { items: vec![("GameDataName".to_string(), Val::Str(exp.to_string()))] };
        let name: String = format!("{prefix}{n}");
        f.events.push(Event { name: name.clone(), kind: Kind::Switch { actor: slot.0, query: slot.1, cases, params: Some(params) } });
        out.push(f.events[*t].name.clone());
    }
    Ok(out)
}

pub fn guard_steps(b: &mut Bf, flow: &str, challenge: &str, steps: &[&str], exp: &str, run: Run, prefix: &str) -> Res<Vec<String>> {
    let f: &mut Flow = flow_mut(b, flow)?;
    let targets: Vec<usize> = (0..f.events.len()).filter(|&i| step_of(f, &f.events[i], challenge).is_some_and(|s| steps.contains(&s))).collect();
    if targets.is_empty() {
        return Err(Error::Msg(format!("open world: no {challenge} step request to guard in {flow}")));
    }
    guard(f, &targets, exp, run, prefix)
}

pub fn pin(b: &mut Bf, flow: &str, names: &[&str], flag: &str, val: u32) -> Res<usize> {
    let f: &mut Flow = flow_mut(b, flow)?;
    let mut n: usize = 0;
    for name in names {
        let e: &mut Event = f.events.iter_mut().find(|e| e.name == *name).ok_or_else(|| Error::Msg(format!("open world: no event {name} in {flow}")))?;
        if text(&e.kind, "GameDataName") != Some(flag) {
            return Err(Error::Msg(format!("open world: {name} in {flow} does not read {flag}")));
        }
        let Kind::Switch { cases, .. } = &mut e.kind else {
            return Err(Error::Msg(format!("open world: {name} in {flow} is not a switch")));
        };
        let to: u16 = cases.iter().find(|c| c.val == val).map(|c| c.ev).ok_or_else(|| Error::Msg(format!("open world: {name} has no case {val}")))?;
        for c in cases.iter_mut() {
            c.ev = to;
        }
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::{Actor, Entry};

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

    fn step(n: &str, who: &str, st: &str, next: u16) -> Event {
        let items: Vec<(String, Val)> = vec![("ChallengeName".to_string(), Val::Str(who.to_string())), ("StepName".to_string(), Val::Str(st.to_string()))];
        Event { name: n.to_string(), kind: Kind::Action { next, actor: 0, act: 0, params: Some(Cont { items }) } }
    }

    fn sample() -> Bf {
        let events: Vec<Event> = vec![
            step("E0", "Q", "Step01", 1),
            step("E1", "Q", "Step05", NONE),
            step("E2", "Other", "Step01", NONE),
            step("E3", "Q", "Step09", NONE),
            Event { name: "E4".to_string(), kind: Kind::Switch { actor: 1, query: 0, cases: vec![Case { val: 0, ev: 1 }, Case { val: 1, ev: 3 }], params: Some(Cont { items: vec![("GameDataName".to_string(), Val::Str("F".to_string()))] }) } },
        ];
        let f: Flow = Flow {
            name: "Q".to_string(),
            actors: vec![actor("ChallengeSystemActor", &[STEP_ACT], &[]), actor(HOLDER, &[], &["EventQueryIsGameDataBool"])],
            events,
            entries: vec![Entry { name: "A".to_string(), subs: Vec::new(), vars: Vec::new(), main: 0, pad: 0 }, Entry { name: "B".to_string(), subs: vec![1], vars: Vec::new(), main: 4, pad: 0 }],
        };
        Bf { ver: [0, 4, 0, 0], bom: 0xFEFF, shift: 3, pad: 0, name: "Q".to_string(), flows: vec![f], tls: Vec::new() }
    }

    fn cases(f: &Flow, n: &str) -> Vec<(u32, String)> {
        let Kind::Switch { cases, .. } = &f.events.iter().find(|e| e.name == n).unwrap().kind else { panic!() };
        cases.iter().map(|c| (c.val, f.events[c.ev as usize].name.clone())).collect()
    }

    #[test]
    fn only_the_named_steps_of_the_named_quest_are_guarded_and_every_path_runs_through_the_guard() {
        let mut b: Bf = sample();
        let hit: Vec<String> = guard_steps(&mut b, "Q", "Q", &["Step01", "Step05"], "X_Exp", Run::WhenFalse, "G").unwrap();
        assert_eq!(hit, vec!["E0".to_string(), "E1".to_string()]);
        let f: &Flow = &b.flows[0];
        assert_eq!(f.events.len(), 7);
        assert_eq!(f.entries[0].main, 5);
        assert_eq!(f.entries[1].subs, vec![6]);
        assert_eq!(cases(f, "G0"), vec![(0, "G1".to_string()), (1, "E0".to_string())]);
        assert_eq!(cases(f, "G1"), vec![(1, "E1".to_string())]);
        assert_eq!(cases(f, "E4"), vec![(0, "G1".to_string()), (1, "E3".to_string())]);
        let Kind::Action { next, .. } = &f.events[0].kind else { panic!() };
        assert_eq!(*next, 6);
        assert_eq!(f.actors[1].queries, vec!["EventQueryIsGameDataBool".to_string(), EXP.to_string()]);
        let Kind::Switch { params, query, .. } = &f.events[5].kind else { panic!() };
        assert_eq!(*query, 1);
        assert_eq!(params.as_ref().and_then(|c| c.get("GameDataName")), Some(&Val::Str("X_Exp".to_string())));
        assert!(guard_steps(&mut b, "Q", "Q", &["Step01"], "X_Exp", Run::WhenFalse, "G").is_err());
        let back: Bf = Bf::parse(&b.write().unwrap()).unwrap();
        assert_eq!(back, b);
    }

    #[test]
    fn a_true_run_guard_skips_to_the_end_when_the_expression_is_false() {
        let mut b: Bf = sample();
        guard_steps(&mut b, "Q", "Q", &["Step09"], "Y_Exp", Run::WhenTrue, "H").unwrap();
        let f: &Flow = &b.flows[0];
        assert_eq!(cases(f, "H0"), vec![(0, "E3".to_string())]);
        assert!(guard_steps(&mut b, "Q", "Q", &["Step77"], "Y_Exp", Run::WhenTrue, "K").is_err());
        assert!(guard_steps(&mut b, "Nope", "Q", &["Step01"], "Y_Exp", Run::WhenTrue, "K").is_err());
    }

    #[test]
    fn a_pin_sends_every_case_to_the_chosen_one_and_checks_the_flag() {
        let mut b: Bf = sample();
        assert_eq!(pin(&mut b, "Q", &["E4"], "F", 0).unwrap(), 1);
        assert_eq!(cases(&b.flows[0], "E4"), vec![(0, "E1".to_string()), (1, "E1".to_string())]);
        assert!(pin(&mut b, "Q", &["E4"], "Other", 0).is_err());
        assert!(pin(&mut b, "Q", &["E0"], "F", 0).is_err());
        assert!(pin(&mut b, "Q", &["E4"], "F", 7).is_err());
    }
}
