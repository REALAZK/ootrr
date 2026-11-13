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
pub const SET_ENUM: &str = "EventTriggerSetGameDataEnum";
pub const SET_INT: &str = "EventTriggerSetGameDataInt";
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
/// guard the letter. The Three Spiritual Stones quest (DoorOfTime inside the game) is readied
/// like the others; its dependency on the castle quest is removed from the quest table, since
/// the game refused every request made at new game while that dependency was in place.
pub const KAKARIKO_STEPS: [(&str, &str); 6] = [
    ("Goto_HyruleCastle", "Ready"),
    ("Goto_HyruleCastle", "Complete"),
    ("DoorOfTime", "Ready"),
    ("DoorOfTime", "Step02"),
    ("Get_StoneOfFire", "Ready"),
    ("Get_StoneOfFire", "KakarikoVillageLetter"),
];
/// The quest table and the row whose dependency is cleared.
pub const QUEST_TABLE: &str = "RSDB/Challenge.Product.111.rstbl.byml.zs";
pub const STONES_QUEST: &str = "Work/Challenge/DoorOfTime.game__challenge__Challenge.gyml";

/// Clears the dependency flag of the stones quest so it can be readied at new game.
pub fn free_quest(y: &mut crate::byml::Byml) -> Res<()> {
    use crate::byml::Node;
    let rows: &mut Vec<Node> = y.root.as_arr_mut().ok_or(Error::Format("start state: the quest table is not a list"))?;
    let row: &mut Node = rows
        .iter_mut()
        .find(|r| matches!(r.get("__RowId"), Some(Node::Str(s)) if s == STONES_QUEST))
        .ok_or(Error::Format("start state: the quest table has no stones quest"))?;
    match row.get("DependFlagName") {
        Some(Node::Str(s)) if !s.is_empty() => {}
        _ => return Err(Error::Format("start state: the stones quest has no dependency, the table is not the unpatched one")),
    }
    row.set("DependFlagName", Node::Str(String::new()));
    Ok(())
}

pub fn stage_free_quest(rf: &mut crate::romfs::Romfs) -> Res<crate::staging::Out> {
    crate::staging::rewrite(rf, QUEST_TABLE, |raw| {
        let mut y: crate::byml::Byml = crate::byml::Byml::parse(raw)?;
        free_quest(&mut y)?;
        y.write()
    })
}

/// The castle story flow. Its forest exit scene marks Hyrule Field as first visited.
pub const FIELD_FLOW_FILE: &str = "Event/EventFlow/Goto_HyruleCastle.bfevfl.zs";
pub const FIELD_FLOW: &str = "Goto_HyruleCastle";
/// The write after Saria's bridge scene that marks the first visit.
pub const FIELD_FIRST_EVENT: &str = "Event218";
/// True from Link's first steps into Hyrule Field until Impa's send off: the field then keeps a
/// fixed daytime light and the clock does not move. Only the send off clears it.
pub const FIELD_FIRST_VISIT: &str = "HyruleField_ChangeENV";

/// Saria's bridge scene still plays with an open Kakariko, and it ran after the new game writes,
/// so it marked the field as first visited and froze the clock on every new game. The write is
/// turned into a clear, and the field starts with its day and night cycle running.
pub fn keep_field_clock(b: &mut crate::bfevfl::Bf) -> Res<()> {
    let f: &mut Flow = b
        .flows
        .iter_mut()
        .find(|x| x.name == FIELD_FLOW)
        .ok_or(Error::Format("start state: no castle story flow"))?;
    let e: &mut Event = f
        .events
        .iter_mut()
        .find(|e| e.name == FIELD_FIRST_EVENT)
        .ok_or(Error::Format("start state: the castle story has no first visit write"))?;
    let c: &mut Cont = e.kind.params_mut().ok_or(Error::Format("start state: the first visit write has no parameters"))?;
    if c.get("GameDataName") != Some(&Val::Str(FIELD_FIRST_VISIT.to_string())) {
        return Err(Error::Format("start state: the first visit write names another flag"));
    }
    let v: &mut Val = c
        .items
        .iter_mut()
        .find(|(k, _)| k == "Value")
        .map(|(_, v)| v)
        .ok_or(Error::Format("start state: the first visit write has no value"))?;
    // The game writes its booleans with the high bit set; anything but zero is true.
    match v {
        Val::Bool(x) if *x != 0 => *v = Val::Bool(0),
        _ => return Err(Error::Format("start state: the first visit write is not the unpatched one")),
    }
    Ok(())
}

