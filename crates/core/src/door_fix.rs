use std::path::Path as P;

use crate::bfevfl::flow::NONE;
use crate::bfevfl::{Bf, Case, Cont, Event, Flow, Kind, Val};
use crate::error::{Error, Res};
use crate::romfs::Romfs;
use crate::staging::{self, Out};

pub const FLOW_FILE: &str = "Event/EventFlow/DoorOfTime.bfevfl.zs";
pub const FLOW: &str = "DoorOfTime";
pub const SEND_OFF: &str = "Event62";
pub const FIRE_HEAD: &str = "Event38";
pub const STEP03: &str = "Event37";
pub const STEP06: &str = "Event57";
pub const OPENED: &str = "Market_Child_TimeTempleDoor_Opened";
pub const SONG: &str = "Dm_MelodyGet_Time";
pub const ESCAPE: &str = "HyruleField_DoorOfTime_Escape";
pub const PREFIX: &str = "OOTRRDoorFix";
/// The request that completes the quest once the Master Sword is drawn.
pub const COMPLETE: &str = "Event93";
/// True once the quest has reached the open door, the only state the vanilla game draws in.
pub const DONE_EXP: &str = "DoorOfTime_IsAfter_Step06_Exp";
const HOLDER: &str = "GameDataActor";
const PLAYED: &str = "EventQueryHavePlayedEvent";
const BOOL: &str = "EventQueryIsGameDataBool";
const BOOL_EXP: &str = "EventQueryIsGameDataBoolExp";
const CHALLENGE: &str = "ChallengeName";
const STEP: &str = "StepName";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub redirected: Vec<String>,
    pub added: usize,
}

fn text<'a>(k: &'a Kind, key: &str) -> Option<&'a str> {
    match k.params()?.get(key)? {
        Val::Str(s) => Some(s.as_str()),
        _ => None,
    }
}

fn find(f: &Flow, name: &str) -> Res<usize> {
    let v: Vec<usize> = (0..f.events.len()).filter(|&i| f.events[i].name == name).collect();
    match v.as_slice() {
        [i] => Ok(*i),
        _ => Err(Error::Msg(format!("door fix: expected one event named {name} in {FLOW}, found {}", v.len()))),
    }
}

fn idx(n: usize) -> Res<u16> {
    if n >= NONE as usize {
        return Err(Error::Format("door fix: too many events"));
    }
    Ok(n as u16)
}

fn query_slot(f: &mut Flow, query: &str) -> Res<(u16, u16)> {
    let i: usize = match f.actors.iter().position(|a| a.name == HOLDER && a.sub.is_empty()) {
        Some(i) => i,
        None => return Err(Error::Format("door fix: the flow has no GameDataActor")),
    };
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

fn retarget(f: &mut Flow, from: u16, to: u16) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for e in f.events.iter_mut() {
        let mut hit: bool = false;
        match &mut e.kind {
            Kind::Action { next, .. } | Kind::Join { next } | Kind::Sub { next, .. } => {
                if *next == from {
                    *next = to;
                    hit = true;
                }
            }
            Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => {
                for c in cases.iter_mut().filter(|c| c.ev == from) {
                    c.ev = to;
                    hit = true;
                }
            }
            Kind::Fork { forks, .. } => {
                for x in forks.iter_mut().filter(|x| **x == from) {
                    *x = to;
                    hit = true;
                }
            }
            Kind::Label { .. } => {}
        }
        if hit {
            out.push(e.name.clone());
        }
    }
    for e in f.entries.iter_mut() {
        let mut hit: bool = false;
        if e.main == from {
            e.main = to;
            hit = true;
        }
        for s in e.subs.iter_mut().filter(|s| **s == from) {
            *s = to;
            hit = true;
        }
        if hit {
            out.push(format!("entry {}", e.name));
        }
    }
    out
}

fn step_event(base: &Event, name: String, step: &str) -> Res<Event> {
    let Kind::Action { actor, act, params, .. } = &base.kind else {
        return Err(Error::Format("door fix: the step template is not an action"));
    };
    let mut c: Cont = params.clone().ok_or(Error::Format("door fix: the step template has no parameters"))?;
    let slot: &mut Val = c
        .items
        .iter_mut()
        .find(|(k, _)| k == STEP)
        .map(|(_, v)| v)
        .ok_or(Error::Format("door fix: the step template has no StepName"))?;
    *slot = Val::Str(step.to_string());
    Ok(Event { name, kind: Kind::Action { next: NONE, actor: *actor, act: *act, params: Some(c) } })
}

