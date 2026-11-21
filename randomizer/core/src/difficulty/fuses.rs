use std::collections::BTreeSet as Set;

use crate::ainb::build::Spec;
use crate::ainb::{audit, root, Ainb, Plug};
use crate::difficulty::hook::{self, flag, PARALLEL};
use crate::difficulty::regen::{IDLE, TIMER};
use crate::error::{Error, Res};
use crate::qol::{edit_zs, Have};
use crate::romfs::Romfs;
use crate::sarc::{self, Entry};
use crate::staging::Out;
use crate::zs;

pub const BOMB_PACK: &str = "Pack/Actor/Item_Bomb.pack.zs";
pub const BOMB_GRAPH: &str = "AI/Item_Bomb.root.ainb";
pub const CHU_PACK: &str = "Pack/Actor/Item_Bombchu.pack.zs";
pub const CHU_GRAPH: &str = "AI/Item_Bombchu_Remake.root.ainb";
pub const STICK_PACK: &str = "Pack/Actor/Weapon_DekuStick.pack.zs";
pub const STICK_GRAPH: &str = "AI/Weapon_DekuStick.root.ainb";
pub const DEATH: &str = "OneShotDeathRequestDeath";
pub const STICK_TERROR: &str = "ExecutePerceptionAdvancedEmitTerror";
pub const CHU_ROOT: &str = "ExecuteBombchuScheduler";
pub const DEATH_REASON: i32 = 10;
pub const FPS: f32 = 60.0;
pub const FLOOR: f32 = 60.0;
pub const BOMB_FRAMES: f32 = 240.0;
pub const CHU_FLOOR: f32 = 210.0;
pub const CHU_FRAMES: f32 = 315.0;
pub const STICK_FLOOR: f32 = 600.0;
pub const STICK_FRAMES: f32 = 1440.0;
pub const HEAP_EXTRA: i32 = 0x1000;
pub const LABEL: &str = "random fuses";
pub const NOTE: &str = "Each bomb, Bombchu and Deku Stick you light gets its own random fuse. A bomb blows up after 1 to 4 seconds instead of 4, a Bombchu after 3.5 to 5.25 seconds (never shorter than normal, so the Bombchu Bowling alley stays playable), and a lit Deku Stick burns for 10 to 24 seconds before it goes out (a stick that burned for only 1 second could not carry a flame to a torch). Bomb flowers, the bomb the Imperial soldier throws and the bombs of enemies are unchanged. Experimental, untested in the game.";

pub fn packs() -> [(&'static str, &'static str); 3] {
    [(BOMB_PACK, BOMB_GRAPH), (CHU_PACK, CHU_GRAPH), (STICK_PACK, STICK_GRAPH)]
}

fn timer(a: &mut Ainb, tag: &str, floor: f32, top: f32, finish: usize) -> Res<usize> {
    let wait: usize = a.add_node(&Spec::node(IDLE, &format!("{tag} idle")))?;
    a.add_node(
        &Spec::node(TIMER, &format!("{tag} fuse"))
            .prop(1, "IsCountUpdateOnReset", flag(false))
            .prop(1, "IsLoop", flag(false))
            .prop(1, "NoChangeWhenChildIsBusy", flag(false))
            .prop(1, "SelectChildOnEnterOnly", flag(false))
            .prop(2, "CountFrames", floor.to_le_bytes().to_vec())
            .prop(2, "RandomAddCountFrames", (top - floor).to_le_bytes().to_vec())
            .spare(2, "CurrentFrame")
            .spare(2, "FinishFrame")
            .kid(wait, "NotFinish")
            .kid(finish, "Finish"),
    )
}

fn death(a: &mut Ainb, tag: &str) -> Res<usize> {
    a.add_node(&Spec::node(DEATH, &format!("{tag} death")).prop(1, "ExecuteEveryFrame", flag(false)).constant(0, "DeathReson", DEATH_REASON.to_le_bytes().to_vec()))
}

fn sole(a: &Ainb, name: &str, what: &str) -> Res<usize> {
    let v: Vec<usize> = a.find_els(name);
    match v.as_slice() {
        [one] => Ok(*one),
        _ => Err(Error::Msg(format!("{LABEL}: the {what} graph has {} {name} nodes, not one", v.len()))),
    }
}

fn into_parallel(a: &mut Ainb, parent: usize, top: usize) -> Res<()> {
    let at: usize = a.params[parent].pl[2][0] as usize;
    let name: u32 = a.pool.add("");
    a.params[parent].add_plug(2, at, Plug { a: top as u32, b: name, extra: Vec::new() })
}