pub fn stage_field_clock(rf: &mut crate::romfs::Romfs) -> Res<crate::staging::Out> {
    crate::staging::rewrite(rf, FIELD_FLOW_FILE, |raw| {
        let mut b: crate::bfevfl::Bf = crate::bfevfl::Bf::parse(raw)?;
        keep_field_clock(&mut b)?;
        b.write()
    })
}
/// The quest step enums written directly after the requests. The Door of Time quest depends on
/// the castle quest and refused every request at new game, so its step is written into game
/// data itself, which the challenge manager reads back on the next scene change.
pub const KAKARIKO_ENUMS: [(&str, &str); 3] = [
    ("Step_Goto_HyruleCastle", "Complete"),
    ("Step_DoorOfTime", "Step02"),
    ("Step_Get_StoneOfFire", "KakarikoVillageLetter"),
];
/// The game counts assignable items as they are picked up with their get scene, and the item
/// shortcut on the D-pad stays hidden until that count moves. Items handed out at new game skip
/// the scene, so the count is written as if one had been collected.
pub const KAKARIKO_INTS: [(&str, i32); 1] = [("UI_GetAssignableItemNum", 2)];
/// Frames to wait after a quest's steps before touching the next quest. A quest that depends on
/// another was refused when asked in the same instant the dependency completed.
pub const SETTLE_FRAMES: i32 = 30;
pub const SYSTEM_ACTOR: &str = "EventSystemActor";
pub const WAIT: &str = "EventWait";
/// Flags the skipped story would have left behind. Saria leaves the forest once the castle quest
/// is complete, so her house scenes are marked as played and the house door unlocked; her bridge
/// gift still plays and stays a check. The owl's lesson opens the Threads of Time menu. Talon's
/// castle actor goes once his scene is marked as played; the ranch copies of Talon and Malon
/// appear from the castle quest itself.
pub const KAKARIKO_FLAGS: [(&str, bool); 35] = [
    (GATE_OPEN, true),
    // The item shortcut guide is marked as seen, so the D-pad shortcut HUD no longer waits
    // for the lesson that normally comes with the first assignable item.
    ("ActionGuide_IsEnd.ItemShortcut", true),
    ("UI_IsNeedOpenShortCutActionGuide", false),
    // True marks Link's first visit to Hyrule Field: the castle story sets it as he leaves the
    // forest and clears it after Impa's send off. While it is true the field keeps its fixed
    // daytime look and the clock stands still. The send off is skipped, so the flag stays at its
    // cleared value here, and the forest exit scene no longer sets it (see `keep_field_clock`).
    (FIELD_FIRST_VISIT, false),
    ("HavePlayedEvent.Dm_Kokiri_0045", true),
    ("HavePlayedEvent.Dm_Kokiri_0050", true),
    ("HavePlayedEvent.KokiriForest_StopInFrontOfLinkHouse", true),
    ("HavePlayedEvent.Dm_Kokiri_0050_After_Near", true),
    ("KokiriForest_LinkHouseDoor_Open", true),
    ("UI_IsFinishChallengeMenuOpenEvent", true),
    ("HavePlayedEvent.HyruleField_Goto_HyruleCastle_Kaepora", true),
    ("HavePlayedEvent.Dm_HrlCastle_0020", true),
    // Every cutscene of the skipped story, from leaving the forest to the Kakariko gate. The
    // clock stayed frozen with only the quest steps written, so the scene flags are set too.
    ("HavePlayedEvent.Dm_Kokiri_0155", true),
    ("HavePlayedEvent.Dm_HrlField_0000", true),
    ("HavePlayedEvent.Dm_HrlField_0005", true),
    ("HavePlayedEvent.Dm_HrlField_0010", true),
    ("HavePlayedEvent.Dm_HrlCastle_0030", true),
    ("HavePlayedEvent.Dm_HrlCastle_0035", true),
    ("HavePlayedEvent.Dm_MelodyGet_Zelda", true),
    ("HavePlayedEvent.Dm_HrlField_0040", true),
    ("HavePlayedEvent.Dm_HrlField_0050", true),
    ("HavePlayedEvent.HyruleField_DoorOfTime_SendOff", true),
    ("HavePlayedEvent.Dm_HrlField_0020", true),
    ("HavePlayedEvent.Dm_Kakariko_0010", true),
    // Set by the castle story right before the owl's first field scene, which is skipped.
    (FIELD_ARRIVED, true),
    // Zelda's courtyard talk leaves these markers. The game's clock in Hyrule Field stands
    // still until they are set: the field stayed at six in the morning on every new game, and
    // a save with these set (and nothing else of the castle story) made the sun move again.
    (ZELDA_TALK[0], true),
    (ZELDA_TALK[1], true),
    (ZELDA_TALK[2], true),
    (ZELDA_TALK[3], true),
    (ZELDA_TALK[4], true),
    (ZELDA_TALK[5], true),
    // Navi's monster lesson in the Deku Tree's first room. Until it has played, the D-pad
    // does not call Navi and targeting an enemy gives no information, so the lesson's
    // markers are written as if it had been seen.
    ("HavePlayedEvent.NaviHint_Trial", true),
    ("HaveTalked.Npc_Navi", true),
    ("ActionGuide_IsEnd.Navi", true),
    ("ActionGuide_IsNew.Navi", true),
];
/// The talk markers Zelda's courtyard scene writes. One of them is the clock gate.
pub const ZELDA_TALK: [&str; 6] = [
    "HaveTalked_Special.Npc_Hylia_Zelda_Child_Talk_Event5",
    "HaveTalked_Special.Npc_Hylia_Zelda_Child_Talk_Event29",
    "HaveTalked_Special.Npc_Hylia_Zelda_Child_Talk_Event50",
    "HaveTalked_Special.Npc_Hylia_Zelda_Child_Talk_Event52",
    "HaveTalked_Special.Npc_Hylia_Zelda_Child_Talk_Event54",
    "HaveTalked_Special.Npc_Hylia_Zelda_Child_Sub_InterestTalk_Zelda_Child_Event102",
];
/// Written when Link first arrives in Hyrule Field in the vanilla story. The field clock waits
/// for it.
pub const FIELD_ARRIVED: &str = "HyruleField_ArriveEntrance";
/// The owl also hands out the amiibo pouch in the vanilla game.
pub const KAKARIKO_ITEMS: [&str; 1] = ["Pouch_Amiibo"];
/// The checks the skipped story would have handed out. Their items are given at the start.
pub const KAKARIKO_LOCATIONS: [&str; 2] = ["HC Zeldas Letter", "Song from Impa"];
/// Malon is at the ranch once the castle quest is complete, so her egg cannot be collected.
/// The check stays in the world but never holds anything that matters.
pub const KAKARIKO_EXCLUDED: [&str; 1] = ["HC Malon Egg"];

