use std::collections::{BTreeMap as BM, BTreeSet as BS};
use std::fmt::Write as W;
use std::path::{Path as P, PathBuf as PB};

use crate::bfevfl::flow::NONE;
use crate::bfevfl::{Actor, Bf, Cont, Entry, Event, Flow, Kind, Val};
use crate::byml::{Byml, Node};
use crate::dungeon_text;
use crate::error::{Error, Res};
use crate::hints::languages;
use crate::placement::{banc_files, hash_of};
use crate::progressive::{add_rows, edit_chest, Built, Size, ANALYZED_TABLE, BIG_FLOW, SETTING_TABLE, SMALL_FLOW};
use crate::romfs::Romfs;
use crate::staging::{banc_out, banc_raw, commit, rewrite, Committed, Out};

pub const DUNGEONS: [&str; 14] = [
    "InsideTheDekuTree",
    "DodongoCavern",
    "InsideJabuJabuBelly",
    "ForestTemple",
    "FireTemple",
    "IceCavern",
    "WaterTemple",
    "BottomOfTheWell",
    "ShadowTemple",
    "SpiritTemple",
    "GerudoTrainingGround",
    "InsideGanonCastle",
    "FinalBattleField",
    "GerudoFortress",
];

pub const KEY: &str = "Item_Dungeon_SmallKey";
pub const POUCH_TABLE: &str = "RSDB/PouchActorInfo.Product.111.rstbl.byml.zs";
pub const DEAL: &str = "PouchSpecialDeal";
pub const NATIVE_DEAL: &str = "SmallKey";
pub const QUIET_DEAL: &str = "NotPushToPouch";
pub const STOCK: &str = "PouchStockable";
pub const TALKER: &str = "SystemTextNPC";
pub const TALK: &str = "EventTalk";
pub const STYLE: i32 = 4;
pub const NATIVE: [&str; 4] = ["EventGetDungeonMap", "EventGetDungeonCompass", "EventGetDungeonBossKey", "EventGetDungeonSmallKey"];
pub const PLAIN: &str = "None";
pub const SMALL_NATIVE: [&str; 1] = ["EventGetDungeonSmallKey"];
pub const EVENT_PREFIX: &str = "OOTRR_Key_";
pub const KEY_DUNGEONS: [&str; 8] = [
    "ForestTemple",
    "FireTemple",
    "WaterTemple",
    "ShadowTemple",
    "SpiritTemple",
    "BottomOfTheWell",
    "GerudoTrainingGround",
    "InsideGanonCastle",
];
pub const DROP: &str = "Drop__DropActor";
pub const HOLDER: &str = "GameDataActor";
const ADD: &str = "EventTriggerAddOrSubGameDataInt";
pub const SET: &str = "EventTriggerSetGameDataBool";
const ROOT_ENTRY: &str = "Success_";
const PREFIX: &str = "Success_Key_";
const ADDED: [&str; 2] = [HOLDER, TALKER];

#[derive(Clone, Debug)]
pub struct Opts {
    pub rst_from: Option<PB>,
    pub banc_from: Option<PB>,
    pub count: Vec<String>,
}

pub fn dungeon(name: &str) -> Res<&'static str> {
    DUNGEONS
        .iter()
        .find(|d| **d == name)
        .copied()
        .ok_or_else(|| Error::Msg(format!("{name} is not a dungeon with a key count, dungeons: {}", DUNGEONS.join(" "))))
}

pub fn key_dungeon(name: &str) -> Option<&'static str> {
    match name {
        "Forest Temple" => Some("ForestTemple"),
        "Fire Temple" => Some("FireTemple"),
        "Water Temple" => Some("WaterTemple"),
        "Shadow Temple" => Some("ShadowTemple"),
        "Spirit Temple" => Some("SpiritTemple"),
        "Bottom of the Well" => Some("BottomOfTheWell"),
        "Gerudo Training Ground" => Some("GerudoTrainingGround"),
        "Ganons Castle" => Some("InsideGanonCastle"),
        _ => None,
    }
}

pub fn scene_dungeon(file: &str) -> Option<&'static str> {
    let scene: &str = file.split('/').nth(1)?;
    KEY_DUNGEONS.iter().copied().find(|d| *d == scene)
}

pub fn entry_name(d: &str) -> String {
    format!("{PREFIX}{d}")
}

pub fn event_name(d: &str, size: Size) -> String {
    let tag: &str = match size {
        Size::Small => "Small",
        Size::Big => "Big",
    };
    format!("OOTRR_Key_{d}_{tag}")
}

pub fn flag(d: &str, member: &str) -> String {
    format!("DungeonInfo.{d}.{member}")
}

pub fn idx(n: usize) -> Res<u16> {
    u16::try_from(n).map_err(|_| Error::Format("dungeon credit: index does not fit in 16 bits"))
}

pub fn push(f: &mut Flow, kind: Kind) -> Res<u16> {
    let i: u16 = idx(f.events.len())?;
    f.events.push(Event { name: format!("Key{i}"), kind });
    Ok(i)
}

fn holder(f: &mut Flow, who: &str) -> Res<usize> {
    if let Some(i) = f.actors.iter().position(|a| a.name == who && a.sub.is_empty()) {
        return Ok(i);
    }
    let t: &Actor = f.actors.first().ok_or(Error::Format("dungeon credit: the flow has no actors"))?;
    let a: Actor = Actor {
        name: who.to_string(),
        sub: String::new(),
        arg: String::new(),
        actions: Vec::new(),
        queries: Vec::new(),
        params: t.params.clone(),
        ep: t.ep,
        cut: t.cut,
    };
    f.actors.push(a);
    Ok(f.actors.len() - 1)
}

