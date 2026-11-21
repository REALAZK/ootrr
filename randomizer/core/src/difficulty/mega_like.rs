use std::fs;
use std::path::PathBuf as PB;

use crate::ainb::build::{Spec, QUERY};
use crate::ainb::{audit, Ainb, Plug};
use crate::byml::{Byml, Node};
use crate::cow_clone::rows::heap;
use crate::difficulty::ambush::{actor, euler, fresh};
use crate::difficulty::hook::{flag, switch};
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
use crate::staging::{Out, BANC_DICT, MARGIN};
use crate::zs;

pub const BASE: &str = "Enemy_LikeLike";
pub const CLASS: &str = "Enemy_LikeLike_Mega";
pub const SEED: &str = "ootrr mega like likes";
pub const SHIELDS: [&str; 3] = ["Weapon_Shield_Mirror", "Weapon_Shield_Hylian", "Weapon_Shield_Deku"];
pub const LIFE_TIMES: i32 = 3;
pub const GRIP_TIMES: f32 = 1.5;
pub const SIZE: f32 = 1.6;
pub const HEAP_EXTRA: i32 = 0x4000;
pub const EAT_GRAPH: &str = "AI/Enemy_LikeLike.action.BasicAttack.module.ainb";
pub const DIE_GRAPH: &str = "AI/Enemy_LikeLike.action.die.module.ainb";
pub const UTILITY_GRAPH: &str = "AI/Enemy_LikeLike.utility.root.ainb";
pub const RIDER: &str = "ExecuteLikeLikeRiderMessage";
pub const COUNT: &str = "QueryGetPouchContentNumByActorName";
pub const POUCH: &str = "OneShotAddOrRemovePouchContent";
pub const GET_BOOL: &str = "QueryGetGameDataBool";
pub const GET_STRUCT: &str = "QueryGetNonStructGameDataStruct";
pub const GET_GROUP: &str = "IsGet";
pub const SET_TEXT: &str = "ExecuteBlackboardSetString";
pub const TABLE_KEY: &str = "Drop__DropTableOverWrite";
pub const TABLES: [&str; 3] = ["", "LikeLike_HyliaShield", "LikeLike_DekuShield"];
pub const SPARKLES: [&str; 3] = ["Eat_Success", "ライト_00", "ライト_01"];
pub const IDLE: &str = "ExecuteGenericDummyAction";
pub const SEQUENCE: u16 = 2;
pub const BY_NUMBER: u16 = 1;
pub const STATE: u16 = 500;
pub const NOTE: &str = "Mega Like Likes: three bigger and tougher Like Likes (a copy of the Like Like under its own actor name that does not use the game's own shield steal, three times the life, a longer grip, and 1.6 times the size) wait in the Deku Tree meadow of Kokiri Forest, as a child and as an adult. When one swallows you it takes one of your shields, any of them, the Mirror Shield first, then the Hylian Shield, then the Deku Shield: the shield leaves the pouch and your hand. Nothing is written down: the missing shield is the record, read from the game's own flags (a shield you have owned and now have none of). When any mega Like Like dies it hands back one such shield, the Mirror Shield first. The Mirror Shield goes straight back into the pouch, with the Farore's Wind sparkle rings and orb around the Like Like as the sign. Experimental, untested in the game.";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub map: &'static str,
    pub name: &'static str,
    pub at: [f32; 3],
    pub turn: f32,
}

pub const SPOTS: [Spot; 6] = [
    Spot { map: "KokiriForest_Child", name: "child_meadow_south", at: [82.0, 113.1, -40.0], turn: 1.2 },
    Spot { map: "KokiriForest_Child", name: "child_meadow_middle", at: [85.0, 113.4, -28.0], turn: 1.2 },
    Spot { map: "KokiriForest_Child", name: "child_meadow_north", at: [88.0, 113.6, -16.0], turn: 1.2 },
    Spot { map: "KokiriForest_Adult", name: "adult_meadow_west", at: [70.0, 112.0, -35.0], turn: 1.2 },
    Spot { map: "KokiriForest_Adult", name: "adult_meadow_south", at: [80.0, 113.0, -40.0], turn: 1.2 },
    Spot { map: "KokiriForest_Adult", name: "adult_meadow_north", at: [88.0, 113.7, -16.0], turn: 1.2 },
];

