//! Forces the Temple of Time door open and lets the Master Sword be drawn without the quest.
//!
//! The door's logic module opens it only when the DoorOfTime quest is at or past Step06 or the
//! altar event asks for it; the opened flag is not wired to it. The sword's module only passes
//! the A press on to the draw event while the quest is exactly at Step06. Neither happens in a
//! seed that skips the Ocarina of Time scenes, so both gates are rewritten here.

use std::path::Path as P;

use crate::ainb::Ainb;
use crate::bfevfl::{Bf, Case, Cont, Event, Flow, Kind, Val};
use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::romfs::Romfs;
use crate::staging::{self, Out};

/// The flow that sends adult Link back to childhood from the pedestal. After its equipment swap
/// the child can be left holding the Master Sword, which only an adult can wield.
pub const RETURN_FILE: &str = "Event/EventFlow/Market_Adult_Field_AdultToChild.bfevfl.zs";
pub const RETURN_FLOW: &str = "Market_Adult_Field_AdultToChild";
pub const SWAP_ENTRY: &str = "ReplaceEquipmentForEraChange";
pub const RETURN_PREFIX: &str = "OOTRRReturn";
const DATA_ACTOR: &str = "GameDataActor";
const SET_STR: &str = "EventTriggerSetGameDataString64";
const IS_STR: &str = "EventQueryIsGameDataString64";
const POUCH_ACTOR: &str = "PouchSystemActor";
const IN_POUCH: &str = "EventQueryIsExistInPouch";
const EQUIP: &str = "PlayerStatus.EquipWeapon";
const MASTER: &str = "Weapon_Sword_Master";
const KOKIRI: &str = "Weapon_Sword_Kokiri";
const NONE: u16 = 0xFFFF;

/// Sheik stands on the temple's centre line right next to the pedestal's sword, between the
/// player and the prompt. She is moved sideways by this much so the prompt is easy to reach.
pub const SHEIK_FILE: &str = "Banc/Market/Market_Adult_Static.bcett.byml.zs";
pub const SHEIK_HASH: u64 = 2669233513522124871;
pub const SHEIK_SHIFT: f32 = -3.0;

const BOOL: usize = 1;

pub const MODULE: &str = "Logic/Market_Common_f252.logic.module.ainb";
pub const GATE: &str = "ActorLogicGateDoor";
pub const DOOR: &str = "Obj_TimeTemple_Door_A_01_cb9f";
pub const INPUTS: [&str; 2] = ["isOpen", "IsSkipAnim"];

pub const SWORD_MODULE: &str = "Logic/Market_Child_Prefab_e700.logic.module.ainb";
pub const REQUEST: &str = "ActorLogicEventRequestActor";
pub const DRAW: &str = "EventRequestActor_7fe2";
pub const STEP_CHECK: &str = "HoldChallengeCurrentStepCheckOrder";
pub const ATTENTION: &str = "ActorLogicAttentionActor";
pub const SWORD: &str = "Obj_TimeTemple_MasterSword_A_01_e482";
pub const CHALLENGE: &str = "Work/Challenge/DoorOfTime.game__challenge__Challenge.gyml";
pub const STEP: &str = "Step06";

/// The adult Temple of Time: the pedestal's sword, which sends you back to childhood, is
/// deleted until the Forest Sage is awakened. The randomizer lets you travel back at any time.
pub const PEDESTAL_MODULE: &str = "Logic/Market_Adult_Prefab_89da.logic.module.ainb";
pub const ADULT_SWORD: &str = "Obj_TimeTemple_MasterSword_A_01_b347";

pub fn door_element(a: &Ainb) -> Option<usize> {
    a.find_els(GATE).into_iter().find(|&i| a.prop_str(i, "InstanceName") == Some(DOOR))
}

pub fn patch(a: &mut Ainb) -> Res<usize> {
    let i: usize = door_element(a).ok_or(Error::Format("door open: the logic module has no gate door for the Temple of Time door"))?;
    let mut cut: usize = 0;
    for name in INPUTS {
        if a.force_bool(i, name, true)? {
            cut += 1;
        }
    }
    if cut == 0 {
        return Err(Error::Format("door open: the door inputs were already constants, the module is not the unpatched one"));
    }
    Ok(cut)
}

/// One entry of the multi-input link table: the element it reads and that element's output index.
pub fn multi_entry(a: &Ainb, j: usize) -> Option<(u16, u16)> {
    let e: &[u8] = a.multi.get(8 * j..8 * j + 4)?;
    Some((u16::from_le_bytes([e[0], e[1]]), u16::from_le_bytes([e[2], e[3]])))
}

