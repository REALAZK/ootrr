use std::path::Path as P;

use crate::bfevfl::{Actor, Bf, Case, Cont, Event, Flow, Kind, Val};
use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::romfs::Romfs;
use crate::staging::{self, Out};

pub const FILE: &str = "Event/EventFlow/OpeningEvent.bfevfl.zs";
pub const FLOW: &str = "OpeningEvent";
pub const MOVIE: &str = "Dm_Kokiri_0000";
pub const SCENE: &str = "Dm_Kokiri_0030";
pub const LIST: &str = "Event/EventSetting/EventSettingComponentList/Dm_Kokiri_0000.engine__event__EventSettingComponentList.bgyml";
pub const SKIP_KEY: &str = "SkipSetting";
const TRANSITION: &str = "EventTransition";
const SKIPPED: &str = "EventQueryIsEventSkipedMostRecent";
const BGM: &str = "EventQueryIsInGameBgmSuspend";
const SET_BOOL: &str = "EventTriggerSetGameDataBool";
const PLAYED: &str = "HavePlayedEvent.";
const TRUE: u32 = 0x8000_0001;
const NONE: u16 = 0xFFFF;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Opts {
    pub movie: bool,
    pub scene: bool,
    pub hold: bool,
}

impl Opts {
    pub const FULL: Opts = Opts { movie: true, scene: true, hold: false };

    pub fn any(&self) -> bool {
        self.movie || self.scene || self.hold
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub flags: Vec<String>,
    pub bgm_forced: bool,
    pub added_events: usize,
}

fn act_of<'a>(f: &'a Flow, e: &'a Event) -> Option<(&'a str, &'a str)> {
    let Kind::Action { actor, act, .. } = &e.kind else {
        return None;
    };
    let a: &Actor = f.actors.get(*actor as usize)?;
    Some((a.name.as_str(), a.actions.get(*act as usize)?.as_str()))
}

fn query_of<'a>(f: &'a Flow, e: &'a Event) -> Option<&'a str> {
    let Kind::Switch { actor, query, .. } = &e.kind else {
        return None;
    };
    f.actors.get(*actor as usize)?.queries.get(*query as usize).map(|s| s.as_str())
}

fn transition(f: &Flow, name: &str) -> Res<usize> {
    let hit = |e: &Event| -> bool {
        act_of(f, e).is_some_and(|(_, a)| a == TRANSITION)
            && e.kind.params().and_then(|c| c.get("EventName")).is_some_and(|v| *v == Val::Str(name.to_string()))
    };
    let v: Vec<usize> = (0..f.events.len()).filter(|&i| hit(&f.events[i])).collect();
    match v.as_slice() {
        [i] => Ok(*i),
        _ => Err(Error::Msg(format!("intro skip: expected one transition to {name} in {FLOW}, found {}", v.len()))),
    }
}

fn next_of(f: &Flow, i: usize) -> Res<u16> {
    match &f.events[i].kind {
        Kind::Action { next, .. } if *next != NONE => Ok(*next),
        _ => Err(Error::Format("intro skip: transition has no next event")),
    }
}

fn case_of(cases: &[Case], v: u32) -> Res<u16> {
    cases.iter().find(|c| c.val == v).map(|c| c.ev).ok_or(Error::Format("intro skip: switch has no case 0"))
}

fn flag_slot(f: &mut Flow) -> Res<(u16, u16)> {
    for (i, a) in f.actors.iter().enumerate() {
        if a.name == "GameDataActor" {
            if let Some(j) = a.actions.iter().position(|s| s == SET_BOOL) {
                return Ok((i as u16, j as u16));
            }
        }
    }
    let base: Actor = f.actors.first().cloned().ok_or(Error::Format("intro skip: flow has no actors"))?;
    if f.actors.len() >= NONE as usize {
        return Err(Error::Format("intro skip: too many actors"));
    }
    f.actors.push(Actor {
        name: "GameDataActor".to_string(),
        sub: String::new(),
        arg: String::new(),
        actions: vec![SET_BOOL.to_string()],
        queries: Vec::new(),
        params: base.params,
        ep: base.ep,
        cut: base.cut,
    });
    Ok(((f.actors.len() - 1) as u16, 0))
}

