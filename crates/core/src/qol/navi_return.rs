//! Navi offers a trip back when called with the D-pad, an itemless stand-in for the N64
//! rule that a saved game restarts at the dungeon entrance, in Kokiri Forest or in the
//! Temple of Time. Navi's actor names `NaviHint:NaviHint_Field` as its common field hint
//! event, which the game ships as a single do-nothing event. This fills that entry with a
//! check of `Sequence_CurrentBanc`, a yes or no question and a map change. The global hint
//! sequence only listens for the D-pad while an area hint is registered, so it is rerouted
//! to listen always.

use crate::ainb::Ainb;
use crate::bfevfl::{Actor, Bf, Case, Cont, Flow, Kind, Val};
use crate::bfevfl::flow::{Event, NONE};
use crate::error::{Error, Res};
use crate::msbt::{self, Msbt};
use crate::qol::{edit_plain, edit_zs, Have};
use crate::romfs::Romfs;
use crate::sarc;
use crate::staging::Out;

pub const FILE: &str = "Event/EventFlow/NaviHint.bfevfl.zs";
pub const FLOW: &str = "NaviHint";
pub const ENTRY: &str = "NaviHint_Field";
pub const MSG: &str = "EventFlowMsg/NaviHint.msbt";
pub const MSG_ID: &str = "EventFlowMsg/NaviHint";
pub const CHOICES_FROM: &str = "EventFlowMsg/Get_StoneOfForest.msbt";
pub const YES: &str = "9998";
pub const NO: &str = "9999";
pub const CURRENT: &str = "Sequence_CurrentBanc";
pub const PREFIX: &str = "OOTRRNavi";

pub const ASK_DUNGEON: &str = "OOTRR_Return_Dungeon";
pub const ASK_CHILD: &str = "OOTRR_Return_Child";
pub const ASK_ADULT: &str = "OOTRR_Return_Adult";

const VOICE: &str = "{3.0:0f0100}{4.0:0800526573706f6e736500}";

pub const NOTE_DUNGEONS: &str = "Calling Navi with the D-pad inside a dungeon or its boss room offers a trip back to the dungeon entrance. Room tutorial hints from Navi are replaced by the question; enemy hints while targeting stay. Experimental and untested in the game.";
pub const NOTE_EVERYWHERE: &str = "Calling Navi with the D-pad offers a trip back: to the entrance inside a dungeon or its boss room, otherwise to Kokiri Forest as a child or the Temple of Time as an adult. Room tutorial hints from Navi are replaced by the question, enemy hints while targeting stay, and the question is in English in every language. Experimental and untested in the game.";

/// Where Navi can send you from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Off,
    Dungeons,
    Everywhere,
}

/// Dungeon maps and boss rooms, with the dungeon map and start position of the entrance.
pub const DUNGEONS: [(&str, &str, &str); 20] = [
    ("InsideTheDekuTree", "InsideTheDekuTree", "Entrance"),
    ("GohmaStage", "InsideTheDekuTree", "Entrance"),
    ("DodongoCavern", "DodongoCavern", "DodongoCavern_MountainTrailSide"),
    ("DodongoStage", "DodongoCavern", "DodongoCavern_MountainTrailSide"),
    ("InsideJabuJabuBelly", "InsideJabuJabuBelly", "InsideJabuJabuBelly_FromZoraFountain"),
    ("BarinadeStage", "InsideJabuJabuBelly", "InsideJabuJabuBelly_FromZoraFountain"),
    ("ForestTemple", "ForestTemple", "ForestTemple"),
    ("PhantomGanonStage", "ForestTemple", "ForestTemple"),
    ("FireTemple", "FireTemple", "FireTemple"),
    ("VolvagiaStage", "FireTemple", "FireTemple"),
    ("WaterTemple", "WaterTemple", "WaterTemple_FromLakeHylia"),
    ("MorphaStage", "WaterTemple", "WaterTemple_FromLakeHylia"),
    ("ShadowTemple", "ShadowTemple", "Entrance"),
    ("BongoBongoStage", "ShadowTemple", "Entrance"),
    ("SpiritTemple", "SpiritTemple", "SpiritTemple_Entrance"),
    ("TwinrovaStage", "SpiritTemple", "SpiritTemple_Entrance"),
    ("BottomOfTheWell", "BottomOfTheWell", "Well_KakarikoSide"),
    ("IceCavern", "IceCavern", "IceCavern_FromZoraFountain"),
    ("GerudoTrainingGround", "GerudoTrainingGround", "GerudoTrainingGround_FromGerudoFortress"),
    ("InsideGanonCastle", "InsideGanonCastle", "Entrance"),
];

