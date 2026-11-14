use std::collections::{BTreeMap as BM, BTreeSet as BS};
use std::fmt::Write as W;
use std::path::{Path as P, PathBuf as PB};

use crate::ainb::Ainb;
use crate::byml::{Byml, Node};
use crate::chest_slot::LIST;
use crate::error::{Error as E, Res};
use crate::flow_locations::dict_id;
use crate::modpack::Pack;
use crate::pickup_pack::actor::{identity, make, pouch_file, Made, ATTENTION_REF, AUTO_GET, POUCH_REF};
use crate::pickup_pack::build::{pool_of, Plan};
use crate::pickup_pack::closure::{actor_file, chain, dangling_with, is_byml, local, norm, select, work, Chain, Pool, COMPONENTS, PARENT};
use crate::pickup_pack::place;
use crate::pickup_pack::rows::{self, ACTOR_INFO, GAME_ACTOR_INFO, POUCH_INFO};
use crate::pickup_pack::sizes;
use crate::pickup_pack::write::{audit, base_dir, earlier, kept, size_table, sized, start, table, tables, Done, File, Opts, Scope};
use crate::pickup_pack::{is_actor_name, pack_of, Layers, Spec, Style, BASE, DONOR, PACK_DICT};
use crate::pickup_persist::{cell_of, chain as death_chain, Cell};
use crate::placement::banc_files;
use crate::romfs::Romfs;
use crate::rsizetable::{Rst, TABLE_PATH};
use crate::sarc::{self, Entry};
use crate::staging::BANC_DICT;
use crate::zs;
use crate::TITLE_ID;

pub const TOKEN: &str = "Obj_Token_Gold";
pub const SKULL: &str = "Enemy_Skulltula_Gold";
pub const SUFFIX: &str = "_GS";
pub const REFS: [&str; 3] = ["AIInfoRef", "BlackboardRef", "DeathRef"];
pub const LIFE_REF: &str = "GameLifeConditionRef";
pub const MANY: &str = "PlayGetEventManyTimes";
pub const AT_MAX: &str = "IsPlayGetEventPouchMax";
pub const HASH_KEY: &str = "GoldTokenGmdHash";
pub const ATTENTION: &str = "IsEnableAttention";
pub const WORKER: &str = "OneShotGoldTokenDeadAfter";
pub const SIGNAL: &str = "ExecuteDeathSignalDeathProcedure";
pub const COLLECTED: i32 = 10;
pub const DEATH_TABLE: &str = "Item_Token_Gold";
pub const SKULL_GRAPH: &str = "AI/Enemy_Skulltula_Gold.utility.root.ainb";
pub const SKULL_DEATH: &str = "System/CombinationDataTableData/Enemy_Skulltula_Gold_game__death__DeathReasonProcedures.pp__CombinationDataTableData.bgyml";
const MAGIC: &[u8] = b"AIB ";
const SELECTOR: u16 = 1;
const CHILDREN: usize = 2;
pub const PHYSICS_REF: &str = "PhysicsRef";
pub const DYNAMIC: u32 = 3672686866;
const SETS: &str = "Phive/ControllerSetParam/";
const BODY: &str = "Body";
const SET_PATH: &str = "ControllerSetPath";
const BODIES: &str = "RigidBodyEntityNamePathAry";
const FILE_PATH: &str = "FilePath";
const MOTION: &str = "MotionType";

pub fn name_of(pickup: &str) -> String {
    format!("{pickup}{SUFFIX}")
}

pub fn parent_of(kit: &str) -> Option<&str> {
    kit.strip_suffix(SUFFIX).filter(|p| !p.is_empty())
}

pub fn default_kit(item: &str) -> String {
    match item.strip_prefix("Item_") {
        Some(t) => format!("Obj_{t}{SUFFIX}"),
        None => name_of(item),
    }
}

pub fn graph(t: &str) -> Option<String> {
    let rest: &str = t.strip_prefix("AI/Root/")?.strip_suffix(".ain")?;
    let file: &str = rest.rsplit('/').next().filter(|f| !f.is_empty())?;
    Some(format!("AI/{file}.ainb"))
}

pub fn leaf(d: &[u8]) -> bool {
    d.get(..4) == Some(MAGIC)
}

pub fn shipped(es: &[Entry]) -> bool {
    sarc::get(es, &actor_file(BASE)).is_some()
}

fn need<'a>(c: &'a Chain, who: &str, k: &str) -> Res<&'a str> {
    c.comps.get(k).map(|s| s.as_str()).ok_or_else(|| E::Msg(format!("{who} has no {k}, so it cannot give or take the token kit")))
}

