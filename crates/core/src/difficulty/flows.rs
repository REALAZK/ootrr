use crate::bfevfl::flow::NONE;
use crate::bfevfl::{Actor, Bf, Case, Cont, Event, Flow, Kind, Val};
use crate::error::{Error, Res};

pub const OVER_FILE: &str = "Event/EventFlow/DmF_SY_GameOver.bfevfl.zs";
pub const OVER_FLOW: &str = "DmF_SY_GameOver";
pub const GOSSIP_FILE: &str = "Event/EventFlow/GossipStone.bfevfl.zs";
pub const GOSSIP_FLOW: &str = "GossipStone";
pub const BOTTLE_FLOWS: [(&str, &str); 3] = [
    ("Event/EventFlow/Dm_SY_DrinkBottle.bfevfl.zs", "Dm_SY_DrinkBottle"),
    ("Event/EventFlow/Dm_SY_OutBottle.bfevfl.zs", "Dm_SY_OutBottle"),
    ("Event/EventFlow/Dm_SY_OutBottleFairy.bfevfl.zs", "Dm_SY_OutBottleFairy"),
];

pub const MAX_LIFE: &str = "PlayerStatus.MaxLife";
pub const HEART: i32 = 4;
pub const FLOOR_HEARTS: i32 = 3;
pub const BATTLE_BGM: &str = "BGM_EnemyBattle";
pub const BOSS_FLAG: &str = "IsBossBattle";
pub const APPEAR_FLAG: &str = "GossipStone_FairySpiritAppearOk";
pub const GENERATOR: &str = "GossipStone_FairyGenerator";
pub const NEVER: u32 = 2;
pub const CLEAR: u32 = 1;

pub const DATA: &str = "GameDataActor";
pub const SOUND: &str = "SoundSystemActor";
pub const IS_INT: &str = "EventQueryIsGameDataInt";
pub const IS_BOOL: &str = "EventQueryIsGameDataBool";
pub const ADD_INT: &str = "EventTriggerAddOrSubGameDataInt";
pub const IS_BGM: &str = "EventQueryIsPlayingBgmName";
pub const RESULT: &str = "EventSetEventResult";

pub const HEART_MARK: &str = "OOTRRLoseHeart";
pub const BOTTLE_MARK: &str = "OOTRRNoBottle";

fn flow<'a>(b: &'a mut Bf, name: &str) -> Res<&'a mut Flow> {
    b.flows.iter_mut().find(|f| f.name == name).ok_or_else(|| Error::Msg(format!("difficulty: the flow {name} is not in its file")))
}

fn actor_at(f: &mut Flow, name: &str) -> Res<usize> {
    if let Some(i) = f.actors.iter().position(|a| a.name == name && a.sub.is_empty()) {
        return Ok(i);
    }
    if f.actors.len() >= NONE as usize {
        return Err(Error::Format("difficulty: too many actors in a flow"));
    }
    let like: &Actor = f.actors.iter().find(|a| a.name.ends_with("SystemActor")).or(f.actors.first()).ok_or(Error::Format("difficulty: a flow has no actors"))?;
    let a: Actor = Actor { name: name.to_string(), sub: String::new(), arg: String::new(), actions: Vec::new(), queries: Vec::new(), params: like.params.clone(), ep: like.ep, cut: like.cut };
    f.actors.push(a);
    Ok(f.actors.len() - 1)
}

fn slot(list: &mut Vec<String>, name: &str) -> u16 {
    if let Some(i) = list.iter().position(|s| s == name) {
        return i as u16;
    }
    list.push(name.to_string());
    (list.len() - 1) as u16
}

fn query(f: &mut Flow, actor: &str, name: &str) -> Res<(u16, u16)> {
    let a: usize = actor_at(f, actor)?;
    Ok((a as u16, slot(&mut f.actors[a].queries, name)))
}

fn action(f: &mut Flow, actor: &str, name: &str) -> Res<(u16, u16)> {
    let a: usize = actor_at(f, actor)?;
    Ok((a as u16, slot(&mut f.actors[a].actions, name)))
}

fn push(f: &mut Flow, name: String, kind: Kind) -> Res<u16> {
    if f.events.len() >= NONE as usize {
        return Err(Error::Format("difficulty: too many events in a flow"));
    }
    f.events.push(Event { name, kind });
    Ok((f.events.len() - 1) as u16)
}