/// Adds the "IsGet" flag of every given item that has one. A pouch add without the get event
/// may leave that flag unset, and story gates read it.
pub fn add_is_get(ops: &mut Ops, has: impl Fn(&str) -> bool) {
    let more: Vec<(String, bool)> = ops
        .items
        .iter()
        .map(|a| format!("IsGet.{a}"))
        .filter(|n| has(n) && !ops.flags.iter().any(|(f, _)| f == n))
        .map(|n| (n, true))
        .collect();
    ops.flags.extend(more);
}

fn owned(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter().map(|(c, s)| ((*c).to_string(), (*s).to_string())).collect()
}

fn flags_of(v: &[(&str, bool)]) -> Vec<(String, bool)> {
    v.iter().map(|(n, b)| ((*n).to_string(), *b)).collect()
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ops {
    pub steps: Vec<(String, String)>,
    /// Quest step enums written straight into game data, name and value.
    pub enums: Vec<(String, String)>,
    /// Integer game data written directly, name and value.
    pub ints: Vec<(String, i32)>,
    pub flags: Vec<(String, bool)>,
    pub items: Vec<String>,
    /// The clock to set, hour and minute, as the first Hyrule Field scenes do.
    pub time: Option<(i32, i32)>,
}

impl Ops {
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty() && self.enums.is_empty() && self.ints.is_empty() && self.flags.is_empty() && self.items.is_empty() && self.time.is_none()
    }

    pub fn len(&self) -> usize {
        self.steps.len() + self.enums.len() + self.ints.len() + self.flags.len() + self.items.len() + usize::from(self.time.is_some())
    }

    pub fn extend(&mut self, o: Ops) {
        self.steps.extend(o.steps);
        self.enums.extend(o.enums);
        self.ints.extend(o.ints);
        self.flags.extend(o.flags);
        self.items.extend(o.items);
        if o.time.is_some() {
            self.time = o.time;
        }
    }
}

