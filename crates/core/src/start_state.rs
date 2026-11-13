//! Story state written when a new game starts: quest steps, flags and pouch items.
//!
//! The remake gates the world on challenge steps and game data flags. Instead of patching each
//! gate, a setting that opens part of the world describes the state the game would be in after
//! the skipped story, and these actions are spliced into the opening event flow right after the
//! game's own first quest step. They use the same flow actions the game's cutscenes use.

use crate::bfevfl::{Actor, Cont, Event, Flow, Kind, Val};
use crate::error::{Error, Res};

pub const FLOW: &str = "OpeningEvent";
pub const ANCHOR_CHALLENGE: &str = "Meet_DekuTree";
pub const ANCHOR_STEP: &str = "Step01";
pub const PREFIX: &str = "OOTRRState";
pub const CHALLENGE_ACTOR: &str = "ChallengeSystemActor";
pub const PROGRESS: &str = "EventExecuteProgressChallengeStep";
pub const DATA_ACTOR: &str = "GameDataActor";
pub const SET_BOOL: &str = "EventTriggerSetGameDataBool";
pub const POUCH_ACTOR: &str = "PouchSystemActor";
pub const POUCH: &str = "EventTriggerAddOrRemovePouchContent";
pub const WORLD_ACTOR: &str = "EventWorldManagerController";
pub const SET_TIME: &str = "EventTrigger_SetTime";
const TRUE: u32 = 0x8000_0001;
const FALSE: u32 = 0;
const NONE: u16 = 0xFFFF;

/// The gate guard's own flag: with it set he stands aside and the gate is open.
pub const GATE_OPEN: &str = "Npc_Hylia_Soldier_003_GateOpen";
/// The story the open Kakariko setting skips: the castle visit, Impa's send off and showing the
/// guard the letter. Each quest is readied first, then moved to the step the skipped story ends at.
pub const KAKARIKO_STEPS: [(&str, &str); 6] = [
    ("Goto_HyruleCastle", "Ready"),
    ("Goto_HyruleCastle", "Complete"),
    ("DoorOfTime", "Ready"),
    ("DoorOfTime", "Step02"),
    ("Get_StoneOfFire", "Ready"),
    ("Get_StoneOfFire", "KakarikoVillageLetter"),
];
/// Flags the skipped story would have left behind. Saria leaves the forest once the castle quest
/// is complete, so her house scenes are marked as played and the house door unlocked; her bridge
/// gift still plays and stays a check. The owl's lesson opens the Threads of Time menu. Talon's
/// castle actor goes once his scene is marked as played; the ranch copies of Talon and Malon
/// appear from the castle quest itself.
pub const KAKARIKO_FLAGS: [(&str, bool); 9] = [
    (GATE_OPEN, true),
    ("HavePlayedEvent.Dm_Kokiri_0045", true),
    ("HavePlayedEvent.Dm_Kokiri_0050", true),
    ("HavePlayedEvent.KokiriForest_StopInFrontOfLinkHouse", true),
    ("HavePlayedEvent.Dm_Kokiri_0050_After_Near", true),
    ("KokiriForest_LinkHouseDoor_Open", true),
    ("UI_IsFinishChallengeMenuOpenEvent", true),
    ("HavePlayedEvent.HyruleField_Goto_HyruleCastle_Kaepora", true),
    ("HavePlayedEvent.Dm_HrlCastle_0020", true),
];
/// The owl also hands out the amiibo pouch in the vanilla game.
pub const KAKARIKO_ITEMS: [&str; 1] = ["Pouch_Amiibo"];
/// The checks the skipped story would have handed out. Their items are given at the start.
pub const KAKARIKO_LOCATIONS: [&str; 2] = ["HC Zeldas Letter", "Song from Impa"];
/// Malon is at the ranch once the castle quest is complete, so her egg cannot be collected.
/// The check stays in the world but never holds anything that matters.
pub const KAKARIKO_EXCLUDED: [&str; 1] = ["HC Malon Egg"];

fn owned(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter().map(|(c, s)| ((*c).to_string(), (*s).to_string())).collect()
}