pub fn action_slot(f: &mut Flow, who: &str, name: &str) -> Res<(u16, u16)> {
    let i: usize = holder(f, who)?;
    let a: &mut Actor = &mut f.actors[i];
    let j: usize = match a.actions.iter().position(|x| x == name) {
        Some(j) => j,
        None => {
            a.actions.push(name.to_string());
            a.actions.len() - 1
        }
    };
    Ok((idx(i)?, idx(j)?))
}

fn add_int(name: String) -> Cont {
    Cont {
        items: vec![
            ("Index".to_string(), Val::Int(-1)),
            ("GameDataName".to_string(), Val::Str(name)),
            ("Value".to_string(), Val::Int(1)),
        ],
    }
}

pub fn set_bool(name: String) -> Cont {
    Cont {
        items: vec![
            ("Index".to_string(), Val::Int(-1)),
            ("Value".to_string(), Val::Bool(1)),
            ("GameDataName".to_string(), Val::Str(name)),
        ],
    }
}

fn give(next: u16) -> Kind {
    let c: Cont = Cont {
        items: vec![
            ("ContentName".to_string(), Val::Str(KEY.to_string())),
            ("IsMiniGame".to_string(), Val::Bool(0)),
        ],
    };
    Kind::Sub { next, params: Some(c), flow: String::new(), entry: ROOT_ENTRY.to_string() }
}

pub fn talk(id: String) -> Cont {
    let off = |k: &str| (k.to_string(), Val::Bool(0));
    let zero = |k: &str| (k.to_string(), Val::Int(0));
    Cont {
        items: vec![
            ("IsWaitEnd".to_string(), Val::Bool(1)),
            ("IsCloseDialog".to_string(), Val::Bool(1)),
            zero("EndDialogOption"),
            ("ASName".to_string(), Val::Str(String::new())),
            off("IsAnimeDriven"),
            off("IsPlayASAllSlot"),
            zero("ChoiceNumber"),
            zero("ChoiceLabel1"),
            zero("ChoiceLabel2"),
            zero("ChoiceLabel3"),
            zero("ChoiceLabel4"),
            zero("ChoiceDefaultNo"),
            zero("ChoiceCancelNo"),
            off("IsNotBecomeSpeaker"),
            off("ChangeMstxtForActor"),
            off("IsAnonymous"),
            off("NotPlayLipSyncAnim"),
            off("IsNotOpenIfSkipedMostRecent"),
            ("IsOverrideStyle".to_string(), Val::Bool(1)),
            ("OverrideStyle".to_string(), Val::Int(STYLE)),
            ("MessageId".to_string(), Val::Str(id)),
            zero("MaintainPerformanceState"),
        ],
    }
}

pub fn credit(f: &mut Flow, d: &str) -> Res<()> {
    let d: &str = dungeon(d)?;
    let name: String = entry_name(d);
    if f.entries.iter().any(|e| e.name == name) {
        return Ok(());
    }
    let pad: u16 = f
        .entries
        .iter()
        .find(|e| e.name == ROOT_ENTRY)
        .map(|e| e.pad)
        .ok_or(Error::Format("dungeon credit: the chest flow has no Success_ entry"))?;
    let (a, add): (u16, u16) = action_slot(f, HOLDER, ADD)?;
    let (_, set): (u16, u16) = action_slot(f, HOLDER, SET)?;
    let (t, say): (u16, u16) = action_slot(f, TALKER, TALK)?;
    let at: u16 = idx(f.events.len())?;
    let last: u16 = idx(f.events.len() + 3)?;
    push(f, Kind::Action { next: at + 1, actor: a, act: add, params: Some(add_int(flag(d, "SmallKeyNum"))) })?;
    push(f, Kind::Action { next: at + 2, actor: a, act: add, params: Some(add_int(flag(d, "SmallKeyTotalNum"))) })?;
    push(f, Kind::Action { next: last, actor: a, act: set, params: Some(set_bool(flag(d, "IsGetSmallKeyOnce"))) })?;
    push(f, give(last + 1))?;
    push(f, Kind::Action { next: NONE, actor: t, act: say, params: Some(talk(dungeon_text::message_id(d))) })?;
    f.entries.push(Entry { name, subs: vec![last], vars: Vec::new(), main: at, pad });
    Ok(())
}

fn native_next(f: &Flow, i: u16) -> Option<u16> {
    native_in(f, i, &NATIVE)
}

fn native_in(f: &Flow, i: u16, kinds: &[&str]) -> Option<u16> {
    let Kind::Action { next, actor, act, .. } = &f.events.get(i as usize)?.kind else {
        return None;
    };
    let n: &String = f.actors.get(*actor as usize)?.actions.get(*act as usize)?;
    kinds.contains(&n.as_str()).then_some(*next)
}

fn targets(k: &Kind) -> Vec<u16> {
    match k {
        Kind::Action { next, .. } | Kind::Join { next } | Kind::Sub { next, .. } => vec![*next],
        Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => cases.iter().map(|c| c.ev).collect(),
        Kind::Fork { join, forks } => forks.iter().copied().chain([*join]).collect(),
        Kind::Label { .. } => Vec::new(),
    }
}

pub fn skip_native(f: &mut Flow) -> Res<usize> {
    skip_native_of(f, &NATIVE)
}