fn ask(name: String, slot: (u16, u16), items: Vec<(String, Val)>, yes: u16, no: u16) -> Event {
    Event {
        name,
        kind: Kind::Switch {
            actor: slot.0,
            query: slot.1,
            cases: vec![Case { val: 0, ev: yes }, Case { val: 1, ev: no }],
            params: Some(Cont { items }),
        },
    }
}

fn item(k: &str, v: &str) -> (String, Val) {
    (k.to_string(), Val::Str(v.to_string()))
}

pub fn patch_flow(b: &mut Bf) -> Res<Report> {
    let f: &mut Flow = b.flows.iter_mut().find(|x| x.name == FLOW).ok_or(Error::Format("door fix: no DoorOfTime flow"))?;
    let send: usize = find(f, SEND_OFF)?;
    let head: usize = find(f, FIRE_HEAD)?;
    let s3: usize = find(f, STEP03)?;
    let s6: usize = find(f, STEP06)?;
    match &f.events[send].kind {
        Kind::Action { next, .. } if *next == NONE && text(&f.events[send].kind, CHALLENGE) == Some("Get_StoneOfFire") => {}
        _ => return Err(Error::Format("door fix: the send off event is not the unpatched Get_StoneOfFire step")),
    }
    if !matches!(f.events[head].kind, Kind::Switch { .. }) {
        return Err(Error::Format("door fix: the stone count head is not a switch"));
    }
    if text(&f.events[s3].kind, STEP) != Some("Step03") || text(&f.events[s3].kind, CHALLENGE) != Some("DoorOfTime") {
        return Err(Error::Format("door fix: the Step03 event is not the DoorOfTime Step03 request"));
    }
    if text(&f.events[s6].kind, STEP) != Some("Step06") || text(&f.events[s6].kind, CHALLENGE) != Some("DoorOfTime") {
        return Err(Error::Format("door fix: the Step06 event is not the DoorOfTime Step06 request"));
    }
    let (ga, qb): (u16, u16) = query_slot(f, BOOL)?;
    let (_, qp): (u16, u16) = query_slot(f, PLAYED)?;
    let base: usize = f.events.len();
    let l1: u16 = idx(base)?;
    let l2: u16 = idx(base + 1)?;
    let l3: u16 = idx(base + 2)?;
    let a5: u16 = idx(base + 3)?;
    let a4: u16 = idx(base + 4)?;
    let s3i: u16 = idx(s3)?;
    let s6i: u16 = idx(s6)?;
    let red: Vec<String> = retarget(f, s3i, l1);
    if red.is_empty() {
        return Err(Error::Format("door fix: nothing leads to the Step03 request"));
    }
    if let Kind::Action { next, .. } = &mut f.events[send].kind {
        *next = idx(head)?;
    }
    let t5: Event = step_event(&f.events[s6], format!("{PREFIX}3"), "Step05")?;
    let t4: Event = step_event(&f.events[s6], format!("{PREFIX}4"), "Step04")?;
    let opened: Vec<(String, Val)> = vec![item("GameDataName", OPENED), ("Index".to_string(), Val::Int(-1))];
    f.events.push(ask(format!("{PREFIX}0"), (ga, qb), opened, s6i, l2));
    f.events.push(ask(format!("{PREFIX}1"), (ga, qp), vec![item("EventName", SONG)], a5, l3));
    f.events.push(ask(format!("{PREFIX}2"), (ga, qp), vec![item("EventName", ESCAPE)], a4, s3i));
    f.events.push(t5);
    f.events.push(t4);
    let mut r: Report = Report { redirected: red, added: 5 };
    if f.events.iter().any(|e| e.name == COMPLETE) {
        guard_complete(f, &mut r)?;
    }
    Ok(r)
}

/// Drawing the Master Sword completes the quest. With the door forced open that can happen at
/// Step02, and a completed quest hides Zelda's escape and the Ocarina of Time, which only
/// appear at Step03 and Step04. The request is now made only once the quest reached Step06.
fn guard_complete(f: &mut Flow, r: &mut Report) -> Res<()> {
    let c: usize = find(f, COMPLETE)?;
    if text(&f.events[c].kind, STEP) != Some("Complete") || text(&f.events[c].kind, CHALLENGE) != Some("DoorOfTime") {
        return Err(Error::Format("door fix: the Complete event is not the DoorOfTime Complete request"));
    }
    let (ga, qe): (u16, u16) = query_slot(f, BOOL_EXP)?;
    let g: u16 = idx(f.events.len())?;
    let red: Vec<String> = retarget(f, idx(c)?, g);
    if red.is_empty() {
        return Err(Error::Format("door fix: nothing leads to the Complete request"));
    }
    f.events.push(ask(format!("{PREFIX}5"), (ga, qe), vec![item("GameDataName", DONE_EXP)], idx(c)?, NONE));
    r.redirected.extend(red);
    r.added += 1;
    Ok(())
}