pub fn banc(map: &str) -> String {
    format!("Banc/KokiriForest/{map}_Static.bcett.byml.zs")
}

pub fn maps() -> Vec<&'static str> {
    let mut m: Vec<&'static str> = Vec::new();
    for s in SPOTS.iter() {
        if !m.contains(&s.map) {
            m.push(s.map);
        }
    }
    m
}

pub fn pack_of(name: &str) -> String {
    format!("Pack/Actor/{name}.pack.zs")
}

pub fn actor_file(name: &str) -> String {
    format!("Actor/{name}.engine__actor__ActorParam.bgyml")
}

pub fn table_file(name: &str) -> String {
    format!("GameParameter/GameParameterTable/{name}.engine__actor__GameParameterTable.bgyml")
}

pub fn life_ref(name: &str) -> String {
    format!("Component/LifeParam/{name}.game__component__LifeParam.bgyml")
}

pub fn life_file(name: &str) -> String {
    format!("Life/LifeParameters/{name}.game__life__LifeParameters.bgyml")
}

pub fn grip_file(name: &str) -> String {
    format!("GameParameter/RestraintParam/{name}.game__enemy__RestraintParam.bgyml")
}

pub fn own() -> [String; 5] {
    [actor_file(CLASS), table_file(CLASS), life_ref(CLASS), life_file(CLASS), grip_file(CLASS)]
}

fn work(p: &str) -> String {
    match p.strip_suffix(".bgyml") {
        Some(s) => format!("Work/{s}.gyml"),
        None => format!("Work/{p}"),
    }
}

fn get<'a>(es: &'a [Entry], p: &str) -> Res<&'a Entry> {
    es.iter().find(|e| e.name == p).ok_or_else(|| E::Msg(format!("mega like likes: the {BASE} pack has no {p}, the game files differ")))
}

fn expect(n: &Node, k: &str, want: &str) -> Res<()> {
    let have: &str = n.get(k).and_then(|v| v.as_str()).unwrap_or("");
    if have != want {
        return Err(E::Msg(format!("mega like likes: {k} holds {have} and not {want}, the game files differ")));
    }
    Ok(())
}

fn comps(y: &mut Byml) -> Res<&mut Node> {
    y.root.get_mut("Components").ok_or(E::Format("mega like likes: a parameter file has no Components"))
}

pub fn actor_param(base: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(base)?;
    let c: &mut Node = comps(&mut y)?;
    expect(c, "GameParameterTableRef", &format!("?{}", table_file(BASE)))?;
    expect(c, "LifeRef", &format!("?{}", life_ref(BASE)))?;
    let mut inner: Node = Node::Map(Vec::new());
    inner.set("GameParameterTableRef", Node::Str(format!("?{}", table_file(CLASS))));
    inner.set("LifeRef", Node::Str(format!("?{}", life_ref(CLASS))));
    y.root = Node::Map(vec![("$parent".to_string(), Node::Str(work(&actor_file(BASE)))), ("Components".to_string(), inner)]);
    y.write()
}

pub fn table(base: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(base)?;
    let c: &mut Node = comps(&mut y)?;
    expect(c, "RestraintParam", &format!("?{}", grip_file(BASE)))?;
    c.set("RestraintParam", Node::Str(format!("?{}", grip_file(CLASS))));
    y.write()
}

pub fn life_component(base: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(base)?;
    expect(&y.root, "LifeParameters", &work(&life_file(BASE)))?;
    y.root.set("LifeParameters", Node::Str(work(&life_file(CLASS))));
    y.write()
}

