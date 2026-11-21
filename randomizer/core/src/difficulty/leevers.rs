use std::fs;
use std::path::PathBuf as PB;

use crate::ainb::{audit, Ainb};
use crate::byml::{Byml, Node};
use crate::cow_clone::rows::heap;
use crate::error::{Error, Res};
use crate::flow_locations::dict_id;
use crate::modpack::Pack;
use crate::pickup_pack::rows::{self, ACTOR_INFO, GAME_ACTOR_INFO, HEAP, ROW_ID};
use crate::pickup_pack::sizes::{file_size, pack_size};
use crate::pickup_pack::tag::{Table, TAG};
use crate::pickup_pack::PACK_DICT;
use crate::romfs::Romfs;
use crate::rsizetable::{self, Rst};
use crate::sarc::{self, Entry};
use crate::staging::{Out, MARGIN};
use crate::zs;

pub const BOOL: usize = 1;
pub const GLOBAL: &str = "Pack/AI.Global.Product.111.pack.zs";
pub const BASE: &str = "Enemy_Leever_Fixed";
pub const SMALL_CLASS: &str = "Enemy_Leever_Gate";
pub const GIANT_CLASS: &str = "Enemy_Leever_Gate_Giant";
pub const SET_VERB: &str = "ExecuteBSASetActionVerb";
pub const DISAPPEAR_VERB: i32 = 96;
pub const SAND_QUERY: &str = "QueryPhysicsIsMaterialContactToSphere";
pub const MODULES: [&str; 4] = ["AI/Leever.action.Run.module.ainb", "AI/Leever.action.Disappear.module.ainb", "AI/Leever.action.InterruptRepelled.module.ainb", "AI/Leever.action.InterruptSmallHit.module.ainb"];
pub const LAND: &str = "AI/Enemy_Leever_LandBattle.actionseq.root.ainb";
pub const LIFE_FILE: &str = "Component/LifeParam/Enemy_Leever.game__component__LifeParam.bgyml";
pub const LIFE_PARAMS_FILE: &str = "Life/LifeParameters/Enemy_Leever.game__life__LifeParameters.bgyml";
pub const ATTACK_FILE: &str = "GameBalance/AttackParam/Enemy_Leever.game__gamebalance__AttackParam.bgyml";
pub const TABLE_FILE: &str = "GameParameter/GameParameterTable/Enemy_Leever_Fixed.engine__actor__GameParameterTable.bgyml";
pub const GIANT_LIFE: i32 = 16;
pub const GIANT_ATTACK: i32 = 3;
pub const NOTE: &str = "At Lon Lon Ranch the guards are Leevers: a giant one with eight times the life and five small ones, which come up out of the grass and fight until you beat them. They are two new kinds copied from the Wasteland's fixed Leever with the sand test and the dive away cut out; the game's own Leevers are not touched.";

pub fn pack_of(name: &str) -> String {
    format!("Pack/Actor/{name}.pack.zs")
}

pub fn actor_file(name: &str) -> String {
    format!("Actor/{name}.engine__actor__ActorParam.bgyml")
}

pub fn life_file(name: &str) -> String {
    format!("Component/LifeParam/{name}.game__component__LifeParam.bgyml")
}

pub fn life_params_file(name: &str) -> String {
    format!("Life/LifeParameters/{name}.game__life__LifeParameters.bgyml")
}

pub fn attack_file(name: &str) -> String {
    format!("GameBalance/AttackParam/{name}.game__gamebalance__AttackParam.bgyml")
}

pub fn table_file(name: &str) -> String {
    format!("GameParameter/GameParameterTable/{name}.engine__actor__GameParameterTable.bgyml")
}

pub fn own(class: &str) -> Vec<String> {
    if class == GIANT_CLASS {
        vec![actor_file(class), life_file(class), life_params_file(class), attack_file(class), table_file(class)]
    } else {
        vec![actor_file(class)]
    }
}

fn work(p: &str) -> String {
    match p.strip_suffix(".bgyml") {
        Some(s) => format!("Work/{s}.gyml"),
        None => format!("Work/{p}"),
    }
}