/// Maps where Navi never offers a trip: the final battle, the boss rush, the Chamber of
/// Sages, demo maps and the miniboss arenas, whose dungeon is not known from the map alone.
pub const STAY: [&str; 22] = [
    "GanondorfStage",
    "GanonStage",
    "CrumblingGanonCastle",
    "FinalBattleField",
    "BossRushIntervalRoom",
    "EnemyMuseum",
    "SageRoom",
    "Demo_ShootingLocation_Tapestry_ED",
    "BigOctoStage",
    "DarkLinkStage",
    "DeadHandStage",
    "DinolfosStage",
    "FlareDancerStage",
    "GerudoThiefStage",
    "IronKnuckleBlackStage",
    "IronKnuckleWhiteStage",
    "LizalfosStage",
    "MegStage",
    "NaboruStage",
    "StalfosStage",
    "StalfosWeakStage",
    "WolfosStage",
];

pub const HOME_CHILD: (&str, &str) = ("KokiriForest_Child", "EntrancePos");
pub const HOME_ADULT: (&str, &str) = ("Market_Adult", "LightPrelude");

/// The actor and action or query indexes for `name` on `actor`, adding either when the flow
/// lacks it. A new actor copies the member settings of the flow's first actor.
fn member(f: &mut Flow, actor: &str, name: &str, query: bool) -> Res<(u16, u16)> {
    if let Some(i) = f.actors.iter().position(|a| a.name == actor && a.sub.is_empty()) {
        let a: &mut Actor = &mut f.actors[i];
        let l: &mut Vec<String> = if query { &mut a.queries } else { &mut a.actions };
        let j: usize = match l.iter().position(|s| s == name) {
            Some(j) => j,
            None => {
                l.push(name.to_string());
                l.len() - 1
            }
        };
        return Ok((idx(i)?, idx(j)?));
    }
    let base: Actor = f.actors.first().cloned().ok_or(Error::Format("navi return: the flow has no actors"))?;
    let one: Vec<String> = vec![name.to_string()];
    f.actors.push(Actor {
        name: actor.to_string(),
        sub: String::new(),
        arg: String::new(),
        actions: if query { Vec::new() } else { one.clone() },
        queries: if query { one } else { Vec::new() },
        params: base.params,
        ep: base.ep,
        cut: base.cut,
    });
    Ok((idx(f.actors.len() - 1)?, 0))
}

fn slot(f: &mut Flow, actor: &str, action: &str) -> Res<(u16, u16)> {
    member(f, actor, action, false)
}

fn query_slot(f: &mut Flow, actor: &str, query: &str) -> Res<(u16, u16)> {
    member(f, actor, query, true)
}

pub fn message_id(label: &str) -> String {
    format!("{MSG_ID}:{label}")
}

fn idx(n: usize) -> Res<u16> {
    u16::try_from(n).ok().filter(|&x| x != NONE).ok_or(Error::Format("navi return: too many events in the flow"))
}

fn push(f: &mut Flow, kind: Kind) -> Res<u16> {
    let i: u16 = idx(f.events.len())?;
    f.events.push(Event { name: format!("{PREFIX}{i}"), kind });
    Ok(i)
}

fn cont(items: Vec<(&str, Val)>) -> Cont {
    Cont { items: items.into_iter().map(|(k, v)| (k.to_string(), v)).collect() }
}