fn flag_event(f: &mut Flow, event: &str, next: u16) -> Res<u16> {
    let (actor, act): (u16, u16) = flag_slot(f)?;
    if f.events.len() >= NONE as usize {
        return Err(Error::Format("intro skip: too many events"));
    }
    let n: usize = f.events.len();
    f.events.push(Event {
        name: format!("EventIntroSkip{n}"),
        kind: Kind::Action {
            next,
            actor,
            act,
            params: Some(Cont {
                items: vec![
                    ("Index".to_string(), Val::Int(-1)),
                    ("GameDataName".to_string(), Val::Str(format!("{PLAYED}{event}"))),
                    ("Value".to_string(), Val::Bool(TRUE)),
                ],
            }),
        },
    });
    Ok(n as u16)
}

fn skip_movie(f: &mut Flow) -> Res<()> {
    let t: usize = transition(f, MOVIE)?;
    let after: u16 = next_of(f, t)?;
    let at: Vec<usize> = (0..f.entries.len()).filter(|&i| f.entries[i].main as usize == t).collect();
    if at.len() != 1 {
        return Err(Error::Format("intro skip: the movie transition is not the start of exactly one entry"));
    }
    f.entries[at[0]].main = after;
    Ok(())
}

fn skip_scene(f: &mut Flow, r: &mut Report, o: Opts) -> Res<()> {
    let t: usize = transition(f, SCENE)?;
    let sw: u16 = next_of(f, t)?;
    let fresh: u16 = {
        let e: &Event = f.events.get(sw as usize).ok_or(Error::Format("intro skip: bad next event"))?;
        if query_of(f, e) != Some(SKIPPED) {
            return Err(Error::Format("intro skip: the event after the scene is not the skipped check"));
        }
        let Kind::Switch { cases, .. } = &e.kind else {
            return Err(Error::Format("intro skip: skipped check is not a switch"));
        };
        case_of(cases, 0)?
    };
    let before: Vec<usize> = (0..f.events.len())
        .filter(|&i| matches!(&f.events[i].kind, Kind::Action { next, .. } if *next as usize == t))
        .collect();
    let refs: usize = f.entries.iter().filter(|e| e.main as usize == t || e.subs.iter().any(|s| *s as usize == t)).count();
    if before.len() != 1 || refs != 0 {
        return Err(Error::Format("intro skip: the scene transition must have exactly one caller"));
    }
    let mut names: Vec<&str> = Vec::new();
    if o.movie {
        names.push(MOVIE);
    }
    if o.scene {
        names.push(SCENE);
    }
    let mut to: u16 = if o.scene { fresh } else { t as u16 };
    for n in names.iter().rev() {
        to = flag_event(f, n, to)?;
    }
    for n in &names {
        r.flags.push(format!("{PLAYED}{n}"));
    }
    r.added_events = names.len();
    if let Kind::Action { next, .. } = &mut f.events[before[0]].kind {
        *next = to;
    }
    if !o.scene {
        return Ok(());
    }
    let b: Vec<usize> = (0..f.events.len()).filter(|&i| query_of(f, &f.events[i]) == Some(BGM)).collect();
    for i in b {
        let to: u16 = match &f.events[i].kind {
            Kind::Switch { cases, .. } => case_of(cases, 0)?,
            _ => continue,
        };
        if let Kind::Switch { cases, .. } = &mut f.events[i].kind {
            for c in cases.iter_mut() {
                c.ev = to;
            }
        }
        r.bgm_forced = true;
    }
    Ok(())
}

pub fn patch(b: &mut Bf, o: Opts) -> Res<Report> {
    let mut r: Report = Report::default();
    let f: &mut Flow = b.flows.iter_mut().find(|x| x.name == FLOW).ok_or(Error::Format("intro skip: no OpeningEvent flow"))?;
    if o.movie || o.scene {
        skip_scene(f, &mut r, o)?;
    }
    if o.movie {
        skip_movie(f)?;
    }
    Ok(r)
}

pub fn unlock(raw: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    let c: &mut Node = y.root.get_mut("Components").ok_or(Error::Format("intro skip: the movie list has no Components"))?;
    if !c.remove(SKIP_KEY) {
        return Err(Error::Format("intro skip: the movie list has no SkipSetting to remove"));
    }
    y.write()
}

