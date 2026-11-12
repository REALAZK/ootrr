use std::collections::{BTreeMap as BM, BTreeSet as BS};

use crate::bfevfl::flow::NONE;
use crate::bfevfl::{Bf, Cont, Entry, Flow, Kind as Ev, Val};
use crate::byml::{Byml, Node};
use crate::dungeon_credit::{action_slot, dungeon, flag, idx, key_dungeon, push, set_bool, skip_native_of, sound, talk, DROP, DUNGEONS, HOLDER, SET, TALK, TALKER};
use crate::dungeon_text::{compose, is_english, name, place_label, FILE, FILE_ID, NAMES, PLACES};
use crate::error::{Error as E, Res};
use crate::flow_locations::dict_id;
use crate::hints::languages;
use crate::msbt::{self, Msbt, Tok};
use crate::placement::hash_of;
use crate::progressive::{add_rows, Size, ANALYZED_TABLE, BIG_FLOW, SETTING_TABLE, SMALL_FLOW};
use crate::romfs::Romfs;
use crate::sarc;
use crate::staging::{rewrite, Out};
use crate::zs;

pub const ROOT_ENTRY: &str = "Success_";
pub const OURS: &str = "OOTRR_";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Boss,
    Map,
    Compass,
}

pub const KINDS: [Kind; 3] = [Kind::Boss, Kind::Map, Kind::Compass];

impl Kind {
    pub fn of(kind: &str) -> Option<Kind> {
        match kind {
            "BossKey" | "GanonBossKey" => Some(Kind::Boss),
            "Map" => Some(Kind::Map),
            "Compass" => Some(Kind::Compass),
            _ => None,
        }
    }

    pub fn of_actor(actor: &str) -> Option<Kind> {
        KINDS.iter().copied().find(|k| k.actor() == actor)
    }

    pub fn actor(self) -> &'static str {
        match self {
            Kind::Boss => "Item_Dungeon_BossKey",
            Kind::Map => "Item_Dungeon_Map",
            Kind::Compass => "Item_Dungeon_Compass",
        }
    }

    pub fn member(self) -> &'static str {
        match self {
            Kind::Boss => "IsGetBossKey",
            Kind::Map => "IsGetMap",
            Kind::Compass => "IsGetCompass",
        }
    }

    pub fn native(self) -> &'static str {
        match self {
            Kind::Boss => "EventGetDungeonBossKey",
            Kind::Map => "EventGetDungeonMap",
            Kind::Compass => "EventGetDungeonCompass",
        }
    }

    pub fn name_label(self) -> &'static str {
        match self {
            Kind::Boss => "Item_Dungeon_BossKey_Name",
            Kind::Map => "Item_Dungeon_Map_Name",
            Kind::Compass => "Item_Dungeon_Compass_Name",
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Kind::Boss => "Boss",
            Kind::Map => "Map",
            Kind::Compass => "Compass",
        }
    }

    fn text_tag(self) -> &'static str {
        match self {
            Kind::Boss => "BossKey",
            Kind::Map => "Map",
            Kind::Compass => "Compass",
        }
    }
}

pub fn struct_of(dungeon: &str) -> Option<&'static str> {
    match dungeon {
        "Deku Tree" => Some("InsideTheDekuTree"),
        "Dodongos Cavern" => Some("DodongoCavern"),
        "Jabu Jabus Belly" => Some("InsideJabuJabuBelly"),
        "Ice Cavern" => Some("IceCavern"),
        _ => key_dungeon(dungeon),
    }
}

pub fn entry_name(k: Kind, d: &str) -> String {
    format!("Success_{}_{d}", k.tag())
}

pub fn event_name(k: Kind, d: &str, size: Size) -> String {
    let s: &str = match size {
        Size::Small => "Small",
        Size::Big => "Big",
    };
    format!("{OURS}{}_{d}_{s}", k.tag())
}

pub fn label(k: Kind, d: &str) -> String {
    format!("{OURS}{}_{d}", k.text_tag())
}