pub fn stage(rf: &mut Romfs) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    outs.push(staging::rewrite(rf, FLOW_FILE, |raw| {
        let mut b: Bf = Bf::parse(raw)?;
        patch_flow(&mut b)?;
        let out: Vec<u8> = b.write()?;
        if Bf::parse(&out)? != b {
            return Err(Error::Format("door fix: the patched flow does not parse back to the edited model"));
        }
        Ok(out)
    })?);
    Ok(outs)
}

pub fn build(romfs: &P, load: &P, name: &str) -> Res<String> {
    let mut rf: Romfs = Romfs::open(romfs)?;
    let outs: Vec<Out> = stage(&mut rf)?;
    let c: staging::Committed = staging::commit(&mut rf, load, name, &outs, None)?;
    Ok(c.report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::{Actor, Entry};
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

    fn step(n: &str, who: &str, step: &str, next: u16) -> Event {
        Event {
            name: n.to_string(),
            kind: Kind::Action {
                next,
                actor: 2,
                act: 0,
                params: Some(Cont {
                    items: vec![
                        ("IsIgnoreCompletedChallenge".to_string(), Val::Bool(0)),
                        ("IsIgnoreChallengeProgressUI".to_string(), Val::Bool(0)),
                        (CHALLENGE.to_string(), Val::Str(who.to_string())),
                        (STEP.to_string(), Val::Str(step.to_string())),
                    ],
                }),
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
                params: Some(Cont { items: vec![("GameDataName".to_string(), Val::Str(name.to_string()))] }),
            },
        }
    }

    fn sample() -> Bf {
        let events: Vec<Event> = vec![
            step("Event7", "DoorOfTime", "Step02", 1),
            step(SEND_OFF, "Get_StoneOfFire", "KakarikoVillageNpc", NONE),
            sw(FIRE_HEAD, 1, "Get_StoneOfFire_IsCompleted_Exp", 3, 4),
            sw("Event39", 1, "Get_StoneOfWater_IsCompleted_Exp", 5, 6),
            sw("Event56", 1, "Get_StoneOfWater_IsCompleted_Exp", 7, 8),
            step(STEP03, "DoorOfTime", "Step03", 9),
            step("Event147", "DoorOfTime", "Step2_1", 10),
            step("Event148", "DoorOfTime", "Step2_2", 10),
            step("Event6", "DoorOfTime", "Step2_0", 10),
            step("Event77", "DoorOfTime", "Step03", 10),
            step(STEP06, "DoorOfTime", "Step06", NONE),
            Event { name: "Event102".to_string(), kind: Kind::Sub { next: 12, params: None, flow: String::new(), entry: "Recovery".to_string() } },
            step(COMPLETE, "DoorOfTime", "Complete", NONE),
        ];
        let f: Flow = Flow {
            name: FLOW.to_string(),
            actors: vec![
                actor("EventSystemActor", &["EventTransition"], &[]),
                actor("GameSystemActor", &["EventWait"], &[]),
                actor("ChallengeSystemActor", &["EventExecuteProgressChallengeStep"], &[]),
                actor(HOLDER, &[], &[PLAYED, "EventQueryIsGameDataBoolExp", BOOL]),
            ],
            events,
            entries: vec![
                Entry { name: "SendOff".to_string(), subs: Vec::new(), vars: Vec::new(), main: 0, pad: 0 },
                Entry { name: "Check".to_string(), subs: Vec::new(), vars: Vec::new(), main: 2, pad: 0 },
                Entry { name: "Rauru".to_string(), subs: Vec::new(), vars: Vec::new(), main: 11, pad: 0 },
            ],
        };
        Bf { ver: [0, 4, 0, 0], bom: 0xFEFF, shift: 3, pad: 0, name: FLOW.to_string(), flows: vec![f], tls: Vec::new() }
    }

    fn reach(f: &Flow) -> BS<usize> {
        let mut seen: BS<usize> = BS::new();
        let mut todo: Vec<usize> = f.entries.iter().flat_map(|e| std::iter::once(e.main).chain(e.subs.iter().copied())).map(|x| x as usize).collect();
        while let Some(i) = todo.pop() {
            if i >= f.events.len() || !seen.insert(i) {
                continue;
            }
            let mut nx: Vec<u16> = Vec::new();
            match &f.events[i].kind {
                Kind::Action { next, .. } | Kind::Join { next } | Kind::Sub { next, .. } => nx.push(*next),
                Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => nx.extend(cases.iter().map(|c| c.ev)),
                Kind::Fork { join, forks } => {
                    nx.push(*join);
                    nx.extend(forks.iter().copied());
                }
                Kind::Label { .. } => {}
            }
            todo.extend(nx.into_iter().filter(|x| *x != NONE).map(|x| x as usize));
        }
        seen
    }

    fn ev<'a>(f: &'a Flow, n: &str) -> &'a Event {
        f.events.iter().find(|e| e.name == n).unwrap()
    }

    fn cases(f: &Flow, n: &str) -> Vec<(u32, String)> {
        let Kind::Switch { cases, .. } = &ev(f, n).kind else { panic!() };
        cases.iter().map(|c| (c.val, f.events[c.ev as usize].name.clone())).collect()
    }

    #[test]
    fn the_quest_completes_only_once_the_door_was_reached() {
        let mut b: Bf = sample();
        let r: Report = patch_flow(&mut b).unwrap();
        assert_eq!(r.added, 6);
        assert!(r.redirected.iter().any(|n| n == "Event102"));
        let f: &Flow = &b.flows[0];
        let Kind::Sub { next, .. } = ev(f, "Event102").kind else { panic!() };
        let g: &Event = &f.events[next as usize];
        assert_eq!(g.name, format!("{PREFIX}5"));
        let Kind::Switch { cases, params, actor, query } = &g.kind else { panic!() };
        assert_eq!(f.events[cases[0].ev as usize].name, COMPLETE);
        assert_eq!(cases[1].ev, NONE);
        assert_eq!(params.as_ref().unwrap().get("GameDataName"), Some(&Val::Str(DONE_EXP.to_string())));
        assert_eq!(f.actors[*actor as usize].queries[*query as usize], BOOL_EXP);
        let mut plain: Bf = sample();
        plain.flows[0].events.truncate(11);
        plain.flows[0].entries.truncate(2);
        assert_eq!(patch_flow(&mut plain).unwrap().added, 5);
    }

    #[test]
    fn the_send_off_now_runs_the_stone_count() {
        let mut b: Bf = sample();
        patch_flow(&mut b).unwrap();
        let f: &Flow = &b.flows[0];
        let Kind::Action { next, .. } = &ev(f, SEND_OFF).kind else { panic!() };
        assert_eq!(f.events[*next as usize].name, FIRE_HEAD);
    }

    #[test]
    fn the_ladder_checks_the_highest_phase_first_and_ends_at_step03() {
        let mut b: Bf = sample();
        let r: Report = patch_flow(&mut b).unwrap();
        let f: &Flow = &b.flows[0];
        assert_eq!(r.added, 6);
        assert_eq!(r.redirected, vec!["Event39".to_string(), "Event102".to_string()]);
        assert_eq!(cases(f, "Event39"), vec![(0, format!("{PREFIX}0")), (1, "Event147".to_string())]);
        let l0: Vec<(u32, String)> = cases(f, &format!("{PREFIX}0"));
        assert_eq!(l0, vec![(0, STEP06.to_string()), (1, format!("{PREFIX}1"))]);
        let l1: Vec<(u32, String)> = cases(f, &format!("{PREFIX}1"));
        assert_eq!(l1, vec![(0, format!("{PREFIX}3")), (1, format!("{PREFIX}2"))]);
        let l2: Vec<(u32, String)> = cases(f, &format!("{PREFIX}2"));
        assert_eq!(l2, vec![(0, format!("{PREFIX}4")), (1, STEP03.to_string())]);
        assert_eq!(text(&ev(f, &format!("{PREFIX}3")).kind, STEP), Some("Step05"));
        assert_eq!(text(&ev(f, &format!("{PREFIX}4")).kind, STEP), Some("Step04"));
        assert_eq!(text(&ev(f, &format!("{PREFIX}3")).kind, CHALLENGE), Some("DoorOfTime"));
        let Kind::Switch { params, .. } = &ev(f, &format!("{PREFIX}0")).kind else { panic!() };
        assert_eq!(params.as_ref().and_then(|c| c.get("GameDataName")), Some(&Val::Str(OPENED.to_string())));
        let Kind::Switch { params, .. } = &ev(f, &format!("{PREFIX}1")).kind else { panic!() };
        assert_eq!(params.as_ref().and_then(|c| c.get("EventName")), Some(&Val::Str(SONG.to_string())));
        let Kind::Switch { params, .. } = &ev(f, &format!("{PREFIX}2")).kind else { panic!() };
        assert_eq!(params.as_ref().and_then(|c| c.get("EventName")), Some(&Val::Str(ESCAPE.to_string())));
    }

    #[test]
    fn a_missing_bool_query_is_added_to_the_existing_actor() {
        let mut b: Bf = sample();
        b.flows[0].actors[3].queries.pop();
        patch_flow(&mut b).unwrap();
        let a: &Actor = &b.flows[0].actors[3];
        assert_eq!(a.queries.last().map(|s| s.as_str()), Some(BOOL));
        assert_eq!(a.queries.len(), 3);
    }

    #[test]
    fn the_new_switches_use_the_right_queries_and_the_actors_are_not_duplicated() {
        let mut b: Bf = sample();
        patch_flow(&mut b).unwrap();
        let f: &Flow = &b.flows[0];
        assert_eq!(f.actors.iter().filter(|a| a.name == HOLDER).count(), 1);
        let q = |n: &str| -> String {
            let Kind::Switch { actor, query, .. } = &ev(f, n).kind else { panic!() };
            f.actors[*actor as usize].queries[*query as usize].clone()
        };
        assert_eq!(q(&format!("{PREFIX}0")), BOOL);
        assert_eq!(q(&format!("{PREFIX}1")), PLAYED);
        assert_eq!(q(&format!("{PREFIX}2")), PLAYED);
        let Kind::Action { actor, act, .. } = &ev(f, &format!("{PREFIX}4")).kind else { panic!() };
        assert_eq!(f.actors[*actor as usize].actions[*act as usize], "EventExecuteProgressChallengeStep");
    }

    #[test]
    fn every_event_is_reachable_and_nothing_that_was_reachable_is_lost() {
        let before: Bf = sample();
        let old: BS<usize> = reach(&before.flows[0]);
        let mut b: Bf = sample();
        patch_flow(&mut b).unwrap();
        let f: &Flow = &b.flows[0];
        let now: BS<usize> = reach(f);
        assert!(old.is_subset(&now));
        assert_eq!(now.len(), f.events.len());
        assert_eq!(now.len(), old.len() + 6);
    }

    #[test]
    fn only_the_step03_arm_and_the_send_off_changed_and_the_rest_is_identical() {
        let before: Bf = sample();
        let mut b: Bf = sample();
        patch_flow(&mut b).unwrap();
        let (x, y): (&Flow, &Flow) = (&before.flows[0], &b.flows[0]);
        assert_eq!(x.actors, y.actors);
        assert_eq!(x.entries, y.entries);
        for i in 0..x.events.len() {
            if x.events[i].name == SEND_OFF || x.events[i].name == "Event39" || x.events[i].name == "Event102" {
                continue;
            }
            assert_eq!(x.events[i], y.events[i]);
        }
        let Kind::Sub { next, .. } = &y.events[11].kind else { panic!() };
        assert_eq!(y.events[*next as usize].name, format!("{PREFIX}5"));
        let Kind::Switch { cases, .. } = &y.events[3].kind else { panic!() };
        assert_eq!(cases[1].ev, 6);
    }

    #[test]
    fn a_patched_flow_round_trips_and_a_second_patch_is_refused() {
        let mut b: Bf = sample();
        patch_flow(&mut b).unwrap();
        let back: Bf = Bf::parse(&b.write().unwrap()).unwrap();
        assert_eq!(back, b);
        assert!(patch_flow(&mut b).is_err());
    }

    #[test]
    fn a_flow_that_does_not_match_is_refused() {
        let mut b: Bf = sample();
        b.flows[0].events[5] = step(STEP03, "DoorOfTime", "Step04", 9);
        assert!(patch_flow(&mut b).is_err());
        let mut c: Bf = sample();
        c.flows[0].events[2].name = "Other".to_string();
        assert!(patch_flow(&mut c).is_err());
    }
}