/// The clock the first Hyrule Field scene sets once the owl has spoken.
pub const KAKARIKO_TIME: (i32, i32) = (7, 30);

/// The Water Stone quest after Ruto's Letter was shown: King Zora has stepped aside, which is
/// step seven of the quest. The quest has no dependency, so its requests go through at new game.
pub const FOUNTAIN_STEPS: [(&str, &str); 2] = [("Get_StoneOfWater", "Ready"), ("Get_StoneOfWater", "Step07")];
pub const FOUNTAIN_ENUMS: [(&str, &str); 1] = [("Step_Get_StoneOfWater", "Step07")];
/// The scene of King Zora moving. A wall in front of the passage to the fountain exists until
/// it has played; the king's own pose comes from the quest step.
pub const FOUNTAIN_FLAGS: [(&str, bool); 1] = [("HavePlayedEvent.Dm_Zora_0010", true)];

/// The scenes that play the first time an area is entered and do nothing else: no flag or quest
/// write of their own, and no quest step requested right after them by the arrival flow. The
/// arrival scenes of the Deku Tree, Zora's Domain, the Lost Woods and the Desert Colossus do
/// have a step after them, and the Water Temple's raises the lake pillars, so they stay.
pub const FIRST_VISIT: [&str; 18] = [
    "Dm_DeathMtTrail_0000",
    "Dm_DeathMtCrater_0000",
    "Dm_DeathMtCrater_0000_FromTop",
    "Dm_DodongoCave_0000",
    "Dm_FireTemple_0000",
    "Dm_GanonCastle_0000",
    "Dm_InsideGanonCastle_0000",
    "Dm_GerudoValley_0000",
    "Dm_Gerudo_0000",
    "Dm_Goron_0000",
    "Dm_Graveyard_0000",
    "Dm_HrlCastle_0000",
    "Dm_Kakariko_0000",
    "Dm_LakeHrl_0000",
    "Dm_SacredForest_0000",
    "Dm_TimeTemple_0000",
    "Dm_ZoraFountain_0000",
    "Dm_ZoraRiver_0000",
];

/// Marks the harmless first-visit scenes as played, so a new game never shows them.
pub fn first_visit() -> Ops {
    Ops { flags: FIRST_VISIT.iter().map(|n| (format!("HavePlayedEvent.{n}"), true)).collect(), ..Ops::default() }
}

pub fn fountain() -> Ops {
    Ops { steps: owned(&FOUNTAIN_STEPS), enums: owned(&FOUNTAIN_ENUMS), ints: Vec::new(), flags: flags_of(&FOUNTAIN_FLAGS), items: Vec::new(), time: None }
}

