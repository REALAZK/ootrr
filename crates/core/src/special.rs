use std::collections::BTreeMap as BM;

use crate::bfevfl::{Bf, Case, Cont, Event, Flow, Kind, Val};
use crate::error::{Error, Res};
use crate::progressive::{event_name, Size};
use crate::romfs::Romfs;
use crate::staging::{rewrite, Out};

pub const BASE: &str = "Dm_SY_Treasurebox";
pub const PARAM: &str = "EventName";
pub const QUERY: &str = "EventQueryIsGameDataBool";
pub const HOLDER: &str = "GameDataActor";
pub const OPEN: &str = "IsTreasureBoxOpen.";
pub const GATES: [(usize, &str); 2] = [(422, "Event442"), (90, "Event129")];
pub const DAMPE_HOOK: u64 = 16041366920820397198;
pub const DAMPE_HEART: u64 = 5534919361313859554;
pub const SPIRIT: u64 = 2490063409521505179;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hook {
    Place,
    Clone { switch: usize, name: &'static str },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spot {
    pub hash: u64,
    pub flow: &'static str,
    pub parent: &'static str,
    pub index: usize,
    pub name: &'static str,
    pub hook: Hook,
}

pub const SPOTS: [Spot; 3] = [
    Spot {
        hash: DAMPE_HOOK,
        flow: "Event/EventFlow/MiniGame_DampeRace.bfevfl.zs",
        parent: "MiniGame_DampeRace_OpenGoalTBox",
        index: 82,
        name: "Event66",
        hook: Hook::Clone { switch: 153, name: "Event241" },
    },
    Spot {
        hash: DAMPE_HEART,
        flow: "Event/EventFlow/MiniGame_DampeRace.bfevfl.zs",
        parent: "MiniGame_DampeRace_OpenGoalTBox",
        index: 82,
        name: "Event66",
        hook: Hook::Clone { switch: 152, name: "Event240" },
    },
    Spot {
        hash: SPIRIT,
        flow: "Event/EventFlow/SageOfSpirit.bfevfl.zs",
        parent: "SageOfSpirit_NaboorIsTakenAway",
        index: 12,
        name: "Event64",
        hook: Hook::Place,
    },
];

pub fn of(hash: u64) -> Option<&'static Spot> {
    SPOTS.iter().find(|s| s.hash == hash)
}

pub fn open_flag(hash: u64) -> String {
    format!("{OPEN}{hash}")
}

fn idx(n: usize) -> Res<u16> {
    u16::try_from(n).map_err(|_| Error::Format("special: index does not fit in 16 bits"))
}

fn flow_of(b: &mut Bf) -> Res<&mut Flow> {
    b.flows.first_mut().ok_or(Error::Format("special: the flow file holds no flow"))
}

fn name_slot(k: &mut Kind) -> Res<&mut Val> {
    let c: &mut Cont = k.params_mut().ok_or(Error::Format("special: the transition has no parameters"))?;
    c.items
        .iter_mut()
        .find(|(n, _)| n == PARAM)
        .map(|(_, v)| v)
        .ok_or(Error::Format("special: the transition has no EventName"))
}

fn text(v: &Val) -> Option<&str> {
    match v {
        Val::Str(s) | Val::Actor(s, _) => Some(s.as_str()),
        _ => None,
    }
}