pub fn life(base: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(base)?;
    let v: i32 = match y.root.get("MaxLife") {
        Some(Node::I32(v)) => *v,
        _ => return Err(E::Format("mega like likes: the Like Like life file has no MaxLife, the game files differ")),
    };
    y.root.set("MaxLife", Node::I32(v.saturating_mul(LIFE_TIMES)));
    y.write()
}

pub fn grip(base: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(base)?;
    for k in ["GachaInputFrame", "MaxRestraintFrame"] {
        let v: f32 = match y.root.get(k) {
            Some(Node::F32(v)) => *v,
            _ => return Err(E::Msg(format!("mega like likes: the Like Like grip file has no {k}, the game files differ"))),
        };
        y.root.set(k, Node::F32(v * GRIP_TIMES));
    }
    y.write()
}

fn text_const(a: &mut Ainb, s: &str) -> Vec<u8> {
    a.pool.add(s).to_le_bytes().to_vec()
}

fn idle(a: &mut Ainb, seed: &str) -> Res<usize> {
    a.add_node(&Spec::node(IDLE, seed))
}

fn count(a: &mut Ainb, seed: &str, shield: &str) -> Res<usize> {
    let name: Vec<u8> = text_const(a, shield);
    a.add_node(&Spec::query(COUNT, seed).idle(3, "ActorName", name).out(0, "Num"))
}

fn owned(a: &mut Ainb, seed: &str, shield: &str) -> Res<usize> {
    let group: Vec<u8> = text_const(a, GET_GROUP);
    let struct_seed: String = format!("{seed} struct");
    let mut st: Spec = Spec::node(GET_STRUCT, &struct_seed)
        .prop(1, "DispError", flag(true))
        .prop(1, "OutputOnlyOnEnter", flag(false))
        .idle(0, "GameDataIndex", (-1i32).to_le_bytes().to_vec())
        .idle(1, "IsDirect", flag(false))
        .constant(3, "GameDataFullName", group)
        .out_ptr("Value", "");
    st.role = QUERY;
    let parent: usize = a.add_node(&st)?;
    let name: Vec<u8> = text_const(a, shield);
    let mut b: Spec = Spec::node(GET_BOOL, seed)
        .prop(1, "DispError", flag(true))
        .prop(1, "OutputOnlyOnEnter", flag(false))
        .idle(0, "GameDataIndex", (-1i32).to_le_bytes().to_vec())
        .constant(3, "GameDataName", name)
        .linked_ptr("ParentStruct", "", parent)
        .out(1, "Value");
    b.role = QUERY;
    a.add_node(&b)
}

fn change(a: &mut Ainb, seed: &str, shield: &str, by: i32, shown: bool) -> Res<usize> {
    let actor: Vec<u8> = text_const(a, &format!("Work/Actor/{shield}.engine__actor__ActorParam.gyml"));
    let reason: Vec<u8> = text_const(a, "Put");
    let spec: Spec = Spec::node(POUCH, seed).prop(1, "ExecuteEveryFrame", flag(false)).constant(0, "Num", by.to_le_bytes().to_vec()).idle(1, "IsEquipDirect", flag(false));
    let spec: Spec = if shown { spec.constant(1, "IsGetEvent", flag(true)) } else { spec.idle(1, "IsGetEvent", flag(false)) };
    a.add_node(&spec.constant(3, "Actor", actor).constant(3, "Reason", reason))
}

fn pick_drop(a: &mut Ainb, seed: &str, name: &str) -> Res<usize> {
    let key: Vec<u8> = text_const(a, TABLE_KEY);
    let value: Vec<u8> = text_const(a, name);
    a.add_node(
        &Spec::node(SET_TEXT, seed)
            .prop(0, "BlackboardType", 1i32.to_le_bytes().to_vec())
            .prop(1, "ExecuteOnce", flag(false))
            .prop(1, "IsIgnoreNotExistsError", flag(false))
            .prop(3, "BlackboardKey", key)
            .constant(3, "String", value),
    )
}