pub fn kakariko(items: &[String]) -> Ops {
    let mut all: Vec<String> = KAKARIKO_ITEMS.iter().map(|s| (*s).to_string()).collect();
    all.extend(items.iter().cloned());
    Ops {
        steps: owned(&KAKARIKO_STEPS),
        enums: owned(&KAKARIKO_ENUMS),
        ints: KAKARIKO_INTS.iter().map(|(n, v)| ((*n).to_string(), *v)).collect(),
        flags: flags_of(&KAKARIKO_FLAGS),
        items: all,
        time: Some(KAKARIKO_TIME),
    }
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

/// The actor and query indexes for `query` on `actor`, adding either when the flow lacks it.
pub fn query_slot(f: &mut Flow, actor: &str, query: &str) -> Res<(u16, u16)> {
    if let Some(i) = f.actors.iter().position(|a| a.name == actor && a.sub.is_empty()) {
        let a: &mut Actor = &mut f.actors[i];
        let j: usize = match a.queries.iter().position(|s| s == query) {
            Some(j) => j,
            None => {
                a.queries.push(query.to_string());
                a.queries.len() - 1
            }
        };
        return Ok((idx(i)?, idx(j)?));
    }
    let base: Actor = f.actors.first().cloned().ok_or(Error::Format("start state: the flow has no actors"))?;
    f.actors.push(Actor {
        name: actor.to_string(),
        sub: String::new(),
        arg: String::new(),
        actions: Vec::new(),
        queries: vec![query.to_string()],
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
    let (ea, ex): (u16, u16) = if ops.enums.is_empty() { (0, 0) } else { slot(f, DATA_ACTOR, SET_ENUM)? };
    let (ia, ix): (u16, u16) = if ops.ints.is_empty() { (0, 0) } else { slot(f, DATA_ACTOR, SET_INT)? };
    let (pa, px): (u16, u16) = if ops.items.is_empty() { (0, 0) } else { slot(f, POUCH_ACTOR, POUCH)? };
    let (wa, wx): (u16, u16) = if ops.time.is_none() { (0, 0) } else { slot(f, WORLD_ACTOR, SET_TIME)? };
    let (sa, sx): (u16, u16) = if ops.steps.is_empty() { (0, 0) } else { slot(f, SYSTEM_ACTOR, WAIT)? };
    let mut chain: Vec<Kind> = Vec::with_capacity(ops.len());
    for (i, (c, s)) in ops.steps.iter().enumerate() {
        let items: Vec<(String, Val)> = vec![
            item("ChallengeName", Val::Str(c.clone())),
            item("StepName", Val::Str(s.clone())),
            item("IsIgnoreCompletedChallenge", flag(true)),
            item("IsIgnoreChallengeProgressUI", flag(true)),
            item("IsWaitEnd", flag(true)),
        ];
        chain.push(Kind::Action { next: NONE, actor: ca, act: cx, params: Some(Cont { items }) });
        if ops.steps.get(i + 1).is_some_and(|(n, _)| n != c) {
            let items: Vec<(String, Val)> = vec![item("Frames", Val::Int(SETTLE_FRAMES)), item("IsWaitEnd", flag(true))];
            chain.push(Kind::Action { next: NONE, actor: sa, act: sx, params: Some(Cont { items }) });
        }
    }
    for (n, v) in &ops.enums {
        let items: Vec<(String, Val)> = vec![item("GameDataName", Val::Str(n.clone())), item("Value", Val::Str(v.clone())), item("Index", Val::Int(-1))];
        chain.push(Kind::Action { next: NONE, actor: ea, act: ex, params: Some(Cont { items }) });
    }
    for (n, v) in &ops.ints {
        let items: Vec<(String, Val)> = vec![item("GameDataName", Val::Str(n.clone())), item("Index", Val::Int(-1)), item("Value", Val::Int(*v))];
        chain.push(Kind::Action { next: NONE, actor: ia, act: ix, params: Some(Cont { items }) });
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
        let n: usize = ops.len() + 2;
        assert_eq!(ops.len(), 6 + 3 + 1 + 35 + 3 + 1);
        let f: &mut Flow = &mut b.flows[0];
        assert_eq!(patch(f, &ops).unwrap(), n);
        assert_eq!(f.events.len(), 3 + n);
        assert_eq!(next_of(&f.events[1]), 3);
        for i in 3..3 + n - 1 {
            assert_eq!(next_of(&f.events[i]), (i + 1) as u16, "event {i}");
        }
        assert_eq!(next_of(&f.events[2 + n]), 2);
        let steps: Vec<Option<&str>> = (3..13).map(|i| text(&f.events[i].kind, "StepName")).collect();
        assert_eq!(steps, vec![Some("Ready"), Some("Complete"), None, Some("Ready"), Some("Step02"), None, Some("Ready"), Some("KakarikoVillageLetter"), None, None]);
        assert_eq!(f.events[5].kind.params().and_then(|c| c.get("Frames")), Some(&Val::Int(SETTLE_FRAMES)));
        assert_eq!(text(&f.events[3].kind, "ChallengeName"), Some("Goto_HyruleCastle"));
        assert_eq!(text(&f.events[12].kind, "GameDataName"), Some("Step_DoorOfTime"));
        assert_eq!(text(&f.events[12].kind, "Value"), Some("Step02"));
        let Kind::Action { actor, act, .. } = &f.events[12].kind else { panic!() };
        assert_eq!(f.actors[*actor as usize].actions[*act as usize], SET_ENUM);
        assert_eq!(text(&f.events[14].kind, "GameDataName"), Some("UI_GetAssignableItemNum"));
        assert_eq!(f.events[14].kind.params().and_then(|c| c.get("Value")), Some(&Val::Int(2)));
        assert_eq!(text(&f.events[15].kind, "GameDataName"), Some(GATE_OPEN));
        assert_eq!(text(&f.events[16].kind, "GameDataName"), Some("ActionGuide_IsEnd.ItemShortcut"));
        assert_eq!(text(&f.events[18].kind, "GameDataName"), Some(FIELD_FIRST_VISIT));
        assert_eq!(f.events[18].kind.params().and_then(|c| c.get("Value")), Some(&Val::Bool(0)));
        assert_eq!(text(&f.events[39].kind, "GameDataName"), Some(FIELD_ARRIVED));
        assert_eq!(text(&f.events[45].kind, "GameDataName"), Some(ZELDA_TALK[5]));
        assert_eq!(text(&f.events[46].kind, "GameDataName"), Some("HavePlayedEvent.NaviHint_Trial"));
        assert_eq!(text(&f.events[50].kind, "Actor"), Some("Pouch_Amiibo"));
        assert_eq!(text(&f.events[1 + n].kind, "Actor"), Some("Item_Melody_ZeldaLullaby"));
        assert_eq!(f.events[2 + n].kind.params().and_then(|c| c.get("TimeHour")), Some(&Val::Int(7)));
        assert!(!ops.flags.iter().any(|(n, _)| n.contains("Dm_Kokiri_0060")));
        assert!(ops.steps.iter().any(|(c, s)| c == "DoorOfTime" && s == "Ready"));
        let names: Vec<&str> = f.actors.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["EventSystemActor", CHALLENGE_ACTOR, DATA_ACTOR, POUCH_ACTOR, WORLD_ACTOR]);
        assert_eq!(f.actors[0].actions, vec!["EventWait".to_string()]);
        assert_eq!(f.actors[2].actions, vec![SET_BOOL.to_string(), SET_ENUM.to_string(), SET_INT.to_string()]);
        assert!(f.actors[2].params.is_some());
        assert!(patch(f, &ops).is_err());
        let bytes: Vec<u8> = b.write().unwrap();
        assert_eq!(Bf::parse(&bytes).unwrap(), b);
    }

    #[test]
    fn is_get_flags_are_added_only_for_items_that_have_one_and_never_twice() {
        let mut ops: Ops = kakariko(&["Item_ZeldaLetter".to_string()]);
        let before: usize = ops.flags.len();
        add_is_get(&mut ops, |n| n == "IsGet.Item_ZeldaLetter" || n == "IsGet.Pouch_Amiibo");
        assert_eq!(ops.flags.len(), before + 2);
        assert!(ops.flags.contains(&("IsGet.Item_ZeldaLetter".to_string(), true)));
        add_is_get(&mut ops, |_| true);
        assert_eq!(ops.flags.len(), before + 2);
    }

    #[test]
    fn the_stones_quest_loses_its_dependency_once() {
        use crate::byml::{Byml, Node};
        let row = |id: &str, dep: &str| {
            Node::Map(vec![
                ("Category".to_string(), Node::U32(1)),
                ("DependFlagName".to_string(), Node::Str(dep.to_string())),
                ("__RowId".to_string(), Node::Str(id.to_string())),
            ])
        };
        let mut y: Byml = Byml { root: Node::Arr(vec![row("Work/Challenge/AllSage.game__challenge__Challenge.gyml", ""), row(STONES_QUEST, "Goto_HyruleCastle_IsCompleted_Exp")]), version: 7, big: false };
        free_quest(&mut y).unwrap();
        let rows: &[Node] = y.root.as_arr().unwrap();
        assert_eq!(rows[1].get("DependFlagName"), Some(&Node::Str(String::new())));
        assert_eq!(rows[0].get("DependFlagName"), Some(&Node::Str(String::new())));
        assert!(free_quest(&mut y).is_err());
        let back: Byml = Byml::parse(&y.write().unwrap()).unwrap();
        assert_eq!(back.root, y.root);
    }

    #[test]
    fn the_first_visit_scenes_become_played_flags() {
        let ops: Ops = first_visit();
        assert_eq!(ops.len(), FIRST_VISIT.len());
        assert!(ops.steps.is_empty() && ops.items.is_empty() && ops.time.is_none());
        assert!(ops.flags.iter().all(|(n, v)| n.starts_with("HavePlayedEvent.Dm_") && *v));
        let mut b: Bf = sample();
        assert_eq!(patch(&mut b.flows[0], &ops).unwrap(), FIRST_VISIT.len());
    }

    #[test]
    fn the_open_fountain_writes_the_water_quest_past_the_letter() {
        let ops: Ops = fountain();
        assert_eq!(ops.len(), 4);
        assert_eq!(ops.steps.last().map(|(c, s)| (c.as_str(), s.as_str())), Some(("Get_StoneOfWater", "Step07")));
        assert_eq!(ops.enums, vec![("Step_Get_StoneOfWater".to_string(), "Step07".to_string())]);
        assert_eq!(ops.flags, vec![("HavePlayedEvent.Dm_Zora_0010".to_string(), true)]);
        let mut b: Bf = sample();
        let n: usize = patch(&mut b.flows[0], &ops).unwrap();
        assert_eq!(n, 4);
        let f: &Flow = &b.flows[0];
        assert_eq!(text(&f.events[3].kind, "StepName"), Some("Ready"));
        assert_eq!(text(&f.events[4].kind, "StepName"), Some("Step07"));
        assert_eq!(text(&f.events[5].kind, "Value"), Some("Step07"));
        assert_eq!(text(&f.events[6].kind, "GameDataName"), Some("HavePlayedEvent.Dm_Zora_0010"));
    }

    #[test]
    fn the_forest_exit_scene_clears_the_first_visit_flag_once() {
        let set = |v: u32| Event {
            name: FIELD_FIRST_EVENT.to_string(),
            kind: Kind::Action {
                next: NONE,
                actor: 0,
                act: 0,
                params: Some(Cont { items: vec![item("Index", Val::Int(-1)), item("GameDataName", Val::Str(FIELD_FIRST_VISIT.to_string())), item("Value", Val::Bool(v))] }),
            },
        };
        let flow = |name: &str, e: Event| Flow {
            name: name.to_string(),
            actors: vec![actor(DATA_ACTOR, &[SET_BOOL])],
            events: vec![e],
            entries: vec![Entry { name: "KokiriForest_ArriveLostWood".to_string(), subs: Vec::new(), vars: Vec::new(), main: 0, pad: 0 }],
        };
        let file = |f: Flow| Bf { ver: [0, 4, 0, 0], bom: 0xFEFF, shift: 3, pad: 0, name: FIELD_FLOW.to_string(), flows: vec![f], tls: Vec::new() };
        let mut b: Bf = file(flow(FIELD_FLOW, set(0x8000_0001)));
        keep_field_clock(&mut b).unwrap();
        assert_eq!(b.flows[0].events[0].kind.params().and_then(|c| c.get("Value")), Some(&Val::Bool(0)));
        assert_eq!(b.flows[0].events[0].kind.params().and_then(|c| c.get("GameDataName")), Some(&Val::Str(FIELD_FIRST_VISIT.to_string())));
        assert!(keep_field_clock(&mut b).is_err());
        let back: Bf = Bf::parse(&b.write().unwrap()).unwrap();
        assert_eq!(back, b);
        assert!(keep_field_clock(&mut file(flow(FLOW, set(1)))).is_err());
        let mut named: Bf = file(flow(FIELD_FLOW, set(1)));
        named.flows[0].events[0].name = "Event217".to_string();
        assert!(keep_field_clock(&mut named).is_err());
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