fn flags_of(v: &[(&str, bool)]) -> Vec<(String, bool)> {
    v.iter().map(|(n, b)| ((*n).to_string(), *b)).collect()
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ops {
    pub steps: Vec<(String, String)>,
    pub flags: Vec<(String, bool)>,
    pub items: Vec<String>,
    /// The clock to set, hour and minute, as the first Hyrule Field scenes do.
    pub time: Option<(i32, i32)>,
}

impl Ops {
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty() && self.flags.is_empty() && self.items.is_empty() && self.time.is_none()
    }

    pub fn len(&self) -> usize {
        self.steps.len() + self.flags.len() + self.items.len() + usize::from(self.time.is_some())
    }

    pub fn extend(&mut self, o: Ops) {
        self.steps.extend(o.steps);
        self.flags.extend(o.flags);
        self.items.extend(o.items);
        if o.time.is_some() {
            self.time = o.time;
        }
    }
}

/// The clock the first Hyrule Field scene sets once the owl has spoken.
pub const KAKARIKO_TIME: (i32, i32) = (7, 30);

pub fn kakariko(items: &[String]) -> Ops {
    let mut all: Vec<String> = KAKARIKO_ITEMS.iter().map(|s| (*s).to_string()).collect();
    all.extend(items.iter().cloned());
    Ops { steps: owned(&KAKARIKO_STEPS), flags: flags_of(&KAKARIKO_FLAGS), items: all, time: Some(KAKARIKO_TIME) }
}

fn text<'a>(k: &'a Kind, key: &str) -> Option<&'a str> {
    match k.params()?.get(key)? {
        Val::Str(s) => Some(s.as_str()),
        _ => None,
    }
}

fn flag(v: bool) -> Val {
    Val::Bool(if v { TRUE } else { FALSE })
}

fn item(k: &str, v: Val) -> (String, Val) {
    (k.to_string(), v)
}

fn idx(n: usize) -> Res<u16> {
    if n >= NONE as usize {
        return Err(Error::Format("start state: too many entries in the flow"));
    }
    Ok(n as u16)
}

/// The actor and action indexes for `action` on `actor`, adding either when the flow lacks it.
/// A new actor copies the member settings of the flow's first actor, as the intro skip does.
pub fn slot(f: &mut Flow, actor: &str, action: &str) -> Res<(u16, u16)> {
    if let Some(i) = f.actors.iter().position(|a| a.name == actor && a.sub.is_empty()) {
        let a: &mut Actor = &mut f.actors[i];
        let j: usize = match a.actions.iter().position(|s| s == action) {
            Some(j) => j,
            None => {
                a.actions.push(action.to_string());
                a.actions.len() - 1
            }
        };
        return Ok((idx(i)?, idx(j)?));
    }
    let base: Actor = f.actors.first().cloned().ok_or(Error::Format("start state: the flow has no actors"))?;
    f.actors.push(Actor {
        name: actor.to_string(),
        sub: String::new(),
        arg: String::new(),
        actions: vec![action.to_string()],
        queries: Vec::new(),
        params: base.params,
        ep: base.ep,
        cut: base.cut,
    });
    Ok((idx(f.actors.len() - 1)?, 0))
}

/// The game's own first quest step in the new game path, after which the state is written.
pub fn anchor(f: &Flow) -> Res<usize> {
    let v: Vec<usize> = (0..f.events.len())
        .filter(|&i| {
            matches!(f.events[i].kind, Kind::Action { .. })
                && text(&f.events[i].kind, "ChallengeName") == Some(ANCHOR_CHALLENGE)
                && text(&f.events[i].kind, "StepName") == Some(ANCHOR_STEP)
        })
        .collect();
    match v.as_slice() {
        [i] => Ok(*i),
        _ => Err(Error::Msg(format!(
            "start state: expected one {ANCHOR_CHALLENGE} {ANCHOR_STEP} request in {FLOW}, found {}",
            v.len()
        ))),
    }
}