pub fn skip_native_of(f: &mut Flow, kinds: &[&str]) -> Res<usize> {
    let jump: Vec<Option<u16>> = (0..f.events.len()).map(|i| idx(i).ok().and_then(|i| native_in(f, i, kinds))).collect();
    let mut n: usize = 0;
    for e in f.events.iter_mut() {
        let Kind::Switch { cases, .. } = &mut e.kind else {
            continue;
        };
        for c in cases.iter_mut() {
            if let Some(Some(to)) = jump.get(c.ev as usize) {
                c.ev = *to;
                n += 1;
            }
        }
    }
    let reached: bool = f.events.iter().flat_map(|e| targets(&e.kind)).chain(f.entries.iter().map(|e| e.main)).any(|t| matches!(jump.get(t as usize), Some(Some(_))));
    if reached {
        return Err(Error::Format("dungeon credit: a native credit action is still reachable, the game files differ"));
    }
    Ok(n)
}

pub fn sound(f: &Flow) -> Vec<String> {
    let n: usize = f.events.len();
    let ok = |t: u16| t == NONE || (t as usize) < n;
    let mut bad: Vec<String> = Vec::new();
    for (i, e) in f.events.iter().enumerate() {
        if targets(&e.kind).into_iter().any(|t| !ok(t)) {
            bad.push(format!("event {i} {} leads outside the flow", e.name));
        }
        let known: bool = match &e.kind {
            Kind::Action { actor, act, .. } => f.actors.get(*actor as usize).is_some_and(|a| (*act as usize) < a.actions.len()),
            Kind::Switch { actor, query, .. } => f.actors.get(*actor as usize).is_some_and(|a| (*query as usize) < a.queries.len()),
            _ => true,
        };
        if !known {
            bad.push(format!("event {i} {} names an action or query its actor does not have", e.name));
        }
    }
    for e in &f.entries {
        if e.main as usize >= n {
            bad.push(format!("entry {} starts outside the flow", e.name));
        }
        for s in &e.subs {
            if !matches!(f.events.get(*s as usize).map(|x| &x.kind), Some(Kind::Sub { .. } | Kind::SubSwitch { .. })) {
                bad.push(format!("entry {} lists event {s} which is not a sub flow call", e.name));
            }
        }
    }
    bad
}

fn same_but_actions(a: &Actor, b: &Actor) -> bool {
    let mut x: Actor = b.clone();
    x.actions.truncate(a.actions.len());
    x == *a && ADDED.contains(&b.name.as_str())
}

fn same_but_skips(f: &Flow, a: &Kind, b: &Kind) -> bool {
    let (Kind::Switch { actor: a0, query: q0, cases: c0, params: p0 }, Kind::Switch { actor: a1, query: q1, cases: c1, params: p1 }) = (a, b) else {
        return false;
    };
    (a0, q0, p0, c0.len()) == (a1, q1, p1, c1.len())
        && c0.iter().zip(c1).all(|(x, y)| x.val == y.val && (x.ev == y.ev || native_next(f, x.ev) == Some(y.ev)))
}

pub fn audit(a: &Flow, b: &Flow) -> Vec<String> {
    let mut bad: Vec<String> = sound(b);
    if a.name != b.name {
        bad.push("the flow name changed".to_string());
    }
    if b.actors.len() < a.actors.len() || b.events.len() < a.events.len() || b.entries.len() < a.entries.len() {
        bad.push("actors, events or entries were removed".to_string());
        return bad;
    }
    for (i, (x, y)) in a.actors.iter().zip(&b.actors).enumerate() {
        if x != y && !same_but_actions(x, y) {
            bad.push(format!("actor {i} {} changed in more than new GameData or talk actions", x.name));
        }
    }
    for y in &b.actors[a.actors.len()..] {
        if !ADDED.contains(&y.name.as_str()) || !y.queries.is_empty() {
            bad.push(format!("actor {} was added", y.name));
        }
    }
    for (i, (x, y)) in a.events.iter().zip(&b.events).enumerate() {
        if x != y && !(x.name == y.name && same_but_skips(a, &x.kind, &y.kind)) {
            bad.push(format!("event {i} {} changed in more than a skipped native credit", x.name));
        }
    }
    for (i, y) in b.events.iter().enumerate().skip(a.events.len()) {
        let fine: bool = match &y.kind {
            Kind::Action { actor, act, .. } => b.actors.get(*actor as usize).is_some_and(|x| match x.name.as_str() {
                HOLDER => x.actions.get(*act as usize).is_some_and(|n| n == ADD || n == SET),
                TALKER => x.actions.get(*act as usize).is_some_and(|n| n == TALK),
                _ => false,
            }),
            Kind::Sub { flow, entry, .. } => flow.is_empty() && entry == ROOT_ENTRY,
            _ => false,
        };
        if !fine {
            bad.push(format!("new event {i} {} is neither a GameData action, a talk nor a call of {ROOT_ENTRY}", y.name));
        }
    }
    for (x, y) in a.entries.iter().zip(&b.entries) {
        if x != y {
            bad.push(format!("entry {} changed", x.name));
        }
    }
    for y in &b.entries[a.entries.len()..] {
        if !y.name.strip_prefix(PREFIX).is_some_and(|d| DUNGEONS.contains(&d)) {
            bad.push(format!("entry {} was added", y.name));
        }
    }
    bad
}

pub fn credit_keys(f: &mut Flow, dungeons: &[&str], big: bool) -> Res<usize> {
    let n: usize = if big { skip_native_of(f, &SMALL_NATIVE)? } else { 0 };
    for d in dungeons {
        credit(f, d)?;
    }
    if let Some(m) = sound(f).first() {
        return Err(Error::Msg(format!("dungeon credit: the chest flow is broken after the key entries, {m}")));
    }
    Ok(n)
}

