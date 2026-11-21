use std::fs;
use std::path::PathBuf as PB;

use crate::byml::{Byml, Node};
use crate::cow_clone::rows::heap;
use crate::difficulty::leevers::{current, sized, Made};
use crate::error::{Error as E, Res};
use crate::flow_locations::dict_id;
use crate::modpack::Pack;
use crate::pickup_pack::rows::{self, ACTOR_INFO, GAME_ACTOR_INFO, HEAP, ROW_ID};
use crate::pickup_pack::sizes::{file_size, pack_size};
use crate::pickup_pack::tag::{Table, TAG};
use crate::pickup_pack::PACK_DICT;
use crate::romfs::Romfs;
use crate::rsizetable::{self, Rst};
use crate::sarc::{self, Entry};
use crate::staging::Out;
use crate::zs;

pub const BASE: &str = "Enemy_Beamos";
pub const CLASS: &str = "Enemy_Beamos_Temple";
pub const RANGE: f32 = 24.0;
pub const ANGLE: f32 = 90.0;

pub fn pack_of(name: &str) -> String {
    format!("Pack/Actor/{name}.pack.zs")
}

pub fn actor_file(name: &str) -> String {
    format!("Actor/{name}.engine__actor__ActorParam.bgyml")
}

pub fn sight_file(name: &str) -> String {
    format!("Component/PerceptionReceiverParam/{name}.game__component__PerceptionReceiverParam.bgyml")
}

pub fn own() -> [String; 2] {
    [actor_file(CLASS), sight_file(CLASS)]
}

fn work(p: &str) -> String {
    match p.strip_suffix(".bgyml") {
        Some(s) => format!("Work/{s}.gyml"),
        None => format!("Work/{p}"),
    }
}

fn get<'a>(es: &'a [Entry], p: &str) -> Res<&'a Entry> {
    es.iter().find(|e| e.name == p).ok_or_else(|| E::Msg(format!("temple beamos: the {BASE} pack has no {p}, the game files differ")))
}

pub fn actor_param(base: &[u8]) -> Res<Vec<u8>> {
    let y: Byml = Byml::parse(base)?;
    let want: String = format!("?{}", sight_file(BASE));
    if y.root.get("Components").and_then(|c| c.get("PerceptionReceiverRef")).and_then(|v| v.as_str()) != Some(want.as_str()) {
        return Err(E::Msg(format!("temple beamos: the actor does not use {want}, the game files differ")));
    }
    let mut inner: Node = Node::Map(Vec::new());
    inner.set("PerceptionReceiverRef", Node::Str(format!("?{}", sight_file(CLASS))));
    let out: Byml = Byml { root: Node::Map(vec![("$parent".to_string(), Node::Str(work(&actor_file(BASE)))), ("Components".to_string(), inner)]), version: 7, big: false };
    out.write()
}

pub fn sight(base: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(base)?;
    let s: &mut Node = y.root.get_mut("SightParam").ok_or(E::Format("temple beamos: the perception file has no sight"))?;
    for (k, v) in [("DiscoverHorizontalRange", RANGE), ("DiscoverAngle", ANGLE)] {
        if !matches!(s.get(k), Some(Node::F32(_))) || !s.set(k, Node::F32(v)) {
            return Err(E::Msg(format!("temple beamos: the sight has no number {k}, the game files differ")));
        }
    }
    y.write()
}

pub fn build(base: &[Entry]) -> Res<Vec<Entry>> {
    if base.iter().any(|e| e.name.contains(CLASS)) {
        return Err(E::Msg(format!("temple beamos: the {BASE} pack already holds files named {CLASS}")));
    }
    let mut out: Vec<Entry> = base.to_vec();
    out.push(Entry { name: actor_file(CLASS), data: actor_param(&get(base, &actor_file(BASE))?.data)? });
    out.push(Entry { name: sight_file(CLASS), data: sight(&get(base, &sight_file(BASE))?.data)? });
    Ok(out)
}