pub fn stage(rf: &mut Romfs, o: Opts) -> Res<(Vec<Out>, Report)> {
    stage_with(rf, o, &crate::start_state::Ops::default())
}

/// Stages the intro skip and, in the same rewrite of the opening flow, the start state writes.
pub fn stage_with(rf: &mut Romfs, o: Opts, state: &crate::start_state::Ops) -> Res<(Vec<Out>, Report)> {
    let mut outs: Vec<Out> = Vec::new();
    let mut r: Report = Report::default();
    if o.movie || o.scene || !state.is_empty() {
        let mut got: Report = Report::default();
        outs.push(staging::rewrite(rf, FILE, |raw| {
            let mut b: Bf = Bf::parse(raw)?;
            if o.movie || o.scene {
                got = patch(&mut b, o)?;
            }
            if !state.is_empty() {
                let f: &mut Flow = b.flows.iter_mut().find(|x| x.name == FLOW).ok_or(Error::Format("start state: no OpeningEvent flow"))?;
                got.added_events += crate::start_state::patch(f, state)?;
            }
            b.write()
        })?);
        r = got;
    }
    if o.hold {
        let raw: Vec<u8> = rf.read(LIST)?;
        let out: Vec<u8> = unlock(&raw)?;
        outs.push(Out { path: LIST.to_string(), old: raw.len(), new: out.len(), z: out });
    }
    Ok((outs, r))
}