fn parallel_above(a: &Ainb, node: usize, up: usize, what: &str) -> Res<usize> {
    let mut at: usize = node;
    for _ in 0..up {
        at = a.parent_of(at).map_err(|_| Error::Msg(format!("{LABEL}: a {what} node has no single parent")))?.0;
    }
    if a.el_type(at) != PARALLEL {
        return Err(Error::Msg(format!("{LABEL}: the {what} graph no longer has the expected parallel")));
    }
    Ok(at)
}

fn checked(a: Ainb, label: &str) -> Res<Vec<u8>> {
    let bad: Vec<String> = audit(&a);
    if !bad.is_empty() {
        return Err(Error::Msg(format!("{LABEL}: {label}: {}", bad.join("; "))));
    }
    let out: Vec<u8> = a.write()?;
    let back: Ainb = Ainb::parse(&out)?;
    if back.write()? != out || back.params != a.params || back.ins != a.ins || back.outs != a.outs || back.qids != a.qids {
        return Err(Error::Msg(format!("{LABEL}: {label}: the written graph does not read back")));
    }
    Ok(out)
}

pub fn bomb(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    if !a.find_els(TIMER).is_empty() {
        return Err(Error::Msg(format!("{LABEL}: the bomb graph is already patched")));
    }
    let site: usize = sole(&a, DEATH, "bomb")?;
    let parent: usize = parallel_above(&a, site, 3, "bomb")?;
    let kill: usize = death(&mut a, "ootrr bomb")?;
    let top: usize = timer(&mut a, "ootrr bomb", FLOOR, BOMB_FRAMES, kill)?;
    into_parallel(&mut a, parent, top)?;
    checked(a, "bomb")
}

pub fn chu(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    if !a.find_els(TIMER).is_empty() {
        return Err(Error::Msg(format!("{LABEL}: the Bombchu graph is already patched")));
    }
    let top_root: usize = root(&a).ok_or_else(|| Error::Msg(format!("{LABEL}: the Bombchu graph has no root")))?;
    if a.el_type(top_root) != PARALLEL || !a.find_els(CHU_ROOT).iter().all(|i| a.parent_of(*i).is_ok_and(|p| p.0 == top_root)) {
        return Err(Error::Msg(format!("{LABEL}: the Bombchu graph no longer starts with a parallel that holds its scheduler")));
    }
    let kill: usize = death(&mut a, "ootrr chu")?;
    let top: usize = timer(&mut a, "ootrr chu", CHU_FLOOR, CHU_FRAMES, kill)?;
    into_parallel(&mut a, top_root, top)?;
    checked(a, "Bombchu")
}

pub fn stick(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    if !a.find_els(TIMER).is_empty() {
        return Err(Error::Msg(format!("{LABEL}: the Deku Stick graph is already patched")));
    }
    let site: usize = sole(&a, STICK_TERROR, "Deku Stick")?;
    let parent: usize = parallel_above(&a, site, 1, "Deku Stick")?;
    let out: usize = death(&mut a, "ootrr stick")?;
    let top: usize = timer(&mut a, "ootrr stick", STICK_FLOOR, STICK_FRAMES, out)?;
    into_parallel(&mut a, parent, top)?;
    checked(a, "Deku Stick")
}

pub fn edit_pack(raw: &[u8]) -> Res<Vec<u8>> {
    let mut es: Vec<Entry> = sarc::parse(raw)?;
    let mut n: usize = 0;
    for e in es.iter_mut() {
        let out: Option<Vec<u8>> = match e.name.as_str() {
            BOMB_GRAPH => Some(bomb(&e.data)?),
            CHU_GRAPH => Some(chu(&e.data)?),
            STICK_GRAPH => Some(stick(&e.data)?),
            _ => None,
        };
        if let Some(d) = out {
            e.data = d;
            n += 1;
        }
    }
    if n != 1 {
        return Err(Error::Msg(format!("{LABEL}: a pack has {n} fuse graphs, not one")));
    }
    sarc::write(&es)
}