fn by_number(a: &mut Ainb, seed: &str, num: usize, none: usize, some: usize) -> Res<usize> {
    let mut s: Spec = Spec::composite(BY_NUMBER, seed)
        .prop(0, "CalculateTiming", vec![0; 4])
        .prop(0, "InputValue", vec![0; 4])
        .prop(1, "ChildFrameSync", flag(false))
        .prop(1, "IsNoSelectIfSameInstance", flag(false))
        .prop(1, "IsNoSelectWhenChildBusy", flag(false))
        .linked(0, "Input", num, 0)
        .kid(none, "")
        .kid(some, "");
    s.link_extra = 8;
    let i: usize = a.add_node(&s)?;
    let at: usize = a.params[i].pl[2][1] as usize;
    for k in 0..2 {
        a.params[i].plugs[at + k].extra = vec![0; 8];
    }
    Ok(i)
}

pub fn sequence(a: &mut Ainb, seed: &str, first: usize, then: usize) -> Res<usize> {
    a.add_node(
        &Spec::composite(SEQUENCE, seed)
            .prop(0, "BusyPolicy", vec![0; 4])
            .prop(0, "NumLoop", vec![0; 4])
            .prop(0, "PlayPolicy", vec![0; 4])
            .prop(0, "ResultPolicy", vec![0; 4])
            .prop(1, "IsDelayJudge", flag(false))
            .prop(1, "IsLoop", flag(false))
            .prop(1, "IsUpdateNextInFrame", flag(true))
            .kid(first, "")
            .kid(then, ""),
    )
}

pub fn take(a: &mut Ainb, tag: &str) -> Res<usize> {
    let mut next: usize = idle(a, &format!("{tag} take none"))?;
    for (n, s) in SHIELDS.iter().enumerate().rev() {
        let gone: usize = change(a, &format!("{tag} take {n} remove"), s, -1, false)?;
        let num: usize = count(a, &format!("{tag} take {n} count"), s)?;
        next = by_number(a, &format!("{tag} take {n} has"), num, next, gone)?;
    }
    Ok(next)
}

pub fn give(a: &mut Ainb, tag: &str) -> Res<usize> {
    let mut next: usize = idle(a, &format!("{tag} give none"))?;
    for (n, s) in SHIELDS.iter().enumerate().rev() {
        let back: usize = if TABLES[n].is_empty() {
            let add: usize = change(a, &format!("{tag} give {n} add"), s, 1, true)?;
            let mut names: Vec<&str> = crate::difficulty::fx::PARTS.iter().map(|p| p.0).collect();
            names.extend(SPARKLES);
            crate::difficulty::fx::with_cues(a, &format!("{tag} give {n} shown"), add, &names)?
        } else { pick_drop(a, &format!("{tag} give {n} drop"), TABLES[n])? };
        let num: usize = count(a, &format!("{tag} give {n} count"), s)?;
        let lost: usize = by_number(a, &format!("{tag} give {n} lost"), num, back, next)?;
        let had: usize = owned(a, &format!("{tag} give {n} owned"), s)?;
        next = a.add_node(&switch(&format!("{tag} give {n} had"), had, lost, next))?;
    }
    Ok(next)
}

fn finish(a: Ainb, what: &str) -> Res<Vec<u8>> {
    let bad: Vec<String> = audit(&a);
    if !bad.is_empty() {
        return Err(E::Msg(format!("mega like likes: {what}: {}", bad.join("; "))));
    }
    let out: Vec<u8> = a.write()?;
    let back: Ainb = Ainb::parse(&out)?;
    if back.write()? != out || back.params != a.params || back.ins != a.ins || back.outs != a.outs || back.qids != a.qids {
        return Err(E::Msg(format!("mega like likes: {what}: the written graph does not read back")));
    }
    Ok(out)
}