pub fn sweep(y: &mut Byml, file: &str) -> Res<usize> {
    let scene: Option<&'static str> = scene_dungeon(file);
    let Some(list) = y.root.get_mut("Actors").and_then(|a| a.as_arr_mut()) else {
        return Ok(0);
    };
    let mut n: usize = 0;
    for a in list.iter_mut() {
        let size: Size = Size::of_gyaml(text(a, "Gyaml"));
        let Some(d) = a.get_mut("Dynamic") else {
            continue;
        };
        if text(d, DROP) != KEY || text(d, "GetEventName").starts_with(EVENT_PREFIX) {
            continue;
        }
        let dungeon: &str = scene.ok_or_else(|| Error::Msg(format!("{file} has a chest with a small key but is not a dungeon scene this build knows")))?;
        d.set("GetEventName", Node::Str(event_name(dungeon, size)));
        n += 1;
    }
    Ok(n)
}

pub fn stage_pouch(rf: &mut Romfs) -> Res<Out> {
    rewrite(rf, POUCH_TABLE, |raw: &[u8]| {
        let mut y: Byml = Byml::parse(raw)?;
        quiet_pouch(&mut y)?;
        y.write()
    })
}

pub fn edit_flow(raw: &[u8], dungeons: &[&str], native: bool) -> Res<(Vec<u8>, usize)> {
    let orig: Bf = Bf::parse(raw)?;
    let mut b: Bf = orig.clone();
    let before: &Flow = orig.flows.first().ok_or(Error::Format("dungeon credit: the flow file holds no flow"))?;
    let f: &mut Flow = b.flows.first_mut().ok_or(Error::Format("dungeon credit: the flow file holds no flow"))?;
    let n: usize = if native { skip_native(f)? } else { 0 };
    for d in dungeons {
        credit(f, d)?;
    }
    let bad: Vec<String> = audit(before, f);
    if let Some(m) = bad.first() {
        return Err(Error::Msg(format!("dungeon credit: the edited flow fails its audit, {m} ({} problems)", bad.len())));
    }
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(Error::Format("dungeon credit: the written flow does not parse back to the edited model"));
    }
    Ok((out, n))
}

fn text<'a>(n: &'a Node, k: &str) -> &'a str {
    n.get(k).and_then(|v| v.as_str()).unwrap_or("")
}

pub fn quiet_pouch(y: &mut Byml) -> Res<bool> {
    let list: &mut Vec<Node> = y.root.as_arr_mut().ok_or(Error::Format("dungeon credit: the pouch table root is not a list"))?;
    let r: &mut Node = list
        .iter_mut()
        .find(|r| text(r, "__RowId") == KEY)
        .ok_or_else(|| Error::Msg(format!("the pouch table has no {KEY} row")))?;
    let have: String = text(r, DEAL).to_string();
    if have != QUIET_DEAL && have != NATIVE_DEAL {
        return Err(Error::Msg(format!("{KEY} has the pouch deal {have} and not {NATIVE_DEAL}, the game files differ")));
    }
    let unstocked: bool = r.remove(STOCK);
    if have == QUIET_DEAL {
        return Ok(unstocked);
    }
    r.set(DEAL, Node::Str(QUIET_DEAL.to_string()));
    Ok(true)
}

pub fn size_of(y: &Byml, hash: u64) -> Option<Size> {
    let list: &[Node] = y.root.get("Actors")?.as_arr()?;
    let a: &Node = list.iter().find(|a| hash_of(a) == Some(hash) && a.get("Dynamic").is_some_and(|d| d.get(DROP).is_some()))?;
    Some(Size::of_gyaml(text(a, "Gyaml")))
}

pub fn plain(y: &mut Byml, hash: u64) -> Option<Size> {
    let size: Size = size_of(y, hash)?;
    let list: &mut Vec<Node> = y.root.get_mut("Actors")?.as_arr_mut()?;
    for a in list.iter_mut().filter(|a| hash_of(a) == Some(hash)) {
        if let Some(d) = a.get_mut("Dynamic").filter(|d| d.get(DROP).is_some()) {
            d.set(DROP, Node::Str(KEY.to_string()));
        }
    }
    Some(size)
}

pub fn place(y: &mut Byml, hash: u64, d: &str) -> Option<Size> {
    let size: Size = size_of(y, hash)?;
    edit_chest(y, hash, KEY, &event_name(d, size))
}

pub fn stage(rf: &mut Romfs, dungeons: &BS<&'static str>, counted: &BS<&str>) -> Res<(Vec<Out>, usize)> {
    let ds: Vec<&str> = dungeons.iter().copied().collect();
    let mut outs: Vec<Out> = Vec::new();
    let mut skipped: usize = 0;
    for (path, native) in [(BIG_FLOW, true), (SMALL_FLOW, false)] {
        outs.push(rewrite(rf, path, |raw: &[u8]| {
            let (b, n): (Vec<u8>, usize) = edit_flow(raw, &ds, native)?;
            skipped += n;
            Ok(b)
        })?);
    }
    for table in [SETTING_TABLE, ANALYZED_TABLE] {
        let analyzed: bool = table == ANALYZED_TABLE;
        outs.push(rewrite(rf, table, |raw: &[u8]| {
            let mut y: Byml = Byml::parse(raw)?;
            for size in [Size::Small, Size::Big] {
                let rows: Vec<(String, Option<String>)> =
                    ds.iter().map(|d| (event_name(d, size), if analyzed { None } else { Some(entry_name(d)) })).collect();
                add_rows(&mut y, size.template(), &rows)?;
            }
            y.write()
        })?);
    }
    outs.push(rewrite(rf, POUCH_TABLE, |raw: &[u8]| {
        let mut y: Byml = Byml::parse(raw)?;
        quiet_pouch(&mut y)?;
        y.write()
    })?);
    for l in languages(rf)? {
        let english: bool = dungeon_text::is_english(&l);
        outs.push(rewrite(rf, &l, |raw: &[u8]| dungeon_text::apply(raw, &ds, counted, english))?);
    }
    Ok((outs, skipped))
}