pub fn rows(info: &mut Byml, game: &mut Byml, base_len: usize, len: usize) -> Res<usize> {
    let mut a: Node = rows::need(info, BASE, ACTOR_INFO)?.clone();
    let h: i32 = heap(rows::own_heap(info, BASE)?, base_len, len)?;
    let v: Node = match a.get(HEAP) {
        Some(Node::U32(_)) => Node::U32(h.max(0) as u32),
        _ => Node::I32(h),
    };
    a.set(HEAP, v);
    a.set(ROW_ID, Node::Str(CLASS.to_string()));
    let mut g: Node = rows::need(game, BASE, GAME_ACTOR_INFO)?.clone();
    g.set(ROW_ID, Node::Str(CLASS.to_string()));
    if rows::find(info, CLASS).is_some() || rows::find(game, CLASS).is_some() {
        return Err(E::Msg(format!("temple beamos: the actor tables already hold {CLASS}")));
    }
    let i: usize = rows::insert(info, a)?;
    let j: usize = rows::insert(game, g)?;
    if i != j {
        return Err(E::Msg(format!("temple beamos: {CLASS} sits at {i} in the actor table and at {j} in the game actor table")));
    }
    Ok(i)
}

pub fn tags(t: &mut Table) -> Res<()> {
    let own: Vec<String> = t.tags_of(BASE);
    if own.is_empty() {
        return Err(E::Msg(format!("temple beamos: {BASE} has no tag row, the game files differ")));
    }
    if t.find(CLASS).is_some() {
        return Err(E::Msg(format!("temple beamos: the tag table already holds {CLASS}")));
    }
    t.insert(CLASS, &own)?;
    Ok(())
}

pub fn stage(rf: &mut Romfs, have: &dyn Fn(&str) -> Option<Vec<u8>>, rst: &Rst) -> Res<Made> {
    let mut m: Made = Made::default();
    if rf.root.join(pack_of(CLASS)).is_file() {
        return Err(E::Msg(format!("temple beamos: the game already ships {CLASS}")));
    }
    let bz: Vec<u8> = match have(&pack_of(BASE)) {
        Some(z) => z,
        None => rf.read(&pack_of(BASE))?,
    };
    let braw: Vec<u8> = rf.zs.decompress(&bz)?;
    let base: Vec<Entry> = sarc::parse(&braw)?;
    let pid: u32 = dict_id(&bz).unwrap_or(PACK_DICT);
    let es: Vec<Entry> = build(&base)?;
    let raw: Vec<u8> = sarc::write(&es)?;
    let near: Vec<(String, usize)> = base.iter().map(|e| (e.name.clone(), e.data.len())).collect();
    for p in own() {
        let len: usize = get(&es, &p)?.data.len();
        m.sizes.push((p.clone(), file_size(rst, &near, &p, len)?));
    }
    m.sizes.push((pack_of(CLASS), pack_size(raw.len())?));
    m.outs.push(Out { path: pack_of(CLASS), z: zs::compress_raw(&raw, Some(pid))?, old: 0, new: raw.len() });
    let (iraw, iid): (Vec<u8>, u32) = current(rf, have, ACTOR_INFO)?;
    let (graw, gid): (Vec<u8>, u32) = current(rf, have, GAME_ACTOR_INFO)?;
    let mut info: Byml = Byml::parse(&iraw)?;
    let mut game: Byml = Byml::parse(&graw)?;
    rows(&mut info, &mut game, braw.len(), raw.len())?;
    for (p, y, old, id) in [(ACTOR_INFO, &info, iraw.len(), iid), (GAME_ACTOR_INFO, &game, graw.len(), gid)] {
        let out: Vec<u8> = y.write()?;
        m.outs.push(Out { path: p.to_string(), z: zs::compress_raw(&out, Some(id))?, old, new: out.len() });
    }
    let (traw, tid): (Vec<u8>, u32) = current(rf, have, TAG)?;
    let mut t: Table = Table::parse(&traw)?;
    tags(&mut t)?;
    let tout: Vec<u8> = t.write()?;
    m.outs.push(Out { path: TAG.to_string(), z: zs::compress_raw(&tout, Some(tid))?, old: traw.len(), new: tout.len() });
    Ok(m)
}

