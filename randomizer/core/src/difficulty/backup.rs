use std::collections::{BTreeMap as Map, BTreeSet as Set};

use crate::ainb::build::Spec;
use crate::ainb::{Ainb, Plug};
use crate::byml::{Byml, Node};
use crate::difficulty::fx;
use crate::difficulty::hook::{self, both, Hook};
use crate::error::{Error, Res};
use crate::pickup_pack::sizes;
use crate::qol::{edit_zs, Have};
use crate::romfs::Romfs;
use crate::rsizetable::Rst;
use crate::sarc::{self, Entry};
use crate::staging::Out;
use crate::zs;

pub use hook::{actor_of, flag, switch, ACTOR_INFO, GLOBAL, GRAPH, GRAPHS, MODULES, STALFOS_GRAPH};

pub const LABEL: &str = "enemy backup";
pub const ADDED: usize = 28 + fx::CUE_ADDED;
pub const HOOK: Hook = Hook { label: LABEL, taken, build };
pub const REQUEST: &str = "OneShotActorHolderTryRequestCreateActor";
pub const WAKE: &str = "OneShotActorHolderTryCancelSleepActor";
pub const READY: &str = "QueryActorHolderIsExistEnableCancelSleepActorHolderData";
pub const EMPTY: &str = "QueryActorHolderIsEmptyActorHolderData";
pub const FIGHT: &str = "QueryTargetIsValid";
pub const PLACE: &str = "QueryActorGetPostionAndRotation";
pub const SPLIT: &str = "QueryMathMtx34fToVec3fMat33f";
pub const LIFT: &str = "QueryMathAddVector3f";
pub const TIMER: &str = "SelectorGenericFrameCounter";
pub const IDLE: &str = "ExecuteGenericDummyAction";
pub const GETB: &str = "QueryBlackboardGetBoolNoError";
pub const SETB: &str = "ExecuteBlackboardSetBool";
pub const KEY: &str = "Backup";
pub const ELIGIBLE: &str = "OotrrBackupEligible";
pub const USED: &str = "OotrrBackupUsed";
pub const TABLE: &str = "Component/Blackboard/BlackboardParamTable/EnemyCommon.engine__component__BlackboardParamTable.bgyml";
pub const BOOLS: &str = "BlackboardParamBoolArray";
pub const CALLERS: &str = "Enemy_";
pub const HELPERS: [(&str, &str, i32); 19] = [
    ("Enemy_Keese", "Component/ActorHolderParam/OotrrBackup.game__component__ActorHolderParam.bgyml", 1),
    ("Enemy_Keese", "Component/ActorHolderParam/OotrrBackupKeese2.game__component__ActorHolderParam.bgyml", 2),
    ("Enemy_Stalfos_Weak", "Component/ActorHolderParam/OotrrBackupStalfosWeak.game__component__ActorHolderParam.bgyml", 1),
    ("Enemy_Stalchild", "Component/ActorHolderParam/OotrrBackupStalchild.game__component__ActorHolderParam.bgyml", 1),
    ("Enemy_ReDead", "Component/ActorHolderParam/OotrrBackupReDead.game__component__ActorHolderParam.bgyml", 1),
    ("Enemy_Wolfos", "Component/ActorHolderParam/OotrrBackupWolfos.game__component__ActorHolderParam.bgyml", 1),
    ("Enemy_Keese_Ice", "Component/ActorHolderParam/OotrrBackupKeeseIce.game__component__ActorHolderParam.bgyml", 1),
    ("Enemy_Lizalfos", "Component/ActorHolderParam/OotrrBackupLizalfos.game__component__ActorHolderParam.bgyml", 1),
    ("Enemy_Guay", "Component/ActorHolderParam/OotrrBackupGuay.game__component__ActorHolderParam.bgyml", 1),
    ("Enemy_Guay", "Component/ActorHolderParam/OotrrBackupGuay2.game__component__ActorHolderParam.bgyml", 2),
    ("Enemy_Skullwalltula", "Component/ActorHolderParam/OotrrBackupSkullwalltula.game__component__ActorHolderParam.bgyml", 1),
    ("Enemy_Dekubaba", "Component/ActorHolderParam/OotrrBackupDekubaba.game__component__ActorHolderParam.bgyml", 1),
    ("Enemy_Floormaster_Little", "Component/ActorHolderParam/OotrrBackupFloormasterLittle.game__component__ActorHolderParam.bgyml", 1),
    ("Enemy_Biri", "Component/ActorHolderParam/OotrrBackupBiri2.game__component__ActorHolderParam.bgyml", 2),
    ("Enemy_Biri", "Component/ActorHolderParam/OotrrBackupBiri.game__component__ActorHolderParam.bgyml", 1),
    ("Enemy_Gohma_Larva", "Component/ActorHolderParam/OotrrBackupGohmaLarva.game__component__ActorHolderParam.bgyml", 1),
    ("Enemy_Stalfos", "Component/ActorHolderParam/OotrrBackupStalfos.game__component__ActorHolderParam.bgyml", 1),
    ("Enemy_Wolfos_White", "Component/ActorHolderParam/OotrrBackupWolfosWhite.game__component__ActorHolderParam.bgyml", 1),
    ("Enemy_Dinolfos", "Component/ActorHolderParam/OotrrBackupDinolfos.game__component__ActorHolderParam.bgyml", 1),
];
pub const HELPER: &str = HELPERS[0].0;
pub const OUT_OF_BOUNDS: &[&str] = &[
    "Enemy_DungeonBoss_",
    "Enemy_MiddleBoss_",
    "Enemy_IronKnuckle_",
    "Enemy_DeadHand",
    "Enemy_PhantomGanon",
    "Enemy_GhostPicturePerformer_",
    "Enemy_KotakeKoumeBattleManager",
    "Enemy_ReDead",
    "Enemy_Leever",
    "Enemy_Peahat",
    "Enemy_Keese",
    "Enemy_BabyDodongo",
    "Enemy_Guay",
    "Enemy_Skullwalltula",
    "Enemy_Floormaster_Little",
    "Enemy_FlyingPot",
    "Enemy_Shabom",
];
pub const CALLS_NOBODY: &[&str] = &["Enemy_Skulltula_Gold", "Enemy_Poe_Amy", "Enemy_Poe_Beth", "Enemy_Poe_Joelle", "Enemy_Poe_SistersBase", "Enemy_Poe_Flat", "Enemy_Poe_Sharp", "Enemy_Scrub_Business", "Enemy_Scrub_ForestStage", "Enemy_Scrub_Mad_Leaf", "Enemy_Scrub_Deku_Leaf", "Enemy_Scrub_Deku_Brothers"];
pub const WHO_CALLS_WHOM: &[(&str, usize)] = &[
    ("Enemy_Stalfos_Weak", 3),
    ("Enemy_Stalfos_ForChild", 2),
    ("Enemy_Stalfos", 5),
    ("Enemy_Stalchild", 0),
    ("Enemy_Gibdo", 4),
    ("Enemy_Wolfos_White", 16),
    ("Enemy_Wolfos", 7),
    ("Enemy_Freezard", 6),
    ("Enemy_Lizalfos", 15),
    ("Enemy_Dinolfos", 17),
    ("Enemy_Dodongo", 0),
    ("Enemy_Moblin_Club", 8),
    ("Enemy_Moblin", 9),
    ("Enemy_Tektite", 8),
    ("Enemy_Dekubaba", 10),
    ("Enemy_Scrub_Mad", 11),
    ("Enemy_Scrub_Deku", 10),
    ("Enemy_SkullKid", 3),
    ("Enemy_Skulltula", 10),
    ("Enemy_Gohma_Larva", 0),
    ("Enemy_Floormaster", 12),
    ("Enemy_Wallmaster", 12),
    ("Enemy_LikeLike", 12),
    ("Enemy_Poe", 0),
    ("Enemy_Amos", 0),
    ("Enemy_Spike", 0),
    ("Enemy_Tailpasaran", 0),
    ("Enemy_Bubble", 0),
    ("Enemy_TorchSlug", 0),
    ("Enemy_Octorok", 0),
    ("Enemy_ShellBlade", 14),
    ("Enemy_Stinger", 14),
    ("Enemy_Bari", 13),
    ("Enemy_Biri", 0),
    ("Enemy_Tentacle", 14),
];
pub const FELL_BACK: &[&str] = &[];
pub const FRAMES: f32 = 600.0;
pub const RANDOM_FRAMES: f32 = 900.0;
pub const RISE: f32 = 0.5;
pub const MAX: i32 = 2;
pub const HEAP_EXTRA: i32 = 0x10000;
pub const ACTOR_DIR: &str = "Actor/";
pub const ACTOR_EXT: &str = ".engine__actor__ActorParam.bgyml";
pub const HOLDER_DIR: &str = "Component/ActorHolderParam/";
pub const NEW: &str = HELPERS[0].1;
pub const MAKE: &str = "QueryGenericMakeOverwriteParam";
pub const MARK: &str = "QueryBlackboardOverwriteParamAddBool";
pub const PEER: (&str, &str) = ("Pack/Actor/Enemy_Bari.pack.zs", "Component/ActorHolderParam/Enemy_Bari.game__component__ActorHolderParam.bgyml");
pub const PARTS: &str = "Components";
pub const SLOT: &str = "ActorHolderRef";
pub const HELD: &str = "ActorHoldSettings";
pub const PARENT: &str = "$parent";
pub const VERSION: u16 = 7;

const DEPTH: usize = 16;
pub const MAT34: &str = "const sead::Matrix34f";
pub const MAT33: &str = "const sead::Matrix33f";
const OVER: &str = "bb::OverwriteParam";

fn vec3(x: f32, y: f32, z: f32) -> Vec<u8> {
    [x, y, z].iter().flat_map(|v| v.to_le_bytes()).collect()
}

pub fn flag_query<'a>(seed: &'a str, key: Vec<u8>) -> Spec<'a> {
    Spec::query(GETB, seed).prop(0, "BlackboardType", 1i32.to_le_bytes().to_vec()).prop(3, "BlackboardKey", key).out(1, "Bool")
}