pub fn point(f: &mut Flow, s: &Spot, family: &str) -> Res<()> {
    let wrapper: String = event_name(family, Size::Big);
    let e: &Event = f.events.get(s.index).ok_or(Error::Format("special: the transition index is out of range"))?;
    if e.name != s.name || !matches!(e.kind, Kind::Action { .. }) {
        return Err(Error::Msg(format!("event {} is not the {} transition, the game files differ", s.index, s.name)));
    }
    let have: String = e
        .kind
        .params()
        .and_then(|c| c.get(PARAM))
        .and_then(text)
        .map(|t| t.to_string())
        .ok_or(Error::Format("special: the transition names no event"))?;
    if have == wrapper {
        return Ok(());
    }
    if have != BASE {
        return Err(Error::Msg(format!("the transition {} starts {have} and not {BASE}, the game files differ", s.name)));
    }
    match s.hook {
        Hook::Place => {
            *name_slot(&mut f.events[s.index].kind)? = Val::Str(wrapper);
        }
        Hook::Clone { switch, name } => {
            let sw: &Event = f.events.get(switch).ok_or(Error::Format("special: the switch index is out of range"))?;
            if sw.name != name {
                return Err(Error::Msg(format!("event {switch} is named {} and not {name}, the game files differ", sw.name)));
            }
            let Kind::Switch { cases, .. } = &sw.kind else {
                return Err(Error::Format("special: the hooked event is not a switch"));
            };
            let done: String = format!("OOTRR_Wrap_{}", s.hash);
            if cases.iter().any(|c| c.val == 0 && f.events.get(c.ev as usize).is_some_and(|x| x.name == done)) {
                return Ok(());
            }
            if !cases.iter().any(|c| c.val == 0 && c.ev as usize == s.index) {
                return Err(Error::Msg(format!("case 0 of {name} does not lead to the transition, the game files differ")));
            }
            let mut kind: Kind = f.events[s.index].kind.clone();
            *name_slot(&mut kind)? = Val::Str(wrapper);
            let at: u16 = idx(f.events.len())?;
            f.events.push(Event { name: format!("OOTRR_Wrap_{}", s.hash), kind });
            if let Kind::Switch { cases, .. } = &mut f.events[switch].kind {
                for c in cases.iter_mut().filter(|c: &&mut Case| c.val == 0 && c.ev as usize == s.index) {
                    c.ev = at;
                }
            }
        }
    }
    Ok(())
}

fn query_slot(f: &mut Flow) -> Res<(u16, u16)> {
    let i: usize = f
        .actors
        .iter()
        .position(|a| a.name == HOLDER && a.sub.is_empty())
        .ok_or(Error::Format("special: the flow has no GameDataActor"))?;
    let j: usize = match f.actors[i].queries.iter().position(|q| q == QUERY) {
        Some(j) => j,
        None => {
            f.actors[i].queries.push(QUERY.to_string());
            f.actors[i].queries.len() - 1
        }
    };
    Ok((idx(i)?, idx(j)?))
}

fn model(f: &Flow, a: u16, q: u16) -> Res<Cont> {
    f.events
        .iter()
        .find_map(|e| match &e.kind {
            Kind::Switch { actor, query, params: Some(c), .. } if *actor == a && *query == q && c.get("GameDataName").is_some() => {
                Some(c.clone())
            }
            _ => None,
        })
        .ok_or(Error::Format("special: no existing GameData bool check to copy the parameters from"))
}

pub fn open_gate(f: &mut Flow, hash: u64) -> Res<usize> {
    let (a, q): (u16, u16) = query_slot(f)?;
    let flag: String = open_flag(hash);
    let proto: Cont = model(f, a, q)?;
    let mut n: usize = 0;
    for (i, name) in GATES {
        let e: &mut Event = f.events.get_mut(i).ok_or(Error::Format("special: a gate event index is out of range"))?;
        if e.name != name {
            return Err(Error::Msg(format!("event {i} is named {} and not {name}, the game files differ", e.name)));
        }
        let Kind::Switch { actor, query, params, .. } = &mut e.kind else {
            return Err(Error::Format("special: a gate event is not a switch"));
        };
        if *actor == a && *query == q {
            continue;
        }
        let mut c: Cont = proto.clone();
        for (k, v) in c.items.iter_mut() {
            if k == "GameDataName" {
                *v = Val::Str(flag.clone());
            }
        }
        *actor = a;
        *query = q;
        *params = Some(c);
        n += 1;
    }
    Ok(n)
}