pub fn build(romfs: &P, load: &P, name: &str, plans: &[(u64, Option<String>)], o: &Opts) -> Res<Built> {
    let mut rf: Romfs = Romfs::open(romfs)?;
    let mut want: BM<u64, Option<&'static str>> = BM::new();
    for (h, d) in plans {
        let x: Option<&'static str> = match d {
            Some(d) => Some(dungeon(d)?),
            None => None,
        };
        if want.insert(*h, x).is_some() {
            return Err(Error::Msg(format!("chest {h} is listed more than once")));
        }
    }
    let used: BS<&'static str> = want.values().flatten().copied().collect();
    let mut outs: Vec<Out> = Vec::new();
    let mut found: BS<u64> = BS::new();
    let mut notes: String = String::new();
    for f in banc_files(&rf)? {
        let raw: Vec<u8> = banc_raw(&mut rf, &f, o.banc_from.as_ref())?;
        let mut y: Byml = Byml::parse(&raw)?;
        let mut hit: bool = false;
        for (h, d) in &want {
            let got: Option<Size> = match d {
                Some(d) => place(&mut y, *h, d),
                None => plain(&mut y, *h),
            };
            let Some(size) = got else {
                continue;
            };
            match d {
                Some(d) => {
                    let _ = writeln!(notes, "chest {h} in {f}: {KEY}, event {} runs {} and credits DungeonInfo.{d}", event_name(d, size), entry_name(d));
                }
                None => {
                    let _ = writeln!(notes, "chest {h} in {f}: {KEY}, event name untouched, credits nothing");
                }
            }
            found.insert(*h);
            hit = true;
        }
        if hit {
            let bytes: Vec<u8> = y.write()?;
            outs.push(banc_out(&mut rf, &f, raw.len(), &bytes)?);
        }
    }
    for h in want.keys() {
        if !found.contains(h) {
            return Err(Error::Msg(format!("no chest with hash {h} and a {DROP} field")));
        }
    }
    let mut counted: BS<&str> = BS::new();
    for d in &o.count {
        let d: &'static str = dungeon(d)?;
        if !used.contains(d) {
            return Err(Error::Msg(format!("{d} is asked to show its key count but no chest credits it")));
        }
        counted.insert(d);
    }
    let (staged, skipped): (Vec<Out>, usize) = stage(&mut rf, &used, &counted)?;
    outs.extend(staged);
    let c: Committed = commit(&mut rf, load, name, &outs, o.rst_from.as_ref())?;
    let mut report: String = c.report;
    report.push_str(&notes);
    let _ = writeln!(report, "{skipped} native credit cases of the big chest flow now skip their action");
    let _ = writeln!(report, "{KEY} pouch deal is {QUIET_DEAL} and it is no longer {STOCK}, so the pick up window shows no count");
    for d in &used {
        let how: &str = if counted.contains(d) { "with the key count" } else { "without a count" };
        let _ = writeln!(report, "{} ends with the text {} {how}", entry_name(d), dungeon_text::message_id(d));
    }
    Ok(Built { report, files: c.files, grown: c.grown })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::Case;

    fn actor(name: &str, actions: &[&str], queries: &[&str]) -> Actor {
        Actor {
            name: name.to_string(),
            sub: String::new(),
            arg: String::new(),
            actions: actions.iter().map(|s| s.to_string()).collect(),
            queries: queries.iter().map(|s| s.to_string()).collect(),
            params: None,
            ep: 0xFFFF,
            cut: 1,
        }
    }

    fn act(name: &str, actor: u16, act: u16, next: u16) -> Event {
        Event { name: name.to_string(), kind: Kind::Action { next, actor, act, params: None } }
    }

    fn small() -> Flow {
        Flow {
            name: "Dm_SY_TreasureboxSmall".to_string(),
            actors: vec![actor("GameSystemActor", &["EventWait"], &[]), actor(HOLDER, &[SET], &["EventQueryIsBool"])],
            events: vec![act("Event2", 0, 0, NONE)],
            entries: vec![Entry { name: ROOT_ENTRY.to_string(), subs: Vec::new(), vars: Vec::new(), main: 0, pad: 0 }],
        }
    }

    fn big() -> Flow {
        let mut acts: Vec<&str> = vec!["EventWait"];
        acts.extend(NATIVE);
        let cases: Vec<Case> = vec![
            Case { val: 0, ev: 1 },
            Case { val: 1, ev: 2 },
            Case { val: 2, ev: 3 },
            Case { val: 3, ev: 4 },
            Case { val: 4, ev: 5 },
        ];
        Flow {
            name: "Dm_SY_Treasurebox".to_string(),
            actors: vec![actor("GameSystemActor", &acts, &[]), actor("TreasureBox_EventStarter", &[], &["EventQueryTreasureBoxIsDungeonCommonItemInside"]), actor(HOLDER, &[], &["EventQueryIsBool"])],
            events: vec![
                Event { name: "Event32".to_string(), kind: Kind::Switch { actor: 1, query: 0, cases, params: None } },
                act("Event23", 0, 1, 6),
                act("Event26", 0, 2, 6),
                act("Event27", 0, 3, 7),
                act("Event30", 0, 4, 8),
                act("Event25", 0, 0, NONE),
                act("Event21", 0, 0, NONE),
                act("Event82", 0, 0, 8),
                act("Event15", 0, 0, NONE),
            ],
            entries: vec![Entry { name: ROOT_ENTRY.to_string(), subs: Vec::new(), vars: Vec::new(), main: 0, pad: 0 }],
        }
    }

    fn bf(f: Flow) -> Bf {
        Bf { ver: [0, 4, 0, 0], bom: 0xFEFF, shift: 3, pad: 0, name: f.name.clone(), flows: vec![f], tls: Vec::new() }
    }

    fn param<'a>(e: &'a Event, k: &str) -> Option<&'a Val> {
        e.kind.params().and_then(|c| c.get(k))
    }

    #[test]
    fn an_entry_adds_to_both_counts_sets_the_once_flag_opens_the_chest_and_then_names_the_dungeon() {
        let mut f: Flow = small();
        credit(&mut f, "ForestTemple").unwrap();
        assert_eq!(f.actors[1].actions, vec![SET.to_string(), ADD.to_string()]);
        assert_eq!((f.actors[2].name.as_str(), f.actors[2].actions.clone()), (TALKER, vec![TALK.to_string()]));
        assert_eq!(f.actors[2].params, f.actors[0].params);
        assert_eq!(f.events.len(), 6);
        let e: &Entry = &f.entries[1];
        assert_eq!((e.name.as_str(), e.main, e.subs.clone()), ("Success_Key_ForestTemple", 1, vec![4]));
        for (i, name, next) in [(1usize, "DungeonInfo.ForestTemple.SmallKeyNum", 2u16), (2, "DungeonInfo.ForestTemple.SmallKeyTotalNum", 3)] {
            let Kind::Action { next: n, actor, act, .. } = &f.events[i].kind else {
                panic!("expected an action");
            };
            assert_eq!((*n, *actor, *act), (next, 1, 1));
            assert_eq!(param(&f.events[i], "GameDataName"), Some(&Val::Str(name.to_string())));
            assert_eq!(param(&f.events[i], "Value"), Some(&Val::Int(1)));
            assert_eq!(param(&f.events[i], "Index"), Some(&Val::Int(-1)));
        }
        let Kind::Action { next, actor, act, .. } = &f.events[3].kind else {
            panic!("expected an action");
        };
        assert_eq!((*next, *actor, *act), (4, 1, 0));
        assert_eq!(param(&f.events[3], "GameDataName"), Some(&Val::Str("DungeonInfo.ForestTemple.IsGetSmallKeyOnce".to_string())));
        assert_eq!(param(&f.events[3], "Value"), Some(&Val::Bool(1)));
        let Kind::Sub { next, flow, entry, .. } = &f.events[4].kind else {
            panic!("expected a sub");
        };
        assert_eq!((*next, flow.as_str(), entry.as_str()), (5, "", ROOT_ENTRY));
        assert_eq!(param(&f.events[4], "ContentName"), Some(&Val::Str(KEY.to_string())));
        assert_eq!(param(&f.events[4], "IsMiniGame"), Some(&Val::Bool(0)));
        let Kind::Action { next, actor, act, params } = &f.events[5].kind else {
            panic!("expected an action");
        };
        assert_eq!((*next, *actor, *act), (NONE, 2, 0));
        assert_eq!(params.as_ref().map(|c| c.items.len()), Some(22));
        assert_eq!(param(&f.events[5], "MessageId"), Some(&Val::Str("EventFlowMsg/Dm_SY_Get:OOTRR_SmallKey_ForestTemple".to_string())));
        assert_eq!(param(&f.events[5], "IsWaitEnd"), Some(&Val::Bool(1)));
        assert_eq!(param(&f.events[5], "IsCloseDialog"), Some(&Val::Bool(1)));
        assert_eq!(param(&f.events[5], "OverrideStyle"), Some(&Val::Int(4)));
        assert!(sound(&f).is_empty());
    }

    #[test]
    fn crediting_twice_changes_nothing_and_two_dungeons_share_the_actions() {
        let mut f: Flow = small();
        credit(&mut f, "ForestTemple").unwrap();
        let before: Flow = f.clone();
        credit(&mut f, "ForestTemple").unwrap();
        assert_eq!(f, before);
        credit(&mut f, "WaterTemple").unwrap();
        assert_eq!(f.actors[1].actions.len(), 2);
        assert_eq!(f.actors[2].actions.len(), 1);
        assert_eq!(f.actors.len(), 3);
        assert_eq!(f.entries.len(), 3);
        assert_eq!((f.entries[2].main, f.entries[2].subs.clone()), (6, vec![9]));
        assert_eq!(param(&f.events[10], "MessageId"), Some(&Val::Str("EventFlowMsg/Dm_SY_Get:OOTRR_SmallKey_WaterTemple".to_string())));
        assert!(audit(&small(), &f).is_empty());
    }

    #[test]
    fn a_flow_without_the_holder_gets_one() {
        let mut f: Flow = small();
        f.actors.pop();
        let before: Flow = f.clone();
        credit(&mut f, "FireTemple").unwrap();
        assert_eq!(f.actors[1].name, HOLDER);
        assert_eq!(f.actors[1].actions, vec![ADD.to_string(), SET.to_string()]);
        assert_eq!(f.actors[2].name, TALKER);
        assert!(sound(&f).is_empty());
        assert!(audit(&before, &f).is_empty());
    }

    #[test]
    fn the_four_native_cases_jump_past_their_action_and_the_fifth_stays() {
        let mut f: Flow = big();
        assert_eq!(skip_native(&mut f).unwrap(), 4);
        let Kind::Switch { cases, .. } = &f.events[0].kind else {
            panic!("expected a switch");
        };
        let to: Vec<u16> = cases.iter().map(|c| c.ev).collect();
        assert_eq!(to, vec![6, 6, 7, 8, 5]);
        assert_eq!(skip_native(&mut f).unwrap(), 0);
        assert!(audit(&big(), &f).is_empty());
    }

    #[test]
    fn a_native_action_reached_some_other_way_is_refused() {
        let mut f: Flow = big();
        f.events[5] = act("Event25", 0, 0, 4);
        assert!(skip_native(&mut f).is_err());
        let mut g: Flow = big();
        g.entries[0].main = 3;
        assert!(skip_native(&mut g).is_err());
    }

    #[test]
    fn the_audit_names_a_change_that_is_not_part_of_the_plan() {
        let mut f: Flow = big();
        skip_native(&mut f).unwrap();
        credit(&mut f, "ForestTemple").unwrap();
        assert!(audit(&big(), &f).is_empty());
        let mut g: Flow = f.clone();
        g.events[7] = act("Event82", 0, 0, NONE);
        assert_eq!(audit(&big(), &g).len(), 1);
        let mut h: Flow = f.clone();
        if let Kind::Switch { cases, .. } = &mut h.events[0].kind {
            cases[4].ev = 6;
        }
        assert_eq!(audit(&big(), &h).len(), 1);
        let mut k: Flow = f.clone();
        k.entries[1].main = 99;
        assert!(!audit(&big(), &k).is_empty());
        let mut m: Flow = f.clone();
        m.entries[1].name = "Other".to_string();
        assert_eq!(audit(&big(), &m).len(), 1);
        let mut n: Flow = f.clone();
        n.actors[3].actions.push("EventTalkWithInstanceNameLabel".to_string());
        n.events[13] = act("Key13", 3, 1, NONE);
        assert_eq!(audit(&big(), &n).len(), 1);
        let mut p: Flow = f.clone();
        p.actors[3].name = "Npc_Navi".to_string();
        assert!(!audit(&big(), &p).is_empty());
    }

    #[test]
    fn the_edited_file_parses_back_and_is_stable() {
        let raw: Vec<u8> = bf(big()).write().unwrap();
        let (out, n): (Vec<u8>, usize) = edit_flow(&raw, &["ForestTemple", "SpiritTemple"], true).unwrap();
        assert_eq!(n, 4);
        let b: Bf = Bf::parse(&out).unwrap();
        assert_eq!(b.flows[0].entries.len(), 3);
        assert_eq!(b.flows[0].events.len(), 19);
        assert_eq!(b.flows[0].actors[2].actions, vec![ADD.to_string(), SET.to_string()]);
        assert_eq!((b.flows[0].actors[3].name.as_str(), b.flows[0].actors[3].actions.clone()), (TALKER, vec![TALK.to_string()]));
        let (again, m): (Vec<u8>, usize) = edit_flow(&out, &["ForestTemple", "SpiritTemple"], true).unwrap();
        assert_eq!((again, m), (out, 0));
        assert!(edit_flow(&raw, &["Market"], true).is_err());
    }

    fn row(id: &str, deal: &str) -> Node {
        Node::Map(vec![(DEAL.to_string(), Node::Str(deal.to_string())), ("PouchStockable".to_string(), Node::Bool(true)), ("__RowId".to_string(), Node::Str(id.to_string()))])
    }

    #[test]
    fn only_the_chest_key_row_loses_the_pouch_deal_and_the_stock_count() {
        let mut y: Byml = Byml { root: Node::Arr(vec![row("Item_Dungeon_Map", QUIET_DEAL), row(KEY, NATIVE_DEAL), row("Obj_Dungeon_SmallKey", NATIVE_DEAL)]), version: 2, big: false };
        assert!(quiet_pouch(&mut y).unwrap());
        let rows: &[Node] = y.root.as_arr().unwrap();
        assert_eq!(text(&rows[1], DEAL), QUIET_DEAL);
        assert_eq!(rows[1].get(STOCK), None);
        assert_eq!(rows[0].get(STOCK), Some(&Node::Bool(true)));
        assert_eq!((text(&rows[2], DEAL), rows[2].get(STOCK)), (NATIVE_DEAL, Some(&Node::Bool(true))));
        assert!(!quiet_pouch(&mut y).unwrap());
        let mut half: Byml = Byml { root: Node::Arr(vec![row(KEY, QUIET_DEAL)]), version: 2, big: false };
        assert!(quiet_pouch(&mut half).unwrap());
        assert_eq!(half.root.as_arr().unwrap()[0].get(STOCK), None);
        let mut odd: Byml = Byml { root: Node::Arr(vec![row(KEY, "HeartContainer")]), version: 2, big: false };
        assert!(quiet_pouch(&mut odd).is_err());
        let mut none: Byml = Byml { root: Node::Arr(vec![row("Obj_Dungeon_SmallKey", NATIVE_DEAL)]), version: 2, big: false };
        assert!(quiet_pouch(&mut none).is_err());
    }

    fn chest(hash: u64, gyaml: &str) -> Node {
        Node::Map(vec![
            (
                "Dynamic".to_string(),
                Node::Map(vec![(DROP.to_string(), Node::Str("TBox_Rupee_Blue".to_string())), ("GetEventName".to_string(), Node::Str("KokiriForest_TreasureBox_NaviChatOff".to_string()))]),
            ),
            ("Gyaml".to_string(), Node::Str(gyaml.to_string())),
            ("Hash".to_string(), Node::U64(hash)),
        ])
    }

    fn dynamic<'a>(y: &'a Byml, i: usize, k: &str) -> &'a str {
        text(y.root.get("Actors").unwrap().as_arr().unwrap()[i].get("Dynamic").unwrap(), k)
    }

    #[test]
    fn a_credited_chest_gets_the_key_and_the_event_and_a_plain_chest_only_the_key() {
        let mut y: Byml = Byml {
            root: Node::Map(vec![("Actors".to_string(), Node::Arr(vec![chest(1, "Obj_TBox_A_01"), chest(2, "Obj_TBox_A_02"), chest(3, "Obj_TBox_A_01")]))]),
            version: 2,
            big: false,
        };
        assert_eq!(place(&mut y, 1, "ForestTemple"), Some(Size::Small));
        assert_eq!(place(&mut y, 2, "ForestTemple"), Some(Size::Big));
        assert_eq!(plain(&mut y, 3), Some(Size::Small));
        assert_eq!(place(&mut y, 4, "ForestTemple"), None);
        assert_eq!(plain(&mut y, 4), None);
        assert_eq!((dynamic(&y, 0, DROP), dynamic(&y, 0, "GetEventName")), (KEY, "OOTRR_Key_ForestTemple_Small"));
        assert_eq!((dynamic(&y, 1, DROP), dynamic(&y, 1, "GetEventName")), (KEY, "OOTRR_Key_ForestTemple_Big"));
        assert_eq!((dynamic(&y, 2, DROP), dynamic(&y, 2, "GetEventName")), (KEY, "KokiriForest_TreasureBox_NaviChatOff"));
    }

    #[test]
    fn the_flag_names_hash_to_the_values_the_game_stores() {
        assert_eq!(crate::chest_slot::key(&flag("ForestTemple", "SmallKeyNum")), 0x90237092);
        assert_eq!(crate::chest_slot::key(&flag("BottomOfTheWell", "IsGetSmallKeyOnce")), 0x9e76cb62);
        assert_eq!(crate::chest_slot::key(&flag("BottomOfTheWell", "SmallKeyTotalNum")), 0x085df1c9);
    }

    #[test]
    fn a_small_key_only_credit_skips_just_the_small_key_case_of_the_big_flow() {
        let mut f: Flow = big();
        let n: usize = credit_keys(&mut f, &["ForestTemple", "FireTemple"], true).unwrap();
        assert_eq!(n, 1);
        assert!(f.entries.iter().any(|e| e.name == "Success_Key_FireTemple"));
        let Kind::Switch { cases, .. } = &f.events[0].kind else {
            panic!("expected a switch");
        };
        let to: Vec<u16> = cases.iter().map(|c| c.ev).collect();
        assert_eq!(to[0], 1);
        assert_eq!(to[1], 2);
        assert_eq!(to[2], 3);
        assert_ne!(to[3], 4);
        let mut s: Flow = small();
        assert_eq!(credit_keys(&mut s, &["ForestTemple"], false).unwrap(), 0);
    }

    #[test]
    fn every_key_chest_is_wrapped_for_the_dungeon_of_its_scene_and_a_foreign_scene_is_refused() {
        let mut y: Byml = Byml {
            root: Node::Map(vec![("Actors".to_string(), Node::Arr(vec![chest(1, "Obj_TBox_A_01"), chest(2, "Obj_TBox_A_02")]))]),
            version: 2,
            big: false,
        };
        place(&mut y, 1, "FireTemple");
        for i in 0..2 {
            let Node::Map(m) = &mut y.root.get_mut("Actors").unwrap().as_arr_mut().unwrap()[i] else {
                panic!("expected a map");
            };
            for (k, v) in m.iter_mut() {
                if k == "Dynamic" {
                    v.set(DROP, Node::Str(KEY.to_string()));
                }
            }
        }
        assert_eq!(sweep(&mut y, "Banc/ForestTemple/ForestTemple_Child_Dynamic.bcett.byml.zs").unwrap(), 1);
        assert_eq!(dynamic(&y, 0, "GetEventName"), "OOTRR_Key_FireTemple_Small");
        assert_eq!(dynamic(&y, 1, "GetEventName"), "OOTRR_Key_ForestTemple_Big");
        assert_eq!(sweep(&mut y, "Banc/ForestTemple/ForestTemple_Child_Dynamic.bcett.byml.zs").unwrap(), 0);
        let mut z: Byml = Byml {
            root: Node::Map(vec![("Actors".to_string(), Node::Arr(vec![chest(9, "Obj_TBox_A_01")]))]),
            version: 2,
            big: false,
        };
        if let Node::Map(m) = &mut z.root.get_mut("Actors").unwrap().as_arr_mut().unwrap()[0] {
            for (k, v) in m.iter_mut() {
                if k == "Dynamic" {
                    v.set(DROP, Node::Str(KEY.to_string()));
                }
            }
        }
        assert!(sweep(&mut z, "Banc/KokiriForest/KokiriForest_Child_Dynamic.bcett.byml.zs").is_err());
        assert_eq!(sweep(&mut y, "Banc/KokiriForest/x.bcett.byml.zs").unwrap(), 0);
    }

    #[test]
    fn item_dungeon_names_map_to_the_key_counts_and_scenes() {
        assert_eq!(key_dungeon("Ganons Castle"), Some("InsideGanonCastle"));
        assert_eq!(key_dungeon("Thieves Hideout"), None);
        assert_eq!(scene_dungeon("Banc/InsideGanonCastle/x.bcett.byml.zs"), Some("InsideGanonCastle"));
        assert_eq!(scene_dungeon("Banc/GanonCastle/x.bcett.byml.zs"), None);
        assert!(KEY_DUNGEONS.iter().all(|d| DUNGEONS.contains(d)));
    }

    #[test]
    fn an_unknown_dungeon_is_refused() {
        assert!(dungeon("KokiriForest").is_err());
        assert!(dungeon(PLAIN).is_err());
        assert_eq!(dungeon("GerudoFortress").unwrap(), "GerudoFortress");
        assert_eq!(entry_name("ForestTemple"), "Success_Key_ForestTemple");
        assert_eq!(event_name("ForestTemple", Size::Big), "OOTRR_Key_ForestTemple_Big");
    }
}