pub fn feed(a: &mut Ainb, from: usize, to: usize, out: &str) -> Res<()> {
    let name: u32 = a.pool.add(out);
    let reader: &mut crate::ainb::Param = a.params.get_mut(to).ok_or(Error::Format("enemy backup: no such reader"))?;
    let n: usize = reader.pl[0][0] as usize;
    let at_reader: Vec<usize> = (0..n.min(reader.plugs.len())).filter(|j| reader.plugs[*j].a as usize == from).collect();
    if at_reader.len() != 1 || n > reader.plugs.len() {
        return Err(Error::Format("enemy backup: the reader does not have one data link from the source"));
    }
    reader.plugs.remove(at_reader[0]);
    reader.pl[0][0] -= 1;
    for g in reader.pl.iter_mut().skip(1) {
        g[1] -= 1;
    }
    let source: &mut crate::ainb::Param = a.params.get_mut(from).ok_or(Error::Format("enemy backup: no such source"))?;
    let at: usize = source.pl[0][0] as usize;
    source.plugs.insert(at, Plug { a: to as u32, b: name, extra: Vec::new() });
    source.pl[0][0] += 1;
    for g in source.pl.iter_mut().skip(1) {
        g[1] += 1;
    }
    Ok(())
}

fn taken(a: &Ainb) -> bool {
    !a.find_els(REQUEST).is_empty()
}

pub fn mark<'a>(seed: &'a str, key: Vec<u8>, v: bool, from: usize) -> Spec<'a> {
    Spec::query(MARK, seed).prop(3, "Key", key).constant(1, "Bool", flag(v)).linked_ptr("OverwriteParam", OVER, from).out_ptr("OverwriteParam", OVER)
}

pub fn request_spec(seed: &str, key: Vec<u8>) -> Spec<'_> {
    Spec::node(REQUEST, seed).prop(1, "ExecuteEveryFrame", flag(false)).prop(1, "IsForeachActor", flag(false)).idle(0, "Index", vec![0; 4]).constant(3, "Key", key)
}

pub fn build(a: &mut Ainb, tag: &str) -> Res<usize> {
    let s = |n: &str| -> String { format!("{tag} {n}") };
    let key: Vec<u8> = a.pool.add(KEY).to_le_bytes().to_vec();
    let ready: usize = a.add_node(&Spec::query(READY, &s("ready")).constant(3, "Key", key.clone()).out(1, "IsExist"))?;
    let empty: usize = a.add_node(&Spec::query(EMPTY, &s("empty")).constant(3, "Key", key.clone()).out(1, "Bool"))?;
    let fight: usize = a.add_node(&Spec::query(FIGHT, &s("fight")).idle(0, "TargetSlot", vec![0; 4]).out(1, "IsValid"))?;
    let place: usize = a.add_node(&Spec::query(PLACE, &s("place")).out_ptr("Mat34F", MAT34))?;
    let split: usize = a.add_node(&Spec::query(SPLIT, &s("split")).linked_ptr("Matrix34f", MAT34, place).out(4, "Translation").spare_ptr("Rotation", MAT33))?;
    feed(a, place, split, "Mat34F")?;
    let lift: usize = a.add_node(&Spec::query(LIFT, &s("lift")).linked(4, "InputA", split, 0).constant(4, "InputB", vec3(0.0, RISE, 0.0)).out(4, "Result"))?;
    feed(a, split, lift, "Translation")?;
    let eligible_key: Vec<u8> = a.pool.add(ELIGIBLE).to_le_bytes().to_vec();
    let used_key: Vec<u8> = a.pool.add(USED).to_le_bytes().to_vec();
    let make: usize = a.add_node(&Spec::query(MAKE, &s("make")).out_ptr("OverwriteParam", OVER))?;
    let mark_eligible: usize = a.add_node(&mark(&s("mark eligible"), eligible_key.clone(), false, make))?;
    feed(a, make, mark_eligible, "OverwriteParam")?;
    let mark_used: usize = a.add_node(&mark(&s("mark used"), used_key.clone(), true, mark_eligible))?;
    feed(a, mark_eligible, mark_used, "OverwriteParam")?;
    let wake: usize = a.add_node(
        &Spec::node(WAKE, &s("wake"))
            .prop(1, "ExecuteEveryFrame", flag(true))
            .prop(1, "IsForeachActor", flag(false))
            .idle(0, "Index", vec![0; 4])
            .constant(3, "Key", key.clone())
            .linked(4, "Pos", lift, 0)
            .idle(4, "Scale", vec3(1.0, 1.0, 1.0))
            .linked_ptr("OverwriteParam", OVER, mark_used)
            .idle_ptr("Rot", MAT33),
    )?;
    feed(a, lift, wake, "Result")?;
    feed(a, mark_used, wake, "OverwriteParam")?;
    let eligible: usize = a.add_node(&flag_query(&s("eligible"), eligible_key))?;
    let used: usize = a.add_node(&flag_query(&s("used"), used_key.clone()))?;
    let latch: usize = a.add_node(
        &Spec::node(SETB, &s("latch"))
            .prop(0, "BlackboardType", 1i32.to_le_bytes().to_vec())
            .prop(1, "ExecuteOnce", flag(true))
            .prop(1, "IsIgnoreNotExistsError", flag(true))
            .prop(3, "BlackboardKey", used_key)
            .constant(1, "Bool", flag(true)),
    )?;
    let call: usize = a.add_node(&request_spec(&s("call"), key.clone()))?;
    let second: usize = a.add_node(&request_spec(&s("call second"), key))?;
    let pair: usize = a.add_node(&both(&s("calls"), call, second))?;
    let calls: usize = fx::with_flash(a, &s("call"), pair)?;
    let request: usize = a.add_node(&both(&s("request"), calls, latch))?;
    let wait: usize = a.add_node(&Spec::node(IDLE, &s("idle wait")))?;
    let timer: usize = a.add_node(
        &Spec::node(TIMER, &s("timer"))
            .prop(1, "IsCountUpdateOnReset", flag(false))
            .prop(1, "IsLoop", flag(false))
            .prop(1, "NoChangeWhenChildIsBusy", flag(false))
            .prop(1, "SelectChildOnEnterOnly", flag(false))
            .prop(2, "CountFrames", FRAMES.to_le_bytes().to_vec())
            .prop(2, "RandomAddCountFrames", RANDOM_FRAMES.to_le_bytes().to_vec())
            .spare(2, "CurrentFrame")
            .spare(2, "FinishFrame")
            .kid(wait, "NotFinish")
            .kid(request, "Finish"),
    )?;
    let spent: usize = a.add_node(&Spec::node(IDLE, &s("idle spent")))?;
    let unused: usize = a.add_node(&hook::switch(&s("unused"), used, spent, timer))?;
    let kept: usize = a.add_node(&Spec::node(IDLE, &s("idle kept")))?;
    let alone: usize = a.add_node(&hook::switch(&s("alone"), empty, unused, kept))?;
    let gate: usize = a.add_node(&hook::switch(&s("gate"), ready, wake, alone))?;
    let calm: usize = a.add_node(&Spec::node(IDLE, &s("idle calm")))?;
    let battle: usize = a.add_node(&hook::switch(&s("battle"), fight, gate, calm))?;
    let off: usize = a.add_node(&Spec::node(IDLE, &s("idle off")))?;
        a.add_node(&hook::switch(&s("guard"), eligible, battle, off))
}

pub fn actor_file(actor: &str) -> String {
    format!("{ACTOR_DIR}{actor}{ACTOR_EXT}")
}

fn packed(path: &str) -> Option<String> {
    let p: &str = path.strip_prefix('?').unwrap_or(path);
    if p.is_empty() {
        return None;
    }
    match p.strip_prefix("Work/") {
        Some(w) => Some(match w.strip_suffix(".gyml") {
            Some(stem) => format!("{stem}.bgyml"),
            None => w.to_string(),
        }),
        None => Some(p.to_string()),
    }
}

pub fn holder_of(es: &[Entry], actor: &str) -> Res<Option<String>> {
    let mut at: Option<String> = Some(actor_file(actor));
    for _ in 0..DEPTH {
        let Some(e) = at.as_deref().and_then(|f| sarc::get(es, f)) else {
            return Ok(None);
        };
        let y: Byml = Byml::parse(&e.data)?;
        if let Some(held) = y.root.get(PARTS).and_then(|c| c.get(SLOT)).and_then(Node::as_str) {
            return Ok(packed(held));
        }
        at = y.root.get(PARENT).and_then(Node::as_str).and_then(packed);
    }
    Ok(None)
}

pub fn entry(helper: &str, max: i32) -> Node {
    Node::Map(vec![
        ("Actor".to_string(), Node::Str(format!("Work/Actor/{helper}.engine__actor__ActorParam.gyml"))),
        ("IsCreateFirstWithHolder".to_string(), Node::Bool(false)),
        ("IsEnableWaitCreateAndIgnition".to_string(), Node::Bool(false)),
        ("MaxActors".to_string(), Node::I32(max)),
    ])
}

pub fn holder_file(helper: &str, max: i32) -> Res<Vec<u8>> {
    let y: Byml = Byml { root: Node::Map(vec![(HELD.to_string(), Node::Map(vec![(KEY.to_string(), entry(helper, max))]))]), version: VERSION, big: false };
    y.write()
}

fn put(raw: &[u8], group: &str, k: &str, v: Node) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    if !matches!(y.root, Node::Map(_)) {
        return Err(Error::Format("enemy backup: a parameter file is not a map"));
    }
    if !matches!(y.root.get(group), Some(Node::Map(_))) {
        y.root.set(group, Node::Map(Vec::new()));
    }
    let m: &mut Node = y.root.get_mut(group).ok_or(Error::Format("enemy backup: a parameter group is missing"))?;
    m.set(k, v.clone());
    let out: Vec<u8> = y.write()?;
    if Byml::parse(&out)?.root.get(group).and_then(|g| g.get(k)) != Some(&v) {
        return Err(Error::Format("enemy backup: a parameter file does not read back"));
    }
    Ok(out)
}

pub fn add_ref(raw: &[u8], file: &str) -> Res<Vec<u8>> {
    put(raw, PARTS, SLOT, Node::Str(format!("?{file}")))
}

pub fn add_key(raw: &[u8], helper: &str, max: i32) -> Res<Vec<u8>> {
    put(raw, HELD, KEY, entry(helper, max))
}