fn cont(items: Vec<(&str, Val)>) -> Option<Cont> {
    Some(Cont { items: items.into_iter().map(|(k, v)| (k.to_string(), v)).collect() })
}

fn finish(b: &Bf) -> Res<Vec<u8>> {
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != *b {
        return Err(Error::Format("difficulty: an edited flow does not read back"));
    }
    Ok(out)
}

fn marked(f: &Flow, mark: &str) -> bool {
    f.events.iter().any(|e| e.name.starts_with(mark))
}

fn is_retry(f: &Flow, e: &Event) -> bool {
    let Kind::Action { actor, act, params: Some(c), .. } = &e.kind else {
        return false;
    };
    let named: bool = f.actors.get(*actor as usize).is_some_and(|a| a.actions.get(*act as usize).is_some_and(|s| s == RESULT));
    named && c.get("EventResult") == Some(&Val::Str("Retry".to_string()))
}

pub fn lose_heart(raw: &[u8]) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    let f: &mut Flow = flow(&mut b, OVER_FLOW)?;
    if marked(f, HEART_MARK) {
        return Err(Error::Format("difficulty: the game over flow is already edited"));
    }
    let at: Vec<usize> = (0..f.events.len()).filter(|&i| is_retry(f, &f.events[i])).collect();
    let [at] = at.as_slice() else {
        return Err(Error::Msg(format!("difficulty: expected one retry result in the game over flow, found {}", at.len())));
    };
    let old: u16 = match &f.events[*at].kind {
        Kind::Action { next, .. } => *next,
        _ => NONE,
    };
    let (qa, q): (u16, u16) = query(f, DATA, IS_INT)?;
    let (aa, a): (u16, u16) = action(f, DATA, ADD_INT)?;
    let sub: u16 = push(
        f,
        format!("{HEART_MARK}Take"),
        Kind::Action { next: old, actor: aa, act: a, params: cont(vec![("Index", Val::Int(-1)), ("GameDataName", Val::Str(MAX_LIFE.to_string())), ("Value", Val::Int(-HEART))]) },
    )?;
    let mut cases: Vec<Case> = vec![Case { val: 2, ev: sub }];
    if old != NONE {
        cases.insert(0, Case { val: 1, ev: old });
        cases.insert(0, Case { val: 0, ev: old });
    }
    let check: u16 = push(
        f,
        format!("{HEART_MARK}Check"),
        Kind::Switch { actor: qa, query: q, cases, params: cont(vec![("Index", Val::Int(-1)), ("GameDataName", Val::Str(MAX_LIFE.to_string())), ("Value", Val::Int(FLOOR_HEARTS * HEART))]) },
    )?;
    if let Kind::Action { next, .. } = &mut f.events[*at].kind {
        *next = check;
    }
    finish(&b)
}

pub fn no_bottles(raw: &[u8], name: &str) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    let f: &mut Flow = flow(&mut b, name)?;
    if marked(f, BOTTLE_MARK) {
        return Err(Error::Format("difficulty: a bottle flow is already edited"));
    }
    if f.entries.is_empty() {
        return Err(Error::Format("difficulty: a bottle flow has no entry"));
    }
    let (ba, bq): (u16, u16) = query(f, DATA, IS_BOOL)?;
    let (sa, sq): (u16, u16) = query(f, SOUND, IS_BGM)?;
    for i in 0..f.entries.len() {
        let main: u16 = f.entries[i].main;
        if main == NONE {
            continue;
        }
        let quiet: u16 = push(
            f,
            format!("{BOTTLE_MARK}Music{i}"),
            Kind::Switch { actor: sa, query: sq, cases: vec![Case { val: CLEAR, ev: main }], params: cont(vec![("BgmName", Val::Str(BATTLE_BGM.to_string()))]) },
        )?;
        let calm: u16 = push(
            f,
            format!("{BOTTLE_MARK}Boss{i}"),
            Kind::Switch { actor: ba, query: bq, cases: vec![Case { val: CLEAR, ev: quiet }], params: cont(vec![("Index", Val::Int(-1)), ("GameDataName", Val::Str(BOSS_FLAG.to_string()))]) },
        )?;
        f.entries[i].main = calm;
    }
    finish(&b)
}

