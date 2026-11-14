//! Quest skips that edit game files: the Gerudo Valley bridge and the one-sale bean salesman.
//! The Epona race, the mask quest and the planted beans are flag list edits
//! (data/start_presets.json).

use crate::ainb::Ainb;
use crate::bfevfl::{Bf, Cont, Event, Flow, Val};
use crate::error::{Error, Res};

pub const EPONA_FILE: &str = "Event/EventFlow/Get_Epona.bfevfl.zs";
pub const BRIDGE_MODULE: &str = "Logic/GerudoValley_Adult_Prefab_9e43.logic.module.ainb";
pub const BEAN_FILE: &str = "Event/EventFlow/Npc_Hylia_047_Child.bfevfl.zs";
pub const BEAN_FLOW: &str = "Npc_Hylia_047_Child";

pub const BRIDGE_EXP: &str = "SageOfSpirit_IsAfter_Step01_Exp";
pub const BRIDGE_FIXED: &str = "Obj_GerudoValley_Bridge_A_02_e280";

/// The bean salesman's last sale (the tenth bean). With Get_MagicBeans at 9 at new game it is
/// the only one left, and its 100 rupees would not fit the child's wallet.
pub const BEAN_SALES: [(&str, &str); 2] = [("Event14", "Price"), ("Event15", "Price")];
pub const BEAN_MINUS: &str = "Event15";
pub const BEAN_OLD_PRICE: i32 = 100;
pub const BEAN_PRICE: i32 = 60;
pub const BEAN_SALESMAN: &str = "ZR Magic Bean Salesman";
pub const BEAN_ITEMS: [&str; 2] = ["Buy Magic Bean", "Magic Bean"];

pub const BRIDGE_NOTE: &str = "The broken Gerudo Valley bridge is repaired from the start, as if the carpenters had been rescued, so adult Link can cross without Epona or the Longshot. The carpenters in the Thieves' Hideout still need rescuing, the guards and the Gerudo Membership Card are unchanged. Experimental and untested in the game.";

fn flow_mut<'a>(b: &'a mut Bf, name: &str) -> Res<&'a mut Flow> {
    b.flows.iter_mut().find(|x| x.name == name).ok_or_else(|| Error::Msg(format!("skips: no flow named {name}")))
}

const BOOL: usize = 1;
const CONST: u32 = 0x180_0000;

/// Turns a linked bool input into the constant `v`, dropping the plug that fed it.
pub fn force_bool(a: &mut Ainb, i: usize, name: &str, v: bool) -> Res<()> {
    let g: usize = a.find_in(i, BOOL, name).ok_or_else(|| Error::Msg(format!("skips: node {i} has no {name} input")))?;
    let x = a.input(BOOL, g).ok_or(Error::Format("skips: input out of range"))?;
    let src: u32 = u32::try_from(x.src).map_err(|_| Error::Format("skips: the input is not a single link"))?;
    let mut hit: Option<(usize, usize)> = None;
    for k in 0..10 {
        if let Some(pos) = a.params[i].group(k).iter().position(|p| p.a == src && a.s(p.b) == name) {
            hit = Some((k, pos));
            break;
        }
    }
    let (k, pos): (usize, usize) = hit.ok_or(Error::Format("skips: the link into the input has no plug"))?;
    let p = &mut a.params[i];
    p.plugs.remove(p.pl[k][1] as usize + pos);
    p.pl[k][0] -= 1;
    for j in k + 1..p.pl.len() {
        p.pl[j][1] -= 1;
    }
    let o: usize = g * 16;
    let rec: &mut [u8] = a.ins[BOOL].get_mut(o..o + 16).ok_or(Error::Format("skips: input out of range"))?;
    rec[4..6].copy_from_slice(&(-1i16).to_le_bytes());
    rec[6..8].copy_from_slice(&0i16.to_le_bytes());
    rec[8..12].copy_from_slice(&CONST.to_le_bytes());
    rec[12..16].copy_from_slice(&u32::from(v).to_le_bytes());
    Ok(())
}

/// The AND node that reads the Spirit quest step and picks the broken or the repaired bridge.
pub fn bridge_gate(a: &Ainb) -> Res<usize> {
    let reads: Vec<usize> = a.find_els("HoldGameDataGetBoolExpSimple").into_iter().filter(|&i| a.in_str(i, "GameDataNameBoolExp") == Some(BRIDGE_EXP)).collect();
    let hits: Vec<usize> = a
        .find_els("OperateAND")
        .into_iter()
        .filter(|&i| a.find_in(i, BOOL, "BoolMulti").and_then(|g| a.input(BOOL, g)).is_some_and(|x| x.src >= 0 && reads.contains(&(x.src as usize))))
        .collect();
    match hits.as_slice() {
        [i] => Ok(*i),
        _ => Err(Error::Msg(format!("skips: expected one bridge gate in {BRIDGE_MODULE}, found {}", hits.len()))),
    }
}