pub fn stage(rf: &mut Romfs, have: Have) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    for (pack, _) in packs() {
        let z: Vec<u8> = hook::zipped(rf, have, pack)?;
        let raw: Vec<u8> = rf.zs.decompress(&z)?;
        let out: Vec<u8> = edit_pack(&raw)?;
        outs.push(Out { path: pack.to_string(), old: raw.len(), new: out.len(), z: zs::compress_raw(&out, zs::dict_of(&z))? });
    }
    let who: Set<String> = packs().iter().filter_map(|(p, _)| hook::actor_of(p).map(str::to_string)).collect();
    outs.push(edit_zs(rf, have, hook::ACTOR_INFO, |raw| hook::grow_heap(raw, &who, HEAP_EXTRA, LABEL))?);
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ainb::show::show;

    const VANILLA_CHU_FRAMES: f32 = 210.0;

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        if p.is_dir() { Some(p) } else { None }
    }

    fn graph_of(rf: &mut Romfs, z: &[u8], name: &str) -> Ainb {
        let es: Vec<Entry> = sarc::parse(&rf.zs.decompress(z).unwrap()).unwrap();
        Ainb::parse(&sarc::get(&es, name).unwrap().data).unwrap()
    }

    #[test]
    fn bombs_run_from_one_second_chus_never_shorter_than_vanilla_and_sticks_keep_ten_seconds() {
        assert_eq!(FLOOR, FPS);
        assert!(BOMB_FRAMES > FLOOR && CHU_FRAMES > CHU_FLOOR && STICK_FRAMES > STICK_FLOOR);
        assert!(STICK_FLOOR >= 10.0 * FPS);
        assert_eq!(CHU_FLOOR / FPS, 3.5);
        assert_eq!(CHU_FLOOR, VANILLA_CHU_FRAMES);
        assert!(CHU_FRAMES <= VANILLA_CHU_FRAMES * 1.5);
        assert_eq!(BOMB_FRAMES / FPS, 4.0);
        assert_eq!(CHU_FRAMES / FPS, 5.25);
        assert_eq!(STICK_FRAMES / FPS, 24.0);
    }

    #[test]
    fn garbage_is_refused() {
        assert!(bomb(b"nope").is_err());
        assert!(chu(b"nope").is_err());
        assert!(stick(b"nope").is_err());
        assert!(edit_pack(b"nope").is_err());
    }

    #[test]
    fn the_game_files_take_a_random_fuse_for_the_bomb_the_bombchu_and_the_stick() {
        let Some(root_dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root_dir).unwrap();
        let before: Vec<Ainb> = packs().iter().map(|(p, g)| {
            let z: Vec<u8> = rf.read(p).unwrap();
            graph_of(&mut rf, &z, g)
        }).collect();
        let outs: Vec<Out> = stage(&mut rf, &|_| None).unwrap();
        assert_eq!(outs.len(), 4);
        let added: [usize; 3] = [3, 3, 3];
        let spans: [(f32, f32); 3] = [(FLOOR, BOMB_FRAMES), (CHU_FLOOR, CHU_FRAMES), (STICK_FLOOR, STICK_FRAMES)];
        for (i, (pack, name)) in packs().iter().enumerate() {
            let o: &Out = outs.iter().find(|o| o.path == *pack).unwrap();
            assert!(o.new > o.old);
            let a: Ainb = graph_of(&mut rf, &o.z, name);
            assert!(audit(&a).is_empty());
            let t: String = show(&a);
            assert!(t.contains(&format!("prop f32 CountFrames = {}", spans[i].0)), "{t}");
            assert!(t.contains(&format!("prop f32 RandomAddCountFrames = {}", spans[i].1 - spans[i].0)), "{t}");
            assert_eq!(a.find_els(TIMER).len(), 1);
            assert_eq!(a.els.len(), before[i].els.len() + added[i]);
        }
        let bomb_a: Ainb = graph_of(&mut rf, &outs[0].z, BOMB_GRAPH);
        assert_eq!(bomb_a.find_els(DEATH).len(), 2);
        assert!(show(&bomb_a).matches("DeathReson = 10").count() >= 2);
        let chu_a: Ainb = graph_of(&mut rf, &outs[1].z, CHU_GRAPH);
        let r: usize = root(&chu_a).unwrap();
        assert_eq!(chu_a.el_type(r), PARALLEL);
        assert_eq!(chu_a.find_els(CHU_ROOT).len(), 1);
        assert_eq!(root(&before[1]), Some(r));
        assert!(show(&graph_of(&mut rf, &outs[2].z, STICK_GRAPH)).contains("DeathReson = 10"));
        let patched: Vec<u8> = bomb(&before_raw(&mut rf)).unwrap();
        assert!(bomb(&patched).is_err());
    }

    fn before_raw(rf: &mut Romfs) -> Vec<u8> {
        let z: Vec<u8> = rf.read(BOMB_PACK).unwrap();
        let es: Vec<Entry> = sarc::parse(&rf.zs.decompress(&z).unwrap()).unwrap();
        sarc::get(&es, BOMB_GRAPH).unwrap().data.clone()
    }
}