fn talk(label: &str) -> Cont {
    let no: Val = Val::Bool(0);
    cont(vec![
        ("IsWaitEnd", Val::Bool(1)),
        ("IsCloseDialog", Val::Bool(1)),
        ("EndDialogOption", Val::Int(0)),
        ("IsAnimeDriven", no.clone()),
        ("IsPlayASAllSlot", no.clone()),
        ("ChoiceLabel3", Val::Int(0)),
        ("ChoiceLabel4", Val::Int(0)),
        ("ChoiceDefaultNo", Val::Int(0)),
        ("ChoiceCancelNo", Val::Int(1)),
        ("IsNotBecomeSpeaker", no.clone()),
        ("ChangeMstxtForActor", no.clone()),
        ("IsAnonymous", no.clone()),
        ("NotPlayLipSyncAnim", no.clone()),
        ("IsOverrideStyle", no.clone()),
        ("IsNotOpenIfSkipedMostRecent", no),
        ("OverrideStyle", Val::Int(1)),
        ("ChoiceNumber", Val::Int(2)),
        ("ChoiceLabel1", Val::Int(YES.parse().unwrap_or(9998))),
        ("ChoiceLabel2", Val::Int(NO.parse().unwrap_or(9999))),
        ("ASName", Val::Str(String::new())),
        ("MessageId", Val::Str(message_id(label))),
        ("MaintainPerformanceState", Val::Int(0)),
    ])
}

fn warp(banc: &str, start: &str) -> Cont {
    cont(vec![
        ("FadePattern", Val::Int(1)),
        ("IsWaitEnd", Val::Bool(1)),
        ("NextPosition", Val::Floats(vec![0.0, 0.0, 0.0])),
        ("NextStartRot", Val::Floats(vec![0.0, 0.0, 0.0])),
        ("BootEventName", Val::Str(String::new())),
        ("IsStartOnEvent", Val::Bool(0)),
        ("NextBanc", Val::Str(banc.to_string())),
        ("NextStartPos", Val::Str(start.to_string())),
        ("EventStarterName", Val::Str(String::new())),
        ("IsItemWarp", Val::Bool(0)),
    ])
}

fn two(yes: u16, no: u16) -> Vec<Case> {
    vec![Case { val: 0, ev: yes }, Case { val: 1, ev: no }]
}

struct Slots {
    talk: (u16, u16),
    choice: (u16, u16),
    warp: (u16, u16),
    map: (u16, u16),
    child: (u16, u16),
}

/// The question, the answer switch and the map change, returning the question.
fn offer(f: &mut Flow, s: &Slots, label: &str, to: (&str, &str), stay: u16) -> Res<u16> {
    let go: u16 = push(f, Kind::Action { next: NONE, actor: s.warp.0, act: s.warp.1, params: Some(warp(to.0, to.1)) })?;
    let pick: u16 = push(f, Kind::Switch { actor: s.choice.0, query: s.choice.1, cases: two(go, stay), params: None })?;
    push(f, Kind::Action { next: pick, actor: s.talk.0, act: s.talk.1, params: Some(talk(label)) })
}

fn is_map(f: &mut Flow, s: &Slots, map: &str, yes: u16, no: u16) -> Res<u16> {
    let c: Cont = cont(vec![("Index", Val::Int(-1)), ("GameDataName", Val::Str(CURRENT.to_string())), ("Value", Val::Str(map.to_string()))]);
    push(f, Kind::Switch { actor: s.map.0, query: s.map.1, cases: two(yes, no), params: Some(c) })
}

fn is_nothing(f: &Flow, i: usize) -> bool {
    let Some(Kind::Action { next, actor, act, .. }) = f.events.get(i).map(|e| &e.kind) else {
        return false;
    };
    let a = f.actors.get(*actor as usize);
    *next == NONE && a.is_some_and(|a| a.name == "Npc_Navi" && a.actions.get(*act as usize).is_some_and(|x| x == "EventDoNothing"))
}