pub fn eaten(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    let site: usize = (0..a.els.len())
        .find(|&i| {
            let kids: &[Plug] = a.params[i].group(2);
            a.el_type(i) == STATE && kids.len() == 3 && {
                let enter: usize = kids[0].a as usize;
                a.el_name(enter) == "ExecuteBlackboardSetBool" && a.prop_str(enter, "BlackboardKey") == Some("RestraintEnemy_IsEnableGuts") && has_predation(&a, kids[1].a as usize)
            }
        })
        .ok_or(E::Format("mega like likes: the Like Like eating state is not where it was, the game files differ"))?;
    let at: usize = a.params[site].pl[2][1] as usize;
    let enter: u32 = a.params[site].plugs[at].a;
    let taken: usize = take(&mut a, "ootrr mega take")?;
    let both: usize = sequence(&mut a, "ootrr mega eaten", taken, enter as usize)?;
    a.params[site].plugs[at].a = both as u32;
    finish(a, "the eating graph")
}

fn has_predation(a: &Ainb, from: usize) -> bool {
    let mut seen: Vec<usize> = Vec::new();
    let mut todo: Vec<usize> = vec![from];
    while let Some(i) = todo.pop() {
        if seen.contains(&i) {
            continue;
        }
        seen.push(i);
        if a.el_name(i) == "OneShotEnemyRequestPredationEatDamageToPlayer" {
            return true;
        }
        todo.extend(a.params[i].group(2).iter().map(|p| p.a as usize));
    }
    false
}

pub fn quiet(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    let nodes: Vec<usize> = a.find_els(RIDER);
    let [rider] = nodes.as_slice() else {
        return Err(E::Format("mega like likes: the Like Like utility graph does not read the rider message once, the game files differ"));
    };
    let (root, pos): (usize, usize) = a.parent_of(*rider).map_err(|_| E::Format("mega like likes: the rider message node has no single parent, the game files differ"))?;
    let at: usize = pos - a.params[root].pl[2][1] as usize;
    let switch: usize = a.params[root].group(2).get(at + 1).map(|p| p.a as usize).ok_or(E::Format("mega like likes: the rider message has no switch after it, the game files differ"))?;
    if a.el_type(switch) != crate::difficulty::hook::SWITCH {
        return Err(E::Format("mega like likes: the node after the rider message is not its switch, the game files differ"));
    }
    a.params[root].remove_plug(2, at + 1)?;
    a.params[root].remove_plug(2, at)?;
    finish(a, "the utility graph")
}

pub fn died(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    let drops: Vec<usize> = a.find_els("OneShotDropReserveDropItemCreate");
    let [drop] = drops.as_slice() else {
        return Err(E::Format("mega like likes: the Like Like death graph does not create its drop once, the game files differ"));
    };
    let mut top: usize = *drop;
    loop {
        match a.parent_of(top) {
            Ok((p, _)) => {
                if a.el_type(p) == SEQUENCE {
                    top = p;
                    break;
                }
                top = p;
            }
            Err(_) => return Err(E::Format("mega like likes: the death graph has no sequence above its drop, the game files differ")),
        }
    }
    let (parent, pos): (usize, usize) = a.parent_of(top).map_err(|_| E::Format("mega like likes: the death sequence has no single parent, the game files differ"))?;
    let given: usize = give(&mut a, "ootrr mega give")?;
    let both: usize = sequence(&mut a, "ootrr mega died", given, top)?;
    a.params[parent].plugs[pos].a = both as u32;
    finish(a, "the death graph")
}

pub fn build(base: &[Entry]) -> Res<(Vec<Entry>, Vec<(String, usize, usize)>)> {
    if base.iter().any(|e| e.name.contains(CLASS)) {
        return Err(E::Msg(format!("mega like likes: the {BASE} pack already holds files named {CLASS}")));
    }
    let mut out: Vec<Entry> = base.to_vec();
    let mut grown: Vec<(String, usize, usize)> = Vec::new();
    for (path, f) in [(EAT_GRAPH, eaten as fn(&[u8]) -> Res<Vec<u8>>), (DIE_GRAPH, died), (UTILITY_GRAPH, quiet)] {
        let e: &mut Entry = out.iter_mut().find(|e| e.name == path).ok_or_else(|| E::Msg(format!("mega like likes: the {BASE} pack has no {path}, the game files differ")))?;
        let old: usize = e.data.len();
        e.data = f(&e.data)?;
        grown.push((path.to_string(), old, e.data.len()));
    }
    out.push(Entry { name: actor_file(CLASS), data: actor_param(&get(base, &actor_file(BASE))?.data)? });
    out.push(Entry { name: table_file(CLASS), data: table(&get(base, &table_file(BASE))?.data)? });
    out.push(Entry { name: life_ref(CLASS), data: life_component(&get(base, &life_ref(BASE))?.data)? });
    out.push(Entry { name: life_file(CLASS), data: life(&get(base, &life_file(BASE))?.data)? });
    out.push(Entry { name: grip_file(CLASS), data: grip(&get(base, &grip_file(BASE))?.data)? });
    Ok((out, grown))
}

