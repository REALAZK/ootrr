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
pub const TOP_HEARTS: i32 = 20;
pub const SET_MAX: &str = "EventTriggerSetMaxLife";
pub const BATTLE_BGM: &str = "BGM_EnemyBattle";
pub const MINIBOSS_BGM: &str = "BGM_MiddleBoss";
pub const FIGHT_BGMS: [&str; 2] = [BATTLE_BGM, MINIBOSS_BGM];
pub const BOSS_FLAG: &str = "IsBossBattle";
pub const APPEAR_FLAG: &str = "GossipStone_FairySpiritAppearOk";
pub const GENERATOR: &str = "GossipStone_FairyGenerator";
pub const NEVER: u32 = 2;
pub const CLEAR: u32 = 1;
pub const FIGHT: u32 = 0;
pub const TRUE: u32 = 0x8000_0001;
pub const FALSE: u32 = 0;
pub const SYSTEM: &str = "GameSystemActor";
pub const AUTOSAVE: &str = "EventTriggerRequestAutoSave";
pub const BOOT_FILE: &str = "Event/EventFlow/ShieldHylianNaviHintBootEvent.bfevfl.zs";
pub const BOOT_FLOW: &str = "ShieldHylianNaviHintBootEvent";
pub const BOOT_KEY: &str = "Sequence_ReservedBootEventName";
pub const RETRY_KEY: &str = "Sequence_LatestRetryType";
pub const RETRY_VALUE: &str = "Normal";
pub const RETRY_RESULT: &str = "Retry";
pub const COUNT_KEY: &str = "DungeonInfo.GerudoFortress.BossRetryNum";
pub const KEEP: Keep = Keep::Count;
pub const SET_STR: &str = "EventTriggerSetGameDataString64";
pub const SET_INT: &str = "EventTriggerSetGameDataInt";
pub const IS_ENUM: &str = "EventQueryIsGameDataEnum";
pub const SET_BOOL: &str = "EventTriggerSetGameDataBool";
pub const PLAYER: &str = "Player_Event";
pub const START: &str = "EventTriggerPlayerEventStartWithDrawn";
pub const PLAY_AS: &str = "EventPlayerPlayASForFlowchart";
pub const PUT_AWAY: &str = "EventPlayerDrawnShieldAfterUseBottle";
pub const REFUSE_AS: &str = "EvtTalkNo";
pub const UI: &str = "UISystemActor";
pub const NOTICE: &str = "EventTriggerRequestOpenSimpleInfo";
pub const NOTICE_TYPE: i32 = 23;

pub const DATA: &str = "GameDataActor";
pub const SOUND: &str = "SoundSystemActor";
pub const IS_INT: &str = "EventQueryIsGameDataInt";
pub const IS_BOOL: &str = "EventQueryIsGameDataBool";
pub const IS_BGM: &str = "EventQueryIsPlayingBgmName";
pub const RESULT: &str = "EventSetEventResult";

pub const OUT_FAIRY_FILE: &str = BOTTLE_FLOWS[2].0;
pub const OUT_FAIRY_FLOW: &str = BOTTLE_FLOWS[2].1;
pub const OUT_FAIRY: &str = "EventPlayerOutBottleFairy";
pub const REVIVE: &str = "EventTriggerPlayerUseFairyRecover";
pub const FAIRY_FLAG: &str = super::fairy::FLAG;

pub const HEART_MARK: &str = "OOTRRLoseHeart";
pub const FAIRY_MARK: &str = "OOTRRFairyHelps";
pub const BOTTLE_MARK: &str = "OOTRRNoBottle";

pub(super) fn flow<'a>(b: &'a mut Bf, name: &str) -> Res<&'a mut Flow> {
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

pub(super) fn query(f: &mut Flow, actor: &str, name: &str) -> Res<(u16, u16)> {
    let a: usize = actor_at(f, actor)?;
    Ok((a as u16, slot(&mut f.actors[a].queries, name)))
}

pub(super) fn action(f: &mut Flow, actor: &str, name: &str) -> Res<(u16, u16)> {
    let a: usize = actor_at(f, actor)?;
    Ok((a as u16, slot(&mut f.actors[a].actions, name)))
}