fn kit_refs(p: &Pool) -> Res<Vec<(&'static str, String)>> {
    let t: Chain = chain(p, TOKEN)?;
    let mut out: Vec<(&'static str, String)> = Vec::new();
    for k in REFS.iter().chain([LIFE_REF].iter()) {
        out.push((*k, need(&t, TOKEN, k)?.to_string()));
    }
    Ok(out)
}

pub(crate) fn inherited(p: &Pool, first: &str, k: &str) -> Res<Option<Node>> {
    let mut seen: Vec<String> = Vec::new();
    let mut cur: Option<String> = Some(first.to_string());
    while let Some(f) = cur {
        if seen.contains(&f) || p.get(&f).is_none() {
            break;
        }
        let y: Byml = p.byml(&f)?;
        if let Some(v) = y.root.get(k) {
            return Ok(Some(v.clone()));
        }
        cur = y.root.get(PARENT).and_then(|v| v.as_str()).and_then(norm);
        seen.push(f);
    }
    Ok(None)
}

fn linked(p: &Pool, from: &str, k: &str) -> Res<Option<String>> {
    Ok(inherited(p, from, k)?.and_then(|v| v.as_str().and_then(norm)))
}

pub fn falls(p: &Pool, c: &Chain) -> Res<bool> {
    let Some(phys) = c.comps.get(PHYSICS_REF).and_then(|v| norm(v)) else {
        return Ok(false);
    };
    let Some(set) = linked(p, &phys, SET_PATH)? else {
        return Ok(false);
    };
    let list: Node = inherited(p, &set, BODIES)?.unwrap_or(Node::Arr(Vec::new()));
    let bodies: &[Node] = list.as_arr().unwrap_or(&[]);
    for b in bodies {
        let Some(f) = b.get(FILE_PATH).and_then(|v| v.as_str()).and_then(norm) else {
            return Ok(false);
        };
        if !inherited(p, &f, MOTION)?.is_some_and(|v| starts(&v, i64::from(DYNAMIC))) {
            return Ok(false);
        }
    }
    Ok(!bodies.is_empty())
}

pub fn near(c: &Chain, base: &Chain) -> bool {
    c.comps.get(ATTENTION_REF).is_some() && c.comps.get(ATTENTION_REF) == base.comps.get(ATTENTION_REF)
}

pub fn auto(p: &Pool, c: &Chain) -> Res<bool> {
    match c.comps.get(POUCH_REF).and_then(|v| norm(v)) {
        Some(f) => pouch_flag(p, &f, AUTO_GET),
        None => Ok(false),
    }
}

pub fn touch(p: &Pool, name: &str) -> Res<bool> {
    let c: Chain = chain(p, name)?;
    Ok(near(&c, &chain(p, BASE)?) && falls(p, &c)? && auto(p, &c)?)
}

fn child(p: &Pool, s: &Spec, own: &Chain) -> Res<Made> {
    let up: String = norm(need(own, &s.item, POUCH_REF)?).filter(|f| p.get(f).is_some()).ok_or_else(|| E::Msg(format!("{} keeps its pouch file outside its pack", s.item)))?;
    let version: u16 = p.byml(&actor_file(TOKEN))?.version;
    let path: String = pouch_file(&s.name);
    let base: Chain = chain(p, BASE)?;
    let mut comps: Node = Node::Map(Vec::new());
    comps.set(POUCH_REF, Node::Str(local(&path)));
    let mut swap: Vec<&str> = Vec::new();
    if !near(own, &base) {
        swap.push(ATTENTION_REF);
    }
    if !falls(p, own)? {
        swap.push(PHYSICS_REF);
    }
    for k in swap {
        let v: &str = need(&base, BASE, k)?;
        if !norm(v).is_some_and(|f| p.get(&f).is_some()) {
            return Err(E::Msg(format!("{} is taken with the button or hangs in the air, its kit needs the {k} of {BASE} and the pack of {BASE} was not given", s.item)));
        }
        comps.set(k, Node::Str(v.to_string()));
    }
    let mut actor: Node = Node::Map(Vec::new());
    actor.set(PARENT, Node::Str(work(&actor_file(&s.item))));
    actor.set(COMPONENTS, comps);
    let mut pouch: Node = Node::Map(Vec::new());
    pouch.set(PARENT, Node::Str(work(&up)));
    if !pouch_flag(p, &up, AUTO_GET)? {
        pouch.set(AUTO_GET, Node::Bool(true));
    }
    Ok(Made {
        actor: (actor_file(&s.name), Byml { root: actor, version, big: false }.write()?),
        pouch: (path, Byml { root: pouch, version, big: false }.write()?),
        identity: identity(p, s, own, &up)?,
        extra: Vec::new(),
    })
}

fn with_kit(p: &Pool, m: Made) -> Res<Made> {
    let mut a: Byml = Byml::parse(&m.actor.1)?;
    let comps: &mut Node = a.root.get_mut(COMPONENTS).ok_or(E::Format("pickup kit: the new actor has no component list"))?;
    for (k, v) in kit_refs(p)? {
        comps.set(k, Node::Str(v));
    }
    let mut q: Byml = Byml::parse(&m.pouch.1)?;
    q.root.set(MANY, Node::Bool(true));
    q.root.set(AT_MAX, Node::Bool(true));
    Ok(Made { actor: (m.actor.0, a.write()?), pouch: (m.pouch.0, q.write()?), identity: m.identity, extra: m.extra })
}

pub fn plan(packs: &[Vec<Entry>], s: &Spec) -> Res<Plan> {
    if !is_actor_name(&s.name) || !is_actor_name(&s.item) {
        return Err(E::Msg(format!("{} or {} is not a plain actor name", s.name, s.item)));
    }
    if s.style == Style::Press {
        return Err(E::Format("pickup kit: the press style is not built for kit pickups, a kit is always taken by touch and falls to the floor, so that it can be reached below a wall or vine skulltula"));
    }
    let mut p: Pool = pool_of(packs)?;
    let own: Chain = chain(&p, &s.item)?;
    let from_pickup: bool = own.files.contains(&actor_file(BASE));
    let tc: Chain = chain(&p, TOKEN)?;
    if s.item == TOKEN || REFS.iter().all(|k| own.comps.get(*k) == tc.comps.get(*k)) {
        return Err(E::Msg(format!("{} already carries the token kit", s.item)));
    }
    let peers: Vec<(String, usize)> = p.files.iter().map(|e| (e.0.clone(), e.1.len())).collect();
    let first: Made = if from_pickup {
        child(&p, s, &own)?
    } else {
        if let Some(f) = select(&p, &s.item)?.iter().find(|f| !p.get(f).is_some_and(is_byml)) {
            return Err(E::Msg(format!("{f} is not a parameter file, an item whose pack carries graph or binary files needs a hand made template")));
        }
        make(&p, s)?
    };
    let m: Made = with_kit(&p, first)?;
    let mut fresh: Vec<(String, usize)> = Vec::new();
    for (f, d) in [&m.actor, &m.pouch] {
        if p.get(f).is_some() {
            return Err(E::Msg(format!("{f} already exists, {} is taken", s.name)));
        }
        p.add(f, d)?;
        fresh.push((f.clone(), d.len()));
    }
    let files: Vec<String> = select(&p, &s.name)?;
    if let Some((f, _)) = fresh.iter().find(|f| !files.contains(&f.0)) {
        return Err(E::Msg(format!("{f} did not make it into the pack")));
    }
    if let Some(f) = files.iter().find(|f| !p.get(f).is_some_and(|d| is_byml(d) || leaf(d))) {
        return Err(E::Msg(format!("{f} is neither a parameter file nor an AI graph, the kit cannot carry it")));
    }
    let c: Chain = chain(&p, &s.name)?;
    let known: BS<String> = p.files.keys().cloned().collect();
    if let Some(f) = dangling_with(&p, &files, &known, &c)?.first() {
        return Err(E::Msg(format!("{f} is referenced but left out of the pack")));
    }
    let es: Vec<Entry> = files.iter().map(|f| Entry { name: f.clone(), data: p.get(f).unwrap_or(&[]).to_vec() }).collect();
    Ok(Plan { spec: s.clone(), files, fresh, peers, sarc: sarc::write(&es)?, identity: m.identity })
}

fn reach(a: &Ainb, from: usize, to: usize, seen: &mut BS<usize>) -> bool {
    if from == to {
        return true;
    }
    if from >= a.params.len() || !seen.insert(from) {
        return false;
    }
    let pl: [u8; 2] = a.params[from].pl[CHILDREN];
    let lo: usize = pl[1] as usize;
    let hi: usize = (lo + pl[0] as usize).min(a.params[from].plugs.len());
    (lo..hi).any(|j| reach(a, a.params[from].plugs[j].a as usize, to, seen))
}

pub fn reasons(graph_raw: &[u8]) -> Res<Vec<i32>> {
    let a: Ainb = Ainb::parse(graph_raw)?;
    let n: usize = a.els.len();
    let w: usize = (0..n).find(|i| a.el_name(*i) == WORKER).ok_or_else(|| E::Msg(format!("pickup kit: the graph has no {WORKER} node")))?;
    if !(0..n).any(|i| a.el_name(i) == SIGNAL) {
        return Err(E::Msg(format!("pickup kit: the graph has no {SIGNAL} node")));
    }
    let mut out: Vec<i32> = Vec::new();
    let mut gated: bool = false;
    for i in (0..n).filter(|i| a.el_type(*i) == SELECTOR) {
        let pl: [u8; 2] = a.params[i].pl[CHILDREN];
        let lo: usize = pl[1] as usize;
        let hi: usize = (lo + pl[0] as usize).min(a.params[i].plugs.len());
        for j in lo..hi {
            let g: &crate::ainb::Plug = &a.params[i].plugs[j];
            if !reach(&a, g.a as usize, w, &mut BS::new()) {
                continue;
            }
            gated = true;
            if j + 1 == hi {
                return Err(E::Format("pickup kit: the graph runs the skulltula row writer on every death reason, a pickup that dies uncollected would save the skulltula"));
            }
            let v: &[u8] = g.extra.get(4..8).ok_or(E::Format("pickup kit: a graph case has no value"))?;
            out.push(i32::from_le_bytes([v[0], v[1], v[2], v[3]]));
        }
    }
    if !gated {
        return Err(E::Format("pickup kit: the graph runs the skulltula row writer without asking for the death reason"));
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn starts(n: &Node, want: i64) -> bool {
    match n {
        Node::Bool(b) => i64::from(*b) == want,
        Node::I32(v) => i64::from(*v) == want,
        Node::U32(v) => i64::from(*v) == want,
        Node::I64(v) => *v == want,
        Node::U64(v) => i64::try_from(*v) == Ok(want),
        _ => false,
    }
}

fn declared(y: &Byml, list: &str, key: &str, want: i64) -> bool {
    let rows: &[Node] = y.root.get(list).and_then(|l| l.as_arr()).unwrap_or(&[]);
    rows.iter().any(|r| r.get("BBKey").and_then(|k| k.as_str()) == Some(key) && r.get("InitVal").is_some_and(|v| starts(v, want)))
}

fn pouch_flag(p: &Pool, first: &str, k: &str) -> Res<bool> {
    Ok(inherited(p, first, k)? == Some(Node::Bool(true)))
}

fn closure_of(p: &Pool, refs: &[String]) -> Res<Vec<String>> {
    let mut seen: BS<String> = BS::new();
    let mut todo: Vec<String> = refs.iter().filter_map(|r| norm(r)).collect();
    while let Some(f) = todo.pop() {
        let Some(d) = p.get(&f) else {
            continue;
        };
        if !seen.insert(f.clone()) || !is_byml(d) {
            continue;
        }
        todo.extend(crate::pickup_pack::closure::texts(d)?.iter().filter_map(|t| norm(t)));
    }
    Ok(seen.into_iter().collect())
}

pub fn is_kit(es: &[Entry], name: &str) -> bool {
    let Ok(p) = pool_of(&[es.to_vec()]) else {
        return false;
    };
    chain(&p, name).is_ok_and(|c| REFS.iter().any(|k| c.comps.get(*k).is_some_and(|v| v.contains("/Item_Token_Gold."))))
}

pub fn verify(es: &[Entry], name: &str, token: &[Entry]) -> Res<Vec<String>> {
    let p: Pool = pool_of(&[es.to_vec()])?;
    let t: Pool = pool_of(&[token.to_vec()])?;
    let c: Chain = chain(&p, name)?;
    let tc: Chain = chain(&t, TOKEN)?;
    let mut refs: Vec<String> = Vec::new();
    for k in REFS {
        let want: &str = need(&tc, TOKEN, k)?;
        if c.comps.get(k).map(|v| v.as_str()) != Some(want) {
            return Err(E::Msg(format!("{name} does not carry the token kit, its {k} is not the one of {TOKEN}, so a skulltula that drops it would never be saved")));
        }
        refs.push(want.to_string());
    }
    let mut names: Vec<String> = es.iter().map(|e| e.name.clone()).collect();
    names.sort();
    if select(&p, name)? != names {
        return Err(E::Msg(format!("{name} holds a file its actor does not reach")));
    }
    let shared: Vec<String> = closure_of(&t, &refs)?;
    for f in &shared {
        if p.get(f) != t.get(f) {
            return Err(E::Msg(format!("{name} does not hold the shipped {f}")));
        }
    }
    let graphs: Vec<&String> = shared.iter().filter(|f| t.get(f).is_some_and(leaf)).collect();
    let [g] = graphs.as_slice() else {
        return Err(E::Msg(format!("the token kit of {TOKEN} has {} AI graphs, one was expected", graphs.len())));
    };
    let on: Vec<i32> = reasons(t.get(g).unwrap_or(&[]))?;
    if on != vec![COLLECTED] {
        return Err(E::Msg(format!("the token graph writes the skulltula rows on death reasons {on:?}, only {COLLECTED} (the end of the get event) is safe")));
    }
    let mut key: bool = false;
    let mut open: bool = false;
    for f in shared.iter().filter(|f| f.contains("/BlackboardParamTable/")) {
        let y: Byml = p.byml(f)?;
        key |= declared(&y, "BlackboardParamU64Array", HASH_KEY, 0);
        open |= declared(&y, "BlackboardParamBoolArray", ATTENTION, 1);
    }
    if !key || !open {
        return Err(E::Msg(format!("the token blackboard no longer declares {HASH_KEY} with 0 and {ATTENTION} with true")));
    }
    let pouch: String = c.comps.get(POUCH_REF).and_then(|v| norm(v)).ok_or_else(|| E::Msg(format!("{name} has no pouch file")))?;
    for k in [MANY, AT_MAX] {
        if !pouch_flag(&p, &pouch, k)? {
            return Err(E::Msg(format!("{name} does not set {k}, so it could be taken without the get event and the skulltula would never be saved")));
        }
    }
    if !touch(&p, name)? {
        return Err(E::Msg(format!("{name} is taken with the button or hangs in the air ({ATTENTION_REF}, {PHYSICS_REF} or {AUTO_GET}), below a wall or vine skulltula it could not be reached")));
    }
    let mut bodies: usize = 0;
    for e in es.iter().filter(|e| e.name.starts_with(SETS)) {
        let Ok(y) = Byml::parse(&e.data) else {
            continue;
        };
        for a in y.root.get("AliasEntityList").and_then(|l| l.as_arr()).unwrap_or(&[]) {
            let got: (Option<&str>, Option<&str>) = (a.get("ControllerName").and_then(|v| v.as_str()), a.get("RigidBodyName").and_then(|v| v.as_str()));
            if got != (Some(BODY), Some(BODY)) {
                return Err(E::Msg(format!("{name} has a rigid body the token graph does not know ({got:?})")));
            }
            bodies += 1;
        }
    }
    if bodies == 0 {
        return Err(E::Msg(format!("{name} has no rigid body named {BODY}, the token graph needs one")));
    }
    Ok(vec![
        format!("{name} carries the token kit: {} shipped files byte for byte, graph {g}", shared.len()),
        format!("{name}: the skulltula rows are written only on death reason {COLLECTED}, the end of the get event, and {MANY} and {AT_MAX} make every pickup play it"),
        format!("{name}: blackboard key {HASH_KEY} starts at 0 and {ATTENTION} starts true, rigid body {BODY} found, taken by touch and every body falls"),
    ])
}

pub fn token_pack(rf: &mut Romfs) -> Res<Vec<Entry>> {
    sarc::parse(&rf.read_zs(&pack_of(TOKEN))?)
}

pub fn world(rf: &mut Romfs) -> Res<Vec<String>> {
    let life: Cell = cell_of(&death_chain(rf, DEATH_TABLE)?, "LifeDepleted");
    if !life.dead || life.ignored {
        return Err(E::Msg(format!("the death table {DEATH_TABLE} no longer removes a collected pickup, the game files differ")));
    }
    let es: Vec<Entry> = sarc::parse(&rf.read_zs(&pack_of(SKULL))?)?;
    let g: &Entry = sarc::get(&es, SKULL_GRAPH).ok_or_else(|| E::Msg(format!("{SKULL} has no {SKULL_GRAPH}, the game files differ")))?;
    if !g.data.windows(HASH_KEY.len()).any(|w| w == HASH_KEY.as_bytes()) {
        return Err(E::Msg(format!("{SKULL} no longer hands {HASH_KEY} to its drop, the game files differ")));
    }
    let dies: Cell = cell_of(&death_chain(rf, SKULL)?, "LifeDepleted");
    if !dies.dead || dies.saved {
        return Err(E::Msg(format!("the death table of {SKULL} writes the saved row when the skulltula dies, an item left behind would be lost")));
    }
    Ok(vec![
        format!("{SKULL} hands {HASH_KEY} to its drop and its own death writes no saved row"),
        format!("death table {DEATH_TABLE}: a collected pickup is removed"),
    ])
}

pub fn clean_base(rf: &Romfs, base: Option<&PB>) -> Res<()> {
    let Some(f) = base.map(|b| b.join(SKULL_DEATH)).filter(|f| f.is_file()) else {
        return Ok(());
    };
    if std::fs::read(&f)? != rf.read(SKULL_DEATH)? {
        return Err(E::Msg(format!("{} carries an edited skulltula death table that writes the saved row at death, remove that file first", f.display())));
    }
    Ok(())
}

pub fn rebodied(pl: &Plan) -> Res<bool> {
    let es: Vec<Entry> = sarc::parse(&pl.sarc)?;
    let f: String = actor_file(&pl.spec.name);
    let e: &Entry = sarc::get(&es, &f).ok_or_else(|| E::Msg(format!("{f} is not in the kit pack")))?;
    Ok(Byml::parse(&e.data)?.root.get(COMPONENTS).and_then(|c| c.get(PHYSICS_REF)).is_some())
}

pub fn copy_rows(ys: &mut [Byml; 3], pl: &Plan) -> Res<usize> {
    let (pickup, name): (&str, &str) = (&pl.spec.item, &pl.spec.name);
    let layer: Option<Node> = if rebodied(pl)? { rows::need(&ys[1], BASE, GAME_ACTOR_INFO)?.get(rows::LAYER).cloned() } else { None };
    let mut at: [usize; 3] = [0; 3];
    for (i, t) in [ACTOR_INFO, GAME_ACTOR_INFO, POUCH_INFO].iter().enumerate() {
        let mut r: Node = rows::need(&ys[i], pickup, t)?.clone();
        r.set(rows::ROW_ID, Node::Str(name.to_string()));
        if let (1, Some(v)) = (i, layer.as_ref()) {
            r.set(rows::LAYER, v.clone());
        }
        at[i] = rows::insert(&mut ys[i], r)?;
    }
    if at[0] != at[1] {
        return Err(E::Msg(format!("the new row sits at {} in the actor table and at {} in the game actor table, the two tables no longer line up", at[0], at[1])));
    }
    if rows::find(&ys[1], name).and_then(|r| r.get(rows::RESPAWN)) != Some(&Node::Bool(true)) {
        return Err(E::Msg(format!("{pickup} has no IsRespawn row to copy")));
    }
    Ok(at[0])
}

pub fn fresh_sizes(rst: &mut Rst, pl: &Plan, shared: &[String]) -> Res<Vec<(String, u32)>> {
    let mut out: Vec<(String, u32)> = vec![(pack_of(&pl.spec.name), sizes::pack_size(pl.sarc.len())?)];
    for (f, len) in &pl.fresh {
        out.push((f.clone(), sizes::file_size(rst, &pl.peers, f, *len)?));
    }
    for (f, n) in &out {
        rst.set_size(f, *n)?;
    }
    if let Some(f) = shared.iter().find(|f| rst.get(f).is_none()) {
        return Err(E::Msg(format!("{f} has no size table entry, the game files differ")));
    }
    Ok(out)
}

pub fn shared_files(token: &[Entry]) -> Res<Vec<String>> {
    let t: Pool = pool_of(&[token.to_vec()])?;
    let tc: Chain = chain(&t, TOKEN)?;
    let mut refs: Vec<String> = Vec::new();
    for k in REFS {
        refs.push(need(&tc, TOKEN, k)?.to_string());
    }
    closure_of(&t, &refs)
}

pub fn sources(rf: &mut Romfs, item: &str, token: &[Entry]) -> Res<Vec<Vec<Entry>>> {
    let own: Vec<Entry> = sarc::parse(&rf.read_zs(&pack_of(item))?)?;
    let mut packs: Vec<Vec<Entry>> = Vec::new();
    if !shipped(&own) {
        for n in [BASE, DONOR] {
            packs.push(sarc::parse(&rf.read_zs(&pack_of(n))?)?);
        }
    } else if !touch(&pool_of(&[own.clone()])?, item)? {
        packs.push(sarc::parse(&rf.read_zs(&pack_of(BASE))?)?);
    }
    packs.push(own);
    packs.push(token.to_vec());
    Ok(packs)
}

fn shipped_tables(rf: &mut Romfs, o: &Opts, modded: bool, pl: &Plan) -> Res<(Vec<File>, i32)> {
    let s: &Spec = &pl.spec;
    let mut ys: Vec<Byml> = Vec::new();
    for t in [ACTOR_INFO, GAME_ACTOR_INFO, POUCH_INFO] {
        ys.push(Byml::parse(&start(rf, o.base.as_ref(), t)?)?);
    }
    let mut three: [Byml; 3] = ys.try_into().map_err(|_| E::Format("pickup kit: three tables were expected"))?;
    copy_rows(&mut three, pl)?;
    let heap: i32 = match rows::find(&three[0], &s.name).and_then(|r| r.get(rows::HEAP)) {
        Some(Node::I32(v)) => *v,
        Some(Node::U32(v)) => i32::try_from(*v).unwrap_or(i32::MAX),
        _ => 0,
    };
    let mut out: Vec<File> = Vec::new();
    for (i, t) in [ACTOR_INFO, GAME_ACTOR_INFO, POUCH_INFO].iter().enumerate() {
        let done: &Byml = &three[i];
        out.push(table(rf, o, modded, t, |y: &mut Byml| {
            *y = done.clone();
            Ok(())
        })?);
    }
    Ok((out, heap))
}

fn drop_files(rf: &mut Romfs, o: &Opts, modded: bool, name: &str, drops: &[u64], note: &mut String) -> Res<Vec<File>> {
    let mut out: Vec<File> = Vec::new();
    if drops.is_empty() {
        return Ok(out);
    }
    let list: Byml = Byml::parse(&start(rf, o.base.as_ref(), LIST)?)?;
    for h in drops {
        place::permanent(&list, *h)?;
        place::permanent(&list, *h ^ 2)?;
        let _ = writeln!(note, "skulltula {h}: both saved rows are permanent ({:#018x} and {:#018x})", h, h ^ 2);
    }
    let mut hit: BM<u64, usize> = BM::new();
    for f in banc_files(rf)? {
        let first: Vec<u8> = start(rf, o.base.as_ref(), &f)?;
        if !drops.iter().any(|h| place::has_hash(&first, *h)) {
            continue;
        }
        let mut y: Byml = Byml::parse(&first)?;
        let mut n: usize = 0;
        for h in drops {
            let k: usize = crate::slice::skull_edit::set_drop(&mut y, *h, name)?;
            if k > 0 {
                *hit.entry(*h).or_insert(0) += k;
                n += k;
                let _ = writeln!(note, "skulltula {h} in {f}: drops {name}");
            }
        }
        if n == 0 {
            continue;
        }
        let raw: Vec<u8> = y.write()?;
        let id: u32 = dict_id(&rf.read(&f)?).unwrap_or(BANC_DICT);
        let old: usize = if modded { first.len() } else { rf.read_zs(&f)?.len() };
        out.push(File { path: f.clone(), z: zs::compress_raw(&raw, Some(id))?, old, raw, fresh: false });
    }
    if let Some(h) = drops.iter().find(|h| !hit.contains_key(h)) {
        return Err(E::Msg(format!("no placed actor has the hash {h}")));
    }
    Ok(out)
}

pub fn run(romfs: &P, load: &P, mod_name: &str, s: &Spec, o: &Opts, drops: &[u64]) -> Res<Done> {
    if !is_actor_name(&s.name) || !is_actor_name(&s.item) {
        return Err(E::Msg(format!("{} or {} is not a plain actor name", s.name, s.item)));
    }
    if !o.places.is_empty() {
        return Err(E::Format("pickup kit: a kit pickup is only for skulltula drops, at a placed spot it would write the saved rows of no skulltula"));
    }
    let o: &Opts = &Opts { base: base_dir(o)?, ..o.clone() };
    let plain: Layers = Layers::default();
    let mut rf: Romfs = Romfs::open(romfs)?;
    if rf.root.join(pack_of(&s.name)).is_file() {
        return Err(E::Msg(format!("the game already ships {}, use that actor", s.name)));
    }
    if !rf.root.join(pack_of(&s.item)).is_file() {
        return Err(E::Msg(format!("the game has no actor pack for {}", s.item)));
    }
    clean_base(&rf, o.base.as_ref())?;
    let token: Vec<Entry> = token_pack(&mut rf)?;
    let packs: Vec<Vec<Entry>> = sources(&mut rf, &s.item, &token)?;
    let from_pickup: bool = packs.iter().rev().nth(1).is_some_and(|es| shipped(es));
    let pl: Plan = plan(&packs, s)?;
    let (mut rst, modded): (Rst, bool) = size_table(&mut rf, o)?;
    let mut note: String = String::new();
    let id: u32 = dict_id(&rf.read(&pack_of(BASE))?).unwrap_or(PACK_DICT);
    let mut files: Vec<File> = vec![File { path: pack_of(&s.name), z: zs::compress_raw(&pl.sarc, Some(id))?, raw: pl.sarc.clone(), old: 0, fresh: true }];
    let (ts, heap): (Vec<File>, i32) = if from_pickup { shipped_tables(&mut rf, o, modded, &pl)? } else { tables(&mut rf, o, modded, s, &plain, &pl.identity)? };
    rows::need(&Byml::parse(&ts[2].raw)?, &pl.identity, POUCH_INFO)?;
    let _ = writeln!(note, "pouch identity {}, the name the save and the get window use", pl.identity);
    files.extend(ts);
    files.extend(drop_files(&mut rf, o, modded, &s.name, drops, &mut note)?);
    let carried: Vec<File> = earlier(&mut rf, o.base.as_ref(), &pack_of(&s.name))?;
    let kept: Vec<File> = kept(&mut rf, o.base.as_ref(), &files)?;
    sized(&mut rst, &pl, &files, &carried, &mut note)?;
    if let Some(f) = shared_files(&token)?.iter().find(|f| rst.get(f).is_none()) {
        return Err(E::Msg(format!("{f} has no size table entry, the game files differ")));
    }
    let rst_z: Vec<u8> = rst.to_zs()?;
    let sc: Scope = Scope { carried: &carried, places: &[], base: o.base.as_ref(), layers: &plain, kept: &kept };
    for l in audit(&mut rf, s, &pl, &files, &rst_z, &sc)? {
        let _ = writeln!(note, "{l}");
    }
    for l in verify(&sarc::parse(&pl.sarc)?, &s.name, &token)?.into_iter().chain(world(&mut rf)?) {
        let _ = writeln!(note, "{l}");
    }
    for h in drops {
        let n: usize = files.iter().filter(|f| f.path.starts_with("Banc/")).filter(|f| Byml::parse(&f.raw).is_ok_and(|y| dropped(&y, *h).as_deref() == Some(s.name.as_str()))).count();
        if n == 0 {
            return Err(E::Msg(format!("audit: skulltula {h} does not drop {}", s.name)));
        }
        let _ = writeln!(note, "skulltula {h} drops {} in {n} file(s)", s.name);
    }
    let mut root: Option<PB> = None;
    if !o.dry {
        let pk: Pack = Pack::create_for(load, TITLE_ID, mod_name, o.host)?;
        for f in files.iter().chain(carried.iter()).chain(kept.iter()) {
            pk.write_romfs(&f.path, &f.z)?;
        }
        pk.write_romfs(TABLE_PATH, &rst_z)?;
        let _ = writeln!(note, "wrote {} files to {}", files.len() + carried.len() + kept.len() + 1, pk.root.display());
        root = Some(pk.root);
    }
    Ok(Done { report: note, root, files, carried, heap, kept })
}

pub fn dropped(y: &Byml, hash: u64) -> Option<String> {
    let list: &[Node] = y.root.get("Actors").and_then(|a| a.as_arr())?;
    let a: &Node = list.iter().find(|a| crate::placement::hash_of(a) == Some(hash))?;
    a.get("Dynamic").and_then(|d| d.get(crate::pots::DROP)).and_then(|v| v.as_str()).map(|v| v.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::modpack::Host;
    use crate::pickup_pack::actor::tests::{pool, spec};
    use crate::pickup_pack::write::run as plain_run;
    use crate::pickup_persist::{check_swaps_with, Extra};

    const TREE: u64 = 2504273934093629549;
    const RUPEE_SPOT: u64 = 11219803099333988363;
    const FIVE: [&str; 5] = [
        "AI/AIInfo/Item_Token_Gold.engine__actor__AIInfo.bgyml",
        "AI/Item_Token_Gold.root.ainb",
        "Component/Blackboard/BlackboardInfo/Item_Token_Gold.engine__component__BlackboardInfo.bgyml",
        "Component/Blackboard/BlackboardParamTable/Item_Token_Gold.engine__component__BlackboardParamTable.bgyml",
        "Component/DeathParam/Item_Token_Gold.game__component__DeathParam.bgyml",
    ];
    const OUT: [&str; 5] = [
        "AI/AIInfo/DropItem.engine__actor__AIInfo.bgyml",
        "Component/Blackboard/BlackboardInfo/Obj_Item_Base.engine__component__BlackboardInfo.bgyml",
        "Component/Blackboard/BlackboardParamTable/CalcInstanceProbeOnActorModelAABBCenter.engine__component__BlackboardParamTable.bgyml",
        "Component/Blackboard/BlackboardParamTable/DropItem.engine__component__BlackboardParamTable.bgyml",
        "Component/DeathParam/DropItem.game__component__DeathParam.bgyml",
    ];

    fn file(pairs: &[(&str, Node)]) -> Vec<u8> {
        let mut root: Node = Node::Map(Vec::new());
        for (k, v) in pairs {
            root.set(k, v.clone());
        }
        Byml { root, version: 7, big: false }.write().unwrap()
    }

    fn s(x: &str) -> Node {
        Node::Str(x.to_string())
    }

    fn count(n: &Node) -> usize {
        match n {
            Node::Map(m) => m.len(),
            _ => 0,
        }
    }

    fn toy_token() -> Vec<Entry> {
        let mut comps: Node = Node::Map(Vec::new());
        comps.set("AIInfoRef", s("?AI/AIInfo/Tok.engine__actor__AIInfo.bgyml"));
        comps.set("BlackboardRef", s("?Component/Blackboard/Tok.bgyml"));
        comps.set("DeathRef", s("?Component/DeathParam/Tok.bgyml"));
        comps.set(LIFE_REF, s("?ActorSystem/GameLifeCondition/Far.bgyml"));
        vec![
            Entry { name: actor_file(TOKEN), data: file(&[(PARENT, s(&work(&actor_file(BASE)))), (COMPONENTS, comps)]) },
            Entry { name: "AI/AIInfo/Tok.engine__actor__AIInfo.bgyml".to_string(), data: file(&[("RootAIRef", s("Work/AI/Root/Tok.root.ain"))]) },
            Entry { name: "AI/Tok.root.ainb".to_string(), data: b"AIB toy".to_vec() },
            Entry { name: "Component/Blackboard/Tok.bgyml".to_string(), data: file(&[("A", Node::Bool(true))]) },
            Entry { name: "Component/DeathParam/Tok.bgyml".to_string(), data: file(&[("B", Node::Bool(true))]) },
        ]
    }

    fn toy_packs() -> Vec<Vec<Entry>> {
        vec![pool().files.into_iter().map(|e| Entry { name: e.0, data: e.1 }).collect(), toy_token()]
    }

    fn kit_spec() -> Spec {
        Spec { name: "Obj_Test_GS".to_string(), ..spec(Style::Touch) }
    }

    fn real() -> Option<PB> {
        let p: PB = std::env::var("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|_| PB::from("C:/o/nup dump/updates/131072/romfs"));
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Some(p)
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    fn load(rf: &mut Romfs, name: &str) -> Vec<Entry> {
        sarc::parse(&rf.read_zs(&pack_of(name)).unwrap()).unwrap()
    }

    fn opts(base: Option<PB>, dry: bool) -> Opts {
        Opts { host: Host::Hookshot, base, rst_from: None, places: Vec::new(), dry }
    }

    fn kit(item: &str) -> Spec {
        Spec { item: item.to_string(), name: default_kit(item), style: Style::Touch, heap: None }
    }

    fn temp(tag: &str) -> PB {
        let p: PB = std::env::temp_dir().join(format!("ootrr_kit_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        p
    }

    fn listed(base: &P) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut todo: Vec<PB> = vec![base.to_path_buf()];
        while let Some(d) = todo.pop() {
            for e in fs::read_dir(&d).unwrap() {
                let p: PB = e.unwrap().path();
                if p.is_dir() {
                    todo.push(p);
                } else {
                    out.push(p.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/"));
                }
            }
        }
        out.sort();
        out
    }

    #[test]
    fn names_and_graph_references_follow_the_shipped_pattern() {
        assert_eq!(name_of("Obj_Rupee_Red"), "Obj_Rupee_Red_GS");
        assert_eq!(default_kit("Item_Hookshot"), "Obj_Hookshot_GS");
        assert_eq!(default_kit("Obj_Heart"), "Obj_Heart_GS");
        assert_eq!(parent_of("Obj_Heart_GS"), Some("Obj_Heart"));
        assert_eq!(parent_of("Obj_Heart"), None);
        assert_eq!(parent_of("_GS"), None);
        assert_eq!(graph("AI/Root/Item_Token_Gold.root.ain").as_deref(), Some("AI/Item_Token_Gold.root.ainb"));
        assert_eq!(graph("AI/Root/Event/PouchContent_EnableWarp.event.root.ain").as_deref(), Some("AI/PouchContent_EnableWarp.event.root.ainb"));
        assert_eq!(graph("AS/Item_Common.root.asb"), None);
        assert_eq!(graph("AI/Root/.ain"), None);
        assert_eq!(norm("Work/AI/Root/Item_Token_Gold.root.ain").as_deref(), Some("AI/Item_Token_Gold.root.ainb"));
        assert!(leaf(b"AIB \x08\x04") && !leaf(b"YB\x07\x00") && !leaf(b"AI"));
    }

    #[test]
    fn a_new_item_kit_keeps_the_item_parts_and_takes_the_four_token_references() {
        let pl: Plan = plan(&toy_packs(), &kit_spec()).unwrap();
        assert!(pl.files.contains(&"AI/Tok.root.ainb".to_string()));
        assert!(pl.files.contains(&"AI/AIInfo/Tok.engine__actor__AIInfo.bgyml".to_string()));
        assert!(!pl.files.contains(&actor_file(TOKEN)));
        let es: Vec<Entry> = sarc::parse(&pl.sarc).unwrap();
        let a: Byml = Byml::parse(&sarc::get(&es, &actor_file("Obj_Test_GS")).unwrap().data).unwrap();
        let c: &Node = a.root.get(COMPONENTS).unwrap();
        assert_eq!(c.get("AIInfoRef").and_then(|v| v.as_str()), Some("?AI/AIInfo/Tok.engine__actor__AIInfo.bgyml"));
        assert_eq!(c.get("DeathRef").and_then(|v| v.as_str()), Some("?Component/DeathParam/Tok.bgyml"));
        assert_eq!(c.get(LIFE_REF).and_then(|v| v.as_str()), Some("?ActorSystem/GameLifeCondition/Far.bgyml"));
        assert_eq!(c.get("ModelInfoRef").and_then(|v| v.as_str()), Some("?Component/ModelInfo/Item_Test.engine__component__ModelInfo.bgyml"));
        let q: Byml = Byml::parse(&sarc::get(&es, &pouch_file("Obj_Test_GS")).unwrap().data).unwrap();
        assert_eq!(q.root.get(MANY), Some(&Node::Bool(true)));
        assert_eq!(q.root.get(AT_MAX), Some(&Node::Bool(true)));
        assert_eq!(q.root.get("PouchActor").and_then(|v| v.as_str()), Some("Work/Actor/Item_Test.engine__actor__ActorParam.gyml"));
        assert!(!is_kit(&es, "Obj_Nobody"));
    }

    #[test]
    fn a_shipped_pickup_kit_is_a_thin_child_with_its_own_pouch_file() {
        let sp: Spec = Spec { item: DONOR.to_string(), name: name_of(DONOR), style: Style::Touch, heap: None };
        let mut packs: Vec<Vec<Entry>> = toy_packs();
        packs[0].push(Entry { name: pouch_file(DONOR), data: file(&[("A", Node::Bool(true))]) });
        let i: usize = packs[0].iter().position(|e| e.name == actor_file(DONOR)).unwrap();
        let mut y: Byml = Byml::parse(&packs[0][i].data).unwrap();
        y.root.get_mut(COMPONENTS).unwrap().set(POUCH_REF, s(&local(&pouch_file(DONOR))));
        packs[0][i].data = y.write().unwrap();
        let pl: Plan = plan(&packs, &sp).unwrap();
        let es: Vec<Entry> = sarc::parse(&pl.sarc).unwrap();
        let a: Byml = Byml::parse(&sarc::get(&es, &actor_file(&sp.name)).unwrap().data).unwrap();
        assert_eq!(a.root.get(PARENT).and_then(|v| v.as_str()), Some(work(&actor_file(DONOR)).as_str()));
        let c: &Node = a.root.get(COMPONENTS).unwrap();
        assert_eq!(count(c), 7);
        assert_eq!(c.get(ATTENTION_REF).and_then(|v| v.as_str()), Some("?Component/Attention/Auto.game__component__AttentionSenderParam.bgyml"));
        assert_eq!(c.get(PHYSICS_REF).and_then(|v| v.as_str()), Some("?Component/Physics/Obj_Item_Base.engine__component__PhysicsParam.bgyml"));
        let q: Byml = Byml::parse(&sarc::get(&es, &pouch_file(&sp.name)).unwrap().data).unwrap();
        assert_eq!(q.root.get(PARENT).and_then(|v| v.as_str()), Some(work(&pouch_file(DONOR)).as_str()));
        assert_eq!(q.root.get(MANY), Some(&Node::Bool(true)));
        assert_eq!(q.root.get(AUTO_GET), Some(&Node::Bool(true)));
        assert!(q.root.get("PouchActor").is_none());
        assert!(pl.files.contains(&actor_file(DONOR)) && pl.files.contains(&actor_file(BASE)));
        assert!(!pl.files.iter().any(|f| f.contains("Attention/Press")));
        assert_eq!(pl.identity, "Item_Arrow_Fire");
        assert!(rebodied(&pl).unwrap());
    }

    fn soft(motion: u32, auto_get: bool) -> Vec<Vec<Entry>> {
        let mut packs: Vec<Vec<Entry>> = toy_packs();
        let mut comps: Node = Node::Map(Vec::new());
        comps.set(POUCH_REF, s(&local(&pouch_file("Obj_Soft"))));
        comps.set(PHYSICS_REF, s("?Component/Physics/Soft.engine__component__PhysicsParam.bgyml"));
        let body: Node = Node::Map(vec![(FILE_PATH.to_string(), s("Work/Phive/RigidBodyEntityParam/Soft_0.phive__RigidBodyEntityParam.gyml"))]);
        packs[0].extend([
            Entry { name: actor_file("Obj_Soft"), data: file(&[(PARENT, s(&work(&actor_file(BASE)))), (COMPONENTS, comps)]) },
            Entry { name: pouch_file("Obj_Soft"), data: file(&[(AUTO_GET, Node::Bool(auto_get))]) },
            Entry { name: "Component/Physics/Soft.engine__component__PhysicsParam.bgyml".to_string(), data: file(&[(SET_PATH, s("Work/Phive/ControllerSetParam/Soft.phive__ControllerSetParam.gyml"))]) },
            Entry { name: "Phive/ControllerSetParam/Soft.phive__ControllerSetParam.bgyml".to_string(), data: file(&[(BODIES, Node::Arr(vec![body]))]) },
            Entry { name: "Phive/RigidBodyEntityParam/Soft_0.phive__RigidBodyEntityParam.bgyml".to_string(), data: file(&[(MOTION, Node::U32(motion))]) },
        ]);
        packs
    }

    fn own_parts(packs: &[Vec<Entry>]) -> (Vec<String>, Option<Node>) {
        let sp: Spec = Spec { item: "Obj_Soft".to_string(), name: "Obj_Soft_GS".to_string(), style: Style::Touch, heap: None };
        let pl: Plan = plan(packs, &sp).unwrap();
        let es: Vec<Entry> = sarc::parse(&pl.sarc).unwrap();
        let a: Byml = Byml::parse(&sarc::get(&es, &actor_file(&sp.name)).unwrap().data).unwrap();
        let Some(Node::Map(m)) = a.root.get(COMPONENTS) else {
            panic!("no components");
        };
        let q: Byml = Byml::parse(&sarc::get(&es, &pouch_file(&sp.name)).unwrap().data).unwrap();
        (m.iter().map(|e| e.0.clone()).collect(), q.root.get(AUTO_GET).cloned())
    }

    #[test]
    fn a_touch_pickup_stays_a_five_line_child_and_each_missing_part_is_added_alone() {
        const KINEMATIC: u32 = 4269567940;
        assert_eq!(crate::murmur::murmur3_32(b"Dynamic", 0), DYNAMIC);
        assert_eq!(crate::murmur::murmur3_32(b"Kinematic", 0), KINEMATIC);
        let p: Pool = pool_of(&soft(DYNAMIC, true)).unwrap();
        assert!(touch(&p, "Obj_Soft").unwrap());
        assert!(!touch(&p, DONOR).unwrap() && !touch(&p, BASE).unwrap());
        let (keys, auto_get): (Vec<String>, Option<Node>) = own_parts(&soft(DYNAMIC, true));
        assert_eq!(keys.len(), 5);
        assert_eq!(auto_get, None);
        let (keys, auto_get): (Vec<String>, Option<Node>) = own_parts(&soft(KINEMATIC, true));
        assert_eq!(keys.len(), 6);
        assert!(keys.contains(&PHYSICS_REF.to_string()) && !keys.contains(&ATTENTION_REF.to_string()));
        assert_eq!(auto_get, None);
        let (keys, auto_get): (Vec<String>, Option<Node>) = own_parts(&soft(DYNAMIC, false));
        assert_eq!(keys.len(), 5);
        assert_eq!(auto_get, Some(Node::Bool(true)));
        let hung: Pool = pool_of(&soft(KINEMATIC, true)).unwrap();
        assert!(!falls(&hung, &chain(&hung, "Obj_Soft").unwrap()).unwrap());
        assert!(auto(&hung, &chain(&hung, "Obj_Soft").unwrap()).unwrap());
    }

    #[test]
    fn the_token_a_press_kit_a_taken_name_and_a_foreign_binary_are_refused() {
        let tok: Spec = Spec { item: TOKEN.to_string(), name: "Obj_Token_Gold_GS".to_string(), style: Style::Touch, heap: None };
        assert!(plan(&toy_packs(), &tok).is_err());
        assert!(plan(&toy_packs(), &Spec { style: Style::Press, ..kit_spec() }).is_err());
        assert!(plan(&toy_packs(), &Spec { name: "Item_Test".to_string(), ..kit_spec() }).is_err());
        assert!(plan(&toy_packs(), &Spec { name: "Obj/Test".to_string(), ..kit_spec() }).is_err());
        let mut odd: Vec<Vec<Entry>> = toy_packs();
        let i: usize = odd[1].iter().position(|e| e.name == "AI/Tok.root.ainb").unwrap();
        odd[1][i].data = b"BFEV".to_vec();
        assert!(plan(&odd, &kit_spec()).is_err());
        assert!(reasons(b"AIB toy").is_err());
    }

    #[test]
    fn the_file_rule_with_graphs_rebuilds_the_real_graph_carrying_pickups() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        for name in [TOKEN, "Obj_Rupee_Green", "Obj_Rupee_Red", "Obj_Heart", "Obj_Heart_Piece", "Obj_Heart_Container"] {
            let es: Vec<Entry> = load(&mut rf, name);
            let mut want: Vec<String> = es.iter().map(|e| e.name.clone()).collect();
            want.sort();
            assert_eq!(select(&pool_of(&[es]).unwrap(), name).unwrap(), want, "{name}");
        }
    }

    #[test]
    fn the_real_token_graph_writes_the_rows_only_at_the_end_of_the_get_event() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let token: Vec<Entry> = token_pack(&mut rf).unwrap();
        let raw: &[u8] = &sarc::get(&token, FIVE[1]).unwrap().data;
        assert_eq!(reasons(raw).unwrap(), vec![COLLECTED]);
        let mut a: Ainb = Ainb::parse(raw).unwrap();
        assert_eq!(a.write().unwrap(), raw);
        let i: usize = (0..a.els.len()).find(|i| a.el_type(*i) == SELECTOR).unwrap();
        a.params[i].plugs[0].extra[4] = 4;
        assert_eq!(reasons(&a.write().unwrap()).unwrap(), vec![4]);
        a.params[i].plugs.swap(0, 1);
        let e: String = reasons(&a.write().unwrap()).unwrap_err().to_string();
        assert!(e.contains("every death reason"), "{e}");
        let piece: Vec<Entry> = load(&mut rf, "Obj_Heart_Piece");
        assert!(reasons(&sarc::get(&piece, "AI/Obj_Heart_Piece.root.ainb").unwrap().data).is_err());
        assert_eq!(shared_files(&token).unwrap(), FIVE);
        assert_eq!(world(&mut rf).unwrap().len(), 2);
    }

    #[test]
    fn the_real_hookshot_kit_is_the_plain_pack_with_five_files_out_and_five_token_files_in() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let token: Vec<Entry> = token_pack(&mut rf).unwrap();
        let src: Vec<Vec<Entry>> = vec![load(&mut rf, BASE), load(&mut rf, DONOR), load(&mut rf, "Item_Hookshot")];
        let plain: Plan = crate::pickup_pack::build::plan(&src, &Spec { name: "Obj_Hookshot".to_string(), ..kit("Item_Hookshot") }).unwrap();
        let mut with: Vec<Vec<Entry>> = src.clone();
        with.push(token.clone());
        let pl: Plan = plan(&with, &kit("Item_Hookshot")).unwrap();
        assert_eq!((plain.files.len(), pl.files.len()), (40, 40));
        let a: BS<String> = plain.files.iter().cloned().collect();
        let b: BS<String> = pl.files.iter().map(|f| f.replace("Obj_Hookshot_GS", "Obj_Hookshot")).collect();
        assert_eq!(a.difference(&b).map(|f| f.as_str()).collect::<Vec<&str>>(), OUT);
        assert_eq!(b.difference(&a).map(|f| f.as_str()).collect::<Vec<&str>>(), FIVE);
        let es: Vec<Entry> = sarc::parse(&pl.sarc).unwrap();
        for f in FIVE {
            assert_eq!(sarc::get(&es, f).map(|e| &e.data), sarc::get(&token, f).map(|e| &e.data), "{f}");
        }
        assert_eq!(verify(&es, "Obj_Hookshot_GS", &token).unwrap().len(), 3);
        assert!(is_kit(&es, "Obj_Hookshot_GS"));
        let bare: Vec<Entry> = sarc::parse(&plain.sarc).unwrap();
        assert!(!is_kit(&bare, "Obj_Hookshot"));
        assert!(verify(&bare, "Obj_Hookshot", &token).is_err());
        let y: Byml = Byml::parse(&sarc::get(&es, &actor_file("Obj_Hookshot_GS")).unwrap().data).unwrap();
        let c: &Node = y.root.get(COMPONENTS).unwrap();
        assert_eq!(count(c), 10);
        assert_eq!(c.get(LIFE_REF).and_then(|v| v.as_str()), Some("?ActorSystem/GameLifeCondition/Landmark300m.game__actor__LifeCondition.bgyml"));
        let mut weak: Vec<Entry> = es.clone();
        let q: &mut Entry = weak.iter_mut().find(|e| e.name == pouch_file("Obj_Hookshot_GS")).unwrap();
        let mut qy: Byml = Byml::parse(&q.data).unwrap();
        qy.root.set(MANY, Node::Bool(false));
        q.data = qy.write().unwrap();
        let e: String = verify(&weak, "Obj_Hookshot_GS", &token).unwrap_err().to_string();
        assert!(e.contains(MANY), "{e}");
    }

    #[test]
    fn the_real_kits_of_the_hookshot_a_rupee_and_a_heart_pass_the_audit() {
        let Some(root) = real() else {
            return;
        };
        for (item, files) in [("Item_Hookshot", 40), ("Obj_Rupee_Red", 57), ("Obj_Heart", 55)] {
            let d: Done = run(&root, P::new("unused"), "unused", &kit(item), &opts(None, true), &[TREE]).unwrap();
            let name: String = default_kit(item);
            assert!(d.report.contains(&format!("pack reads back: {files} files")), "{}", d.report);
            assert!(d.report.contains("8363 -> 8364 rows, 8363 earlier rows unchanged"), "{item}");
            assert!(d.report.contains("395 -> 396 rows, 395 earlier rows unchanged"), "{item}");
            assert!(d.report.contains(&format!("{name} carries the token kit: 5 shipped files byte for byte")), "{item}");
            assert!(d.report.contains(&format!("skulltula {TREE} drops {name} in 1 file(s)")), "{item}");
            assert_eq!(d.files.iter().filter(|f| f.path.starts_with("Banc/")).count(), 1);
            assert!(d.root.is_none());
        }
    }

    #[test]
    fn the_real_press_pickups_become_touch_kits_that_fall_and_keep_their_pouch_identity() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let token: Vec<Entry> = token_pack(&mut rf).unwrap();
        let base: Pool = pool_of(&[load(&mut rf, BASE)]).unwrap();
        let bc: Chain = chain(&base, BASE).unwrap();
        assert!(falls(&base, &bc).unwrap() && !auto(&base, &bc).unwrap());
        for (item, who, files) in [("Obj_Arrow_Fire", "Item_Arrow_Fire", 49), ("Obj_Heart_Container", "Obj_Heart_Container", 48), ("Obj_Heart_Piece", "Obj_Heart_Piece", 47)] {
            let own: Pool = pool_of(&[load(&mut rf, item)]).unwrap();
            let oc: Chain = chain(&own, item).unwrap();
            assert_eq!((near(&oc, &bc), falls(&own, &oc).unwrap(), auto(&own, &oc).unwrap()), (false, false, false), "{item}");
            let src: Vec<Vec<Entry>> = sources(&mut rf, item, &token).unwrap();
            assert_eq!(src.len(), 3, "{item}");
            assert!(plan(&src[1..], &kit(item)).unwrap_err().to_string().contains("was not given"), "{item}");
            let pl: Plan = plan(&src, &kit(item)).unwrap();
            assert_eq!((pl.files.len(), pl.identity.as_str(), rebodied(&pl).unwrap()), (files, who, true), "{item}");
            let name: String = default_kit(item);
            let es: Vec<Entry> = sarc::parse(&pl.sarc).unwrap();
            let p: Pool = pool_of(&[es.clone()]).unwrap();
            let c: Chain = chain(&p, &name).unwrap();
            assert!(touch(&p, &name).unwrap(), "{item}");
            assert_eq!(c.comps.get(ATTENTION_REF), bc.comps.get(ATTENTION_REF), "{item}");
            assert_eq!(c.comps.get(PHYSICS_REF), bc.comps.get(PHYSICS_REF), "{item}");
            assert_eq!(c.comps.get("ModelInfoRef"), oc.comps.get("ModelInfoRef"), "{item}");
            assert_eq!(c.comps.get("ActorNameRef"), oc.comps.get("ActorNameRef"), "{item}");
            assert_eq!(c.comps.get("EventPerformerRef"), oc.comps.get("EventPerformerRef"), "{item}");
            let y: Byml = Byml::parse(&sarc::get(&es, &actor_file(&name)).unwrap().data).unwrap();
            assert_eq!(count(y.root.get(COMPONENTS).unwrap()), 7, "{item}");
            assert!(!es.iter().any(|e| e.name.contains("AttentionSenderParam/PickUpItem") || e.name.contains("Attention/ClientParam/Item_Catch")), "{item}");
            assert!(es.iter().any(|e| e.name == "Phive/RigidBodyEntityParam/DropItem_0.phive__RigidBodyEntityParam.bgyml"), "{item}");
            assert_eq!(verify(&es, &name, &token).unwrap().len(), 3, "{item}");
            let mut weak: Vec<Entry> = es.clone();
            let q: &mut Entry = weak.iter_mut().find(|e| e.name == pouch_file(&name)).unwrap();
            let mut qy: Byml = Byml::parse(&q.data).unwrap();
            qy.root.set(AUTO_GET, Node::Bool(false));
            q.data = qy.write().unwrap();
            let e: String = verify(&weak, &name, &token).unwrap_err().to_string();
            assert!(e.contains("hangs in the air"), "{e}");
        }
        for item in ["Obj_Rupee_Red", "Obj_Heart", "Obj_Bomb", "Obj_Shield_Deku", "Obj_Armor_Zora"] {
            let src: Vec<Vec<Entry>> = sources(&mut rf, item, &token).unwrap();
            assert_eq!(src.len(), 2, "{item}");
            let pl: Plan = plan(&src, &kit(item)).unwrap();
            let es: Vec<Entry> = sarc::parse(&pl.sarc).unwrap();
            let y: Byml = Byml::parse(&sarc::get(&es, &actor_file(&default_kit(item))).unwrap().data).unwrap();
            assert_eq!(count(y.root.get(COMPONENTS).unwrap()), 5, "{item}");
            assert!(!rebodied(&pl).unwrap(), "{item}");
            let q: Byml = Byml::parse(&sarc::get(&es, &pouch_file(&default_kit(item))).unwrap().data).unwrap();
            assert_eq!(q.root.get(AUTO_GET), None, "{item}");
        }
        assert_eq!(sources(&mut rf, "Item_Hookshot", &token).unwrap().len(), 4);
    }

    #[test]
    fn the_real_press_kits_on_the_vine_skulltula_take_the_row_layer_of_the_falling_body() {
        let Some(root) = real() else {
            return;
        };
        const VINE: u64 = 16101480770659370603;
        for (item, files, who, deal) in [("Obj_Heart_Piece", 47, "Obj_Heart_Piece", "HeartPiece"), ("Obj_Heart_Container", 48, "Obj_Heart_Container", "HeartContainer"), ("Obj_Arrow_Fire", 49, "Item_Arrow_Fire", "None")] {
            let d: Done = run(&root, P::new("unused"), "unused", &kit(item), &opts(None, true), &[VINE]).unwrap();
            let name: String = default_kit(item);
            assert!(d.report.contains(&format!("pouch identity {who},")), "{}", d.report);
            assert!(d.report.contains(&format!("pack reads back: {files} files")), "{item}");
            assert!(d.report.contains("new row IsRespawn Some(Bool(true)) layer SmallObject"), "{item}");
            assert!(d.report.contains(&format!("new row deal {deal} ")), "{item}");
            assert!(d.report.contains("taken by touch and every body falls"), "{item}");
            assert!(d.report.contains(&format!("skulltula {VINE} drops {name} in 1 file(s)")), "{item}");
            let g: &File = d.files.iter().find(|f| f.path == GAME_ACTOR_INFO).unwrap();
            let y: Byml = Byml::parse(&g.raw).unwrap();
            assert_eq!(rows::find(&y, item).and_then(|r| r.get(rows::LAYER)).and_then(|v| v.as_str()), Some("Object"), "{item}");
            assert_eq!(rows::find(&y, &name).and_then(|r| r.get(rows::LAYER)).and_then(|v| v.as_str()), Some("SmallObject"), "{item}");
        }
    }

    #[test]
    fn a_kit_refuses_a_placed_spot_a_wrong_actor_and_an_edited_death_table() {
        let Some(root) = real() else {
            return;
        };
        let no: &P = P::new("unused");
        let mut placed: Opts = opts(None, true);
        placed.places.push(RUPEE_SPOT);
        assert!(run(&root, no, "unused", &kit("Item_Hookshot"), &placed, &[]).is_err());
        assert!(run(&root, no, "unused", &kit("Item_Hookshot"), &opts(None, true), &[RUPEE_SPOT]).is_err());
        assert!(run(&root, no, "unused", &kit("Item_Hookshot"), &opts(None, true), &[5]).is_err());
        assert!(run(&root, no, "unused", &kit(TOKEN), &opts(None, true), &[]).is_err());
        assert!(run(&root, no, "unused", &kit("Item_Nothing"), &opts(None, true), &[]).is_err());
        let base: PB = temp("edited");
        let f: PB = base.join(SKULL_DEATH);
        fs::create_dir_all(f.parent().unwrap()).unwrap();
        let rf: Romfs = Romfs::open(&root).unwrap();
        fs::write(&f, rf.read(SKULL_DEATH).unwrap()).unwrap();
        assert!(run(&root, no, "unused", &kit("Item_Hookshot"), &opts(Some(base.clone()), true), &[]).is_ok());
        fs::write(&f, b"edited").unwrap();
        let e: String = run(&root, no, "unused", &kit("Item_Hookshot"), &opts(Some(base.clone()), true), &[]).unwrap_err().to_string();
        assert!(e.contains("writes the saved row at death"), "{e}");
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_plain_pack_then_two_kits_chain_and_a_second_run_writes_the_same_bytes() {
        let Some(root) = real() else {
            return;
        };
        let load: PB = temp("chain");
        let plain: Spec = Spec { name: "Obj_Hookshot".to_string(), ..kit("Item_Hookshot") };
        let first: PB = plain_run(&root, &load, "chain", &plain, &opts(None, false)).unwrap().root.unwrap();
        let base: PB = first.join("romfs");
        run(&root, &load, "chain", &kit("Item_Hookshot"), &opts(Some(base.clone()), false), &[TREE]).unwrap();
        let d: Done = run(&root, &load, "chain", &kit("Obj_Heart"), &opts(Some(base.clone()), false), &[]).unwrap();
        assert!(d.report.contains("8365 -> 8366 rows, 8365 earlier rows unchanged"), "{}", d.report);
        assert_eq!(d.carried.len(), 2);
        for n in ["Obj_Hookshot", "Obj_Hookshot_GS", "Obj_Heart_GS"] {
            assert!(base.join(pack_of(n)).is_file(), "{n}");
        }
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let x: Extra = Extra::load(&mut rf, &base).unwrap();
        let e: String = check_swaps_with(&mut rf, &["Obj_Hookshot_GS"], &x).unwrap_err().to_string();
        assert!(e.contains("token kit"), "{e}");
        assert!(check_swaps_with(&mut rf, &["Obj_Hookshot"], &x).is_ok());
        let names: Vec<String> = listed(&base);
        assert_eq!(names.len(), 8);
        let before: Vec<Vec<u8>> = names.iter().map(|f| fs::read(base.join(f)).unwrap()).collect();
        run(&root, &load, "chain", &kit("Obj_Heart"), &opts(Some(base.clone()), false), &[]).unwrap();
        for (f, b) in names.iter().zip(before.iter()) {
            assert_eq!(&fs::read(base.join(f)).unwrap(), b, "{f}");
        }
        let _ = fs::remove_dir_all(&load);
    }
}