fn named(a: &Ainb, kind: &str, instance: &str) -> Option<usize> {
    a.find_els(kind).into_iter().find(|&i| a.prop_str(i, "InstanceName") == Some(instance))
}

/// The sword's draw request is `AND(step == Step06, A pressed)`. This points the step input at
/// the press as well, so the request is `AND(A pressed, A pressed)` and the quest no longer matters.
pub fn patch_sword(a: &mut Ainb) -> Res<()> {
    let req: usize = named(a, REQUEST, DRAW).ok_or(Error::Format("door open: the sword module has no draw request actor"))?;
    let g: usize = a.find_in(req, BOOL, "EventRequestTrigger").ok_or(Error::Format("door open: the draw request has no trigger input"))?;
    let x = a.input(BOOL, g).ok_or(Error::Format("door open: input out of range"))?;
    if x.src < 0 {
        return Err(Error::Format("door open: the draw trigger is not linked to a gate"));
    }
    let and: usize = x.src as usize;
    let g2: usize = a.find_in(and, BOOL, "BoolMulti").ok_or(Error::Format("door open: the draw gate has no multi input"))?;
    let m = a.input(BOOL, g2).ok_or(Error::Format("door open: input out of range"))?;
    if m.src > -100 || m.plug < 1 {
        return Err(Error::Format("door open: the draw gate input is not a multi link"));
    }
    let start: usize = (-100 - i32::from(m.src)) as usize;
    let count: usize = m.plug as usize;
    let mut step: Option<usize> = None;
    let mut press: Option<usize> = None;
    for j in start..start + count {
        let (n, _) = multi_entry(a, j).ok_or(Error::Format("door open: the multi link table is too short"))?;
        let n: usize = n as usize;
        if n >= a.els.len() {
            return Err(Error::Format("door open: the multi link names a missing element"));
        }
        if a.el_name(n) == STEP_CHECK && a.in_str(n, "ChallengeName") == Some(CHALLENGE) && a.in_str(n, "ChallengeStepName") == Some(STEP) {
            step = Some(j);
        }
        if a.el_name(n) == ATTENTION && a.prop_str(n, "InstanceName") == Some(SWORD) {
            press = Some(j);
        }
    }
    let (s, p): (usize, usize) = match (step, press) {
        (Some(s), Some(p)) => (s, p),
        _ => return Err(Error::Format("door open: the draw gate does not pair the Step06 check with the sword press, the module is not the unpatched one")),
    };
    let from: [u8; 4] = a.multi[8 * p..8 * p + 4].try_into().map_err(|_| Error::Format("door open: bad multi entry"))?;
    a.multi[8 * s..8 * s + 4].copy_from_slice(&from);
    Ok(())
}

/// Keeps the adult pedestal's sword alive whatever the Forest Sage quest says.
pub fn patch_pedestal(a: &mut Ainb) -> Res<()> {
    let i: usize = named(a, ATTENTION, ADULT_SWORD).ok_or(Error::Format("door open: the adult module has no pedestal sword"))?;
    if !a.force_bool(i, "Logic_Delete", false)? {
        return Err(Error::Format("door open: the pedestal sword was already a constant, the module is not the unpatched one"));
    }
    Ok(())
}

fn item(k: &str, v: Val) -> (String, Val) {
    (k.to_string(), v)
}

fn fidx(n: usize) -> Res<u16> {
    if n >= NONE as usize {
        return Err(Error::Format("door open: too many events in the flow"));
    }
    Ok(n as u16)
}