pub(super) fn push(f: &mut Flow, name: String, kind: Kind) -> Res<u16> {
    if f.events.len() >= NONE as usize {
        return Err(Error::Format("difficulty: too many events in a flow"));
    }
    f.events.push(Event { name, kind });
    Ok((f.events.len() - 1) as u16)
}

pub(super) fn cont(items: Vec<(&str, Val)>) -> Option<Cont> {
    Some(Cont { items: items.into_iter().map(|(k, v)| (k.to_string(), v)).collect() })
}

pub(super) fn finish(b: &Bf) -> Res<Vec<u8>> {
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != *b {
        return Err(Error::Format("difficulty: an edited flow does not read back"));
    }
    Ok(out)
}

pub(super) fn marked(f: &Flow, mark: &str) -> bool {
    f.events.iter().any(|e| e.name.starts_with(mark))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keep {
    Count,
    CountSave,
}

pub fn lose_heart(raw: &[u8]) -> Res<Vec<u8>> {
    lose_heart_with(raw)
}

pub fn lose_heart_boot(raw: &[u8]) -> Res<Vec<u8>> {
    lose_heart_boot_with(raw, KEEP)
}

fn trigger(f: &mut Flow, name: String, next: u16, actor: &str, act: &str, params: Option<Cont>) -> Res<u16> {
    let (a, n): (u16, u16) = action(f, actor, act)?;
    push(f, name, Kind::Action { next, actor: a, act: n, params })
}

fn int_check(f: &mut Flow, name: String, key: &str, value: i32, cases: [u16; 3]) -> Res<u16> {
    let (qa, q): (u16, u16) = query(f, DATA, IS_INT)?;
    let list: Vec<Case> = cases.iter().enumerate().map(|(i, ev)| Case { val: i as u32, ev: *ev }).collect();
    push(f, name, Kind::Switch { actor: qa, query: q, cases: list, params: cont(vec![("Index", Val::Int(-1)), ("GameDataName", Val::Str(key.to_string())), ("Value", Val::Int(value))]) })
}

fn set_key(key: &str, value: Val) -> Option<Cont> {
    cont(vec![("Index", Val::Int(-1)), ("GameDataName", Val::Str(key.to_string())), ("Value", value)])
}

fn is_retry(f: &Flow, e: &Event) -> bool {
    let Kind::Action { actor, act, params: Some(c), .. } = &e.kind else {
        return false;
    };
    let named: bool = f.actors.get(*actor as usize).is_some_and(|a| a.name == SYSTEM && a.actions.get(*act as usize).is_some_and(|s| s == RESULT));
    named && c.get("EventResult") == Some(&Val::Str(RETRY_RESULT.to_string()))
}

fn redirect(f: &mut Flow, upto: usize, from: u16, to: u16) {
    let swap = |n: &mut u16| {
        if *n == from {
            *n = to;
        }
    };
    for e in f.events.iter_mut().take(upto) {
        match &mut e.kind {
            Kind::Action { next, .. } | Kind::Join { next } | Kind::Sub { next, .. } => swap(next),
            Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => cases.iter_mut().for_each(|c| swap(&mut c.ev)),
            Kind::Fork { forks, .. } => forks.iter_mut().for_each(swap),
            Kind::Label { .. } => {}
        }
    }
    for e in f.entries.iter_mut() {
        swap(&mut e.main);
    }
}

pub fn lose_heart_with(raw: &[u8]) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    let f: &mut Flow = flow(&mut b, OVER_FLOW)?;
    if marked(f, HEART_MARK) {
        return Err(Error::Format("difficulty: the game over flow is already edited"));
    }
    let hits: Vec<usize> = (0..f.events.len()).filter(|&i| is_retry(f, &f.events[i])).collect();
    if hits.len() != 1 {
        return Err(Error::Msg(format!("difficulty: the game over flow has {} retry results, expected one", hits.len())));
    }
    let retry: u16 = hits[0] as u16;
    let old: usize = f.events.len();
    let reserve: u16 = trigger(f, format!("{HEART_MARK}Reserve"), retry, DATA, SET_STR, set_key(BOOT_KEY, Val::Str(BOOT_FLOW.to_string())))?;
    let mut first: u16 = reserve;
    for h in FLOOR_HEARTS + 1..=TOP_HEARTS {
        let store: u16 = trigger(f, format!("{HEART_MARK}Store{h}"), reserve, DATA, SET_INT, set_key(COUNT_KEY, Val::Int((h - 1) * HEART)))?;
        first = int_check(f, format!("{HEART_MARK}Level{h}"), MAX_LIFE, h * HEART, [first, store, first])?;
    }
    redirect(f, old, retry, first);
    finish(&b)
}

