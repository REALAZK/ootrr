//! Forces the Temple of Time door open and lets the Master Sword be drawn without the quest.
//!
//! The door's logic module opens it only when the DoorOfTime quest is at or past Step06 or the
//! altar event asks for it; the opened flag is not wired to it. The sword's module only passes
//! the A press on to the draw event while the quest is exactly at Step06. Neither happens in a
//! seed that skips the Ocarina of Time scenes, so both gates are rewritten here.

use std::path::Path as P;

use crate::ainb::Ainb;
use crate::error::{Error, Res};
use crate::romfs::Romfs;
use crate::staging::{self, Out};

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
    Ok(vec![rewrite(rf, MODULE, |a| patch(a).map(|_| ()))?, rewrite(rf, SWORD_MODULE, patch_sword)?])
}

pub fn build(romfs: &P, load: &P, name: &str) -> Res<String> {
    let mut rf: Romfs = Romfs::open(romfs)?;
    let outs: Vec<Out> = stage(&mut rf)?;
    let c: staging::Committed = staging::commit(&mut rf, load, name, &outs, None)?;
    Ok(c.report)
}