pub fn rows(info: &mut Byml, game: &mut Byml, base_len: usize, len: usize) -> Res<usize> {
    let mut a: Node = rows::need(info, BASE, ACTOR_INFO)?.clone();
    let h: i32 = heap(rows::own_heap(info, BASE)?, base_len, len)?.checked_add(HEAP_EXTRA).ok_or(E::Format("mega like likes: the heap size does not fit"))?;
    let v: Node = match a.get(HEAP) {
        Some(Node::U32(_)) => Node::U32(h.max(0) as u32),
        _ => Node::I32(h),
    };
    a.set(HEAP, v);
    a.set(ROW_ID, Node::Str(CLASS.to_string()));
    let mut g: Node = rows::need(game, BASE, GAME_ACTOR_INFO)?.clone();
    g.set(ROW_ID, Node::Str(CLASS.to_string()));
    if rows::find(info, CLASS).is_some() || rows::find(game, CLASS).is_some() {
        return Err(E::Msg(format!("mega like likes: the actor tables already hold {CLASS}")));
    }
    let i: usize = rows::insert(info, a)?;
    let j: usize = rows::insert(game, g)?;
    if i != j {
        return Err(E::Msg(format!("mega like likes: {CLASS} sits at {i} in the actor table and at {j} in the game actor table")));
    }
    Ok(i)
}

pub fn tags(t: &mut Table) -> Res<()> {
    let own: Vec<String> = t.tags_of(BASE);
    if own.is_empty() {
        return Err(E::Msg(format!("mega like likes: {BASE} has no tag row, the game files differ")));
    }
    if t.find(CLASS).is_some() {
        return Err(E::Msg(format!("mega like likes: the tag table already holds {CLASS}")));
    }
    t.insert(CLASS, &own)?;
    Ok(())
}

pub fn place(y: &mut Byml, s: &Spot, raws: &[&[u8]]) -> Res<u64> {
    let id: u64 = fresh(raws, &format!("{SEED} {} {}", s.map, s.name));
    let mut m: Node = actor(CLASS, id, s.at, euler(0.0, s.turn));
    m.set("Dynamic", Node::Map(vec![("Drop__DropTable".to_string(), Node::Str("None".to_string()))]));
    m.set("Scale", crate::difficulty::ambush::floats([SIZE; 3]));
    let acts: &mut Vec<Node> = y.root.get_mut("Actors").and_then(|a| a.as_arr_mut()).ok_or(E::Format("mega like likes: the map has no actor list"))?;
    acts.push(m);
    Ok(id)
}

#[derive(Clone, Debug, Default)]
pub struct Made {
    pub outs: Vec<Out>,
    pub sizes: Vec<(String, u32)>,
    pub grown: Vec<(String, usize, usize)>,
    pub lines: Vec<String>,
}

fn current(rf: &mut Romfs, have: &dyn Fn(&str) -> Option<Vec<u8>>, p: &str) -> Res<(Vec<u8>, u32)> {
    let z: Vec<u8> = match have(p) {
        Some(z) => z,
        None => rf.read(p)?,
    };
    let id: u32 = dict_id(&z).or_else(|| dict_id(&rf.read(p).unwrap_or_default())).unwrap_or(BANC_DICT);
    Ok((rf.zs.decompress(&z)?, id))
}