pub fn lose_heart_boot_with(raw: &[u8], keep: Keep) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    let f: &mut Flow = flow(&mut b, BOOT_FLOW)?;
    if marked(f, HEART_MARK) {
        return Err(Error::Format("difficulty: the boot flow is already edited"));
    }
    let main: u16 = f.entries.first().map(|e| e.main).filter(|m| *m != NONE).ok_or(Error::Format("difficulty: the boot flow has no entry"))?;
    let (ta, tn, tp): (u16, u16, Option<Cont>) = match f.events.get(main as usize).map(|e| &e.kind) {
        Some(Kind::Action { actor, act, params, .. }) => (*actor, *act, params.clone()),
        _ => return Err(Error::Format("difficulty: the boot flow does not start with a transition")),
    };
    let mut tail: u16 = NONE;
    if keep == Keep::CountSave {
        tail = trigger(f, format!("{HEART_MARK}Save"), tail, SYSTEM, AUTOSAVE, None)?;
    }
    let mut first: u16 = tail;
    for h in FLOOR_HEARTS..TOP_HEARTS {
        let to: i32 = h * HEART;
        let set: u16 = trigger(f, format!("{HEART_MARK}Take{h}"), tail, PLAYER, SET_MAX, cont(vec![("MaxLife", Val::Int(to))]))?;
        let above: u16 = int_check(f, format!("{HEART_MARK}Above{h}"), MAX_LIFE, to, [tail, tail, set])?;
        first = int_check(f, format!("{HEART_MARK}Count{h}"), COUNT_KEY, to, [first, above, first])?;
    }
    let wait: u16 = push(f, format!("{HEART_MARK}Wait"), Kind::Action { next: first, actor: ta, act: tn, params: tp })?;
    let (qa, q): (u16, u16) = query(f, DATA, IS_ENUM)?;
    let gate: u16 = push(
        f,
        format!("{HEART_MARK}Gate"),
        Kind::Switch { actor: qa, query: q, cases: vec![Case { val: 0, ev: wait }, Case { val: 1, ev: main }], params: set_key_enum() },
    )?;
    f.entries[0].main = gate;
    finish(&b)
}

fn set_key_enum() -> Option<Cont> {
    cont(vec![("Index", Val::Int(-1)), ("GameDataName", Val::Str(RETRY_KEY.to_string())), ("Value", Val::Str(RETRY_VALUE.to_string()))])
}

fn does(f: &Flow, e: &Event, name: &str) -> bool {
    let Kind::Action { actor, act, .. } = &e.kind else {
        return false;
    };
    f.actors.get(*actor as usize).is_some_and(|a| a.name == PLAYER && a.actions.get(*act as usize).is_some_and(|s| s == name))
}

pub fn fairy_used(raw: &[u8], name: &str, used: &str) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    let f: &mut Flow = flow(&mut b, name)?;
    if marked(f, FAIRY_MARK) {
        return Err(Error::Format("difficulty: a fairy flow is already edited"));
    }
    let at: Vec<usize> = (0..f.events.len()).filter(|&i| does(f, &f.events[i], used)).collect();
    if at.is_empty() {
        return Err(Error::Msg(format!("difficulty: the flow {name} never runs {used}")));
    }
    let (aa, a): (u16, u16) = action(f, DATA, SET_BOOL)?;
    for (n, i) in at.into_iter().enumerate() {
        let old: u16 = match &f.events[i].kind {
            Kind::Action { next, .. } => *next,
            _ => NONE,
        };
        let tell: u16 = push(
            f,
            format!("{FAIRY_MARK}Tell{n}"),
            Kind::Action { next: old, actor: aa, act: a, params: cont(vec![("Index", Val::Int(-1)), ("GameDataName", Val::Str(FAIRY_FLAG.to_string())), ("Value", Val::Bool(TRUE))]) },
        )?;
        if let Kind::Action { next, .. } = &mut f.events[i].kind {
            *next = tell;
        }
    }
    finish(&b)
}