pub fn patch_flow(b: &mut Bf, mode: Mode) -> Res<usize> {
    if mode == Mode::Off {
        return Err(Error::Format("navi return: the option is off"));
    }
    let f: &mut Flow = b.flows.iter_mut().find(|f| f.name == FLOW).ok_or(Error::Format("navi return: the NaviHint flow is not in the file"))?;
    let e: usize = f.entries.iter().position(|e| e.name == ENTRY).ok_or(Error::Format("navi return: the flow has no NaviHint_Field entry"))?;
    let stay: u16 = f.entries[e].main;
    if !is_nothing(f, stay as usize) {
        return Err(Error::Format("navi return: NaviHint_Field is not the do-nothing event, the flow is already edited or differs"));
    }
    let before: usize = f.events.len();
    let s: Slots = Slots {
        talk: slot(f, "Npc_Navi", "EventTalk")?,
        choice: query_slot(f, "GameSystemActor", "EventQueryTalkChoice2")?,
        warp: slot(f, "GameSystemActor", "EventReserveBancChange")?,
        map: query_slot(f, "GameDataActor", "EventQueryIsGameDataString64")?,
        child: query_slot(f, "Player_Event", "EventQueryPlayerIsChild")?,
    };
    let mut tail: u16 = stay;
    if mode == Mode::Everywhere {
        let kid: u16 = offer(f, &s, ASK_CHILD, HOME_CHILD, stay)?;
        let adult: u16 = offer(f, &s, ASK_ADULT, HOME_ADULT, stay)?;
        tail = push(f, Kind::Switch { actor: s.child.0, query: s.child.1, cases: two(kid, adult), params: None })?;
        for m in STAY.iter().rev() {
            tail = is_map(f, &s, m, stay, tail)?;
        }
    }
    let mut asked: Vec<(&str, u16)> = Vec::new();
    for (_, banc, start) in DUNGEONS {
        if !asked.iter().any(|(b, _)| *b == banc) {
            let q: u16 = offer(f, &s, ASK_DUNGEON, (banc, start), stay)?;
            asked.push((banc, q));
        }
    }
    for (map, banc, _) in DUNGEONS.iter().rev() {
        let q: u16 = asked.iter().find(|(b, _)| b == banc).map(|x| x.1).ok_or(Error::Format("navi return: a dungeon lost its question"))?;
        tail = is_map(f, &s, map, q, tail)?;
    }
    f.entries[e].main = tail;
    Ok(f.events.len() - before)
}

pub fn patch(raw: &[u8], mode: Mode) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    patch_flow(&mut b, mode)?;
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(Error::Format("navi return: the written flow does not parse back to the edited model"));
    }
    Ok(out)
}

fn ask(label: &str) -> &'static str {
    match label {
        ASK_DUNGEON => "Should we head back to the entrance?",
        ASK_CHILD => "Should we head back home to the forest?",
        _ => "Should we head back to the Temple of Time?",
    }
}

/// Adds the questions in English and copies the localized "Let's go!" and "Not yet..."
/// answers that the Deku Tree warp uses into the NaviHint texts of one language archive.
pub fn apply_text(raw: &[u8], mode: Mode) -> Res<Vec<u8>> {
    let mut es: Vec<sarc::Entry> = sarc::parse(raw)?;
    let from: Msbt = msbt::parse(&sarc::get(&es, CHOICES_FROM).ok_or(Error::Format("navi return: the language archive has no Get_StoneOfForest texts"))?.data)?;
    let mut m: Msbt = msbt::parse(&sarc::get(&es, MSG).ok_or(Error::Format("navi return: the language archive has no NaviHint texts"))?.data)?;
    let mut labels: Vec<&str> = vec![ASK_DUNGEON];
    if mode == Mode::Everywhere {
        labels.extend([ASK_CHILD, ASK_ADULT]);
    }
    for l in labels {
        m.put_markup(l, &format!("{VOICE}{}", ask(l)))?;
    }
    for c in [YES, NO] {
        let v: Vec<u8> = from.raw(c).ok_or(Error::Format("navi return: the Deku Tree warp has no answer texts"))?.to_vec();
        if m.find(c).is_some() {
            m.set_raw(c, v)?;
        } else {
            m.add_raw(c, v)?;
        }
    }
    sarc::set(&mut es, MSG, m.write()?);
    sarc::write(&es)
}