pub fn bridge_bytes(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    let fixed: Vec<usize> = a.find_els("ActorLogicSimpleActor").into_iter().filter(|&i| a.prop_str(i, "InstanceName") == Some(BRIDGE_FIXED)).collect();
    if fixed.len() != 1 {
        return Err(Error::Msg(format!("skips: expected one repaired bridge actor in {BRIDGE_MODULE}, found {}", fixed.len())));
    }
    let g: usize = bridge_gate(&a)?;
    force_bool(&mut a, g, "BoolMulti", true)?;
    let out: Vec<u8> = a.write()?;
    let b: Ainb = Ainb::parse(&out)?;
    let audit: Vec<String> = crate::ainb::audit(&b);
    if !audit.is_empty() {
        return Err(Error::Msg(format!("skips: the bridge module fails its audit: {}", audit.join("; "))));
    }
    let forced = b.find_in(g, BOOL, "BoolMulti").and_then(|x| b.input(BOOL, x));
    if b.write()? != out || !forced.is_some_and(|x| x.src < 0 && x.val == 1u32.to_le_bytes()) {
        return Err(Error::Format("skips: the bridge module does not read back"));
    }
    Ok(out)
}

fn set_int(c: &mut Cont, key: &str, old: i32, new: i32, at: &str) -> Res<()> {
    let v: &mut Val = c.items.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v).ok_or_else(|| Error::Msg(format!("skips: {at} has no {key}")))?;
    if *v != Val::Int(old) {
        return Err(Error::Msg(format!("skips: {at} {key} is {v:?}, expected {old}, the game files differ")));
    }
    *v = Val::Int(new);
    Ok(())
}

pub fn patch_beans(b: &mut Bf) -> Res<()> {
    let f: &mut Flow = flow_mut(b, BEAN_FLOW)?;
    for (ev, key) in BEAN_SALES {
        let e: &mut Event = f.events.iter_mut().find(|e| e.name == ev).ok_or_else(|| Error::Msg(format!("skips: no event {ev} in {BEAN_FLOW}")))?;
        let c: &mut Cont = e.kind.params_mut().ok_or_else(|| Error::Msg(format!("skips: {ev} has no parameters")))?;
        set_int(c, key, BEAN_OLD_PRICE, BEAN_PRICE, ev)?;
        if ev == BEAN_MINUS {
            set_int(c, "MinusRupeeValue", -BEAN_OLD_PRICE, -BEAN_PRICE, ev)?;
        }
    }
    Ok(())
}

pub fn bean_bytes(raw: &[u8]) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    patch_beans(&mut b)?;
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(Error::Format("skips: the bean salesman flow does not parse back"));
    }
    Ok(out)
}

pub fn is_bean_sale(name: &str) -> bool {
    name == BEAN_SALESMAN
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::romfs::Romfs;
    use std::path::PathBuf as PB;

    fn real() -> Option<Romfs> {
        let p: PB = std::env::var("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|_| PB::from("C:/o/nup dump/updates/131072/romfs"));
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Romfs::open(&p).ok()
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    #[test]
    fn the_bridge_module_creates_the_repaired_bridge_from_the_start() {
        let Some(rf) = real() else {
            return;
        };
        let raw: Vec<u8> = rf.read(BRIDGE_MODULE).unwrap();
        let a: Ainb = Ainb::parse(&raw).unwrap();
        let g: usize = bridge_gate(&a).unwrap();
        let out: Vec<u8> = bridge_bytes(&raw).unwrap();
        let b: Ainb = Ainb::parse(&out).unwrap();
        let x = b.input(BOOL, b.find_in(g, BOOL, "BoolMulti").unwrap()).unwrap();
        assert!(x.src < 0);
        assert_eq!(x.val, 1u32.to_le_bytes().to_vec());
        assert!(bridge_gate(&b).is_err());
        assert!(crate::ainb::audit(&b).is_empty());
        assert_eq!(b.els.len(), a.els.len());
    }

    #[test]
    fn the_bean_salesman_keeps_one_sale_at_sixty_rupees() {
        let Some(mut rf) = real() else {
            return;
        };
        let raw: Vec<u8> = rf.read_zs(BEAN_FILE).unwrap();
        let out: Vec<u8> = bean_bytes(&raw).unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        let f: &Flow = b.flows.iter().find(|x| x.name == BEAN_FLOW).unwrap();
        for (ev, key) in BEAN_SALES {
            let e: &Event = f.events.iter().find(|e| e.name == ev).unwrap();
            assert_eq!(e.kind.params().unwrap().get(key), Some(&Val::Int(BEAN_PRICE)));
        }
        assert!(bean_bytes(&out).is_err());
    }
}