pub(super) type Step<'a> = (&'a str, &'a str, Option<Cont>);

pub(super) fn refusal(f: &mut Flow, tag: &str, i: usize, before: Vec<Step>, after: Vec<Step>) -> Res<u16> {
    let mut steps: Vec<Step> = before;
    steps.push((PLAYER, PLAY_AS, shake()));
    steps.extend(after);
    let mut next: u16 = NONE;
    for (n, (actor, name, params)) in steps.into_iter().enumerate().rev() {
        let (a, act): (u16, u16) = action(f, actor, name)?;
        next = push(f, format!("{tag}Refuse{i}_{n}"), Kind::Action { next, actor: a, act, params })?;
    }
    Ok(next)
}

fn shake() -> Option<Cont> {
    cont(vec![
        ("IsWaitEnd", Val::Bool(TRUE)),
        ("IsAllSlot", Val::Bool(FALSE)),
        ("ForceFadeFrame", Val::Int(-1)),
        ("KeepOneTimeAnm", Val::Bool(FALSE)),
        ("ASCommand", Val::Str(REFUSE_AS.to_string())),
        ("UseAnmDriven", Val::Bool(FALSE)),
        ("ClothWarpMode", Val::Int(0)),
    ])
}

fn notice() -> Option<Cont> {
    cont(vec![("Type", Val::Int(NOTICE_TYPE))])
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
        let away: Option<Cont> = cont(vec![("IsWaitEnd", Val::Bool(TRUE)), ("IsImmediateDrawn", Val::Bool(FALSE))]);
        let deny: u16 = refusal(f, BOTTLE_MARK, i, vec![(PLAYER, START, None), (UI, NOTICE, notice())], vec![(PLAYER, PUT_AWAY, away)])?;
        let mut quiet: u16 = main;
        for (n, bgm) in FIGHT_BGMS.iter().enumerate().rev() {
            quiet = push(
                f,
                format!("{BOTTLE_MARK}Music{i}_{n}"),
                Kind::Switch { actor: sa, query: sq, cases: vec![Case { val: FIGHT, ev: deny }, Case { val: CLEAR, ev: quiet }], params: cont(vec![("BgmName", Val::Str(bgm.to_string()))]) },
            )?;
        }
        let calm: u16 = push(
            f,
            format!("{BOTTLE_MARK}Boss{i}"),
            Kind::Switch { actor: ba, query: bq, cases: vec![Case { val: FIGHT, ev: deny }, Case { val: CLEAR, ev: quiet }], params: cont(vec![("Index", Val::Int(-1)), ("GameDataName", Val::Str(BOSS_FLAG.to_string()))]) },
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

    fn over_file() -> Vec<u8> {
        let screen: Event = Event { name: "Screen".to_string(), kind: Kind::Switch { actor: 0, query: 0, cases: vec![Case { val: 0, ev: 1 }, Case { val: 1, ev: 2 }], params: None } };
        file(
            OVER_FLOW,
            vec![sys(SYSTEM, &[RESULT], &["EventQueryGameOverScreen"])],
            vec![screen, act("Retry", 0, 0, 3, vec![("IsWaitEnd", Val::Bool(TRUE)), ("EventResult", Val::Str("Retry".into()))]), act("Title", 0, 0, 3, vec![("EventResult", Val::Str("Title".into()))]), act("Next", 0, 0, NONE, vec![])],
            &[0],
        )
    }

    fn name_of(f: &Flow, e: u16) -> (String, String) {
        match &f.events[e as usize].kind {
            Kind::Action { actor, act, .. } => (f.actors[*actor as usize].name.clone(), f.actors[*actor as usize].actions[*act as usize].clone()),
            Kind::Switch { actor, query, .. } => (f.actors[*actor as usize].name.clone(), f.actors[*actor as usize].queries[*query as usize].clone()),
            _ => panic!(),
        }
    }

    fn cases_of(f: &Flow, e: u16) -> Vec<Case> {
        let Kind::Switch { cases, .. } = &f.events[e as usize].kind else { panic!() };
        cases.clone()
    }

    fn next_of(f: &Flow, e: u16) -> u16 {
        let Kind::Action { next, .. } = &f.events[e as usize].kind else { panic!() };
        *next
    }

    fn arg(f: &Flow, e: u16, k: &str) -> Val {
        f.events[e as usize].kind.params().unwrap().get(k).unwrap().clone()
    }

    #[test]
    fn a_retry_reserves_the_boot_event_and_a_title_choice_is_left_alone() {
        let plain: Vec<u8> = over_file();
        let done: Vec<u8> = lose_heart_with(&plain).unwrap();
        let b: Bf = Bf::parse(&done).unwrap();
        let f: &Flow = &b.flows[0];
        let rungs: usize = (TOP_HEARTS - FLOOR_HEARTS) as usize;
        assert_eq!(f.events.len(), 4 + 1 + 2 * rungs);
        let reserve: u16 = 4;
        let top: u16 = (4 + 2 * rungs) as u16;
        assert_eq!(cases_of(f, 0), vec![Case { val: 0, ev: top }, Case { val: 1, ev: 2 }], "only the retry choice is redirected");
        assert_eq!(name_of(f, reserve), (DATA.to_string(), SET_STR.to_string()));
        assert_eq!(next_of(f, reserve), 1, "the reservation runs before the retry result");
        assert_eq!(arg(f, reserve, "GameDataName"), Val::Str(BOOT_KEY.into()));
        assert_eq!(arg(f, reserve, "Value"), Val::Str(BOOT_FLOW.into()));
        assert!(lose_heart_with(&done).is_err());
        let none: Vec<u8> = file(OVER_FLOW, vec![sys(SYSTEM, &[RESULT], &[])], vec![act("Other", 0, 0, NONE, vec![])], &[0]);
        assert!(lose_heart_with(&none).is_err());
    }

    #[test]
    fn the_count_variant_remembers_the_level_at_the_retry() {
        let out: Vec<u8> = lose_heart_with(&over_file()).unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        let f: &Flow = &b.flows[0];
        let rungs: usize = (TOP_HEARTS - FLOOR_HEARTS) as usize;
        assert_eq!(f.events.len(), 4 + 1 + 2 * rungs);
        let reserve: u16 = 4;
        let top: u16 = (4 + 2 * rungs) as u16;
        assert_eq!(cases_of(f, 0)[0].ev, top);
        assert_eq!(name_of(f, top), (DATA.to_string(), IS_INT.to_string()));
        assert_eq!(arg(f, top, "GameDataName"), Val::Str(MAX_LIFE.into()));
        assert_eq!(arg(f, top, "Value"), Val::Int(80));
        let c: Vec<Case> = cases_of(f, top);
        assert_eq!((c[1].val, c[0].ev, c[2].ev), (1, top - 2, top - 2));
        let store: u16 = c[1].ev;
        assert_eq!(name_of(f, store), (DATA.to_string(), SET_INT.to_string()));
        assert_eq!(arg(f, store, "GameDataName"), Val::Str(COUNT_KEY.into()));
        assert_eq!(arg(f, store, "Value"), Val::Int(76), "twenty hearts are remembered as nineteen");
        assert_eq!(next_of(f, store), reserve);
        let low: u16 = 6;
        assert_eq!(arg(f, low, "Value"), Val::Int(16), "four hearts is the lowest level that is remembered");
        assert_eq!(cases_of(f, low)[0].ev, reserve, "three hearts or fewer remember nothing");
        assert_eq!(arg(f, 5, "Value"), Val::Int(12));
    }

    fn boot_file() -> Vec<u8> {
        file(
            BOOT_FLOW,
            vec![sys("EventSystemActor", &["EventTransition"], &[]), sys(PLAYER, &[], &[]), sys(DATA, &[SET_BOOL], &[])],
            vec![act("Wait", 0, 0, 1, vec![("EventName", Val::Str("DefaultBootEvent".into()))]), act("Hint", 0, 0, NONE, vec![])],
            &[0],
        )
    }

    #[test]
    fn the_boot_event_cuts_to_the_remembered_level_and_never_saves() {
        let out: Vec<u8> = lose_heart_boot_with(&boot_file(), Keep::Count).unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        let f: &Flow = &b.flows[0];
        assert!(!f.actors.iter().any(|a| a.actions.iter().any(|s| s == AUTOSAVE)));
        let gate: u16 = f.entries[0].main;
        assert_eq!(name_of(f, gate), (DATA.to_string(), IS_ENUM.to_string()));
        assert_eq!(arg(f, gate, "GameDataName"), Val::Str(RETRY_KEY.into()));
        assert_eq!(arg(f, gate, "Value"), Val::Str(RETRY_VALUE.into()));
        let c: Vec<Case> = cases_of(f, gate);
        assert_eq!(c[1], Case { val: 1, ev: 0 }, "any other boot runs the original transition and the hint");
        let wait: u16 = c[0].ev;
        assert_eq!(name_of(f, wait), ("EventSystemActor".to_string(), "EventTransition".to_string()));
        assert_eq!(arg(f, wait, "EventName"), Val::Str("DefaultBootEvent".into()), "a retry also waits for the usual boot event first");
        let top: u16 = next_of(f, wait);
        assert_eq!(arg(f, top, "GameDataName"), Val::Str(COUNT_KEY.into()));
        assert_eq!(arg(f, top, "Value"), Val::Int(76));
        let above: u16 = cases_of(f, top)[1].ev;
        assert_eq!(arg(f, above, "GameDataName"), Val::Str(MAX_LIFE.into()));
        assert_eq!(arg(f, above, "Value"), Val::Int(76));
        let a: Vec<Case> = cases_of(f, above);
        let take: u16 = a[2].ev;
        assert_eq!(name_of(f, take), (PLAYER.to_string(), SET_MAX.to_string()));
        assert_eq!(arg(f, take, "MaxLife"), Val::Int(76), "only a larger maximum is cut down to the remembered one");
        assert_eq!(a[0].ev, a[1].ev);
        assert_eq!(a[0].ev, next_of(f, take));
        assert_eq!(a[0].ev, NONE, "the boot event ends after the cut");
        assert_eq!(arg(f, 4, "Value"), Val::Int(12));
        assert_eq!(arg(f, 4, "GameDataName"), Val::Str(COUNT_KEY.into()));
        assert!(lose_heart_boot_with(&out, Keep::Count).is_err());
        assert!(lose_heart_boot_with(&file("Other", vec![sys(DATA, &[], &[])], vec![act("A", 0, 0, NONE, vec![])], &[0]), Keep::Count).is_err());
    }

    #[test]
    fn the_save_variant_requests_an_autosave_after_the_cut() {
        let out: Vec<u8> = lose_heart_boot_with(&boot_file(), Keep::CountSave).unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        let f: &Flow = &b.flows[0];
        let gate: u16 = f.entries[0].main;
        let wait: u16 = cases_of(f, gate)[0].ev;
        let top: u16 = next_of(f, wait);
        let above: u16 = cases_of(f, top)[1].ev;
        let a: Vec<Case> = cases_of(f, above);
        let take: u16 = a[2].ev;
        let save: u16 = next_of(f, take);
        assert_eq!(name_of(f, save), (SYSTEM.to_string(), AUTOSAVE.to_string()));
        assert_eq!(a[0].ev, save);
        assert_eq!(next_of(f, save), NONE);
    }

    #[test]
    fn the_flag_is_raised_right_after_each_use_of_a_fairy() {
        let raw: Vec<u8> = file(
            OUT_FAIRY_FLOW,
            vec![sys(PLAYER, &["Start", OUT_FAIRY], &[]), sys("PouchSystemActor", &[OUT_FAIRY], &[])],
            vec![act("A", 0, 0, 1, vec![]), act("B", 0, 1, 2, vec![("IsWaitEnd", Val::Bool(TRUE))]), act("C", 1, 0, 3, vec![]), act("D", 0, 1, NONE, vec![])],
            &[0],
        );
        let out: Vec<u8> = fairy_used(&raw, OUT_FAIRY_FLOW, OUT_FAIRY).unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        let f: &Flow = &b.flows[0];
        assert_eq!(f.events.len(), 6);
        let next = |e: usize| -> u16 {
            let Kind::Action { next, .. } = &f.events[e].kind else { panic!() };
            *next
        };
        assert_eq!((next(0), next(1), next(2), next(3), next(4), next(5)), (1, 4, 3, 5, 2, NONE));
        for e in [4, 5] {
            let Kind::Action { actor, act, params, .. } = &f.events[e].kind else { panic!() };
            assert_eq!((f.actors[*actor as usize].name.as_str(), f.actors[*actor as usize].actions[*act as usize].as_str()), (DATA, SET_BOOL));
            let c: &Cont = params.as_ref().unwrap();
            assert_eq!((c.get("GameDataName"), c.get("Value"), c.get("Index")), (Some(&Val::Str(FAIRY_FLAG.into())), Some(&Val::Bool(TRUE)), Some(&Val::Int(-1))));
        }
        assert!(fairy_used(&out, OUT_FAIRY_FLOW, OUT_FAIRY).is_err());
        assert!(fairy_used(&raw, OUT_FAIRY_FLOW, REVIVE).is_err());
        assert!(fairy_used(&raw, "Other", OUT_FAIRY).is_err());
    }

    #[test]
    fn every_bottle_entry_is_gated_on_boss_fights_and_battle_music() {
        let raw: Vec<u8> = file("Dm_SY_OutBottle", vec![sys("PouchSystemActor", &["Use"], &[])], vec![act("A", 0, 0, NONE, vec![]), act("B", 0, 0, NONE, vec![])], &[0, 1]);
        let out: Vec<u8> = no_bottles(&raw, "Dm_SY_OutBottle").unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        let f: &Flow = &b.flows[0];
        assert_eq!(f.events.len(), 16);
        assert_eq!(f.actors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), vec!["PouchSystemActor", DATA, SOUND, PLAYER, UI]);
        let name = |e: u16| -> String {
            let Kind::Action { actor, act, .. } = &f.events[e as usize].kind else { panic!() };
            f.actors[*actor as usize].actions[*act as usize].clone()
        };
        let next = |e: u16| -> u16 {
            let Kind::Action { next, .. } = &f.events[e as usize].kind else { panic!() };
            *next
        };
        for (i, e) in f.entries.iter().enumerate() {
            let Kind::Switch { cases, params, .. } = &f.events[e.main as usize].kind else { panic!() };
            assert_eq!(params.as_ref().unwrap().get("GameDataName"), Some(&Val::Str(BOSS_FLAG.into())));
            assert_eq!((cases[0].val, cases[1].val), (FIGHT, CLEAR));
            let deny: u16 = cases[0].ev;
            let mut at: u16 = cases[1].ev;
            for bgm in FIGHT_BGMS {
                let Kind::Switch { cases, params, .. } = &f.events[at as usize].kind else { panic!() };
                assert_eq!(params.as_ref().unwrap().get("BgmName"), Some(&Val::Str(bgm.into())));
                assert_eq!((cases[0], cases[1].val), (Case { val: FIGHT, ev: deny }, CLEAR));
                at = cases[1].ev;
            }
            assert_eq!(at, i as u16, "calm music on both counts reaches the game's own start");
            let chain: Vec<String> = vec![deny, next(deny), next(next(deny)), next(next(next(deny)))].into_iter().map(|e| name(e)).collect();
            assert_eq!(chain, vec![START.to_string(), NOTICE.to_string(), PLAY_AS.to_string(), PUT_AWAY.to_string()], "the refusal shows the game's notice, shakes the head and puts the bottle away");
            let Kind::Action { params, .. } = &f.events[next(deny) as usize].kind else { panic!() };
            assert_eq!(params.as_ref().unwrap().get("Type"), Some(&Val::Int(NOTICE_TYPE)));
            assert_eq!(next(next(next(next(deny)))), NONE, "a refused bottle still ends by putting the bottle away");
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