/// The global sequence that turns a D-pad press into a Navi hint event.
pub const SEQ: &str = "Sequence/NaviHintEvent.module.ainb";
const AREA_QUERY: &str = "SeqQueryIsNaviAreaHint";
const END: &str = "SeqOneShotEnd";
const BOOL: usize = 1;

fn branch(a: &Ainb, i: usize, name: &str) -> Option<usize> {
    let p = a.params.get(i)?;
    p.plugs.iter().position(|x| a.s(x.b) == name)
}

/// The selector that asks whether an area hint is registered, with the plug index of its
/// False branch and the element its True branch leads to.
fn area_selector(a: &Ainb) -> Res<(usize, usize, usize)> {
    let q: Vec<usize> = a.find_els(AREA_QUERY);
    let [q] = q.as_slice() else {
        return Err(Error::Msg(format!("navi return: expected one {AREA_QUERY} in the hint sequence, found {}", q.len())));
    };
    let reads = |i: usize| a.find_in(i, BOOL, "Input").and_then(|g| a.input(BOOL, g)).is_some_and(|x| x.src as i64 == *q as i64);
    let s: Vec<usize> = (0..a.params.len()).filter(|&i| a.el_type(i) == 7 && reads(i)).collect();
    let [s] = s.as_slice() else {
        return Err(Error::Format("navi return: the area hint selector of the hint sequence was not found"));
    };
    let f: usize = branch(a, *s, "False").ok_or(Error::Format("navi return: the area hint selector has no False branch"))?;
    let t: usize = branch(a, *s, "True").ok_or(Error::Format("navi return: the area hint selector has no True branch"))?;
    Ok((*s, f, a.params[*s].plugs[t].a as usize))
}

/// Sends the "no area hint" case into the same D-pad check the area hint case uses, and makes
/// that check request `NaviHint_Field` instead of the area hint name. The area hint name is
/// not cleared once a hint is played, so a bare reroute replays the last tutorial hint (the
/// Deku Tree's `NaviHint_Trial`) on every press. Area tutorial hints are replaced by the
/// question; lock-on enemy hints are untouched.
pub fn patch_seq(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    let (s, f, t): (usize, usize, usize) = area_selector(&a)?;
    let now: usize = a.params[s].plugs[f].a as usize;
    if a.el_name(now) != END {
        return Err(Error::Format("navi return: the no area hint branch does not end, the sequence is already edited or differs"));
    }
    let tf: usize = branch(&a, t, "False").ok_or(Error::Format("navi return: the area hint path has no False branch"))?;
    let press: u32 = a.params[t].plugs[tf].a;
    if a.el_type(press as usize) != 7 {
        return Err(Error::Format("navi return: the area hint path does not lead to the button check"));
    }
    a.params[s].plugs[f].a = press;
    let (r, g): (usize, usize) = area_request(&a)?;
    force_str(&mut a, r, g, ENTRY)?;
    let out: Vec<u8> = a.write()?;
    let b: Ainb = Ainb::parse(&out)?;
    let (s2, f2, _) = area_selector(&b)?;
    if b.params[s2].plugs[f2].a != press || b.in_str(r, "EventName") != Some(ENTRY) || !crate::ainb::audit(&b).is_empty() {
        return Err(Error::Format("navi return: the written hint sequence does not read back"));
    }
    Ok(out)
}

const STR: usize = 3;
const REQUEST: &str = "SeqOneShotRequestEvent";
const FILE_NAME: &str = "SeqQueryExtractFileName";
const AREA_NAME: &str = "SeqQueryGetNaviAreaHintName";

fn src_of(a: &Ainb, i: usize, name: &str) -> Option<usize> {
    let g: usize = a.find_in(i, STR, name)?;
    let x = a.input(STR, g)?;
    usize::try_from(x.src).ok()
}