pub fn stage(rf: &mut Romfs, have: &dyn Fn(&str) -> Option<Vec<u8>>, rst: &Rst) -> Res<Made> {
    let mut m: Made = Made::default();
    if rf.root.join(pack_of(CLASS)).is_file() {
        return Err(E::Msg(format!("mega like likes: the game already ships {CLASS}")));
    }
    let bz: Vec<u8> = match have(&pack_of(BASE)) {
        Some(z) => z,
        None => rf.read(&pack_of(BASE))?,
    };
    let braw: Vec<u8> = rf.zs.decompress(&bz)?;
    let base: Vec<Entry> = sarc::parse(&braw)?;
    let pid: u32 = dict_id(&bz).unwrap_or(PACK_DICT);
    let (es, grown): (Vec<Entry>, Vec<(String, usize, usize)>) = build(&base)?;
    let raw: Vec<u8> = sarc::write(&es)?;
    let near: Vec<(String, usize)> = base.iter().map(|e| (e.name.clone(), e.data.len())).collect();
    for p in own() {
        let len: usize = get(&es, &p)?.data.len();
        m.sizes.push((p.clone(), file_size(rst, &near, &p, len)?));
    }
    m.sizes.push((pack_of(CLASS), pack_size(raw.len())?));
    m.grown = grown;
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
    for map in maps() {
        let file: String = banc(map);
        let (mraw, mid): (Vec<u8>, u32) = current(rf, have, &file)?;
        let dynamic: String = file.replace("_Static.", "_Dynamic.");
        let land: Vec<u8> = current(rf, have, &dynamic).map(|x| x.0).unwrap_or_default();
        let mut y: Byml = Byml::parse(&mraw)?;
        let mut n: usize = 0;
        for s in SPOTS.iter().filter(|s| s.map == map) {
            place(&mut y, s, &[&mraw, &land])?;
            n += 1;
        }
        let mout: Vec<u8> = y.write()?;
        m.outs.push(Out { path: file.clone(), z: zs::compress_raw(&mout, Some(mid))?, old: mraw.len(), new: mout.len() });
        m.lines.push(format!("{file}: {n} {CLASS} placed, scale {SIZE}"));
    }
    m.lines.push(format!("{CLASS}: new actor class, pack {} bytes with {} files ({} of its own, three graphs changed)", raw.len(), es.len(), own().len()));
    Ok(m)
}

pub fn sized(rf: &mut Romfs, t: &mut Rst, m: &Made) -> Res<usize> {
    let mut n: usize = 0;
    for (p, sz) in &m.sizes {
        t.set_size(p, *sz)?;
        n += 1;
    }
    for (p, old, new) in &m.grown {
        if new > old && t.grow(p, *old as u64, *new as u64, MARGIN)?.is_some() {
            n += 1;
        }
    }
    for o in m.outs.iter().filter(|o| !m.sizes.iter().any(|s| s.0 == o.path)) {
        n += crate::staging::grow_out(rf, t, o)?;
    }
    Ok(n)
}