/// After the era change's equipment swap: if the child ended up with the Master Sword equipped,
/// equip the Kokiri Sword when the pouch has it, or nothing.
pub fn patch_return(b: &mut Bf) -> Res<usize> {
    let f: &mut Flow = b.flows.iter_mut().find(|x| x.name == RETURN_FLOW).ok_or(Error::Format("door open: no adult to child flow"))?;
    if f.events.iter().any(|e| e.name.starts_with(RETURN_PREFIX)) {
        return Err(Error::Format("door open: the adult to child flow already carries the sword patch"));
    }
    let swaps: Vec<usize> = (0..f.events.len()).filter(|&i| matches!(&f.events[i].kind, Kind::Sub { entry, .. } if entry == SWAP_ENTRY)).collect();
    let [at] = swaps.as_slice() else {
        return Err(Error::Msg(format!("door open: expected one {SWAP_ENTRY} call in {RETURN_FLOW}, found {}", swaps.len())));
    };
    let Kind::Sub { next: old, .. } = f.events[*at].kind.clone() else { unreachable!() };
    let (da, dq): (u16, u16) = crate::start_state::query_slot(f, DATA_ACTOR, IS_STR)?;
    let (da2, dx): (u16, u16) = crate::start_state::slot(f, DATA_ACTOR, SET_STR)?;
    let (pa, pq): (u16, u16) = crate::start_state::query_slot(f, POUCH_ACTOR, IN_POUCH)?;
    let base: usize = f.events.len();
    let (s1, s2, a1, a2): (u16, u16, u16, u16) = (fidx(base)?, fidx(base + 1)?, fidx(base + 2)?, fidx(base + 3)?);
    let equip = |v: &str| Some(Cont { items: vec![item("GameDataName", Val::Str(EQUIP.to_string())), item("Value", Val::Str(v.to_string())), item("Index", Val::Int(-1))] });
    f.events.push(Event {
        name: format!("{RETURN_PREFIX}{base}"),
        kind: Kind::Switch { actor: da, query: dq, cases: vec![Case { val: 0, ev: s2 }, Case { val: 1, ev: old }], params: equip(MASTER) },
    });
    f.events.push(Event {
        name: format!("{RETURN_PREFIX}{}", base + 1),
        kind: Kind::Switch {
            actor: pa,
            query: pq,
            cases: vec![Case { val: 0, ev: a1 }, Case { val: 1, ev: a2 }],
            params: Some(Cont { items: vec![item("Actor", Val::Str(KOKIRI.to_string())), item("Num", Val::Int(1))] }),
        },
    });
    f.events.push(Event { name: format!("{RETURN_PREFIX}{}", base + 2), kind: Kind::Action { next: old, actor: da2, act: dx, params: equip(KOKIRI) } });
    f.events.push(Event { name: format!("{RETURN_PREFIX}{}", base + 3), kind: Kind::Action { next: old, actor: da2, act: dx, params: equip("") } });
    if let Kind::Sub { next, .. } = &mut f.events[*at].kind {
        *next = s1;
    }
    Ok(4)
}

/// Moves Sheik sideways off the temple's centre line.
pub fn patch_sheik(y: &mut Byml) -> Res<()> {
    let actors: &mut Vec<Node> = y.root.get_mut("Actors").and_then(|n| n.as_arr_mut()).ok_or(Error::Format("door open: the adult Market map has no actors"))?;
    let a: &mut Node = actors
        .iter_mut()
        .find(|a| matches!(a.get("Hash"), Some(Node::U64(h)) if *h == SHEIK_HASH))
        .ok_or(Error::Format("door open: the adult Market map has no Sheik"))?;
    let t: &mut Vec<Node> = a.get_mut("Translate").and_then(|n| n.as_arr_mut()).ok_or(Error::Format("door open: Sheik has no position"))?;
    let Some(Node::F32(z)) = t.get(2).cloned() else {
        return Err(Error::Format("door open: Sheik's position is not three floats"));
    };
    if (z - -34.0).abs() > 0.01 {
        return Err(Error::Format("door open: Sheik is not at her vanilla spot, the map is not the unpatched one"));
    }
    t[2] = Node::F32(z + SHEIK_SHIFT);
    Ok(())
}

pub fn stage_sheik(rf: &mut Romfs) -> Res<Out> {
    let raw: Vec<u8> = staging::banc_raw(rf, SHEIK_FILE, None)?;
    let mut y: Byml = Byml::parse(&raw)?;
    patch_sheik(&mut y)?;
    let out: Vec<u8> = y.write()?;
    staging::banc_out(rf, SHEIK_FILE, raw.len(), &out)
}

fn rewrite(rf: &mut Romfs, path: &str, f: impl FnOnce(&mut Ainb) -> Res<()>) -> Res<Out> {
    let raw: Vec<u8> = rf.read(path)?;
    let mut a: Ainb = Ainb::parse(&raw)?;
    f(&mut a)?;
    let out: Vec<u8> = a.write()?;
    if Ainb::parse(&out)?.write()? != out {
        return Err(Error::Format("door open: the patched module does not round trip"));
    }
    Ok(Out { path: path.to_string(), old: raw.len(), new: out.len(), z: out })
}