fn blown(f: &Flow, e: &Event) -> Option<u32> {
    let Kind::Action { actor, params: Some(c), .. } = &e.kind else {
        return None;
    };
    if !f.actors.get(*actor as usize).is_some_and(|a| a.name == GENERATOR) {
        return None;
    }
    match c.get("IsBlewOcarina") {
        Some(Val::Bool(v)) => Some(*v),
        _ => None,
    }
}

fn on(v: u32) -> bool {
    v & 0x7FFF_FFFF != 0
}

fn blows(f: &Flow, e: &Event) -> bool {
    blown(f, e).is_some_and(on)
}

fn appears(f: &Flow, e: &Event) -> bool {
    let Kind::Switch { actor, query, params: Some(c), .. } = &e.kind else {
        return false;
    };
    let named: bool = f.actors.get(*actor as usize).is_some_and(|a| a.name == DATA && a.queries.get(*query as usize).is_some_and(|s| s == IS_BOOL));
    named && c.get("GameDataName") == Some(&Val::Str(APPEAR_FLAG.to_string()))
}

pub fn no_gossip(raw: &[u8]) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    let f: &mut Flow = flow(&mut b, GOSSIP_FLOW)?;
    let blow: Vec<usize> = (0..f.events.len()).filter(|&i| blows(f, &f.events[i])).collect();
    let gate: Vec<usize> = (0..f.events.len()).filter(|&i| appears(f, &f.events[i])).collect();
    if blow.len() != 1 || gate.len() != 1 {
        return Err(Error::Msg(format!("difficulty: the gossip stone flow has {} ocarina marks and {} fairy gates, expected one of each", blow.len(), gate.len())));
    }
    let off: u32 = f.events.iter().filter_map(|e| blown(f, e)).find(|v| !on(*v)).ok_or(Error::Format("difficulty: the gossip stone flow never clears its ocarina mark"))?;
    if let Some(c) = f.events[blow[0]].kind.params_mut() {
        for (k, v) in c.items.iter_mut() {
            if k == "IsBlewOcarina" {
                *v = Val::Bool(off);
            }
        }
    }
    if let Kind::Switch { cases, .. } = &mut f.events[gate[0]].kind {
        for c in cases.iter_mut() {
            c.val = NEVER;
        }
    }
    finish(&b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::Entry;

    fn sys(name: &str, actions: &[&str], queries: &[&str]) -> Actor {
        Actor {
            name: name.to_string(),
            sub: String::new(),
            arg: String::new(),
            actions: actions.iter().map(|s| s.to_string()).collect(),
            queries: queries.iter().map(|s| s.to_string()).collect(),
            params: None,
            ep: 0xFFFF,
            cut: 1,
        }
    }

    fn file(name: &str, actors: Vec<Actor>, events: Vec<Event>, mains: &[u16]) -> Vec<u8> {
        let entries: Vec<Entry> = mains.iter().enumerate().map(|(i, m)| Entry { name: format!("Entry{i}"), subs: Vec::new(), vars: Vec::new(), main: *m, pad: 0 }).collect();
        Bf { ver: [0, 4, 0, 0], bom: 0xFEFF, shift: 3, pad: 0, name: name.to_string(), flows: vec![Flow { name: name.to_string(), actors, events, entries }], tls: Vec::new() }.write().unwrap()
    }

    fn act(name: &str, actor: u16, act: u16, next: u16, items: Vec<(&str, Val)>) -> Event {
        Event { name: name.to_string(), kind: Kind::Action { next, actor, act, params: cont(items) } }
    }

    #[test]
    fn a_retry_takes_one_heart_only_above_three() {
        let raw: Vec<u8> = file(
            OVER_FLOW,
            vec![sys("GameSystemActor", &[RESULT], &[])],
            vec![act("Title", 0, 0, 2, vec![("EventResult", Val::Str("Title".into()))]), act("Retry", 0, 0, 2, vec![("EventResult", Val::Str("Retry".into()))]), act("End", 0, 0, NONE, vec![("EventResult", Val::Str("Done".into()))])],
            &[1],
        );
        let out: Vec<u8> = lose_heart(&raw).unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        let f: &Flow = &b.flows[0];
        assert_eq!(f.events.len(), 5);
        let Kind::Action { next, .. } = &f.events[1].kind else { panic!() };
        assert_eq!(*next, 4);
        let Kind::Action { next, .. } = &f.events[0].kind else { panic!() };
        assert_eq!(*next, 2, "going back to the title screen costs nothing");
        let Kind::Switch { cases, params, actor, query } = &f.events[4].kind else { panic!() };
        assert_eq!(cases, &vec![Case { val: 0, ev: 2 }, Case { val: 1, ev: 2 }, Case { val: 2, ev: 3 }]);
        assert_eq!(params.as_ref().unwrap().get("Value"), Some(&Val::Int(12)));
        assert_eq!(f.actors[*actor as usize].queries[*query as usize], IS_INT);
        let Kind::Action { next, params, actor, act } = &f.events[3].kind else { panic!() };
        assert_eq!(*next, 2);
        assert_eq!(params.as_ref().unwrap().get("Value"), Some(&Val::Int(-4)));
        assert_eq!(params.as_ref().unwrap().get("GameDataName"), Some(&Val::Str(MAX_LIFE.into())));
        assert_eq!(f.actors[*actor as usize].actions[*act as usize], ADD_INT);
        assert!(lose_heart(&out).is_err());
    }

    #[test]
    fn every_bottle_entry_is_gated_on_boss_fights_and_battle_music() {
        let raw: Vec<u8> = file("Dm_SY_OutBottle", vec![sys("PouchSystemActor", &["Use"], &[])], vec![act("A", 0, 0, NONE, vec![]), act("B", 0, 0, NONE, vec![])], &[0, 1]);
        let out: Vec<u8> = no_bottles(&raw, "Dm_SY_OutBottle").unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        let f: &Flow = &b.flows[0];
        assert_eq!(f.events.len(), 6);
        assert_eq!(f.actors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), vec!["PouchSystemActor", DATA, SOUND]);
        for (i, e) in f.entries.iter().enumerate() {
            let Kind::Switch { cases, params, .. } = &f.events[e.main as usize].kind else { panic!() };
            assert_eq!(params.as_ref().unwrap().get("GameDataName"), Some(&Val::Str(BOSS_FLAG.into())));
            assert_eq!((cases.len(), cases[0].val), (1, CLEAR));
            let Kind::Switch { cases, params, .. } = &f.events[cases[0].ev as usize].kind else { panic!() };
            assert_eq!(params.as_ref().unwrap().get("BgmName"), Some(&Val::Str(BATTLE_BGM.into())));
            assert_eq!(cases, &vec![Case { val: CLEAR, ev: i as u16 }]);
        }
        assert!(no_bottles(&out, "Dm_SY_OutBottle").is_err());
        assert!(no_bottles(&raw, "Other").is_err());
    }

    #[test]
    fn the_gossip_stone_never_hears_the_ocarina() {
        let gate: Event = Event { name: "Gate".into(), kind: Kind::Switch { actor: 1, query: 0, cases: vec![Case { val: 0, ev: 0 }], params: cont(vec![("Index", Val::Int(-1)), ("GameDataName", Val::Str(APPEAR_FLAG.into()))]) } };
        let raw: Vec<u8> = file(
            GOSSIP_FLOW,
            vec![sys(GENERATOR, &["EventGameDataSetIsBlewOcarinaToGossipStone"], &[]), sys(DATA, &[], &[IS_BOOL])],
            vec![act("Off", 0, 0, 1, vec![("IsBlewOcarina", Val::Bool(0x8000_0000))]), act("On", 0, 0, 2, vec![("IsBlewOcarina", Val::Bool(0x8000_0001))]), gate],
            &[0],
        );
        let b: Bf = Bf::parse(&no_gossip(&raw).unwrap()).unwrap();
        let f: &Flow = &b.flows[0];
        assert!(!f.events.iter().any(|e| blows(f, e)));
        let Kind::Switch { cases, .. } = &f.events[2].kind else { panic!() };
        assert_eq!(cases, &vec![Case { val: NEVER, ev: 0 }]);
        assert!(no_gossip(&b.write().unwrap()).is_err());
    }
}