pub fn write(rf: &mut Romfs, pk: &Pack, rst: &mut Option<Rst>) -> Res<(usize, Vec<String>)> {
    let root: PB = pk.work.join("romfs");
    if rst.is_none() {
        *rst = Some(rsizetable::load(rf)?);
    }
    let t: &mut Rst = rst.as_mut().ok_or(E::Format("mega like likes: no size table"))?;
    let have = |p: &str| -> Option<Vec<u8>> { fs::read(p.split('/').fold(root.clone(), |a, s| a.join(s))).ok() };
    let m: Made = stage(rf, &have, t)?;
    for o in &m.outs {
        pk.write_romfs(&o.path, &o.z)?;
    }
    sized(rf, t, &m)?;
    Ok((m.outs.len(), m.lines))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> Option<PB> {
        let p: PB = ootrr_game::test_romfs::path();
        if p.join("Pack/Actor").is_dir() { Some(p) } else { None }
    }

    fn base() -> Option<Vec<Entry>> {
        let mut rf: Romfs = Romfs::open(&root()?).unwrap();
        Some(sarc::parse(&rf.read_zs(&pack_of(BASE)).unwrap()).unwrap())
    }

    #[test]
    fn the_clone_adds_five_files_and_changes_three_graphs() {
        let Some(b) = base() else { return };
        let (es, grown): (Vec<Entry>, Vec<(String, usize, usize)>) = build(&b).unwrap();
        assert_eq!(es.len(), b.len() + 5);
        assert_eq!(grown.len(), 3);
        for p in own() {
            assert!(sarc::get(&es, &p).is_some(), "{p}");
        }
        let y: Byml = Byml::parse(&sarc::get(&es, &life_file(CLASS)).unwrap().data).unwrap();
        let v: Byml = Byml::parse(&sarc::get(&b, &life_file(BASE)).unwrap().data).unwrap();
        let (Some(Node::I32(a)), Some(Node::I32(c))) = (y.root.get("MaxLife"), v.root.get("MaxLife")) else { panic!() };
        assert_eq!(*a, *c * LIFE_TIMES);
        let t: Byml = Byml::parse(&sarc::get(&es, &table_file(CLASS)).unwrap().data).unwrap();
        assert_eq!(t.root.get("Components").and_then(|c| c.get("RestraintParam")).and_then(|v| v.as_str()), Some(format!("?{}", grip_file(CLASS)).as_str()));
    }

    #[test]
    fn the_eating_graph_takes_a_shield_before_the_vanilla_enter_step_and_reads_back() {
        let Some(b) = base() else { return };
        let old: Vec<u8> = sarc::get(&b, EAT_GRAPH).unwrap().data.clone();
        let new: Vec<u8> = eaten(&old).unwrap();
        let a: Ainb = Ainb::parse(&new).unwrap();
        assert_eq!(a.find_els(POUCH).len(), 3);
        assert_eq!(a.find_els(COUNT).len(), 3);
        assert!(audit(&a).is_empty());
    }

    #[test]
    fn the_death_graph_gives_back_the_mirror_shield_first_and_reads_back() {
        let Some(b) = base() else { return };
        let old: Vec<u8> = sarc::get(&b, DIE_GRAPH).unwrap().data.clone();
        let new: Vec<u8> = died(&old).unwrap();
        let a: Ainb = Ainb::parse(&new).unwrap();
        assert_eq!(a.find_els(POUCH).len(), 1);
        assert_eq!(a.find_els(SET_TEXT).len(), 2);
        assert_eq!(a.find_els(GET_BOOL).len(), 3);
        let flags: Vec<String> = a.find_els(GET_BOOL).iter().map(|&i| a.in_str(i, "GameDataName").unwrap_or("").to_string()).collect();
        assert!(flags.contains(&"Weapon_Shield_Mirror".to_string()));
        assert_eq!(a.find_els(GET_STRUCT).len(), 3);
        assert!(audit(&a).is_empty());
        let root: usize = a.find_els("OneShotDropReserveDropItemCreate")[0];
        assert!(a.parent_of(root).is_ok());
    }

    #[test]
    fn six_spots_lie_in_the_two_forest_maps() {
        assert_eq!(SPOTS.len(), 6);
        assert_eq!(maps(), vec!["KokiriForest_Child", "KokiriForest_Adult"]);
        assert_eq!(SPOTS.iter().filter(|s| s.map == "KokiriForest_Child").count(), 3);
    }

    #[test]
    fn a_spot_adds_one_scaled_actor() {
        let mut y: Byml = Byml { root: Node::Map(vec![("Actors".to_string(), Node::Arr(vec![]))]), version: 7, big: false };
        place(&mut y, &SPOTS[0], &[]).unwrap();
        let acts: &[Node] = y.root.get("Actors").unwrap().as_arr().unwrap();
        assert_eq!(acts.len(), 1);
        assert_eq!(acts[0].get("Gyaml").and_then(|v| v.as_str()), Some(CLASS));
        assert!(acts[0].get("Scale").is_some());
    }
}