fn get<'a>(es: &'a [Entry], p: &str) -> Res<&'a Entry> {
    es.iter().find(|e| e.name == p).ok_or_else(|| Error::Msg(format!("leever gate: the pack has no {p}, the game files differ")))
}

pub fn unbury(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    let queries: Vec<usize> = a.find_els(SAND_QUERY);
    if queries.len() != 1 {
        return Err(Error::Format("leevers: the graph does not have exactly one sand test"));
    }
    let q: i16 = queries[0] as i16;
    let mut cut: Vec<(usize, String)> = Vec::new();
    for i in 0..a.els.len() {
        for g in a.input_ids(i, BOOL) {
            let Some(x) = a.input(BOOL, g) else {
                continue;
            };
            if x.src == q {
                cut.push((i, a.s(x.name).to_string()));
            }
        }
    }
    if cut.is_empty() {
        return Err(Error::Format("leevers: nothing reads the sand test"));
    }
    for (i, name) in &cut {
        a.force_bool(*i, name, false)?;
    }
    let bad: Vec<String> = audit(&a);
    if !bad.is_empty() {
        return Err(Error::Msg(format!("leevers: {}", bad.join("; "))));
    }
    let out: Vec<u8> = a.write()?;
    let back: Ainb = Ainb::parse(&out)?;
    if back.write()? != out || back.params != a.params || back.ins != a.ins {
        return Err(Error::Format("leevers: the written graph does not read back"));
    }
    Ok(out)
}

pub fn stay(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    let verbs: Vec<usize> = a.find_els(SET_VERB).into_iter().filter(|i| a.in_s32(*i, "ActionVerb") == Some(DISAPPEAR_VERB)).collect();
    if verbs.len() != 1 {
        return Err(Error::Format("leevers: the graph does not have exactly one disappear verb"));
    }
    let mut cut: usize = 0;
    for i in 0..a.els.len() {
        if a.el_type(i) != 2 {
            continue;
        }
        for j in (0..a.params[i].group(2).len()).rev() {
            if a.params[i].group(2)[j].a as usize == verbs[0] {
                a.params[i].remove_plug(2, j)?;
                cut += 1;
            }
        }
    }
    if cut != 1 {
        return Err(Error::Format("leevers: the disappear verb is not the last step of exactly one sequence"));
    }
    let bad: Vec<String> = audit(&a);
    if !bad.is_empty() {
        return Err(Error::Msg(format!("leevers: {}", bad.join("; "))));
    }
    let out: Vec<u8> = a.write()?;
    let back: Ainb = Ainb::parse(&out)?;
    if back.write()? != out || back.params != a.params || back.ins != a.ins {
        return Err(Error::Format("leevers: the written graph does not read back"));
    }
    Ok(out)
}


pub fn actor_param(class: &str) -> Res<Vec<u8>> {
    let mut inner: Node = Node::Map(Vec::new());
    if class == GIANT_CLASS {
        inner.set("GameParameterTableRef", Node::Str(format!("?{}", table_file(class))));
        inner.set("LifeRef", Node::Str(format!("?{}", life_file(class))));
    }
    let y: Byml = Byml { root: Node::Map(vec![("$parent".to_string(), Node::Str(work(&actor_file(BASE)))), ("Components".to_string(), inner)]), version: 7, big: false };
    y.write()
}

fn set_int(y: &mut Byml, key: &str, v: i32) -> Res<()> {
    let n: Node = match y.root.get(key) {
        Some(Node::I32(_)) => Node::I32(v),
        Some(Node::U32(_)) => Node::U32(v as u32),
        Some(Node::F32(_)) => Node::F32(v as f32),
        _ => return Err(Error::Msg(format!("leever gate: the file has no number {key}, the game files differ"))),
    };
    y.root.set(key, n);
    Ok(())
}

pub fn life(base: &[u8], class: &str) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(base)?;
    if !matches!(y.root.get("LifeParameters"), Some(Node::Str(_))) {
        return Err(Error::Format("leever gate: the life file does not name its life parameters"));
    }
    y.root.set("LifeParameters", Node::Str(work(&life_params_file(class))));
    y.write()
}