/// The request that plays the registered area hint, found as the request whose event name
/// is the file name of the area hint name, with the index of its EventName input.
fn area_request(a: &Ainb) -> Res<(usize, usize)> {
    let hits: Vec<usize> = a
        .find_els(REQUEST)
        .into_iter()
        .filter(|&i| src_of(a, i, "EventName").is_some_and(|e| a.el_name(e) == FILE_NAME && src_of(a, e, "Path").is_some_and(|p| a.el_name(p) == AREA_NAME)))
        .collect();
    let [r] = hits.as_slice() else {
        return Err(Error::Msg(format!("navi return: expected one area hint request in the hint sequence, found {}", hits.len())));
    };
    let g: usize = a.find_in(*r, STR, "EventName").ok_or(Error::Format("navi return: the area hint request has no EventName"))?;
    Ok((*r, g))
}

/// Turns string input `g` of element `i` into the constant `v`, cutting the link that fed it
/// from both the input record and the element's plug list.
fn force_str(a: &mut Ainb, i: usize, g: usize, v: &str) -> Res<()> {
    let x = a.input(STR, g).ok_or(Error::Format("navi return: input out of range"))?;
    let src: u32 = u32::try_from(x.src).map_err(|_| Error::Format("navi return: the input is already a constant"))?;
    let name: String = a.s(x.name).to_string();
    let mut hit: Option<(usize, usize)> = None;
    for k in 0..10 {
        if let Some(pos) = a.params[i].group(k).iter().position(|p| p.a == src && a.s(p.b) == name) {
            hit = Some((k, pos));
            break;
        }
    }
    let (k, pos): (usize, usize) = hit.ok_or(Error::Format("navi return: the link into the input has no plug"))?;
    let p = &mut a.params[i];
    p.plugs.remove(p.pl[k][1] as usize + pos);
    p.pl[k][0] -= 1;
    for j in k + 1..p.pl.len() {
        p.pl[j][1] -= 1;
    }
    let off: u32 = a.pool.add(v);
    let o: usize = g * 16;
    let rec: &mut [u8] = a.ins[STR].get_mut(o..o + 16).ok_or(Error::Format("navi return: input out of range"))?;
    rec[4..6].copy_from_slice(&(-1i16).to_le_bytes());
    rec[6..8].copy_from_slice(&0i16.to_le_bytes());
    rec[8..12].copy_from_slice(&0x80_0000u32.to_le_bytes());
    rec[12..16].copy_from_slice(&off.to_le_bytes());
    Ok(())
}

pub const LIST: &str = crate::chest_slot::LIST;

/// New game values that mark Navi's monster lesson in the Deku Tree's first room as done.
/// Until it has played, the hint sequence ignores the D-pad, so the question could not be
/// asked before the Deku Tree. Marking it played also keeps that lesson from showing.
pub const EARLY: [&str; 4] = ["HavePlayedEvent.NaviHint_Trial", "HaveTalked.Npc_Navi", "ActionGuide_IsEnd.Navi", "ActionGuide_IsNew.Navi"];

fn early_navi(raw: &[u8]) -> Res<Vec<u8>> {
    let mut y: crate::byml::Byml = crate::byml::Byml::parse(raw)?;
    let edits: Vec<crate::start_defaults::Edit> = EARLY.iter().map(|n| crate::start_defaults::Edit::Bool(n.to_string(), true)).collect();
    crate::start_defaults::apply(&mut y, &edits)?;
    y.write()
}