pub fn write(rf: &mut Romfs, pk: &Pack, rst: &mut Option<Rst>) -> Res<usize> {
    let root: PB = pk.work.join("romfs");
    if rst.is_none() {
        *rst = Some(rsizetable::load(rf)?);
    }
    let t: &mut Rst = rst.as_mut().ok_or(E::Format("temple beamos: no size table"))?;
    let have = |p: &str| -> Option<Vec<u8>> { fs::read(p.split('/').fold(root.clone(), |a, s| a.join(s))).ok() };
    let m: Made = stage(rf, &have, t)?;
    for o in &m.outs {
        pk.write_romfs(&o.path, &o.z)?;
    }
    sized(rf, t, &m)?;
    Ok(m.outs.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> Option<PB> {
        let p: PB = ootrr_game::test_romfs::path();
        if p.join("Pack/Actor").is_dir() { Some(p) } else { None }
    }

    fn base(rf: &mut Romfs) -> Vec<Entry> {
        sarc::parse(&rf.read_zs(&pack_of(BASE)).unwrap()).unwrap()
    }

    fn number(y: &Byml, k: &str) -> f32 {
        match y.root.get("SightParam").and_then(|s| s.get(k)) {
            Some(Node::F32(v)) => *v,
            other => panic!("no number {k}: {other:?}"),
        }
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn the_clone_adds_two_files_and_sees_further_than_the_ordinary_beamos() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let b: Vec<Entry> = base(&mut rf);
        let es: Vec<Entry> = build(&b).unwrap();
        assert_eq!(es.len(), b.len() + 2);
        for e in &b {
            assert_eq!(get(&es, &e.name).unwrap().data, e.data, "{}", e.name);
        }
        let was: Byml = Byml::parse(&get(&b, &sight_file(BASE)).unwrap().data).unwrap();
        let now: Byml = Byml::parse(&get(&es, &sight_file(CLASS)).unwrap().data).unwrap();
        assert!(number(&was, "DiscoverHorizontalRange") < 9.5 && number(&now, "DiscoverHorizontalRange") > 9.5);
        assert_eq!(number(&now, "DiscoverHorizontalRange"), RANGE);
        let a: Byml = Byml::parse(&get(&es, &actor_file(CLASS)).unwrap().data).unwrap();
        assert_eq!(a.root.get("Components").and_then(|c| c.get("PerceptionReceiverRef")).and_then(|v| v.as_str()), Some(format!("?{}", sight_file(CLASS)).as_str()));
    }

    #[test]
    fn the_ordinary_beamos_pack_and_its_files_are_never_written() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let t: Rst = rsizetable::load(&mut rf).unwrap();
        let m: Made = stage(&mut rf, &none, &t).unwrap();
        assert!(m.outs.iter().all(|o| o.path != pack_of(BASE)));
        assert!(m.outs.iter().any(|o| o.path == pack_of(CLASS)));
        assert!(m.sizes.iter().all(|(p, _)| !p.contains(&format!("/{BASE}."))));
        let o: crate::difficulty::Opts = crate::difficulty::Opts { encounters: true, extra: 100, ..crate::difficulty::Opts::default() };
        let outs: Vec<Out> = crate::difficulty::stage(&mut rf, &o, &none, "seed").unwrap();
        assert!(outs.iter().all(|x| x.path != pack_of(BASE)));
        let packed: Vec<Entry> = sarc::parse(&rf.zs.decompress(&m.outs.iter().find(|o| o.path == pack_of(CLASS)).unwrap().z).unwrap()).unwrap();
        let kept: Byml = Byml::parse(&get(&packed, &sight_file(BASE)).unwrap().data).unwrap();
        let shipped: Byml = Byml::parse(&get(&base(&mut rf), &sight_file(BASE)).unwrap().data).unwrap();
        assert_eq!(kept.root, shipped.root);
    }

    #[test]
    fn a_sight_without_the_numbers_is_refused() {
        let y: Byml = Byml { root: Node::Map(vec![("SightParam".to_string(), Node::Map(Vec::new()))]), version: 7, big: false };
        assert!(sight(&y.write().unwrap()).is_err());
    }

    #[test]
    fn the_actor_rows_and_tags_follow_the_ordinary_beamos() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let t: Rst = rsizetable::load(&mut rf).unwrap();
        let m: Made = stage(&mut rf, &none, &t).unwrap();
        let info: Byml = Byml::parse(&rf.zs.decompress(&m.outs.iter().find(|o| o.path == ACTOR_INFO).unwrap().z).unwrap()).unwrap();
        let game: Byml = Byml::parse(&rf.zs.decompress(&m.outs.iter().find(|o| o.path == GAME_ACTOR_INFO).unwrap().z).unwrap()).unwrap();
        assert!(rows::find(&info, CLASS).is_some() && rows::find(&game, CLASS).is_some() && rows::find(&info, BASE).is_some());
        let tt: Table = Table::parse(&rf.zs.decompress(&m.outs.iter().find(|o| o.path == TAG).unwrap().z).unwrap()).unwrap();
        assert_eq!(tt.tags_of(CLASS), tt.tags_of(BASE));
    }
}