pub fn life_params(base: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(base)?;
    set_int(&mut y, "MaxLife", GIANT_LIFE)?;
    y.write()
}

pub fn attack(base: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(base)?;
    set_int(&mut y, "BaseAttackPower", GIANT_ATTACK)?;
    y.write()
}

pub fn table(class: &str) -> Res<Vec<u8>> {
    let y: Byml = Byml { root: Node::Map(vec![("$parent".to_string(), Node::Str(work(TABLE_FILE))), ("Components".to_string(), Node::Map(vec![("gamebalance_AttackParam".to_string(), Node::Str(format!("?{}", attack_file(class))))]))]), version: 7, big: false };
    y.write()
}

pub fn build(base: &[Entry], modules: &[Entry], class: &str) -> Res<Vec<Entry>> {
    if base.iter().any(|e| e.name.contains(class)) {
        return Err(Error::Msg(format!("leever gate: the {BASE} pack already holds files named {class}")));
    }
    let mut out: Vec<Entry> = base.to_vec();
    let at: usize = out.iter().position(|e| e.name == LAND).ok_or_else(|| Error::Msg(format!("leever gate: the pack has no {LAND}, the game files differ")))?;
    out[at].data = stay(&unbury(&out[at].data)?)?;
    for m in MODULES {
        if out.iter().any(|e| e.name == m) {
            return Err(Error::Msg(format!("leever gate: the {BASE} pack already holds {m}")));
        }
        out.push(Entry { name: m.to_string(), data: unbury(&get(modules, m)?.data)? });
    }
    out.push(Entry { name: actor_file(class), data: actor_param(class)? });
    if class == GIANT_CLASS {
        out.push(Entry { name: life_file(class), data: life(&get(base, LIFE_FILE)?.data, class)? });
        out.push(Entry { name: life_params_file(class), data: life_params(&get(base, LIFE_PARAMS_FILE)?.data)? });
        out.push(Entry { name: attack_file(class), data: attack(&get(base, ATTACK_FILE)?.data)? });
        out.push(Entry { name: table_file(class), data: table(class)? });
    }
    Ok(out)
}

pub fn rows(info: &mut Byml, game: &mut Byml, class: &str, base_len: usize, len: usize) -> Res<usize> {
    let mut a: Node = rows::need(info, BASE, ACTOR_INFO)?.clone();
    let h: i32 = heap(rows::own_heap(info, BASE)?, base_len, len)?;
    let v: Node = match a.get(HEAP) {
        Some(Node::U32(_)) => Node::U32(h.max(0) as u32),
        _ => Node::I32(h),
    };
    a.set(HEAP, v);
    a.set(ROW_ID, Node::Str(class.to_string()));
    let mut g: Node = rows::need(game, BASE, GAME_ACTOR_INFO)?.clone();
    g.set(ROW_ID, Node::Str(class.to_string()));
    if rows::find(info, class).is_some() || rows::find(game, class).is_some() {
        return Err(Error::Msg(format!("leever gate: the actor tables already hold {class}")));
    }
    let i: usize = rows::insert(info, a)?;
    let j: usize = rows::insert(game, g)?;
    if i != j {
        return Err(Error::Msg(format!("leever gate: {class} sits at {i} in the actor table and at {j} in the game actor table")));
    }
    Ok(i)
}

pub fn tags(t: &mut Table, class: &str) -> Res<()> {
    let own: Vec<String> = t.tags_of(BASE);
    if own.is_empty() {
        return Err(Error::Msg(format!("leever gate: {BASE} has no tag row, the game files differ")));
    }
    if t.find(class).is_some() {
        return Err(Error::Msg(format!("leever gate: the tag table already holds {class}")));
    }
    t.insert(class, &own)?;
    Ok(())
}

#[derive(Clone, Debug, Default)]
pub struct Made {
    pub outs: Vec<Out>,
    pub sizes: Vec<(String, u32)>,
    pub grown: Vec<(String, usize, usize)>,
}