/// Splices the state writes after the anchor. Returns how many events were added.
pub fn patch(f: &mut Flow, ops: &Ops) -> Res<usize> {
    if ops.is_empty() {
        return Ok(0);
    }
    if f.events.iter().any(|e| e.name.starts_with(PREFIX)) {
        return Err(Error::Format("start state: the flow already carries a state patch"));
    }
    let at: usize = anchor(f)?;
    let Kind::Action { next: old, .. } = f.events[at].kind else {
        return Err(Error::Format("start state: the anchor is not an action"));
    };
    let (ca, cx): (u16, u16) = slot(f, CHALLENGE_ACTOR, PROGRESS)?;
    let (da, dx): (u16, u16) = if ops.flags.is_empty() { (0, 0) } else { slot(f, DATA_ACTOR, SET_BOOL)? };
    let (pa, px): (u16, u16) = if ops.items.is_empty() { (0, 0) } else { slot(f, POUCH_ACTOR, POUCH)? };
    let (wa, wx): (u16, u16) = if ops.time.is_none() { (0, 0) } else { slot(f, WORLD_ACTOR, SET_TIME)? };
    let mut chain: Vec<Kind> = Vec::with_capacity(ops.len());
    for (c, s) in &ops.steps {
        let items: Vec<(String, Val)> = vec![
            item("ChallengeName", Val::Str(c.clone())),
            item("StepName", Val::Str(s.clone())),
            item("IsIgnoreCompletedChallenge", flag(true)),
            item("IsIgnoreChallengeProgressUI", flag(true)),
            item("IsWaitEnd", flag(true)),
        ];
        chain.push(Kind::Action { next: NONE, actor: ca, act: cx, params: Some(Cont { items }) });
    }
    for (n, v) in &ops.flags {
        let items: Vec<(String, Val)> = vec![item("Index", Val::Int(-1)), item("GameDataName", Val::Str(n.clone())), item("Value", flag(*v))];
        chain.push(Kind::Action { next: NONE, actor: da, act: dx, params: Some(Cont { items }) });
    }
    for a in &ops.items {
        let items: Vec<(String, Val)> = vec![
            item("Actor", Val::Str(a.clone())),
            item("Num", Val::Int(1)),
            item("IsSetMessageTargetActorName", flag(false)),
            item("IsEquipDIrect", flag(false)),
            item("IsGetEvent", flag(false)),
        ];
        chain.push(Kind::Action { next: NONE, actor: pa, act: px, params: Some(Cont { items }) });
    }
    if let Some((h, m)) = ops.time {
        let items: Vec<(String, Val)> = vec![item("TimeHour", Val::Int(h)), item("TimeMin", Val::Int(m))];
        chain.push(Kind::Action { next: NONE, actor: wa, act: wx, params: Some(Cont { items }) });
    }
    let base: usize = f.events.len();
    idx(base + chain.len())?;
    let n: usize = chain.len();
    for (i, mut k) in chain.into_iter().enumerate() {
        if let Kind::Action { next, .. } = &mut k {
            *next = if i + 1 < n { idx(base + i + 1)? } else { old };
        }
        f.events.push(Event { name: format!("{PREFIX}{}", base + i), kind: k });
    }
    if let Kind::Action { next, .. } = &mut f.events[at].kind {
        *next = idx(base)?;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::{Bf, Entry};

    fn actor(n: &str, acts: &[&str]) -> Actor {
        Actor {
            name: n.to_string(),
            sub: String::new(),
            arg: String::new(),
            actions: acts.iter().map(|s| s.to_string()).collect(),
            queries: Vec::new(),
            params: Some(Cont { items: vec![("EventMemberCreateMethod".to_string(), Val::Int(0))] }),
            ep: 0xFFFF,
            cut: 1,
        }
    }

    fn step(n: &str, c: &str, s: &str, next: u16) -> Event {
        Event {
            name: n.to_string(),
            kind: Kind::Action {
                next,
                actor: 1,
                act: 0,
                params: Some(Cont { items: vec![item("ChallengeName", Val::Str(c.to_string())), item("StepName", Val::Str(s.to_string()))] }),
            },
        }
    }

    fn sample() -> Bf {
        let events: Vec<Event> = vec![
            Event { name: "E0".to_string(), kind: Kind::Action { next: 1, actor: 0, act: 0, params: None } },
            step("E1", ANCHOR_CHALLENGE, ANCHOR_STEP, 2),
            Event { name: "E2".to_string(), kind: Kind::Action { next: NONE, actor: 0, act: 0, params: None } },
        ];
        let f: Flow = Flow {
            name: FLOW.to_string(),
            actors: vec![actor("EventSystemActor", &["EventWait"]), actor(CHALLENGE_ACTOR, &[PROGRESS])],
            events,
            entries: vec![Entry { name: "Opening_Introduction".to_string(), subs: Vec::new(), vars: Vec::new(), main: 0, pad: 0 }],
        };
        Bf { ver: [0, 4, 0, 0], bom: 0xFEFF, shift: 3, pad: 0, name: FLOW.to_string(), flows: vec![f], tls: Vec::new() }
    }

    fn next_of(e: &Event) -> u16 {
        match e.kind {
            Kind::Action { next, .. } => next,
            _ => panic!("not an action"),
        }
    }

    #[test]
    fn the_state_chain_runs_after_the_first_quest_step_and_lands_back_on_the_old_path() {
        let mut b: Bf = sample();
        let ops: Ops = kakariko(&["Item_ZeldaLetter".to_string(), "Item_Melody_ZeldaLullaby".to_string()]);
        let n: usize = ops.len();
        assert_eq!(n, 6 + 9 + 3 + 1);
        let f: &mut Flow = &mut b.flows[0];
        assert_eq!(patch(f, &ops).unwrap(), n);
        assert_eq!(f.events.len(), 3 + n);
        assert_eq!(next_of(&f.events[1]), 3);
        for i in 3..3 + n - 1 {
            assert_eq!(next_of(&f.events[i]), (i + 1) as u16, "event {i}");
        }
        assert_eq!(next_of(&f.events[2 + n]), 2);
        assert_eq!(text(&f.events[3].kind, "ChallengeName"), Some("Goto_HyruleCastle"));
        assert_eq!(text(&f.events[8].kind, "StepName"), Some("KakarikoVillageLetter"));
        assert_eq!(text(&f.events[9].kind, "GameDataName"), Some(GATE_OPEN));
        assert_eq!(text(&f.events[18].kind, "Actor"), Some("Pouch_Amiibo"));
        assert_eq!(text(&f.events[1 + n].kind, "Actor"), Some("Item_Melody_ZeldaLullaby"));
        assert_eq!(f.events[2 + n].kind.params().and_then(|c| c.get("TimeHour")), Some(&Val::Int(7)));
        assert!(!ops.flags.iter().any(|(n, _)| n.contains("Dm_Kokiri_0060")));
        let names: Vec<&str> = f.actors.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["EventSystemActor", CHALLENGE_ACTOR, DATA_ACTOR, POUCH_ACTOR, WORLD_ACTOR]);
        assert_eq!(f.actors[2].actions, vec![SET_BOOL.to_string()]);
        assert!(f.actors[2].params.is_some());
        assert!(patch(f, &ops).is_err());
        let bytes: Vec<u8> = b.write().unwrap();
        assert_eq!(Bf::parse(&bytes).unwrap(), b);
    }

    #[test]
    fn empty_ops_change_nothing_and_a_flow_without_the_anchor_is_refused() {
        let mut b: Bf = sample();
        assert_eq!(patch(&mut b.flows[0], &Ops::default()).unwrap(), 0);
        assert_eq!(b, sample());
        b.flows[0].events.remove(1);
        assert!(patch(&mut b.flows[0], &kakariko(&[])).is_err());
    }

    #[test]
    fn an_existing_action_slot_is_reused_and_a_missing_action_is_added_to_the_actor() {
        let mut b: Bf = sample();
        let f: &mut Flow = &mut b.flows[0];
        assert_eq!(slot(f, CHALLENGE_ACTOR, PROGRESS).unwrap(), (1, 0));
        assert_eq!(slot(f, "EventSystemActor", SET_BOOL).unwrap(), (0, 1));
        assert_eq!(f.actors[0].actions, vec!["EventWait".to_string(), SET_BOOL.to_string()]);
        assert_eq!(slot(f, POUCH_ACTOR, POUCH).unwrap(), (2, 0));
    }
}