pub fn stage(rf: &mut Romfs, mode: Mode, have: Have) -> Res<Vec<Out>> {
    if mode == Mode::Off {
        return Ok(Vec::new());
    }
    let mut outs: Vec<Out> = vec![edit_zs(rf, have, FILE, |raw| patch(raw, mode))?, edit_plain(rf, have, SEQ, patch_seq)?, edit_zs(rf, have, LIST, early_navi)?];
    for l in crate::hints::languages(rf)? {
        outs.push(edit_zs(rf, have, &l, |raw| apply_text(raw, mode))?);
    }
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::Actor;
    use crate::bfevfl::flow::Entry;

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

    fn sample() -> Bf {
        let f: Flow = Flow {
            name: FLOW.to_string(),
            actors: vec![actor("Npc_Navi", &["EventTalk", "EventDoNothing"]), actor("GameDataActor", &[]), actor("Player_Event", &[]), actor("GameSystemActor", &["EventWait"])],
            events: vec![
                Event { name: "Event0".to_string(), kind: Kind::Action { next: NONE, actor: 0, act: 0, params: None } },
                Event { name: "Event5".to_string(), kind: Kind::Action { next: NONE, actor: 0, act: 1, params: None } },
            ],
            entries: vec![Entry { name: ENTRY.to_string(), subs: Vec::new(), vars: Vec::new(), main: 1, pad: 0 }],
        };
        Bf { ver: [0, 4, 0, 0], bom: 0xFEFF, shift: 3, pad: 0, name: FLOW.to_string(), flows: vec![f], tls: Vec::new() }
    }

    fn query_name(f: &Flow, actor: u16, q: u16) -> String {
        format!("{}.{}", f.actors[actor as usize].name, f.actors[actor as usize].queries[q as usize])
    }

    /// Walks the patched entry for one map and age, answering the question with `yes`.
    fn walk(f: &Flow, map: &str, child: bool, yes: bool) -> Option<(String, String)> {
        let mut i: u16 = f.entries[0].main;
        for _ in 0..500 {
            match &f.events[i as usize].kind {
                Kind::Switch { actor, query, cases, params } => {
                    let q: String = query_name(f, *actor, *query);
                    let truth: bool = match q.as_str() {
                        "GameDataActor.EventQueryIsGameDataString64" => {
                            let c: &Cont = params.as_ref().unwrap();
                            assert_eq!(c.get("GameDataName"), Some(&Val::Str(CURRENT.to_string())));
                            c.get("Value") == Some(&Val::Str(map.to_string()))
                        }
                        "Player_Event.EventQueryPlayerIsChild" => child,
                        "GameSystemActor.EventQueryTalkChoice2" => yes,
                        x => panic!("unexpected query {x}"),
                    };
                    i = cases.iter().find(|c| c.val == if truth { 0 } else { 1 }).unwrap().ev;
                }
                Kind::Action { next, actor, act, params } => {
                    let a: &str = &f.actors[*actor as usize].actions[*act as usize];
                    if a == "EventReserveBancChange" {
                        let c: &Cont = params.as_ref().unwrap();
                        let s = |k: &str| match c.get(k) {
                            Some(Val::Str(x)) => x.clone(),
                            _ => panic!("no {k}"),
                        };
                        assert_eq!(*next, NONE);
                        return Some((s("NextBanc"), s("NextStartPos")));
                    }
                    if *next == NONE {
                        return None;
                    }
                    i = *next;
                }
                k => panic!("unexpected event {k:?}"),
            }
        }
        panic!("the walk does not end");
    }

    #[test]
    fn every_dungeon_map_and_boss_room_returns_to_its_entrance_after_a_yes() {
        for mode in [Mode::Dungeons, Mode::Everywhere] {
            let mut b: Bf = sample();
            patch_flow(&mut b, mode).unwrap();
            let f: &Flow = &b.flows[0];
            for (map, banc, start) in DUNGEONS {
                for child in [true, false] {
                    assert_eq!(walk(f, map, child, true), Some((banc.to_string(), start.to_string())), "{map}");
                    assert_eq!(walk(f, map, child, false), None, "{map}");
                }
            }
        }
    }

    #[test]
    fn the_overworld_goes_home_by_age_only_when_everywhere() {
        let mut b: Bf = sample();
        patch_flow(&mut b, Mode::Dungeons).unwrap();
        assert_eq!(walk(&b.flows[0], "HyruleField_Child", true, true), None);
        let mut b: Bf = sample();
        patch_flow(&mut b, Mode::Everywhere).unwrap();
        let f: &Flow = &b.flows[0];
        let home = |p: (&str, &str)| Some((p.0.to_string(), p.1.to_string()));
        assert_eq!(walk(f, "HyruleField_Child", true, true), home(HOME_CHILD));
        assert_eq!(walk(f, "GoronCity_Adult", false, true), home(HOME_ADULT));
        assert_eq!(walk(f, "Market_Adult", false, false), None);
        for m in STAY {
            assert_eq!(walk(f, m, true, true), None, "{m}");
            assert_eq!(walk(f, m, false, true), None, "{m}");
        }
    }

    #[test]
    fn the_patch_refuses_a_second_run_and_the_off_mode() {
        let mut b: Bf = sample();
        assert!(patch_flow(&mut b, Mode::Off).is_err());
        patch_flow(&mut b, Mode::Dungeons).unwrap();
        assert!(patch_flow(&mut b, Mode::Dungeons).is_err());
    }

    #[test]
    fn the_map_lists_do_not_overlap_and_dungeon_questions_are_shared() {
        for (m, _, _) in DUNGEONS {
            assert!(!STAY.contains(&m), "{m}");
        }
        let mut b: Bf = sample();
        let n: usize = patch_flow(&mut b, Mode::Dungeons).unwrap();
        assert_eq!(n, DUNGEONS.len() + 12 * 3);
    }

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("C:/o/nup dump/updates/131072/romfs"));
        if p.is_dir() {
            Some(p)
        } else {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            None
        }
    }

    #[test]
    fn the_dump_flow_and_every_language_take_the_edit() {
        let Some(root) = dump() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let none = |_: &str| -> Option<Vec<u8>> { None };
        let outs: Vec<Out> = stage(&mut rf, Mode::Everywhere, &none).unwrap();
        assert!(outs.len() > 1);
        assert_eq!(outs[1].path, SEQ);
        let raw: Vec<u8> = rf.read(SEQ).unwrap();
        let a: Ainb = Ainb::parse(&raw).unwrap();
        assert_eq!(a.write().unwrap(), raw);
        let (s, f, t) = area_selector(&a).unwrap();
        assert_eq!((s, a.params[s].plugs[f].a, t), (9, 0, 21));
        let e: Ainb = Ainb::parse(&outs[1].z).unwrap();
        assert_eq!(e.params[9].plugs[f].a, 17);
        assert_eq!(area_request(&a).unwrap().0, 19);
        assert_eq!(e.in_str(19, "EventName"), Some(ENTRY));
        assert_eq!(e.in_str(7, "EventName"), a.in_str(7, "EventName"));
        assert!(crate::ainb::audit(&e).is_empty());
        assert_eq!(e.params[19].plugs.len() + 1, a.params[19].plugs.len());
        assert!(patch_seq(&outs[1].z).is_err());
        let b: Bf = Bf::parse(&rf.zs.decompress(&outs[0].z).unwrap()).unwrap();
        let f: &Flow = b.flows.iter().find(|f| f.name == FLOW).unwrap();
        let e: &Entry = f.entries.iter().find(|e| e.name == ENTRY).unwrap();
        assert!(!is_nothing(f, e.main as usize));
        assert_eq!(outs[2].path, LIST);
        let y: crate::byml::Byml = crate::byml::Byml::parse(&rf.zs.decompress(&outs[2].z).unwrap()).unwrap();
        let mut again: crate::byml::Byml = y.clone();
        let edits: Vec<crate::start_defaults::Edit> = EARLY.iter().map(|n| crate::start_defaults::Edit::Bool(n.to_string(), true)).collect();
        assert_eq!(crate::start_defaults::apply(&mut again, &edits).unwrap(), 0);
        for o in &outs[3..] {
            let es: Vec<sarc::Entry> = sarc::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap();
            let m: Msbt = msbt::parse(&sarc::get(&es, MSG).unwrap().data).unwrap();
            for l in [ASK_DUNGEON, ASK_CHILD, ASK_ADULT, YES, NO] {
                assert!(m.find(l).is_some(), "{} {l}", o.path);
            }
        }
    }
}