pub fn current(rf: &mut Romfs, have: &dyn Fn(&str) -> Option<Vec<u8>>, p: &str) -> Res<(Vec<u8>, u32)> {
    let z: Vec<u8> = match have(p) {
        Some(z) => z,
        None => rf.read(p)?,
    };
    let id: u32 = dict_id(&z).or_else(|| dict_id(&rf.read(p).unwrap_or_default())).unwrap_or(crate::staging::BANC_DICT);
    Ok((rf.zs.decompress(&z)?, id))
}

pub fn stage(rf: &mut Romfs, have: &dyn Fn(&str) -> Option<Vec<u8>>, rst: &Rst, classes: &[&str]) -> Res<Made> {
    let mut m: Made = Made::default();
    let bz: Vec<u8> = match have(&pack_of(BASE)) {
        Some(z) => z,
        None => rf.read(&pack_of(BASE))?,
    };
    let braw: Vec<u8> = rf.zs.decompress(&bz)?;
    let base: Vec<Entry> = sarc::parse(&braw)?;
    let pid: u32 = dict_id(&bz).unwrap_or(PACK_DICT);
    let modules: Vec<Entry> = sarc::parse(&rf.read_zs(GLOBAL)?)?;
    let near: Vec<(String, usize)> = base.iter().map(|e| (e.name.clone(), e.data.len())).collect();
    let (iraw, iid): (Vec<u8>, u32) = current(rf, have, ACTOR_INFO)?;
    let (graw, gid): (Vec<u8>, u32) = current(rf, have, GAME_ACTOR_INFO)?;
    let (traw, tid): (Vec<u8>, u32) = current(rf, have, TAG)?;
    let mut info: Byml = Byml::parse(&iraw)?;
    let mut game: Byml = Byml::parse(&graw)?;
    let mut tag: Table = Table::parse(&traw)?;
    for class in classes {
        if rf.root.join(pack_of(class)).is_file() {
            return Err(Error::Msg(format!("leever gate: the game already ships {class}")));
        }
        let es: Vec<Entry> = build(&base, &modules, class)?;
        let raw: Vec<u8> = sarc::write(&es)?;
        for p in own(class) {
            m.sizes.push((p.clone(), file_size(rst, &near, &p, get(&es, &p)?.data.len())?));
        }
        for e in es.iter().filter(|e| rst.get(&e.name).is_some()) {
            let was: usize = base.iter().chain(modules.iter()).find(|b| b.name == e.name).map(|b| b.data.len()).unwrap_or(e.data.len());
            if e.data.len() > was {
                m.grown.push((e.name.clone(), was, e.data.len()));
            }
        }
        m.sizes.push((pack_of(class), pack_size(raw.len())?));
        m.outs.push(Out { path: pack_of(class), z: zs::compress_raw(&raw, Some(pid))?, old: 0, new: raw.len() });
        rows(&mut info, &mut game, class, braw.len(), raw.len())?;
        tags(&mut tag, class)?;
    }
    for (p, y, old, id) in [(ACTOR_INFO, &info, iraw.len(), iid), (GAME_ACTOR_INFO, &game, graw.len(), gid)] {
        let out: Vec<u8> = y.write()?;
        m.outs.push(Out { path: p.to_string(), z: zs::compress_raw(&out, Some(id))?, old, new: out.len() });
    }
    let tout: Vec<u8> = tag.write()?;
    m.outs.push(Out { path: TAG.to_string(), z: zs::compress_raw(&tout, Some(tid))?, old: traw.len(), new: tout.len() });
    Ok(m)
}

pub fn sized(rf: &mut Romfs, t: &mut Rst, m: &Made) -> Res<usize> {
    let mut n: usize = 0;
    for (p, sz) in &m.sizes {
        t.set_size(p, *sz)?;
        n += 1;
    }
    for (p, was, now) in &m.grown {
        if t.grow(p, *was as u64, *now as u64, MARGIN)?.is_some() {
            n += 1;
        }
    }
    for o in m.outs.iter().filter(|o| !m.sizes.iter().any(|s| s.0 == o.path) && o.old > 0) {
        n += crate::staging::grow_out(rf, t, o)?;
    }
    Ok(n)
}