pub fn edit_flow(raw: &[u8], wraps: &[(&Spot, &str)], gate: Option<u64>) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    let f: &mut Flow = flow_of(&mut b)?;
    for (s, fam) in wraps {
        point(f, s, fam)?;
    }
    if let Some(h) = gate {
        open_gate(f, h)?;
    }
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(Error::Format("special: the written flow does not parse back to the edited model"));
    }
    Ok(out)
}

pub fn stage(rf: &mut Romfs, wraps: &[(u64, String)], touched: &[u64]) -> Res<(Vec<Out>, Vec<(String, String)>)> {
    let mut by: BM<&str, Vec<(&Spot, &str)>> = BM::new();
    let mut kids: Vec<(String, String)> = Vec::new();
    for (h, fam) in wraps {
        let s: &Spot = of(*h).ok_or(Error::Format("special: a wrapped chest is not a known special chest"))?;
        by.entry(s.flow).or_default().push((s, fam.as_str()));
        kids.push((s.parent.to_string(), event_name(fam, Size::Big)));
    }
    let gate: Option<u64> = touched.contains(&DAMPE_HOOK).then_some(DAMPE_HOOK);
    if gate.is_some() {
        by.entry(SPOTS[0].flow).or_default();
    }
    let mut outs: Vec<Out> = Vec::new();
    for (path, ws) in &by {
        let g: Option<u64> = if *path == SPOTS[0].flow { gate } else { None };
        outs.push(rewrite(rf, path, |raw| edit_flow(raw, ws, g))?);
    }
    kids.sort();
    kids.dedup();
    Ok((outs, kids))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::flow::NONE;
    use crate::bfevfl::{Actor, Entry};

    fn filler(n: usize) -> Event {
        Event { name: format!("F{n}"), kind: Kind::Join { next: NONE } }
    }

    fn transition(name: &str, to: &str) -> Event {
        Event {
            name: name.to_string(),
            kind: Kind::Action {
                next: 88,
                actor: 0,
                act: 0,
                params: Some(Cont {
                    items: vec![(PARAM.to_string(), Val::Str(to.to_string())), ("IsWaitEnd".to_string(), Val::Bool(1))],
                }),
            },
        }
    }

    fn switch(name: &str, zero: u16, one: u16) -> Event {
        Event {
            name: name.to_string(),
            kind: Kind::Switch {
                actor: 1,
                query: 0,
                cases: vec![Case { val: 0, ev: zero }, Case { val: 1, ev: one }],
                params: Some(Cont { items: vec![("ActorName".to_string(), Val::Str("Item_Hookshot".to_string()))] }),
            },
        }
    }

    fn model_check() -> Event {
        Event {
            name: "Event277".to_string(),
            kind: Kind::Switch {
                actor: 1,
                query: 1,
                cases: vec![Case { val: 0, ev: 1 }, Case { val: 1, ev: 2 }],
                params: Some(Cont {
                    items: vec![("Index".to_string(), Val::Int(-1)), ("GameDataName".to_string(), Val::Str("Some.Flag".to_string()))],
                }),
            },
        }
    }

    fn flow() -> Bf {
        let mut events: Vec<Event> = (0..430).map(filler).collect();
        events[82] = transition("Event66", BASE);
        events[153] = switch("Event241", 82, 152);
        events[152] = switch("Event240", 82, 154);
        events[422] = switch("Event442", 405, 111);
        events[90] = switch("Event129", 92, 183);
        events[198] = model_check();
        let actor = |n: &str, q: &[&str]| Actor {
            name: n.to_string(),
            sub: String::new(),
            arg: String::new(),
            actions: Vec::new(),
            queries: q.iter().map(|x| x.to_string()).collect(),
            params: None,
            ep: 0xFFFF,
            cut: 1,
        };
        Bf {
            ver: [0, 4, 0, 0],
            bom: 0xFEFF,
            shift: 3,
            pad: 0,
            name: "T".to_string(),
            flows: vec![Flow {
                name: "T".to_string(),
                actors: vec![actor("EventSystemActor", &[]), actor(HOLDER, &["EventQueryCheckIsGet", QUERY])],
                events,
                entries: vec![Entry { name: "E".to_string(), subs: Vec::new(), vars: Vec::new(), main: 83, pad: 0 }],
            }],
            tls: Vec::new(),
        }
    }

    fn ev(b: &Bf, i: usize) -> &Event {
        &b.flows[0].events[i]
    }

    #[test]
    fn the_hookshot_chest_gets_its_own_wrapper_transition_and_the_heart_piece_chest_keeps_the_original() {
        let raw: Vec<u8> = flow().write().unwrap();
        let out: Vec<u8> = edit_flow(&raw, &[(of(DAMPE_HOOK).unwrap(), "Hookshot")], None).unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        let Kind::Switch { cases, .. } = &ev(&b, 153).kind else { panic!() };
        let to: usize = cases.iter().find(|c| c.val == 0).unwrap().ev as usize;
        assert_eq!(to, 430);
        assert_eq!(ev(&b, to).kind.params().and_then(|c| c.get(PARAM)), Some(&Val::Str("OOTRR_Prog_Hookshot_Big".to_string())));
        assert_eq!(ev(&b, 82).kind.params().and_then(|c| c.get(PARAM)), Some(&Val::Str(BASE.to_string())));
        let Kind::Switch { cases, .. } = &ev(&b, 152).kind else { panic!() };
        assert_eq!(cases.iter().find(|c| c.val == 0).unwrap().ev, 82);
        let again: Vec<u8> = edit_flow(&out, &[(of(DAMPE_HOOK).unwrap(), "Hookshot")], None).unwrap();
        assert_eq!(Bf::parse(&again).unwrap().flows[0].events.len(), 431);
    }

    #[test]
    fn the_spirit_transition_is_changed_in_place() {
        let mut b: Bf = flow();
        b.flows[0].events[12] = transition("Event64", BASE);
        let raw: Vec<u8> = b.write().unwrap();
        let out: Vec<u8> = edit_flow(&raw, &[(of(SPIRIT).unwrap(), "Strength")], None).unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        assert_eq!(ev(&b, 12).kind.params().and_then(|c| c.get(PARAM)), Some(&Val::Str("OOTRR_Prog_Strength_Big".to_string())));
    }

    #[test]
    fn the_gate_tests_the_chest_flag_and_keeps_its_cases() {
        let raw: Vec<u8> = flow().write().unwrap();
        let out: Vec<u8> = edit_flow(&raw, &[], Some(DAMPE_HOOK)).unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        for i in [422usize, 90] {
            let Kind::Switch { actor, query, cases, params } = &ev(&b, i).kind else { panic!() };
            assert_eq!((*actor, *query), (1, 1));
            assert_eq!(params.as_ref().and_then(|c| c.get("GameDataName")), Some(&Val::Str(open_flag(DAMPE_HOOK))));
            assert_eq!(params.as_ref().and_then(|c| c.get("Index")), Some(&Val::Int(-1)));
            assert_eq!(cases.len(), 2);
        }
        let Kind::Switch { cases, .. } = &ev(&b, 422).kind else { panic!() };
        assert_eq!((cases[0].ev, cases[1].ev), (405, 111));
    }

    #[test]
    fn a_game_that_differs_is_refused() {
        let mut b: Bf = flow();
        b.flows[0].events[82].name = "Other".to_string();
        let raw: Vec<u8> = b.write().unwrap();
        assert!(edit_flow(&raw, &[(of(DAMPE_HOOK).unwrap(), "Hookshot")], None).is_err());
    }

    #[test]
    fn the_chest_flag_names_hash_like_the_game_does() {
        assert_eq!(crate::chest_slot::key(&open_flag(10831981390482426294)), 0x316d8a20);
        assert_eq!(crate::chest_slot::key(&open_flag(DAMPE_HOOK)), crate::chest_slot::key("IsTreasureBoxOpen.16041366920820397198"));
    }
}