pub fn stage(rf: &mut Romfs) -> Res<Vec<Out>> {
    let back: Out = staging::rewrite(rf, RETURN_FILE, |raw| {
        let mut b: Bf = Bf::parse(raw)?;
        patch_return(&mut b)?;
        let out: Vec<u8> = b.write()?;
        if Bf::parse(&out)? != b {
            return Err(Error::Format("door open: the patched flow does not round trip"));
        }
        Ok(out)
    })?;
    Ok(vec![
        rewrite(rf, MODULE, |a| patch(a).map(|_| ()))?,
        rewrite(rf, SWORD_MODULE, patch_sword)?,
        rewrite(rf, PEDESTAL_MODULE, patch_pedestal)?,
        back,
        stage_sheik(rf)?,
    ])
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
            params: Some(Cont { items: vec![("EventMemberCreateMethod".to_string(), Val::Int(0))] }),
            ep: 0xFFFF,
            cut: 1,
        }
    }

    fn sample() -> Bf {
        let events: Vec<Event> = vec![
            Event { name: "E0".to_string(), kind: Kind::Action { next: 1, actor: 0, act: 0, params: None } },
            Event { name: "E1".to_string(), kind: Kind::Sub { next: 2, params: None, flow: SWAP_ENTRY.to_string(), entry: SWAP_ENTRY.to_string() } },
            Event { name: "E2".to_string(), kind: Kind::Action { next: NONE, actor: 1, act: 0, params: None } },
        ];
        let f: Flow = Flow {
            name: RETURN_FLOW.to_string(),
            actors: vec![actor("EventSystemActor", &["EventTransition"], &[]), actor(DATA_ACTOR, &["EventTriggerSetGameDataBool"], &["EventQueryIsGameDataBool"])],
            events,
            entries: vec![Entry { name: RETURN_FLOW.to_string(), subs: Vec::new(), vars: Vec::new(), main: 0, pad: 0 }],
        };
        Bf { ver: [0, 4, 0, 0], bom: 0xFEFF, shift: 3, pad: 0, name: RETURN_FLOW.to_string(), flows: vec![f], tls: Vec::new() }
    }

    #[test]
    fn the_return_patch_reequips_the_kokiri_sword_or_nothing_after_the_swap() {
        let mut b: Bf = sample();
        assert_eq!(patch_return(&mut b).unwrap(), 4);
        let f: &Flow = &b.flows[0];
        let Kind::Sub { next, .. } = f.events[1].kind else { panic!() };
        assert_eq!(next, 3);
        let Kind::Switch { actor, query, cases, .. } = &f.events[3].kind else { panic!() };
        assert_eq!(f.actors[*actor as usize].queries[*query as usize], IS_STR);
        assert_eq!((cases[0].ev, cases[1].ev), (4, 2));
        let Kind::Switch { actor, query, cases, .. } = &f.events[4].kind else { panic!() };
        assert_eq!(f.actors[*actor as usize].name, POUCH_ACTOR);
        assert_eq!(f.actors[*actor as usize].queries[*query as usize], IN_POUCH);
        assert_eq!((cases[0].ev, cases[1].ev), (5, 6));
        for (i, want) in [(5, KOKIRI), (6, "")] {
            let Kind::Action { next, actor, act, params } = &f.events[i].kind else { panic!() };
            assert_eq!(*next, 2);
            assert_eq!(f.actors[*actor as usize].actions[*act as usize], SET_STR);
            assert_eq!(params.as_ref().unwrap().get("Value"), Some(&Val::Str(want.to_string())));
        }
        assert!(patch_return(&mut b).is_err());
        let bytes: Vec<u8> = b.write().unwrap();
        assert_eq!(Bf::parse(&bytes).unwrap(), b);
    }

    #[test]
    fn sheik_moves_sideways_only_from_her_vanilla_spot() {
        let sheik = |z: f32| {
            Node::Map(vec![
                ("Gyaml".to_string(), Node::Str("Npc_Hylia_Sheik_Adult".to_string())),
                ("Hash".to_string(), Node::U64(SHEIK_HASH)),
                ("Translate".to_string(), Node::Arr(vec![Node::F32(139.0), Node::F32(106.35), Node::F32(z)])),
            ])
        };
        let mut y: Byml = Byml { root: Node::Map(vec![("Actors".to_string(), Node::Arr(vec![sheik(-34.0)]))]), version: 7, big: false };
        patch_sheik(&mut y).unwrap();
        let t: &[Node] = y.root.get("Actors").unwrap().as_arr().unwrap()[0].get("Translate").unwrap().as_arr().unwrap();
        assert_eq!(t[2], Node::F32(-37.0));
        assert!(patch_sheik(&mut y).is_err());
        let back: Byml = Byml::parse(&y.write().unwrap()).unwrap();
        assert_eq!(back.root, y.root);
    }
}

pub fn build(romfs: &P, load: &P, name: &str) -> Res<String> {
    let mut rf: Romfs = Romfs::open(romfs)?;
    let outs: Vec<Out> = stage(&mut rf)?;
    let c: staging::Committed = staging::commit(&mut rf, load, name, &outs, None)?;
    Ok(c.report)
}