pub fn write(rf: &mut Romfs, pk: &Pack, rst: &mut Option<Rst>, classes: &[&str]) -> Res<usize> {
    if classes.is_empty() {
        return Ok(0);
    }
    let root: PB = pk.work.join("romfs");
    if rst.is_none() {
        *rst = Some(rsizetable::load(rf)?);
    }
    let t: &mut Rst = rst.as_mut().ok_or(Error::Format("leever gate: no size table"))?;
    let have = |p: &str| -> Option<Vec<u8>> { fs::read(p.split('/').fold(root.clone(), |a, s| a.join(s))).ok() };
    let m: Made = stage(rf, &have, t, classes)?;
    for o in &m.outs {
        pk.write_romfs(&o.path, &o.z)?;
    }
    sized(rf, t, &m)?;
    Ok(m.outs.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        if p.is_dir() { Some(p) } else { None }
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    fn readers(a: &Ainb) -> usize {
        let q: i16 = a.find_els(SAND_QUERY)[0] as i16;
        (0..a.els.len()).map(|i| a.input_ids(i, BOOL).into_iter().filter(|g| a.input(BOOL, *g).is_some_and(|x| x.src == q)).count()).sum()
    }

    fn entries(rf: &mut Romfs, path: &str) -> Vec<Entry> {
        sarc::parse(&rf.read_zs(path).unwrap()).unwrap()
    }

    fn made(rf: &mut Romfs) -> Made {
        let t: Rst = rsizetable::load(rf).unwrap();
        stage(rf, &none, &t, &[SMALL_CLASS, GIANT_CLASS]).unwrap()
    }

    #[test]
    fn a_graph_without_a_sand_test_is_refused() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let es: Vec<Entry> = entries(&mut rf, &pack_of("Enemy_Leever"));
        assert!(unbury(&get(&es, "AI/Enemy_Leever.action.root.ainb").unwrap().data).is_err());
    }

    #[test]
    fn the_clone_graphs_do_not_read_the_sand_test_and_keep_every_other_node() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let base: Vec<Entry> = entries(&mut rf, &pack_of(BASE));
        let modules: Vec<Entry> = entries(&mut rf, GLOBAL);
        let es: Vec<Entry> = build(&base, &modules, SMALL_CLASS).unwrap();
        for n in MODULES.iter().copied().chain([LAND]) {
            let from: &[Entry] = if n == LAND { &base } else { &modules };
            let old: Ainb = Ainb::parse(&get(from, n).unwrap().data).unwrap();
            let new: Ainb = Ainb::parse(&get(&es, n).unwrap().data).unwrap();
            assert!(readers(&old) > 0, "{n}");
            assert_eq!(readers(&new), 0, "{n}");
            assert!(audit(&new).is_empty(), "{n}");
            assert_eq!(new.els.len(), old.els.len(), "{n}");
        }
    }

    #[test]
    fn the_clone_packs_add_files_and_change_only_the_land_battle_graph() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let base: Vec<Entry> = entries(&mut rf, &pack_of(BASE));
        let modules: Vec<Entry> = entries(&mut rf, GLOBAL);
        for class in [SMALL_CLASS, GIANT_CLASS] {
            let es: Vec<Entry> = build(&base, &modules, class).unwrap();
            assert_eq!(es.len(), base.len() + MODULES.len() + own(class).len(), "{class}");
            for b in &base {
                assert_eq!(get(&es, &b.name).unwrap().data != b.data, b.name == LAND, "{}", b.name);
            }
            assert!(own(class).iter().all(|p| es.iter().any(|e| e.name == *p)));
        }
    }

    #[test]
    fn the_giant_has_eight_times_the_life_and_more_attack_and_the_small_clone_keeps_the_fixed_ones() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let base: Vec<Entry> = entries(&mut rf, &pack_of(BASE));
        let modules: Vec<Entry> = entries(&mut rf, GLOBAL);
        let small: f32 = match Byml::parse(&get(&base, LIFE_PARAMS_FILE).unwrap().data).unwrap().root.get("MaxLife") {
            Some(Node::F32(v)) => *v,
            Some(Node::I32(v)) => *v as f32,
            Some(Node::U32(v)) => *v as f32,
            other => panic!("no life: {other:?}"),
        };
        assert_eq!(GIANT_LIFE as f32, small * 8.0);
        let es: Vec<Entry> = build(&base, &modules, GIANT_CLASS).unwrap();
        let l: Byml = Byml::parse(&get(&es, &life_params_file(GIANT_CLASS)).unwrap().data).unwrap();
        assert_eq!(l.root.get("MaxLife").map(|n| format!("{n:?}")), Byml::parse(&life_params(&get(&base, LIFE_PARAMS_FILE).unwrap().data).unwrap()).unwrap().root.get("MaxLife").map(|n| format!("{n:?}")));
        let a: Byml = Byml::parse(&get(&es, &attack_file(GIANT_CLASS)).unwrap().data).unwrap();
        let was: Byml = Byml::parse(&get(&base, ATTACK_FILE).unwrap().data).unwrap();
        assert_eq!(was.root.get("BaseAttackPower").map(|n| format!("{n:?}")), Some("I32(2)".to_string()));
        assert_eq!(a.root.get("BaseAttackPower").map(|n| format!("{n:?}")), Some(format!("I32({GIANT_ATTACK})")));
        let small_es: Vec<Entry> = build(&base, &modules, SMALL_CLASS).unwrap();
        assert!(small_es.iter().all(|e| e.name != life_file(SMALL_CLASS)));
    }

    #[test]
    fn the_vanilla_leever_packs_and_the_shared_ai_pack_are_never_written() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let vanilla: Vec<String> = ["Enemy_Leever", "Enemy_Leever_Big", "Enemy_Leever_Fixed"].iter().map(|n| pack_of(n)).chain([GLOBAL.to_string()]).collect();
        let m: Made = made(&mut rf);
        assert!(m.outs.iter().all(|o| !vanilla.contains(&o.path)));
        let paths: Vec<&str> = m.outs.iter().map(|o| o.path.as_str()).collect();
        assert!(paths.contains(&pack_of(SMALL_CLASS).as_str()) && paths.contains(&pack_of(GIANT_CLASS).as_str()));
        let o: crate::difficulty::Opts = crate::difficulty::Opts { gates: true, ..crate::difficulty::Opts::default() };
        let outs: Vec<Out> = crate::difficulty::stage(&mut rf, &o, &none, "seed").unwrap();
        assert!(!outs.is_empty());
        assert!(outs.iter().all(|x| !vanilla.contains(&x.path)));
    }

    #[test]
    fn the_actor_rows_and_tags_of_both_clones_follow_the_fixed_leever() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let m: Made = made(&mut rf);
        let info: Byml = Byml::parse(&rf.zs.decompress(&m.outs.iter().find(|o| o.path == ACTOR_INFO).unwrap().z).unwrap()).unwrap();
        let game: Byml = Byml::parse(&rf.zs.decompress(&m.outs.iter().find(|o| o.path == GAME_ACTOR_INFO).unwrap().z).unwrap()).unwrap();
        for c in [SMALL_CLASS, GIANT_CLASS] {
            assert!(rows::find(&info, c).is_some() && rows::find(&game, c).is_some(), "{c}");
        }
        assert!(rows::find(&info, BASE).is_some());
        let t: Table = Table::parse(&rf.zs.decompress(&m.outs.iter().find(|o| o.path == TAG).unwrap().z).unwrap()).unwrap();
        assert_eq!(t.tags_of(SMALL_CLASS), t.tags_of(BASE));
        assert_eq!(t.tags_of(GIANT_CLASS), t.tags_of(BASE));
    }

    #[test]
    fn the_edit_is_the_same_every_time() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let a: Made = made(&mut rf);
        let b: Made = made(&mut rf);
        assert!(a.outs.iter().zip(b.outs.iter()).all(|(x, y)| x.path == y.path && x.z == y.z));
        assert_eq!(a.sizes, b.sizes);
    }
}