pub fn excluded(actor: &str) -> bool {
    OUT_OF_BOUNDS.iter().chain(CALLS_NOBODY).any(|p| actor.starts_with(p))
}

pub fn helper_of(actor: &str) -> usize {
    WHO_CALLS_WHOM.iter().find(|r| actor.starts_with(r.0)).map_or(0, |r| r.1)
}

pub fn flag_row(key: &str, start: bool) -> Node {
    Node::Map(vec![
        ("BBKey".to_string(), Node::Str(key.to_string())),
        ("InitVal".to_string(), Node::Bool(start)),
        ("IsInstanceParam".to_string(), Node::Bool(true)),
        ("IsRecord".to_string(), Node::Bool(false)),
        ("IsSafeParamForExternalTools".to_string(), Node::Bool(false)),
    ])
}

pub fn state_row(key: &str, start: bool) -> Node {
    let mut row: Node = flag_row(key, start);
    row.set("IsInstanceParam", Node::Bool(false));
    row
}

pub fn add_flags(raw: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    let list: &mut Vec<Node> = y.root.get_mut(BOOLS).and_then(Node::as_arr_mut).ok_or(Error::Format("enemy backup: the common blackboard has no bool list"))?;
    if list.iter().any(|r| matches!(r.get("BBKey").and_then(Node::as_str), Some(ELIGIBLE | USED))) {
        return Err(Error::Format("enemy backup: the common blackboard already has the backup flags"));
    }
    list.push(state_row(ELIGIBLE, true));
    list.push(state_row(USED, false));
    let n: usize = list.len();
    let out: Vec<u8> = y.write()?;
    let back: Byml = Byml::parse(&out)?;
    let rows: &[Node] = back.root.get(BOOLS).and_then(Node::as_arr).ok_or(Error::Format("enemy backup: the common blackboard does not read back"))?;
    if rows.len() != n || rows.last() != Some(&state_row(USED, false)) || back.root != y.root {
        return Err(Error::Format("enemy backup: the common blackboard does not read back"));
    }
    Ok(out)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub who: Set<String>,
    pub off: Set<String>,
    pub bare: Map<String, usize>,
    pub held: Map<String, usize>,
    pub shared: Set<String>,
    pub live: Set<String>,
    pub tabled: Set<String>,
}

fn settle(m: &mut Map<String, usize>, file: String, helper: usize) {
    let at: &mut usize = m.entry(file).or_insert(helper);
    if *at != helper {
        *at = 0;
    }
}

impl Plan {
    pub fn note(&mut self, es: &[Entry], actor: &str) -> Res<()> {
        self.who.insert(actor.to_string());
        if !actor.starts_with(CALLERS) {
            return Ok(());
        }
        if excluded(actor) {
            self.off.insert(actor.to_string());
            return Ok(());
        }
        self.live.extend(hook::shared_of(es, &self.shared));
        let helper: usize = helper_of(actor);
        match holder_of(es, actor)? {
            Some(h) => settle(&mut self.held, h, helper),
            None => settle(&mut self.bare, actor_file(actor), helper),
        };
        Ok(())
    }

    pub fn callers(&self) -> impl Iterator<Item = &String> {
        self.who.iter().filter(|a| a.starts_with(CALLERS) && !self.off.contains(*a))
    }

    pub fn hosts(&self, graph: &str, actor: &str) -> bool {
        if self.shared.contains(graph) {
            return self.live.contains(graph);
        }
        actor.starts_with(CALLERS) && !excluded(actor)
    }

    pub fn wants(&self, name: &str) -> bool {
        name == TABLE || self.bare.contains_key(name) || self.held.contains_key(name)
    }
}

pub fn patch_entries(es: &mut Vec<Entry>, plan: &Plan, actor: &str) -> Res<Set<String>> {
    let mut hit: Set<String> = Set::new();
    let mut fresh: Set<usize> = Set::new();
    for e in es.iter_mut() {
        let new: Vec<u8> = if hook::is_caller(e) {
            if !plan.hosts(&e.name, actor) {
                continue;
            }
            match hook::patch_graph(&e.data, &HOOK)? {
                Some(out) => out,
                None => continue,
            }
        } else if let Some(h) = plan.bare.get(&e.name) {
            fresh.insert(*h);
            add_ref(&e.data, HELPERS[*h].1)?
        } else if let Some(h) = plan.held.get(&e.name) {
            add_key(&e.data, HELPERS[*h].0, HELPERS[*h].2)?
        } else if e.name == TABLE {
            add_flags(&e.data)?
        } else {
            continue;
        };
        e.data = new;
        hit.insert(e.name.clone());
    }
    for h in fresh {
        let (helper, file, max): (&str, &str, i32) = HELPERS[h];
        if sarc::get(es, file).is_some() {
            return Err(Error::Format("enemy backup: the pack already has a backup holder"));
        }
        sarc::set(es, file, holder_file(helper, max)?);
        hit.insert(file.to_string());
    }
    Ok(hit)
}

fn visit(rf: &mut Romfs, have: Have, path: &str, plan: &mut Plan, seen: &mut Set<String>) -> Res<Option<Out>> {
    let (z, old, mut es): (Vec<u8>, usize, Vec<Entry>) = hook::opened(rf, have, path)?;
    let actor: String = actor_of(path).unwrap_or("").to_string();
    let hit: Set<String> = patch_entries(&mut es, plan, &actor)?;
    if hit.is_empty() {
        return Ok(None);
    }
    if hit.contains(TABLE) {
        plan.tabled.insert(actor);
    }
    seen.extend(hit);
    let out: Vec<u8> = sarc::write(&es)?;
    Ok(Some(Out { path: path.to_string(), old, new: out.len(), z: zs::compress_raw(&out, zs::dict_of(&z))? }))
}

pub fn stage(rf: &mut Romfs, have: Have, dir: &str) -> Res<(Plan, Vec<Out>)> {
    let mut plan: Plan = Plan { shared: hook::callers(&hook::opened(rf, have, GLOBAL)?.2), ..Plan::default() };
    let mut index: Vec<(String, bool, Vec<String>)> = Vec::new();
    for f in rf.list(dir)? {
        let Some(name) = actor_of(&f).map(str::to_string) else {
            continue;
        };
        let es: Vec<Entry> = hook::opened(rf, have, &f)?.2;
        if hook::runs(&es, &plan.shared) {
            plan.note(&es, &name)?;
        }
        let own: bool = es.iter().any(|e| hook::is_caller(e) && plan.hosts(&e.name, &name));
        let keep: Vec<String> = es.iter().filter(|e| e.name == TABLE || e.name.starts_with(HOLDER_DIR) || (e.name.starts_with(ACTOR_DIR) && e.name.ends_with(ACTOR_EXT))).map(|e| e.name.clone()).collect();
        if own || !keep.is_empty() {
            index.push((f, own, keep));
        }
    }
    if plan.who.is_empty() {
        return Err(Error::Format("enemy backup: no actor runs the damage color graph"));
    }
    let mut seen: Set<String> = Set::new();
    let mut outs: Vec<Out> = Vec::new();
    outs.push(visit(rf, have, GLOBAL, &mut plan, &mut seen)?.ok_or(Error::Msg(format!("enemy backup: the AI pack has no graph that runs {GRAPH}")))?);
    for (f, own, keep) in &index {
        if *own || keep.iter().any(|n| plan.wants(n)) {
            outs.extend(visit(rf, have, f, &mut plan, &mut seen)?);
        }
    }
    if let Some(lost) = plan.bare.keys().chain(plan.held.keys()).find(|n| !seen.contains(*n)) {
        return Err(Error::Msg(format!("enemy backup: {lost} is in no actor pack")));
    }
    let grow: Set<String> = plan.who.union(&plan.tabled).cloned().collect();
    outs.push(edit_zs(rf, have, ACTOR_INFO, |raw| hook::grow_heap(raw, &grow, HEAP_EXTRA, LABEL))?);
    Ok((plan, outs))
}