pub fn message_id(k: Kind, d: &str) -> String {
    format!("{FILE_ID}:{}", label(k, d))
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Grants {
    pub skip: Vec<Kind>,
    pub sweep: Vec<Kind>,
    pub credits: BM<u64, (Kind, &'static str)>,
}

impl Grants {
    pub fn active(&self) -> bool {
        !self.skip.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Marks {
    pub used: BS<(Kind, &'static str)>,
    pub hit: BS<u64>,
}

fn text<'a>(n: &'a Node, k: &str) -> &'a str {
    n.get(k).and_then(|v| v.as_str()).unwrap_or("")
}

pub fn scene_of(file: &str) -> Option<&'static str> {
    let s: &str = file.split('/').nth(1)?;
    DUNGEONS.iter().copied().find(|d| *d == s)
}

pub fn mark(y: &mut Byml, file: &str, g: &Grants, m: &mut Marks) -> Res<usize> {
    if g.sweep.is_empty() && g.credits.is_empty() {
        return Ok(0);
    }
    let scene: Option<&'static str> = scene_of(file);
    let Some(list) = y.root.get_mut("Actors").and_then(|a| a.as_arr_mut()) else {
        return Ok(0);
    };
    let mut n: usize = 0;
    for a in list.iter_mut() {
        let size: Size = Size::of_gyaml(text(a, "Gyaml"));
        let h: Option<u64> = hash_of(a);
        let Some(dy) = a.get_mut("Dynamic") else {
            continue;
        };
        if dy.get(DROP).is_none() {
            continue;
        }
        let drop: String = text(dy, DROP).to_string();
        let got: Option<(Kind, &'static str)> = h.and_then(|h| g.credits.get(&h)).copied();
        let (k, d): (Kind, &'static str) = match got {
            Some((k, d)) => {
                if drop != k.actor() {
                    return Err(E::Msg(format!("chest {} in {file} holds {drop} where {} was expected", h.unwrap_or(0), k.actor())));
                }
                if let Some(x) = h {
                    m.hit.insert(x);
                }
                (k, d)
            }
            None => {
                let Some(k) = Kind::of_actor(&drop).filter(|k| g.sweep.contains(k)) else {
                    continue;
                };
                if text(dy, "GetEventName").starts_with(OURS) {
                    continue;
                }
                let d: &'static str = scene.ok_or_else(|| E::Msg(format!("{file} has a chest with {drop} but is not a dungeon scene this build knows")))?;
                (k, d)
            }
        };
        m.used.insert((k, d));
        let ev: String = event_name(k, d, size);
        if text(dy, "GetEventName") == ev {
            continue;
        }
        dy.set("GetEventName", Node::Str(ev));
        n += 1;
    }
    Ok(n)
}

pub fn all_marked(g: &Grants, m: &Marks) -> Res<()> {
    match g.credits.keys().find(|h| !m.hit.contains(h)) {
        Some(h) => Err(E::Msg(format!("chest {h} that holds a dungeon item was not found in the scenes, the game files differ"))),
        None => Ok(()),
    }
}

fn give(next: u16, actor: &str) -> Ev {
    let c: Cont = Cont {
        items: vec![
            ("ContentName".to_string(), Val::Str(actor.to_string())),
            ("IsMiniGame".to_string(), Val::Bool(0)),
        ],
    };
    Ev::Sub { next, params: Some(c), flow: String::new(), entry: ROOT_ENTRY.to_string() }
}

pub fn credit(f: &mut Flow, k: Kind, d: &str) -> Res<()> {
    let d: &str = dungeon(d)?;
    let name: String = entry_name(k, d);
    if f.entries.iter().any(|e| e.name == name) {
        return Ok(());
    }
    let pad: u16 = f
        .entries
        .iter()
        .find(|e| e.name == ROOT_ENTRY)
        .map(|e| e.pad)
        .ok_or(E::Format("dungeon grant: the chest flow has no Success_ entry"))?;
    let (a, set): (u16, u16) = action_slot(f, HOLDER, SET)?;
    let (t, say): (u16, u16) = action_slot(f, TALKER, TALK)?;
    let at: u16 = idx(f.events.len())?;
    push(f, Ev::Action { next: at + 1, actor: a, act: set, params: Some(set_bool(flag(d, k.member()))) })?;
    push(f, give(at + 2, k.actor()))?;
    push(f, Ev::Action { next: NONE, actor: t, act: say, params: Some(talk(message_id(k, d))) })?;
    f.entries.push(Entry { name, subs: vec![at + 1], vars: Vec::new(), main: at, pad });
    Ok(())
}

pub fn edit_flow(f: &mut Flow, skip: &[Kind], used: &BS<(Kind, &'static str)>) -> Res<usize> {
    let natives: Vec<&str> = skip.iter().map(|k| k.native()).collect();
    let n: usize = if natives.is_empty() { 0 } else { skip_native_of(f, &natives)? };
    for (k, d) in used {
        credit(f, *k, d)?;
    }
    if let Some(m) = sound(f).first() {
        return Err(E::Msg(format!("dungeon grant: the chest flow is broken after the new entries, {m}")));
    }
    Ok(n)
}

pub fn merge(rf: &mut Romfs, outs: &mut Vec<Out>, path: &str, f: impl FnOnce(&[u8]) -> Res<Vec<u8>>) -> Res<()> {
    match outs.iter().position(|o| o.path == path) {
        Some(i) => {
            let id: Option<u32> = dict_id(&outs[i].z);
            let raw: Vec<u8> = rf.zs.decompress(&outs[i].z)?;
            let b: Vec<u8> = f(&raw)?;
            outs[i].new = b.len();
            outs[i].z = zs::compress_raw(&b, id)?;
        }
        None => outs.push(rewrite(rf, path, f)?),
    }
    Ok(())
}

fn flow_file(raw: &[u8], skip: &[Kind], used: &BS<(Kind, &'static str)>) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    let f: &mut Flow = b.flows.first_mut().ok_or(E::Format("dungeon grant: the flow file has no flow"))?;
    edit_flow(f, skip, used)?;
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(E::Format("dungeon grant: the written flow does not parse back to the edited model"));
    }
    Ok(out)
}

pub fn stage(rf: &mut Romfs, outs: &mut Vec<Out>, g: &Grants, used: &BS<(Kind, &'static str)>) -> Res<()> {
    if !g.active() {
        return Ok(());
    }
    merge(rf, outs, BIG_FLOW, |raw: &[u8]| flow_file(raw, &g.skip, used))?;
    if used.is_empty() {
        return Ok(());
    }
    merge(rf, outs, SMALL_FLOW, |raw: &[u8]| flow_file(raw, &[], used))?;
    for table in [SETTING_TABLE, ANALYZED_TABLE] {
        let analyzed: bool = table == ANALYZED_TABLE;
        merge(rf, outs, table, |raw: &[u8]| {
            let mut y: Byml = Byml::parse(raw)?;
            for size in [Size::Small, Size::Big] {
                let rows: Vec<(String, Option<String>)> =
                    used.iter().map(|(k, d)| (event_name(*k, d, size), if analyzed { None } else { Some(entry_name(*k, d)) })).collect();
                add_rows(&mut y, size.template(), &rows)?;
            }
            y.write()
        })?;
    }
    Ok(())
}

fn book(es: &[sarc::Entry], n: &str) -> Res<Msbt> {
    let e: &sarc::Entry = sarc::get(es, n).ok_or_else(|| E::Msg(format!("the language archive has no {n}")))?;
    msbt::parse(&e.data)
}

pub fn text_file(raw: &[u8], used: &BS<(Kind, &'static str)>, english: bool) -> Res<Vec<u8>> {
    let mut es: Vec<sarc::Entry> = sarc::parse(raw)?;
    let names: Msbt = book(&es, NAMES)?;
    let places: Msbt = book(&es, PLACES)?;
    let mut m: Msbt = book(&es, FILE)?;
    for (k, d) in used {
        let item: Vec<Tok> = name(&names, k.name_label())?;
        let place: Vec<Tok> = name(&places, place_label(d))?;
        m.put_tokens(&label(*k, d), &compose(d, &item, &place, english, false)?)?;
    }
    sarc::set(&mut es, FILE, m.write()?);
    sarc::write(&es)
}

pub fn stage_text(rf: &mut Romfs, more: &mut Vec<Out>, used: &BS<(Kind, &'static str)>) -> Res<()> {
    if used.is_empty() {
        return Ok(());
    }
    for l in languages(rf)? {
        let english: bool = is_english(&l);
        merge(rf, more, &l, |raw: &[u8]| text_file(raw, used, english))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::{Actor, Case, Event};

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
        Event { name: name.to_string(), kind: Ev::Action { next, actor, act, params: None } }
    }

    fn big() -> Flow {
        let acts: Vec<&str> = vec!["EventWait", "EventGetDungeonMap", "EventGetDungeonCompass", "EventGetDungeonBossKey", "EventGetDungeonSmallKey"];
        let cases: Vec<Case> = (0..5).map(|i| Case { val: i, ev: i as u16 + 1 }).collect();
        Flow {
            name: "Dm_SY_Treasurebox".to_string(),
            actors: vec![actor("GameSystemActor", &acts, &[]), actor("TreasureBox_EventStarter", &[], &["EventQueryTreasureBoxIsDungeonCommonItemInside"])],
            events: vec![
                Event { name: "Event32".to_string(), kind: Ev::Switch { actor: 1, query: 0, cases, params: None } },
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

    fn cases(f: &Flow) -> Vec<u16> {
        let Ev::Switch { cases, .. } = &f.events[0].kind else {
            panic!("expected a switch");
        };
        cases.iter().map(|c| c.ev).collect()
    }

    fn param<'a>(e: &'a Event, k: &str) -> Option<&'a Val> {
        e.kind.params().and_then(|c| c.get(k))
    }

    #[test]
    fn names_and_kinds_line_up() {
        assert_eq!(Kind::of("GanonBossKey"), Some(Kind::Boss));
        assert_eq!(Kind::of("SmallKey"), None);
        assert_eq!(Kind::of_actor("Item_Dungeon_Compass"), Some(Kind::Compass));
        assert_eq!(Kind::of_actor("Item_Dungeon_SmallKey"), None);
        assert_eq!(struct_of("Ganons Castle"), Some("InsideGanonCastle"));
        assert_eq!(struct_of("Deku Tree"), Some("InsideTheDekuTree"));
        assert_eq!(struct_of("Thieves Hideout"), None);
        assert_eq!(event_name(Kind::Boss, "FireTemple", Size::Big), "OOTRR_Boss_FireTemple_Big");
        assert_eq!(entry_name(Kind::Map, "IceCavern"), "Success_Map_IceCavern");
        assert_eq!(message_id(Kind::Boss, "ForestTemple"), "EventFlowMsg/Dm_SY_Get:OOTRR_BossKey_ForestTemple");
        assert!(!event_name(Kind::Boss, "FireTemple", Size::Small).starts_with(crate::dungeon_credit::EVENT_PREFIX));
        for k in KINDS {
            assert!(crate::dungeon_credit::NATIVE.contains(&k.native()));
        }
    }

    #[test]
    fn a_boss_key_entry_sets_the_flag_gives_the_key_and_names_the_dungeon() {
        let mut f: Flow = big();
        let used: BS<(Kind, &'static str)> = BS::from([(Kind::Boss, "WaterTemple")]);
        assert_eq!(edit_flow(&mut f, &[Kind::Boss], &used).unwrap(), 1);
        assert_eq!(cases(&f), vec![1, 2, 7, 4, 5]);
        let e: &Entry = f.entries.iter().find(|e| e.name == "Success_Boss_WaterTemple").unwrap();
        let at: usize = e.main as usize;
        assert_eq!(e.subs, vec![at as u16 + 1]);
        assert_eq!(param(&f.events[at], "GameDataName"), Some(&Val::Str("DungeonInfo.WaterTemple.IsGetBossKey".to_string())));
        assert_eq!(param(&f.events[at], "Value"), Some(&Val::Bool(1)));
        assert_eq!(param(&f.events[at + 1], "ContentName"), Some(&Val::Str("Item_Dungeon_BossKey".to_string())));
        assert_eq!(param(&f.events[at + 2], "MessageId"), Some(&Val::Str("EventFlowMsg/Dm_SY_Get:OOTRR_BossKey_WaterTemple".to_string())));
        assert!(sound(&f).is_empty());
        let before: Flow = f.clone();
        assert_eq!(edit_flow(&mut f, &[Kind::Boss], &used).unwrap(), 0);
        assert_eq!(f, before);
    }

    #[test]
    fn maps_and_compasses_skip_only_their_own_cases() {
        let mut f: Flow = big();
        let used: BS<(Kind, &'static str)> = BS::from([(Kind::Map, "IceCavern"), (Kind::Compass, "IceCavern")]);
        edit_flow(&mut f, &[Kind::Map, Kind::Compass], &used).unwrap();
        assert_eq!(cases(&f), vec![6, 6, 3, 4, 5]);
        assert!(f.entries.iter().any(|e| e.name == "Success_Map_IceCavern"));
        assert!(f.entries.iter().any(|e| e.name == "Success_Compass_IceCavern"));
        assert!(edit_flow(&mut big(), &[], &BS::from([(Kind::Map, "KokiriForest")])).is_err());
    }

    fn chest(hash: u64, gyaml: &str, drop: &str, ev: Option<&str>) -> Node {
        let mut dy: Vec<(String, Node)> = vec![(DROP.to_string(), Node::Str(drop.to_string()))];
        if let Some(e) = ev {
            dy.push(("GetEventName".to_string(), Node::Str(e.to_string())));
        }
        Node::Map(vec![("Dynamic".to_string(), Node::Map(dy)), ("Gyaml".to_string(), Node::Str(gyaml.to_string())), ("Hash".to_string(), Node::U64(hash))])
    }

    fn scene(list: Vec<Node>) -> Byml {
        Byml { root: Node::Map(vec![("Actors".to_string(), Node::Arr(list))]), version: 2, big: false }
    }

    fn event(y: &Byml, i: usize) -> String {
        text(y.root.get("Actors").unwrap().as_arr().unwrap()[i].get("Dynamic").unwrap(), "GetEventName").to_string()
    }

    #[test]
    fn credited_chests_name_their_dungeon_and_home_chests_their_scene() {
        let mut y: Byml = scene(vec![
            chest(1, "Obj_TBox_A_01", "Item_Dungeon_BossKey", None),
            chest(2, "Obj_TBox_A_02", "Item_Dungeon_BossKey", Some("ForesetTemple_GetBossKey")),
            chest(3, "Obj_TBox_A_02", "Item_Dungeon_Map", Some("FireTemple_GetMap")),
            chest(4, "Obj_TBox_A_01", "TBox_Rupee_Blue", None),
        ]);
        let g: Grants = Grants { skip: vec![Kind::Boss], sweep: vec![Kind::Boss], credits: BM::from([(1, (Kind::Boss, "ShadowTemple"))]) };
        let mut m: Marks = Marks::default();
        assert_eq!(mark(&mut y, "Banc/FireTemple/FireTemple_Static.bcett.byml.zs", &g, &mut m).unwrap(), 2);
        assert_eq!(event(&y, 0), "OOTRR_Boss_ShadowTemple_Small");
        assert_eq!(event(&y, 1), "OOTRR_Boss_FireTemple_Big");
        assert_eq!(event(&y, 2), "FireTemple_GetMap");
        assert_eq!(event(&y, 3), "");
        assert_eq!(m.used, BS::from([(Kind::Boss, "FireTemple"), (Kind::Boss, "ShadowTemple")]));
        assert!(all_marked(&g, &m).is_ok());
        assert_eq!(mark(&mut y, "Banc/FireTemple/FireTemple_Static.bcett.byml.zs", &g, &mut m).unwrap(), 0);
        assert!(all_marked(&Grants { credits: BM::from([(9, (Kind::Map, "IceCavern"))]), ..g.clone() }, &m).is_err());
        let mut z: Byml = scene(vec![chest(5, "Obj_TBox_A_01", "Item_Dungeon_BossKey", None)]);
        assert!(mark(&mut z, "Banc/KokiriForest/KokiriForest_Static.bcett.byml.zs", &g, &mut Marks::default()).is_err());
        let mut w: Byml = scene(vec![chest(1, "Obj_TBox_A_01", "TBox_Rupee_Blue", None)]);
        assert!(mark(&mut w, "Banc/KokiriForest/KokiriForest_Static.bcett.byml.zs", &g, &mut Marks::default()).is_err());
    }
}