pub fn build(romfs: &P, load: &P, name: &str, o: Opts) -> Res<(String, Report)> {
    if !o.any() {
        return Err(Error::Format("intro skip: nothing selected"));
    }
    let mut rf: Romfs = Romfs::open(romfs)?;
    let (outs, r): (Vec<Out>, Report) = stage(&mut rf, o)?;
    let c: staging::Committed = staging::commit(&mut rf, load, name, &outs, None)?;
    Ok((c.report, r))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::Entry;

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

    fn trans(n: &str, to: &str, next: u16) -> Event {
        Event {
            name: n.to_string(),
            kind: Kind::Action {
                next,
                actor: 0,
                act: 0,
                params: Some(Cont { items: vec![("EventName".to_string(), Val::Str(to.to_string()))] }),
            },
        }
    }

    fn plain(n: &str, next: u16) -> Event {
        Event { name: n.to_string(), kind: Kind::Action { next, actor: 1, act: 0, params: None } }
    }

    fn sw(n: &str, actor: u16, cases: Vec<Case>) -> Event {
        Event { name: n.to_string(), kind: Kind::Switch { actor, query: 0, cases, params: None } }
    }

    fn sample() -> Bf {
        let events: Vec<Event> = vec![
            trans("E0", SCENE, 1),
            sw("E1", 0, vec![Case { val: 0, ev: 3 }, Case { val: 1, ev: 4 }]),
            plain("E2", 0),
            plain("E3", 5),
            plain("E4", 5),
            sw("E5", 2, vec![Case { val: 0, ev: 6 }, Case { val: 1, ev: 7 }]),
            plain("E6", 7),
            plain("E7", NONE),
            trans("E8", MOVIE, 9),
            plain("E9", NONE),
        ];
        let f: Flow = Flow {
            name: FLOW.to_string(),
            actors: vec![actor("EventSystemActor", &[TRANSITION], &[SKIPPED]), actor("GameSystemActor", &["EventWait"], &[]), actor("SoundSystemActor", &[], &[BGM])],
            events,
            entries: vec![
                Entry { name: "A".to_string(), subs: Vec::new(), vars: Vec::new(), main: 2, pad: 0 },
                Entry { name: "B".to_string(), subs: Vec::new(), vars: Vec::new(), main: 8, pad: 0 },
            ],
        };
        Bf { ver: [0, 4, 0, 0], bom: 0xFEFF, shift: 3, pad: 0, name: FLOW.to_string(), flows: vec![f], tls: Vec::new() }
    }

    fn flag_names(f: &Flow) -> Vec<String> {
        f.events
            .iter()
            .filter_map(|e| e.kind.params().and_then(|c| c.get("GameDataName")).and_then(|v| if let Val::Str(s) = v { Some(s.clone()) } else { None }))
            .collect()
    }

    #[test]
    fn the_scene_caller_now_skips_to_the_fresh_branch_and_sets_the_flag() {
        let mut b: Bf = sample();
        let r: Report = patch(&mut b, Opts { movie: false, scene: true, hold: false }).unwrap();
        let f: &Flow = &b.flows[0];
        assert_eq!(r.flags, vec![format!("{PLAYED}{SCENE}")]);
        assert!(r.bgm_forced);
        let n: usize = f.events.len() - 1;
        assert!(matches!(&f.events[2].kind, Kind::Action { next, .. } if *next as usize == n));
        assert!(matches!(&f.events[n].kind, Kind::Action { next: 3, .. }));
        assert_eq!(flag_names(f), vec![format!("{PLAYED}{SCENE}")]);
        let Kind::Switch { cases, .. } = &f.events[5].kind else { panic!() };
        assert!(cases.iter().all(|c| c.ev == 6));
        assert_eq!(f.entries[1].main, 8);
    }

    #[test]
    fn the_movie_entry_starts_after_the_movie_and_the_flag_comes_before_the_scene() {
        let mut b: Bf = sample();
        let r: Report = patch(&mut b, Opts { movie: true, scene: false, hold: false }).unwrap();
        let f: &Flow = &b.flows[0];
        assert!(!r.bgm_forced);
        assert_eq!(f.entries[1].main, 9);
        assert_eq!(f.entries[0].main, 2);
        let n: usize = f.events.len() - 1;
        assert!(matches!(&f.events[2].kind, Kind::Action { next, .. } if *next as usize == n));
        assert!(matches!(&f.events[n].kind, Kind::Action { next: 0, .. }));
        assert_eq!(flag_names(f), vec![format!("{PLAYED}{MOVIE}")]);
        assert_eq!(f.actors.last().map(|a| a.name.as_str()), Some("GameDataActor"));
    }

    #[test]
    fn both_parts_chain_both_flags_through_one_actor_and_survive_a_round_trip() {
        let mut b: Bf = sample();
        let r: Report = patch(&mut b, Opts::FULL).unwrap();
        assert_eq!(r.added_events, 2);
        let f: &Flow = &b.flows[0];
        assert_eq!(f.actors.iter().filter(|a| a.name == "GameDataActor").count(), 1);
        assert_eq!(flag_names(f), vec![format!("{PLAYED}{SCENE}"), format!("{PLAYED}{MOVIE}")]);
        let Kind::Action { next, .. } = &f.events[2].kind else { panic!() };
        let first: &Event = &f.events[*next as usize];
        assert_eq!(first.kind.params().and_then(|c| c.get("GameDataName")), Some(&Val::Str(format!("{PLAYED}{MOVIE}"))));
        let back: Bf = Bf::parse(&b.write().unwrap()).unwrap();
        assert_eq!(back, b);
    }

    #[test]
    fn a_flow_that_does_not_match_is_refused() {
        let mut b: Bf = sample();
        b.flows[0].events[8] = plain("E8", NONE);
        assert!(patch(&mut b, Opts::FULL).is_err());
        let mut c: Bf = sample();
        c.flows[0].entries[0].main = 0;
        assert!(patch(&mut c, Opts { movie: false, scene: true, hold: false }).is_err());
    }

    #[test]
    fn the_hold_edit_removes_only_the_skip_lock() {
        let y: Byml = Byml {
            root: Node::Map(vec![
                ("$parent".to_string(), Node::Str("P".to_string())),
                (
                    "Components".to_string(),
                    Node::Map(vec![("EventPlayEnvSetting".to_string(), Node::Str("E".to_string())), (SKIP_KEY.to_string(), Node::Str("S".to_string()))]),
                ),
            ]),
            version: 7,
            big: false,
        };
        let out: Vec<u8> = unlock(&y.write().unwrap()).unwrap();
        let back: Byml = Byml::parse(&out).unwrap();
        let c: &Node = back.root.get("Components").unwrap();
        assert!(c.get(SKIP_KEY).is_none());
        assert!(c.get("EventPlayEnvSetting").is_some());
        assert!(back.root.get("$parent").is_some());
        assert!(unlock(&out).is_err());
    }
}