pub fn size(rf: &mut Romfs, t: &mut Rst) -> Res<()> {
    let es: Vec<Entry> = sarc::parse(&rf.read_zs(PEER.0)?)?;
    let peer: &Entry = sarc::get(&es, PEER.1).ok_or(Error::Msg(format!("enemy backup: {} has no {}", PEER.0, PEER.1)))?;
    for (helper, file, max) in HELPERS {
        let n: u32 = sizes::file_size(t, &[(peer.name.clone(), peer.data.len())], file, holder_file(helper, max)?.len())?;
        t.set_size(file, n)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ainb::show::show;
    use crate::ainb::root;
    use crate::difficulty::hook::{PARALLEL, SWITCH};
    use crate::difficulty::{fairy, regen};
    use crate::pickup_pack::rows;
    use std::collections::BTreeMap as Map;

    fn file(name: &str, items: Vec<(&str, Node)>) -> Entry {
        let y: Byml = Byml { root: Node::Map(items.into_iter().map(|(k, v)| (k.to_string(), v)).collect()), version: VERSION, big: false };
        Entry { name: name.to_string(), data: y.write().unwrap() }
    }

    fn parts(items: Vec<(&str, &str)>) -> Node {
        Node::Map(items.into_iter().map(|(k, v)| (k.to_string(), Node::Str(v.to_string()))).collect())
    }

    const D: &str = "Component/ActorHolderParam/Enemy_D.game__component__ActorHolderParam.bgyml";

    fn pack() -> Vec<Entry> {
        vec![
            file(&actor_file("Enemy_A"), vec![("Components", parts(vec![("ASRef", "?AS/a.asb"), ("LifeRef", "?Life/a.bgyml")]))]),
            file(&actor_file("Enemy_B"), vec![("$parent", Node::Str("Work/Actor/Enemy_A.engine__actor__ActorParam.gyml".to_string())), ("Components", parts(vec![("LifeRef", "?Life/b.bgyml")]))]),
            file(&actor_file("Enemy_C"), vec![("$parent", Node::Str("Work/Actor/Enemy_D.engine__actor__ActorParam.gyml".to_string()))]),
            file(&actor_file("Enemy_D"), vec![("Components", parts(vec![("ActorHolderRef", "?Component/ActorHolderParam/Enemy_D.game__component__ActorHolderParam.bgyml")]))]),
            file(&actor_file("Enemy_E"), vec![("$parent", Node::Str("Work/Actor/Enemy_D.engine__actor__ActorParam.gyml".to_string())), ("Components", parts(vec![("ActorHolderRef", "")]))]),
            file(D, vec![("ActorHoldSettings", Node::Map(vec![("Weapon".to_string(), Node::Map(vec![("MaxActors".to_string(), Node::I32(3))]))]))]),
        ]
    }

    #[test]
    fn a_holder_is_found_through_the_parents() {
        let es: Vec<Entry> = pack();
        assert_eq!(holder_of(&es, "Enemy_A").unwrap(), None);
        assert_eq!(holder_of(&es, "Enemy_B").unwrap(), None);
        assert_eq!(holder_of(&es, "Enemy_C").unwrap().as_deref(), Some(D));
        assert_eq!(holder_of(&es, "Enemy_D").unwrap().as_deref(), Some(D));
        assert_eq!(holder_of(&es, "Enemy_E").unwrap(), None, "an empty slot switches the parent's holder off");
        assert_eq!(holder_of(&es, "Enemy_Missing").unwrap(), None);
        assert_eq!(packed("Work/Actor/X.engine__actor__ActorParam.gyml").as_deref(), Some("Actor/X.engine__actor__ActorParam.bgyml"));
        assert_eq!(packed("?Component/X.bgyml").as_deref(), Some("Component/X.bgyml"));
        assert_eq!(packed(""), None);
    }

    #[test]
    fn a_pack_gains_the_call_slot_once_for_every_planned_actor() {
        let mut es: Vec<Entry> = pack();
        let mut plan: Plan = Plan::default();
        for a in ["Enemy_A", "Enemy_B", "Enemy_C", "Npc_D"] {
            plan.note(&es, a).unwrap();
        }
        assert_eq!((plan.who.len(), plan.callers().count(), plan.bare.len(), plan.held.len()), (4, 3, 2, 1));
        assert!(plan.wants(TABLE) && plan.wants(&actor_file("Enemy_A")) && plan.wants(D));
        assert!(!plan.wants(&actor_file("Enemy_C")) && !plan.wants(GRAPH));
        let hit: Set<String> = patch_entries(&mut es, &plan, "Enemy_A").unwrap();
        assert_eq!(hit.len(), 4);
        for a in ["Enemy_A", "Enemy_B"] {
            assert_eq!(holder_of(&es, a).unwrap().as_deref(), Some(NEW));
        }
        let a: Byml = Byml::parse(&sarc::get(&es, &actor_file("Enemy_A")).unwrap().data).unwrap();
        let Some(Node::Map(m)) = a.root.get(PARTS) else { panic!() };
        assert_eq!(m.iter().map(|e| e.0.as_str()).collect::<Vec<&str>>(), vec!["ASRef", SLOT, "LifeRef"]);
        let fresh: Byml = Byml::parse(&sarc::get(&es, NEW).unwrap().data).unwrap();
        assert_eq!(fresh.root.get(HELD).and_then(|h| h.get(KEY)), Some(&entry(HELPER, 1)));
        let d: Byml = Byml::parse(&sarc::get(&es, D).unwrap().data).unwrap();
        let Some(Node::Map(h)) = d.root.get(HELD) else { panic!() };
        assert_eq!(h.iter().map(|e| e.0.as_str()).collect::<Vec<&str>>(), vec![KEY, "Weapon"]);
        assert_eq!(h[0].1, entry(HELPER, 1));
        assert!(patch_entries(&mut es, &plan, "Enemy_A").is_err(), "a patched pack is not patched twice");
        let mut other: Vec<Entry> = vec![file("Life/a.bgyml", vec![("MaxLife", Node::I32(1))])];
        assert!(patch_entries(&mut other, &plan, "Enemy_A").unwrap().is_empty());
        assert_eq!(other.len(), 1);
    }

    #[test]
    fn the_called_actor_is_made_on_demand_and_one_at_a_time() {
        let e: Node = entry(HELPER, 1);
        assert_eq!(e.get("Actor").and_then(Node::as_str), Some("Work/Actor/Enemy_Keese.engine__actor__ActorParam.gyml"));
        assert_eq!(e.get("IsCreateFirstWithHolder"), Some(&Node::Bool(false)));
        assert_eq!(e.get("MaxActors"), Some(&Node::I32(1)));
        assert_eq!(holder_file(HELPER, 1).unwrap(), holder_file(HELPER, 1).unwrap());
        assert_eq!(entry(HELPERS[1].0, HELPERS[1].2).get("MaxActors"), Some(&Node::I32(2)));
        assert!(HELPERS.iter().all(|h| h.1.starts_with(HOLDER_DIR) && h.1.ends_with(".game__component__ActorHolderParam.bgyml")));
        assert!(HELPERS.iter().all(|h| (1..=MAX).contains(&h.2)), "no caller can ever have more than {MAX} helpers alive");
        let files: Set<&str> = HELPERS.iter().map(|h| h.1).collect();
        assert_eq!(files.len(), HELPERS.len());
        assert!(NEW.starts_with(HOLDER_DIR));
    }

    #[test]
    fn bosses_and_their_kin_are_left_out_and_helpers_follow_the_caller() {
        for a in ["Enemy_DungeonBoss_Gohma", "Enemy_MiddleBoss_DarkLink", "Enemy_IronKnuckle_Black", "Enemy_DeadHand", "Enemy_DeadHand_Hand", "Enemy_PhantomGanonHorse", "Enemy_GhostPicturePerformer_PhantomGanon", "Enemy_Guay_Boss"] {
            assert!(excluded(a), "{a}");
        }
        for a in ["Enemy_Stalfos", "Enemy_Skulltula", "Enemy_Dodongo", "Enemy_Octorok"] {
            assert!(!excluded(a), "{a}");
        }
        for a in ["Enemy_Leever", "Enemy_Leever_Big", "Enemy_Leever_Fixed", "Enemy_ReDead", "Enemy_ReDead_ClimaxOnly", "Enemy_Peahat", "Enemy_Peahat_Larva", "Enemy_Keese_Truth", "Enemy_Guay_Flock", "Enemy_Floormaster_Little_Truth"] {
            assert!(excluded(a), "{a} can never call");
        }
        for a in ["Enemy_Skulltula_Gold", "Enemy_Poe_Amy", "Enemy_Poe_Beth", "Enemy_Poe_Joelle", "Enemy_ReDead_AlwaysRespawn"] {
            assert!(excluded(a), "{a} calls nobody");
        }
        for a in ["Enemy_Scrub_Business", "Enemy_Scrub_Business_Leaf", "Enemy_Scrub_Business_Seed", "Enemy_Scrub_ForestStage", "Enemy_Scrub_ForestStage_Elder", "Enemy_Scrub_ForestStage_Leaf", "Enemy_Scrub_Mad_Leaf", "Enemy_Scrub_Deku_Leaf", "Enemy_Scrub_Deku_Brothers"] {
            assert!(excluded(a), "{a} is harmless and never calls");
        }
        for a in ["Enemy_Scrub_Mad", "Enemy_Scrub_Deku"] {
            assert!(!excluded(a), "{a} fights and may call");
        }
        let option_3: [(&str, &str, i32); 43] = [
            ("Enemy_Stalfos", "Enemy_Wolfos", 1),
            ("Enemy_Stalfos_ForChild", "Enemy_Stalfos_Weak", 1),
            ("Enemy_Stalfos_Weak", "Enemy_Stalchild", 1),
            ("Enemy_Stalchild", "Enemy_Keese", 1),
            ("Enemy_Stalchild_Big", "Enemy_Keese", 1),
            ("Enemy_Gibdo", "Enemy_ReDead", 1),
            ("Enemy_Wolfos", "Enemy_Lizalfos", 1),
            ("Enemy_Wolfos_White", "Enemy_Stalfos", 1),
            ("Enemy_Freezard", "Enemy_Keese_Ice", 1),
            ("Enemy_Freezard_Fixed", "Enemy_Keese_Ice", 1),
            ("Enemy_Lizalfos", "Enemy_Gohma_Larva", 1),
            ("Enemy_Dinolfos", "Enemy_Wolfos_White", 1),
            ("Enemy_Dodongo", "Enemy_Keese", 1),
            ("Enemy_Moblin_Club", "Enemy_Guay", 1),
            ("Enemy_Moblin", "Enemy_Guay", 2),
            ("Enemy_Tektite_Red", "Enemy_Guay", 1),
            ("Enemy_Tektite_Blue", "Enemy_Guay", 1),
            ("Enemy_Dekubaba", "Enemy_Skullwalltula", 1),
            ("Enemy_Dekubaba_Withered", "Enemy_Skullwalltula", 1),
            ("Enemy_Scrub_Mad", "Enemy_Dekubaba", 1),
            ("Enemy_Scrub_Deku", "Enemy_Skullwalltula", 1),
            ("Enemy_SkullKid", "Enemy_Stalchild", 1),
            ("Enemy_Skulltula", "Enemy_Skullwalltula", 1),
            ("Enemy_Gohma_Larva", "Enemy_Keese", 1),
            ("Enemy_Floormaster", "Enemy_Floormaster_Little", 1),
            ("Enemy_Wallmaster", "Enemy_Floormaster_Little", 1),
            ("Enemy_LikeLike", "Enemy_Floormaster_Little", 1),
            ("Enemy_Poe", "Enemy_Keese", 1),
            ("Enemy_Poe_Big", "Enemy_Keese", 1),
            ("Enemy_Poe_Hyrule", "Enemy_Keese", 1),
            ("Enemy_Amos", "Enemy_Keese", 1),
            ("Enemy_Spike", "Enemy_Keese", 1),
            ("Enemy_Tailpasaran", "Enemy_Keese", 1),
            ("Enemy_Bubble_Blue", "Enemy_Keese", 1),
            ("Enemy_Bubble_Green", "Enemy_Keese", 1),
            ("Enemy_Bubble_White", "Enemy_Keese", 1),
            ("Enemy_Bubble_Red", "Enemy_Keese", 1),
            ("Enemy_TorchSlug", "Enemy_Keese", 1),
            ("Enemy_Octorok", "Enemy_Keese", 1),
            ("Enemy_ShellBlade", "Enemy_Biri", 1),
            ("Enemy_Stinger", "Enemy_Biri", 1),
            ("Enemy_Bari", "Enemy_Biri", 2),
            ("Enemy_Biri", "Enemy_Keese", 1),
        ];
        for (a, helper, n) in option_3 {
            assert!(!excluded(a), "{a} calls");
            let h: (&str, &str, i32) = HELPERS[helper_of(a)];
            assert_eq!((h.0, h.2), (helper, n), "{a}");
            assert!(h.0 != a, "{a} never calls its own kind");
        }
        assert_eq!(HELPERS[helper_of("Enemy_Tentacle_Red")].0, "Enemy_Biri");
        assert_eq!(helper_of("Enemy_Unlisted"), 0);
        let mut m: Map<String, usize> = Map::new();
        settle(&mut m, "f".to_string(), 2);
        settle(&mut m, "f".to_string(), 2);
        assert_eq!(m["f"], 2);
        settle(&mut m, "f".to_string(), 1);
        assert_eq!(m["f"], 0, "a holder shared by callers that want different helpers falls back to the Keese");
        let mut es: Vec<Entry> = pack();
        let mut plan: Plan = Plan::default();
        plan.note(&es, "Enemy_MiddleBoss_X").unwrap();
        plan.note(&es, "Enemy_A").unwrap();
        assert_eq!((plan.who.len(), plan.off.len(), plan.callers().count(), plan.bare.len()), (2, 1, 1, 1));
        let table: Entry = Entry { name: TABLE.to_string(), data: Byml { root: Node::Map(vec![(BOOLS.to_string(), Node::Arr(vec![flag_row("IsX", false)]))]), version: VERSION, big: false }.write().unwrap() };
        es.push(table);
        let mut boss: Vec<Entry> = es.clone();
        assert!(patch_entries(&mut boss, &plan, "Enemy_MiddleBoss_X").unwrap().contains(TABLE));
        assert!(patch_entries(&mut es, &plan, "Enemy_A").unwrap().contains(TABLE));
        let flags = |es: &[Entry]| -> Vec<Node> { Byml::parse(&sarc::get(es, TABLE).unwrap().data).unwrap().root.get(BOOLS).and_then(Node::as_arr).unwrap().to_vec() };
        assert_eq!(flags(&es), vec![flag_row("IsX", false), state_row(ELIGIBLE, true), state_row(USED, false)]);
        assert_eq!(sarc::get(&es, TABLE).unwrap().data, sarc::get(&boss, TABLE).unwrap().data, "every copy of the table gets the same keys, whoever owns it");
        assert!(add_flags(&sarc::get(&es, TABLE).unwrap().data).is_err());
    }

    #[test]
    fn graphs_of_bosses_are_left_alone_and_shared_ones_follow_their_users() {
        let mut plan: Plan = Plan::default();
        plan.shared.insert("AI/EnemyBase.utility.root.ainb".to_string());
        plan.shared.insert("AI/Enemy_IronKnuckle.utility.root.ainb".to_string());
        plan.live.insert("AI/EnemyBase.utility.root.ainb".to_string());
        assert!(plan.hosts("AI/EnemyBase.utility.root.ainb", "AI.Global.Product.111"));
        assert!(!plan.hosts("AI/Enemy_IronKnuckle.utility.root.ainb", "AI.Global.Product.111"));
        assert!(plan.hosts("AI/Enemy_Stalfos.utility.root.ainb", "Enemy_Stalfos"));
        assert!(!plan.hosts("AI/Enemy_DungeonBoss_Gohma.utility.root.ainb", "Enemy_DungeonBoss_Gohma"));
        assert!(!plan.hosts("AI/Npc_X.utility.root.ainb", "Npc_X"));
    }

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        if p.is_dir() { Some(p) } else { None }
    }

    fn graph(rf: &mut Romfs, pack: &str, name: &str) -> Ainb {
        let es: Vec<Entry> = sarc::parse(&rf.read_zs(pack).unwrap()).unwrap();
        Ainb::parse(&sarc::get(&es, name).unwrap().data).unwrap()
    }

    fn lines(a: &Ainb, i: usize) -> Vec<String> {
        let all: String = show(a);
        let from: usize = all.find(&format!("element {i} type")).unwrap();
        let body: &str = &all[from..];
        let end: usize = body.find(&format!("\nelement {} type", i + 1)).unwrap_or(body.len());
        body[..end].lines().skip(1).map(str::to_string).collect()
    }

    fn field(l: &str) -> String {
        l.split(" = ").next().unwrap_or(l).to_string()
    }

    fn shape(a: &Ainb, i: usize) -> Vec<String> {
        lines(a, i)
            .into_iter()
            .filter(|l| !l.contains("plug "))
            .map(|l| {
                let wired: &str = if l.contains(" from ") { " wired" } else { "" };
                let tail: &str = if l.trim_start().starts_with("in ") { l.rsplit(" flags ").next().unwrap_or("") } else { "" };
                format!("{}{wired} {tail}", field(&l))
            })
            .collect()
    }

    fn fields(a: &Ainb, i: usize) -> Vec<String> {
        lines(a, i).into_iter().filter(|l| !l.contains("plug ")).map(|l| field(&l)).collect()
    }

    fn plugs(a: &Ainb, i: usize) -> (Vec<(String, usize)>, Vec<[u8; 2]>) {
        (a.params[i].plugs.iter().map(|g| (a.s(g.b).to_string(), g.extra.len())).collect(), a.params[i].pl.to_vec())
    }

    fn one(a: &Ainb, name: &str) -> usize {
        a.find_els(name)[0]
    }

    fn a_once(a: &Ainb, i: usize) -> bool {
        lines(a, i).iter().any(|l| l.contains("ExecuteOnce = true"))
    }

    fn classes(a: &Ainb, i: usize) -> Vec<String> {
        let p: &crate::ainb::Param = &a.params[i];
        let word = |b: &[u8]| -> String { a.s(u32::from_le_bytes([b[4], b[5], b[6], b[7]])).to_string() };
        let ins = (p.ioe[5][0] as usize..(p.ioe[5][0] + p.ioe[5][1]) as usize).map(|g| word(&a.ins[5][g * 20..]));
        let outs = (p.ioe[5][2] as usize..(p.ioe[5][2] + p.ioe[5][3]) as usize).map(|g| word(&a.outs[5][g * 8..]));
        ins.chain(outs).collect()
    }

    fn patched(a: &Ainb) -> Ainb {
        let out: Vec<u8> = hook::patch_graph(&a.write().unwrap(), &HOOK).unwrap().unwrap();
        Ainb::parse(&out).unwrap()
    }

    fn base_graph(rf: &mut Romfs) -> Ainb {
        graph(rf, GLOBAL, "AI/EnemyBase.utility.root.ainb")
    }

    #[test]
    fn a_file_that_is_not_a_graph_is_refused_and_the_module_has_no_call() {
        assert!(hook::patch_graph(b"nope", &HOOK).is_err());
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let es: Vec<Entry> = sarc::parse(&rf.read_zs(GLOBAL).unwrap()).unwrap();
        assert_eq!(hook::patch_graph(&sarc::get(&es, GRAPH).unwrap().data, &HOOK).unwrap(), None);
        assert_eq!(hook::patch_graph(&sarc::get(&es, "AI/EnemyCommon.common.guardbreakomen.module.ainb").unwrap().data, &HOOK).unwrap(), None);
    }

    #[test]
    fn a_caller_gains_a_battle_timer_that_calls_and_wakes_one_helper() {
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let old: Ainb = base_graph(&mut rf);
        let a: Ainb = patched(&old);
        assert_eq!(a.els.len(), old.els.len() + ADDED);
        assert_eq!(root(&a), root(&old));
        let call: usize = a.find_els(MODULES[0])[0];
        let (parent, _): (usize, usize) = a.parent_of(call).unwrap();
        assert_eq!(a.el_type(parent), PARALLEL);
        let kids = |i: usize| -> Vec<usize> { a.params[i].group(2).iter().map(|g| g.a as usize).collect() };
        let before: usize = old.params[parent].group(2).len();
        let guard: usize = kids(parent)[before];
        assert_eq!((kids(parent).len(), a.el_type(guard)), (before + 1, SWITCH));
        let battle: usize = kids(guard)[0];
        assert_eq!((a.el_type(battle), a.el_name(kids(guard)[1])), (SWITCH, IDLE));
        let gate: usize = kids(battle)[0];
        assert_eq!((a.el_type(gate), a.el_name(kids(battle)[1])), (SWITCH, IDLE));
        let alone: usize = kids(gate)[1];
        assert_eq!((a.el_name(kids(gate)[0]), a.el_type(alone)), (WAKE, SWITCH));
        let unused: usize = kids(alone)[0];
        assert_eq!((a.el_type(unused), a.el_name(kids(alone)[1])), (SWITCH, IDLE));
        let timer: usize = kids(unused)[1];
        assert_eq!((a.el_name(kids(unused)[0]), a.el_name(timer)), (IDLE, TIMER));
        let request: usize = kids(timer)[1];
        assert_eq!((a.el_name(kids(timer)[0]), a.el_type(request)), (IDLE, PARALLEL));
        let calls: usize = kids(request)[0];
        assert_eq!((a.el_type(calls), a.el_name(kids(request)[1])), (PARALLEL, SETB));
        let pair: usize = kids(calls)[0];
        assert_eq!((a.el_type(pair), a.el_name(kids(calls)[1])), (PARALLEL, fx::EMIT), "the call flashes");
        assert_eq!(a.in_str(kids(calls)[1], "XLinkKeyName"), Some(fx::KEY));
        assert_eq!((a.el_name(kids(pair)[0]), a.el_name(kids(pair)[1])), (REQUEST, REQUEST), "two tries: the holder's MaxActors decides whether one or two come");
        let source = |i: usize| -> usize { a.input(1, a.input_ids(i, 1)[0]).unwrap().src as usize };
        let reads = |i: usize| -> String { a.el_name(source(i)).to_string() };
        assert_eq!((reads(guard), reads(unused), reads(gate), reads(alone), reads(battle)), (GETB.to_string(), GETB.to_string(), READY.to_string(), EMPTY.to_string(), FIGHT.to_string()));
        assert_eq!((a.prop_str(source(guard), "BlackboardKey"), a.prop_str(source(unused), "BlackboardKey"), a.prop_str(kids(request)[1], "BlackboardKey")), (Some(ELIGIBLE), Some(USED), Some(USED)));
        for n in [READY, EMPTY, WAKE, REQUEST] {
            assert_eq!(a.in_str(one(&a, n), "Key"), Some(KEY), "{n}");
        }
        let ptr_from = |i: usize| -> usize { a.input(5, a.input_ids(i, 5)[0]).unwrap().src as usize };
        let wake: usize = one(&a, WAKE);
        let used_mark: usize = ptr_from(wake);
        let eligible_mark: usize = ptr_from(used_mark);
        assert_eq!((a.el_name(used_mark), a.el_name(eligible_mark), a.el_name(ptr_from(eligible_mark))), (MARK, MARK, MAKE), "the helper is woken with the overwrite marks");
        assert_eq!((a.prop_str(used_mark, "Key"), a.prop_str(eligible_mark, "Key")), (Some(USED), Some(ELIGIBLE)));
        let marked: String = lines(&a, used_mark).join("
") + &lines(&a, eligible_mark).join("
");
        assert!(marked.contains("in bool Bool = true flags 0x800000") && marked.contains("in bool Bool = false flags 0x800000"), "{marked}");
        let shown: String = show(&a);
        for line in ["prop bool ExecuteOnce = true", "prop bool IsIgnoreNotExistsError = true", "in bool Bool = true flags 0x800000", "prop bool IsLoop = false", "prop f32 CountFrames = 600", "prop f32 RandomAddCountFrames = 900", "in vec3 InputB = (0, 0.5, 0) flags 0x800000", "\"NotFinish\"", "\"Finish\""] {
            assert!(shown.contains(line), "{line} is missing from\n{shown}");
        }
        assert!(hook::patch_graph(&a.write().unwrap(), &HOOK).is_err(), "a patched graph is not patched twice");
    }

    #[test]
    fn the_game_nodes_have_the_shapes_the_patch_copies() {
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let base: Ainb = base_graph(&mut rf);
        let mine: Ainb = patched(&base);
        let lure: Ainb = graph(&mut rf, "Pack/Actor/Weapon_Lure.pack.zs", "AI/Weapon_Lure.root.ainb");
        let tektite: Ainb = graph(&mut rf, "Pack/Actor/Enemy_Tektite_Blue.pack.zs", "AI/Enemy_Tektite.action.interruptboomerang.module.ainb");
        let set: usize = tektite.find_els(SETB).into_iter().find(|i| a_once(&tektite, *i)).unwrap();
        assert_eq!(shape(&mine, one(&mine, SETB)), shape(&tektite, set));
        assert_eq!(plugs(&mine, one(&mine, SETB)), plugs(&tektite, set));
        assert_eq!(classes(&mine, one(&mine, SETB)), classes(&tektite, set));
        assert_eq!(tektite.prop_str(set, "BlackboardKey"), Some("IsStunLightEnable"));
        let amos: Ainb = graph(&mut rf, "Pack/Actor/Enemy_Amos.pack.zs", "AI/Amos.brain.inaccessibletotarget.module.ainb");
        let get: usize = amos.find_els(GETB)[0];
        assert_eq!(mine.find_els(GETB).len(), 2);
        for g in mine.find_els(GETB) {
            assert_eq!(shape(&mine, g), shape(&amos, get));
            assert_eq!(plugs(&mine, g), plugs(&amos, get));
            assert_eq!(classes(&mine, g), classes(&amos, get));
        }
        for n in [READY, EMPTY, PLACE] {
            assert_eq!(shape(&mine, one(&mine, n)), shape(&lure, one(&lure, n)), "{n}");
            assert_eq!(plugs(&mine, one(&mine, n)), plugs(&lure, one(&lure, n)), "{n}");
            assert_eq!(classes(&mine, one(&mine, n)), classes(&lure, one(&lure, n)), "{n}");
        }
        let leever: Ainb = graph(&mut rf, "Pack/Actor/Enemy_Leever_Big.pack.zs", "AI/Leever_Big.action.Run.module.ainb");
        let spawned: usize = leever.find_els(MARK).into_iter().find(|i| leever.prop_str(*i, "Key") == Some("IsSpawnFromLeeverBig")).unwrap();
        let theirs_wake: usize = leever.params[spawned].plugs[0].a as usize;
        assert_eq!(leever.el_name(theirs_wake), WAKE, "the game wakes the Leevers a big Leever made with a mark saying who made them");
        assert_eq!(fields(&mine, one(&mine, WAKE)), fields(&leever, theirs_wake));
        let wired = |x: &Ainb, i: usize| -> Vec<String> { shape(x, i).into_iter().filter(|l| l.contains("OverwriteParam")).collect() };
        assert_eq!(wired(&mine, one(&mine, WAKE)), wired(&leever, theirs_wake));
        assert_eq!(classes(&mine, one(&mine, WAKE)), classes(&leever, theirs_wake));
        for m in mine.find_els(MARK) {
            assert_eq!(fields(&mine, m), fields(&leever, spawned));
            assert_eq!(plugs(&mine, m), plugs(&leever, spawned));
            assert_eq!(classes(&mine, m), classes(&leever, spawned));
        }
        let made: usize = leever.find_els(MAKE)[0];
        assert_eq!(shape(&mine, one(&mine, MAKE)), shape(&leever, made));
        assert_eq!(plugs(&mine, one(&mine, MAKE)), plugs(&leever, made));
        assert_eq!(classes(&mine, one(&mine, MAKE)), classes(&leever, made));
        let dodongo: Ainb = graph(&mut rf, "Pack/Actor/Enemy_Dodongo.pack.zs", "AI/Enemy_Dodongo.utility.root.ainb");
        for n in [SPLIT] {
            assert_eq!(shape(&mine, one(&mine, n)), shape(&dodongo, one(&dodongo, n)), "{n}");
            assert_eq!(plugs(&mine, one(&mine, n)), plugs(&dodongo, one(&dodongo, n)), "{n}");
            assert_eq!(classes(&mine, one(&mine, n)), classes(&dodongo, one(&dodongo, n)), "{n}");
        }
        let gohma: Ainb = graph(&mut rf, "Pack/Actor/Enemy_DungeonBoss_Gohma.pack.zs", "AI/DungeonBossGohma.brain.hangingbattle.module.ainb");
        assert_eq!(shape(&mine, one(&mine, FIGHT)), shape(&gohma, one(&gohma, FIGHT)));
        let bari: Ainb = graph(&mut rf, "Pack/Actor/Enemy_Bari.pack.zs", "AI/Bari.action.interruptdie.module.ainb");
        assert_eq!(fields(&mine, one(&mine, LIFT)), fields(&bari, one(&bari, LIFT)));
        assert_eq!(plugs(&mine, one(&mine, LIFT)), plugs(&bari, one(&bari, LIFT)));
        let twin: Ainb = graph(&mut rf, "Pack/Actor/Enemy_DungeonBoss_Twinrova.pack.zs", "AI/Enemy_DungeonBoss_Twinrova.utility.root.ainb");
        let theirs: usize = twin.find_els(TIMER).into_iter().find(|i| shape(&twin, *i).iter().any(|l| l.contains("IsLoop"))).unwrap();
        let ours: usize = mine.find_els(TIMER).into_iter().find(|i| *i >= base.els.len()).unwrap();
        assert_eq!(shape(&mine, ours), shape(&twin, theirs));
        let glass: Ainb = graph(&mut rf, GLOBAL, "AI/EnemyCommon.common.enableglasstruth.module.ainb");
        let sw: usize = (0..glass.els.len()).find(|i| glass.el_type(*i) == SWITCH).unwrap();
        for r in mine.find_els(REQUEST) {
            assert_eq!(shape(&mine, r), shape(&lure, one(&lure, REQUEST)));
            assert_eq!(plugs(&mine, r), plugs(&lure, one(&lure, REQUEST)));
            assert_eq!(classes(&mine, r), classes(&lure, one(&lure, REQUEST)));
        }
        let switches: Vec<usize> = (base.els.len()..mine.els.len()).filter(|i| mine.el_type(*i) == SWITCH).collect();
        assert_eq!(switches.len(), 5);
        for i in switches {
            assert_eq!(shape(&mine, i), shape(&glass, sw));
            assert_eq!(plugs(&mine, i), plugs(&glass, sw));
        }
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn the_game_actors_that_run_a_caller_can_all_call_the_helper() {
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let (plan, outs): (Plan, Vec<Out>) = stage(&mut rf, &none, "Pack/Actor").unwrap();
        for a in ["Enemy_Stalfos", "Enemy_Stalfos_ForChild", "Enemy_Keese", "Enemy_Octorok", "Enemy_Poe", "Enemy_Dodongo", "Enemy_DungeonBoss_Gohma"] {
            assert!(plan.who.contains(a), "{a} runs a caller");
        }
        for a in ["Enemy_Peahat_Larva", "Enemy_Peahat_Larva_Independent", "Player_Child", "Player_Adult", "Npc_Navi", "Enemy_Stalfos_Dictionary"] {
            assert!(!plan.who.contains(a), "{a} does not run a caller");
        }
        assert!(plan.bare.contains_key(&actor_file("Enemy_Stalfos")) && !plan.bare.contains_key(&actor_file("Enemy_Poe")));
        assert!(plan.held.contains_key("Component/ActorHolderParam/Enemy_Poe.game__component__ActorHolderParam.bgyml"));
        let last: usize = outs.len() - 1;
        assert_eq!((outs[0].path.as_str(), outs[last].path.as_str()), (GLOBAL, ACTOR_INFO));
        assert_eq!(zs::dict_of(&outs[0].z), zs::dict_of(&rf.read(GLOBAL).unwrap()), "the AI pack keeps its dictionary");
        let mut copies: Map<String, Vec<u8>> = Map::new();
        let mut packs: Map<String, Vec<Entry>> = Map::new();
        let mut shared: Vec<Entry> = Vec::new();
        for x in &outs[..last] {
            assert!(x.new > x.old, "{} grew", x.path);
            let now: Vec<Entry> = sarc::parse(&rf.zs.decompress(&x.z).unwrap()).unwrap();
            let old: Vec<Entry> = sarc::parse(&rf.read_zs(&x.path).unwrap()).unwrap();
            for g in GRAPHS {
                assert_eq!(sarc::get(&now, g).map(|e| &e.data), sarc::get(&old, g).map(|e| &e.data), "{g} is vanilla in {}", x.path);
            }
            for e in &now {
                match sarc::get(&old, &e.name) {
                    Some(o) if o.data == e.data => continue,
                    Some(o) if hook::is_caller(o) => continue,
                    Some(_) => assert!(plan.wants(&e.name), "{} changed in {}", e.name, x.path),
                    None => assert!(HELPERS.iter().any(|h| h.1 == e.name), "{}", x.path),
                }
                let first: &Vec<u8> = copies.entry(e.name.clone()).or_insert_with(|| e.data.clone());
                assert_eq!(first, &e.data, "every copy of {} is the same", e.name);
            }
            assert_eq!(now.len(), old.len() + HELPERS.iter().filter(|h| sarc::get(&now, h.1).is_some() && sarc::get(&old, h.1).is_none()).count(), "{}", x.path);
            if x.path == GLOBAL {
                shared = now;
            } else {
                packs.insert(actor_of(&x.path).unwrap().to_string(), now);
            }
        }
        let table: Vec<u8> = copies.get(TABLE).unwrap().clone();
        let total: usize = rf.list("Pack/Actor").unwrap().iter().filter(|f| sarc::parse(&rf.read_zs(f).unwrap()).unwrap().iter().any(|e| e.name == TABLE)).count();
        let staged: usize = packs.values().filter(|es| sarc::get(es, TABLE).is_some()).count();
        assert_eq!(staged, total, "every copy of the common blackboard table carries the backup keys");
        assert!(packs.values().filter_map(|es| sarc::get(es, TABLE)).all(|t| t.data == table));
        let dead: usize = plan.who.iter().filter(|a| !packs.contains_key(*a)).count();
        assert!(dead > 0 && plan.tabled.iter().all(|a| packs.contains_key(a)));
        for a in ["Npc_SkullDad", "Obj_InsideJabu_Jabu_SwayingFloor_A_01"] {
            assert!(plan.who.contains(a) && !plan.callers().any(|c| c == a), "{a} runs a caller and is no backup caller");
        }
        let hooked = |es: &[Entry]| -> Vec<String> { es.iter().filter(|e| hook::is_caller(e) && Ainb::parse(&e.data).unwrap().find_els(REQUEST).len() > 0).map(|e| e.name.clone()).collect() };
        let in_global: Vec<String> = hooked(&shared);
        assert!(in_global.contains(&"AI/EnemyBase.utility.root.ainb".to_string()) && in_global.contains(&"AI/Enemy_Stalfos.utility.root.ainb".to_string()));
        assert!(!in_global.contains(&"AI/Enemy_IronKnuckle.utility.root.ainb".to_string()), "a graph only bosses use is left alone");
        assert!(!in_global.contains(&"AI/DungeonBossBase.utility.root.ainb".to_string()));
        let count = |e: &Entry| -> usize { Ainb::parse(&e.data).unwrap().find_els(REQUEST).len() };
        for (a, es) in &packs {
            let want: bool = a.starts_with(CALLERS) && !excluded(a);
            let old: Vec<Entry> = sarc::parse(&rf.read_zs(&format!("Pack/Actor/{a}.pack.zs")).unwrap()).unwrap();
            for e in es.iter().filter(|e| hook::is_caller(e)) {
                let was: &Entry = sarc::get(&old, &e.name).unwrap();
                assert_eq!(count(e) > count(was), want, "{a} {}", e.name);
            }
        }
        assert!(plan.held.keys().all(|h| h.starts_with("Component/ActorHolderParam/Enemy_")), "{:?}", plan.held);
        let mut fell: Set<String> = Set::new();
        for a in plan.callers() {
            let es: &Vec<Entry> = packs.get(a).unwrap_or_else(|| panic!("the pack of {a} was not staged"));
            let h: String = holder_of(es, a).unwrap().unwrap_or_else(|| panic!("{a} has no holder"));
            let y: Byml = Byml::parse(&sarc::get(es, &h).unwrap_or_else(|| panic!("{a} lacks {h}")).data).unwrap();
            let want: usize = plan.held.get(&h).or(plan.bare.get(&actor_file(a))).copied().unwrap_or_else(|| panic!("{a} has no helper"));
            assert_eq!(y.root.get(HELD).and_then(|m| m.get(KEY)), Some(&entry(HELPERS[want].0, HELPERS[want].2)), "{a} through {h}");
            if want != helper_of(a) {
                fell.insert(a.to_string());
            }
            let flags: Byml = Byml::parse(&sarc::get(es, TABLE).unwrap_or_else(|| panic!("{a} has no common blackboard")).data).unwrap();
            let rows: &[Node] = flags.root.get(BOOLS).and_then(Node::as_arr).unwrap();
            assert_eq!(&rows[rows.len() - 2..], &[state_row(ELIGIBLE, true), state_row(USED, false)], "{a}");
        }
        assert_eq!(fell, FELL_BACK.iter().map(|a| a.to_string()).collect::<Set<String>>(), "callers whose holder is shared with another group fall back to the Keese");
        assert!(plan.off.len() >= 30, "{:?}", plan.off);
        for a in &plan.off {
            assert!(excluded(a) && !plan.callers().any(|c| c == a), "{a}");
        }
        for a in ["Enemy_Octorok", "Enemy_Bari", "Enemy_Biri", "Enemy_Stinger", "Enemy_Freezard", "Enemy_Gibdo", "Enemy_Wallmaster", "Enemy_Floormaster", "Enemy_Stalchild", "Enemy_Stalfos", "Enemy_Lizalfos", "Enemy_Wolfos", "Enemy_Moblin", "Enemy_Skulltula", "Enemy_Dekubaba", "Enemy_Scrub_Mad", "Enemy_Amos", "Enemy_TorchSlug", "Enemy_Tektite_Blue"] {
            assert!(plan.callers().any(|c| c == a), "{a} is a caller");
        }
        for a in ["Enemy_ReDead", "Enemy_ReDead_AlwaysRespawn", "Enemy_Leever", "Enemy_Peahat", "Enemy_Keese", "Enemy_Keese_Ice", "Enemy_Keese_Fire", "Enemy_BabyDodongo", "Enemy_Guay", "Enemy_Skullwalltula", "Enemy_Floormaster_Little", "Enemy_FlyingPot", "Enemy_Shabom", "Enemy_Skulltula_Gold", "Enemy_Poe_Amy", "Enemy_Poe_Beth", "Enemy_Poe_Joelle"] {
            assert!(!plan.callers().any(|c| c == a), "{a} never calls");
        }
        for (helper, _, _) in HELPERS {
            assert!(rf.read(&format!("Pack/Actor/{helper}.pack.zs")).is_ok(), "{helper} has a pack");
            assert!(plan.who.contains(helper), "{helper} runs the shared graph, so the mark it is woken with is read by its own guard");
            let es: &Vec<Entry> = packs.get(helper).unwrap_or_else(|| panic!("the pack of {helper} was not staged"));
            let k: Map<String, bool> = keys_of(es, helper);
            assert_eq!((k.get(ELIGIBLE), k.get(USED)), (Some(&false), Some(&false)), "{helper} has both marked keys as plain keys of its own blackboard");
        }
        let peer: Byml = Byml::parse(&sarc::get(&sarc::parse(&rf.read_zs(PEER.0).unwrap()).unwrap(), PEER.1).unwrap().data).unwrap();
        assert_eq!((peer.version, peer.big), (VERSION, false));
        let was: Byml = rf.read_byml(ACTOR_INFO).unwrap();
        let now: Byml = Byml::parse(&rf.zs.decompress(&outs[last].z).unwrap()).unwrap();
        let (a, b): (&[Node], &[Node]) = (was.root.as_arr().unwrap(), now.root.as_arr().unwrap());
        assert_eq!(a.len(), b.len());
        let heap = |row: &Node| -> Option<i32> {
            match row.get(rows::HEAP)? {
                Node::I32(v) => Some(*v),
                Node::U32(v) => i32::try_from(*v).ok(),
                _ => None,
            }
        };
        let mut grown: usize = 0;
        for (x, y) in a.iter().zip(b.iter()) {
            if x == y {
                continue;
            }
            let name: &str = rows::id(x).unwrap();
            assert!(plan.who.contains(name) || plan.tabled.contains(name), "{name} changed");
            assert_eq!(heap(y), Some(heap(x).unwrap() + HEAP_EXTRA));
            grown += 1;
        }
        assert!(grown >= 80, "{grown} actors grew");
        let mut t: Rst = crate::rsizetable::load(&mut rf).unwrap();
        let caller: &str = "AI/EnemyBase.utility.root.ainb";
        let before: Option<u32> = t.get(caller);
        let stalfos: &str = "Actor/Enemy_Stalfos.engine__actor__ActorParam.bgyml";
        let param: Option<u32> = t.get(stalfos);
        assert!(before.is_some() && param.is_some() && HELPERS.iter().all(|h| t.get(h.1).is_none()));
        for o in &outs {
            crate::staging::grow_out(&mut rf, &mut t, o).unwrap();
        }
        assert!(t.get(caller) > before, "the size table entry of {caller} grew");
        assert!(t.get(stalfos) > param);
        size(&mut rf, &mut t).unwrap();
        for (helper, file, max) in HELPERS {
            assert!(t.get(file).is_some_and(|n| n as usize > holder_file(helper, max).unwrap().len()), "{file}");
        }
        println!("{} actors, {} callers, {} off, {} bare, {} held, {} files", plan.who.len(), plan.callers().count(), plan.off.len(), plan.bare.len(), plan.held.len(), outs.len());
    }

    #[test]
    fn the_vanilla_damage_color_module_is_untouched_in_every_combination_of_the_three() {
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let es: Vec<Entry> = sarc::parse(&rf.read_zs(GLOBAL).unwrap()).unwrap();
        let module: &Entry = sarc::get(&es, GRAPH).unwrap();
        let old: Ainb = Ainb::parse(&module.data).unwrap();
        assert_eq!((old.els.len(), root(&old), old.el_name(0)), (1, Some(0), "ExecuteSetEnemyDamageColorAndLight"));
        let hooks: [Hook; 3] = [regen::HOOK, fairy::HOOK, HOOK];
        let added: [usize; 3] = [regen::ADDED, fairy::ADDED, ADDED];
        let base: Ainb = Ainb::parse(&sarc::get(&es, "AI/EnemyBase.utility.root.ainb").unwrap().data).unwrap();
        let call: usize = base.find_els(MODULES[0])[0];
        let (parent, _): (usize, usize) = base.parent_of(call).unwrap();
        for mask in 0..8usize {
            let mut packed: Vec<Entry> = es.clone();
            let mut extra: usize = 0;
            for (i, h) in hooks.iter().enumerate().filter(|(i, _)| mask & (1 << i) != 0) {
                assert!(hook::patch_entries(&mut packed, h, &|_| true).unwrap());
                extra += added[i];
            }
            for g in GRAPHS {
                assert_eq!(sarc::get(&packed, g).map(|e| &e.data), sarc::get(&es, g).map(|e| &e.data), "{g} with mask {mask}");
            }
            let out: Ainb = Ainb::parse(&sarc::get(&packed, "AI/EnemyBase.utility.root.ainb").unwrap().data).unwrap();
            assert_eq!(out.els.len(), base.els.len() + extra, "mask {mask}");
            assert_eq!(out.params[parent].group(2).len(), base.params[parent].group(2).len() + mask.count_ones() as usize, "mask {mask}");
            assert_eq!(out.params[call], base.params[call], "the call itself is untouched with mask {mask}");
            assert_eq!(out.find_els(MODULES[0]), vec![call]);
        }
    }
    fn blackboard_type(a: &Ainb, i: usize) -> Option<i32> {
        show(a).split("\nelement ").nth(i + 1)?.lines().find_map(|l| l.trim().strip_prefix("prop s32 BlackboardType = "))?.parse().ok()
    }

    fn own_keys(a: &Ainb) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut at: usize = 56;
        for t in 0..7 {
            for _ in 0..u16::from_le_bytes([a.bb[t * 8], a.bb[t * 8 + 1]]) {
                out.push(a.s(u32::from_le_bytes(a.bb[at..at + 4].try_into().unwrap()) & 0x3f_ffff).to_string());
                at += 8;
            }
        }
        out
    }

    fn rows_in(es: &[Entry], file: &str, out: &mut Map<String, bool>, depth: usize) {
        let Some(e) = sarc::get(es, file) else { return };
        let y: Byml = Byml::parse(&e.data).unwrap();
        let Node::Map(m) = &y.root else { return };
        for r in m.iter().filter_map(|(_, v)| v.as_arr()).flatten() {
            if let Some(k) = r.get("BBKey").and_then(Node::as_str) {
                out.insert(k.to_string(), r.get("IsInstanceParam") == Some(&Node::Bool(true)));
            }
        }
        if let Some(up) = y.root.get(PARENT).and_then(Node::as_str).and_then(packed).filter(|_| depth < DEPTH) {
            rows_in(es, &up, out, depth + 1);
        }
    }

    fn keys_of(es: &[Entry], actor: &str) -> Map<String, bool> {
        let mut info: Option<String> = None;
        let mut at: Option<String> = Some(actor_file(actor));
        for _ in 0..DEPTH {
            let Some(y) = at.as_deref().and_then(|f| sarc::get(es, f)).map(|e| Byml::parse(&e.data).unwrap()) else { break };
            if let Some(r) = y.root.get(PARTS).and_then(|c| c.get("BlackboardRef")).and_then(Node::as_str) {
                info = packed(r);
                break;
            }
            at = y.root.get(PARENT).and_then(Node::as_str).and_then(packed);
        }
        let mut out: Map<String, bool> = Map::new();
        for _ in 0..DEPTH {
            let Some(y) = info.as_deref().and_then(|f| sarc::get(es, f)).map(|e| Byml::parse(&e.data).unwrap()) else { break };
            for t in y.root.get("BlackboardParamTables").and_then(Node::as_arr).unwrap_or(&[]) {
                if let Some(f) = t.as_str().and_then(packed) {
                    rows_in(es, &f, &mut out, 0);
                }
            }
            info = y.root.get(PARENT).and_then(Node::as_str).and_then(packed);
        }
        out
    }

    fn table_keys(rf: &mut Romfs) -> Map<String, bool> {
        let mut out: Map<String, bool> = Map::new();
        for f in rf.list("Pack/Actor").unwrap() {
            let es: Vec<Entry> = sarc::parse(&rf.read_zs(&f).unwrap()).unwrap();
            for e in es.iter().filter(|e| e.name.contains("BlackboardParamTable/")) {
                let mut rows: Map<String, bool> = Map::new();
                rows_in(&es, &e.name, &mut rows, DEPTH);
                for (k, inst) in rows {
                    *out.entry(k).or_insert(inst) |= inst;
                }
            }
        }
        out
    }

    #[test]
    fn the_game_reads_placement_keys_from_graphs_but_never_writes_one() {
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let rows: Map<String, bool> = table_keys(&mut rf);
        let mut graphs: Vec<Entry> = sarc::parse(&rf.read_zs(GLOBAL).unwrap()).unwrap();
        for f in rf.list("Pack/Actor").unwrap() {
            graphs.extend(sarc::parse(&rf.read_zs(&f).unwrap()).unwrap().into_iter().filter(|e| e.name.ends_with(".ainb")));
        }
        let (mut plain, mut reads, mut placed): (usize, usize, Vec<String>) = (0, 0, Vec::new());
        for e in &graphs {
            let Ok(a) = Ainb::parse(&e.data) else { continue };
            for i in (0..a.els.len()).filter(|i| a.el_name(*i).contains("Blackboard")) {
                if blackboard_type(&a, i) != Some(1) {
                    continue;
                }
                match a.prop_str(i, "BlackboardKey").and_then(|k| rows.get(k)) {
                    Some(true) if a.el_name(i).starts_with("Query") => reads += 1,
                    Some(true) => placed.push(format!("{} {}", e.name, a.el_name(i))),
                    Some(false) => plain += 1,
                    None => {}
                }
            }
        }
        assert!(placed.is_empty(), "{placed:?}");
        assert!(reads > 20, "{reads} nodes read a placement key");
        assert!(plain > 3000, "{plain} nodes read or write a plain actor key");
    }

    #[test]
    fn every_key_a_backup_node_uses_is_a_plain_key_where_its_blackboard_type_looks_for_every_actor_that_runs_it() {
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let (plan, outs): (Plan, Vec<Out>) = stage(&mut rf, &none, "Pack/Actor").unwrap();
        let mut staged: Map<String, Vec<Entry>> = Map::new();
        for x in &outs[..outs.len() - 1] {
            staged.insert(x.path.clone(), sarc::parse(&rf.zs.decompress(&x.z).unwrap()).unwrap());
        }
        let old_global: Vec<Entry> = sarc::parse(&rf.read_zs(GLOBAL).unwrap()).unwrap();
        let mut uses: Map<String, Vec<(i32, String)>> = Map::new();
        let mut hosted = |path: &str, es: &[Entry], old: &[Entry]| {
            for e in es.iter().filter(|e| hook::is_caller(e)) {
                let a: Ainb = Ainb::parse(&e.data).unwrap();
                let first: usize = sarc::get(old, &e.name).map(|o| Ainb::parse(&o.data).unwrap().els.len()).unwrap_or(0);
                let mut keys: Vec<(i32, String)> = Vec::new();
                for i in first..a.els.len() {
                    let Some(t) = blackboard_type(&a, i) else { continue };
                    let k: String = a.prop_str(i, "BlackboardKey").unwrap().to_string();
                    if t == 0 {
                        assert!(own_keys(&a).contains(&k), "{k} is not a key of {} in {path}", e.name);
                    }
                    keys.push((t, k));
                }
                if !keys.is_empty() {
                    uses.insert(format!("{path}:{}", e.name), keys);
                }
            }
        };
        hosted(GLOBAL, staged.get(GLOBAL).unwrap(), &old_global);
        let mut packs: Vec<(String, Vec<Entry>)> = Vec::new();
        for f in rf.list("Pack/Actor").unwrap() {
            let old: Vec<Entry> = sarc::parse(&rf.read_zs(&f).unwrap()).unwrap();
            let now: Vec<Entry> = staged.get(&f).cloned().unwrap_or_else(|| old.clone());
            hosted(&f, &now, &old);
            packs.push((f, now));
        }
        let global: Set<String> = uses.keys().filter_map(|k| k.strip_prefix(&format!("{GLOBAL}:"))).map(str::to_string).collect();
        assert!(global.contains("AI/EnemyBase.utility.root.ainb"), "{global:?}");
        let (mut checked, mut lacking): (usize, Vec<String>) = (0, Vec::new());
        for (f, now) in &packs {
            let actor: &str = actor_of(f).unwrap();
            let mut run: Vec<String> = hook::shared_of(now, &global).into_iter().map(|g| format!("{GLOBAL}:{g}")).collect();
            run.extend(uses.keys().filter(|k| k.starts_with(&format!("{f}:"))).cloned());
            if run.is_empty() {
                continue;
            }
            let keys: Map<String, bool> = keys_of(now, actor);
            for g in &run {
                for (_, k) in uses.get(g).unwrap().iter().filter(|u| u.0 == 1) {
                    match keys.get(k) {
                        Some(false) => checked += 1,
                        Some(true) => panic!("{actor} runs {g}, whose node names the placement key {k}: the game resolves it to no handle"),
                        None => lacking.push(format!("{actor} {g} {k}")),
                    }
                }
            }
        }
        assert!(lacking.is_empty(), "{} uses of a key an actor does not have: {lacking:?}", lacking.len());
        assert!(checked >= 2 * plan.callers().count(), "{checked}");
    }
}
