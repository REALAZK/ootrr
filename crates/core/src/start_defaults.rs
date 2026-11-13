use std::path::Path as P;

use crate::byml::{Byml, Node};
use crate::chest_slot::LIST;
use crate::error::{Error, Res};
use crate::flow_locations::dict_id;
use crate::murmur::murmur3_32;
use crate::romfs::Romfs;
use crate::staging::{commit, rewrite, Committed, Out};
use crate::zs;

pub mod opening;
pub mod presets;

pub const NEW_GAME: i32 = 8 | 32;
pub const KEPT: i32 = 8 | 16 | 32;
pub const SAVED: i32 = 0;

#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    Bool(String, bool),
    Enum(String, String),
    Int(String, i32),
    Grant(String, i32),
}

pub fn key(name: &str) -> u32 {
    murmur3_32(name.as_bytes(), 0)
}

pub fn got(actor: &str) -> String {
    format!("IsGet.{actor}")
}

pub fn stock(actor: &str) -> String {
    format!("PouchData.{actor}.StockNum")
}

fn num(n: &Node) -> Option<i64> {
    match n {
        Node::I32(x) => Some(*x as i64),
        Node::U32(x) => Some(*x as i64),
        Node::I64(x) => Some(*x),
        Node::U64(x) => i64::try_from(*x).ok(),
        _ => None,
    }
}

fn hash_of(n: &Node) -> Option<u32> {
    n.get("Hash").and_then(num).map(|h| h as u32)
}

fn tables(y: &Byml) -> &[(String, Node)] {
    match y.root.get("Data") {
        Some(Node::Map(m)) => m,
        _ => &[],
    }
}

fn kind_of(y: &Byml, h: u32) -> Option<String> {
    tables(y)
        .iter()
        .find(|(_, v)| v.as_arr().is_some_and(|a| a.iter().any(|r| hash_of(r) == Some(h))))
        .map(|(k, _)| k.clone())
}

fn row<'a>(y: &'a mut Byml, ty: &str, name: &str) -> Res<&'a mut Node> {
    let h: u32 = key(name);
    let found: bool = y
        .root
        .get("Data")
        .and_then(|d| d.get(ty))
        .and_then(|t| t.as_arr())
        .is_some_and(|a| a.iter().any(|r| hash_of(r) == Some(h)));
    if !found {
        return Err(match kind_of(y, h) {
            Some(k) => Error::Msg(format!("start defaults: {name} is a {k} flag, not a {ty} flag")),
            None => Error::Msg(format!("start defaults: the flag list has no flag named {name}")),
        });
    }
    y.root
        .get_mut("Data")
        .and_then(|d| d.get_mut(ty))
        .and_then(|t| t.as_arr_mut())
        .and_then(|a| a.iter_mut().find(|r| hash_of(r) == Some(h)))
        .ok_or(Error::Format("start defaults: the flag row vanished"))
}

pub fn allowed(reset: i32, save: i32) -> bool {
    save == SAVED && reset & !KEPT == 0 && reset & NEW_GAME != 0
}

fn check(r: &Node, name: &str) -> Res<()> {
    let reset: i32 = r.get("ResetTypeValue").and_then(num).unwrap_or(-1) as i32;
    let save: i32 = r.get("SaveFileIndex").and_then(num).unwrap_or(-1) as i32;
    if save != SAVED {
        return Err(Error::Msg(format!("start defaults: {name} is not saved in progress.sav (save file {save}), a new game value would not stick")));
    }
    if !allowed(reset, save) {
        return Err(Error::Msg(format!(
            "start defaults: {name} has reset mask {reset}, which is not only the new game resets (8, 16, 32), so the game would put the value back during play or never apply it"
        )));
    }
    Ok(())
}

pub fn set_bool(y: &mut Byml, name: &str, v: bool) -> Res<bool> {
    let r: &mut Node = row(y, "Bool", name)?;
    check(r, name)?;
    let old: Option<Node> = r.get("DefaultValue").cloned();
    if !matches!(old, Some(Node::Bool(_))) {
        return Err(Error::Msg(format!("start defaults: {name} has no Bool default")));
    }
    r.set("DefaultValue", Node::Bool(v));
    Ok(old != Some(Node::Bool(v)))
}

pub fn set_int(y: &mut Byml, name: &str, v: i32) -> Res<bool> {
    let r: &mut Node = row(y, "Int", name)?;
    check(r, name)?;
    let old: Option<Node> = r.get("DefaultValue").cloned();
    if !matches!(old, Some(Node::I32(_))) {
        return Err(Error::Msg(format!("start defaults: {name} has no Int default")));
    }
    r.set("DefaultValue", Node::I32(v));
    Ok(old != Some(Node::I32(v)))
}

pub fn set_enum(y: &mut Byml, name: &str, v: &str) -> Res<bool> {
    let r: &mut Node = row(y, "Enum", name)?;
    check(r, name)?;
    let names: Vec<&str> = r.get("RawValues").and_then(|x| x.as_arr()).unwrap_or(&[]).iter().filter_map(|x| x.as_str()).collect();
    if !names.contains(&v) {
        return Err(Error::Msg(format!("start defaults: {name} has no value {v} (it has {})", names.join(", "))));
    }
    let old: Option<Node> = r.get("DefaultValue").cloned();
    if !matches!(old, Some(Node::U32(_))) {
        return Err(Error::Msg(format!("start defaults: {name} has no Enum default")));
    }
    let h: Node = Node::U32(key(v));
    r.set("DefaultValue", h.clone());
    Ok(old != Some(h))
}

pub fn grant(y: &mut Byml, actor: &str, n: i32) -> Res<bool> {
    let a: bool = set_bool(y, &got(actor), true)?;
    let b: bool = set_int(y, &stock(actor), n)?;
    Ok(a || b)
}

pub fn apply_one(y: &mut Byml, e: &Edit) -> Res<bool> {
    match e {
        Edit::Bool(n, v) => set_bool(y, n, *v),
        Edit::Int(n, v) => set_int(y, n, *v),
        Edit::Enum(n, v) => set_enum(y, n, v),
        Edit::Grant(a, n) => grant(y, a, *n),
    }
}

pub fn apply(y: &mut Byml, edits: &[Edit]) -> Res<usize> {
    let mut n: usize = 0;
    for e in edits {
        if apply_one(y, e)? {
            n += 1;
        }
    }
    Ok(n)
}

fn edit_raw(raw: &[u8], edits: &[Edit]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    apply(&mut y, edits)?;
    y.write()
}

pub fn stage(rf: &mut Romfs, more: &mut Vec<Out>, edits: &[Edit]) -> Res<()> {
    if edits.is_empty() {
        return Ok(());
    }
    match more.iter().position(|o| o.path == LIST) {
        Some(i) => {
            let id: Option<u32> = dict_id(&more[i].z);
            let raw: Vec<u8> = rf.zs.decompress(&more[i].z)?;
            let out: Vec<u8> = edit_raw(&raw, edits)?;
            more[i].new = out.len();
            more[i].z = zs::compress_raw(&out, id)?;
        }
        None => {
            let o: Out = rewrite(rf, LIST, |raw| edit_raw(raw, edits))?;
            more.push(o);
        }
    }
    Ok(())
}

pub fn build(romfs: &P, load: &P, name: &str, edits: &[Edit], step: Option<&str>) -> Res<String> {
    if edits.is_empty() && step.is_none() {
        return Err(Error::Format("start defaults: nothing to change"));
    }
    let mut rf: Romfs = Romfs::open(romfs)?;
    let mut outs: Vec<Out> = Vec::new();
    stage(&mut rf, &mut outs, edits)?;
    if let Some(s) = step {
        opening::stage(&mut rf, &mut outs, s)?;
    }
    let c: Committed = commit(&mut rf, load, name, &outs, None)?;
    Ok(c.report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf as PB;

    fn flag(name: &str, def: Node, reset: i32, save: i32) -> Node {
        Node::Map(vec![
            ("DefaultValue".to_string(), def),
            ("Hash".to_string(), Node::U32(key(name))),
            ("ResetTypeValue".to_string(), Node::I32(reset)),
            ("SaveFileIndex".to_string(), Node::I32(save)),
        ])
    }

    fn quest(name: &str, vals: &[&str]) -> Node {
        Node::Map(vec![
            ("DefaultValue".to_string(), Node::U32(key(vals[0]))),
            ("Hash".to_string(), Node::U32(key(name))),
            ("RawValues".to_string(), Node::Arr(vals.iter().map(|v| Node::Str(v.to_string())).collect())),
            ("ResetTypeValue".to_string(), Node::I32(8)),
            ("SaveFileIndex".to_string(), Node::I32(0)),
            ("Values".to_string(), Node::Arr(vals.iter().map(|v| Node::U64(key(v) as u64)).collect())),
        ])
    }

    fn sample() -> Byml {
        let bools: Vec<Node> = vec![
            flag("Gate", Node::Bool(false), 8, 0),
            flag("Scene", Node::Bool(false), 9, 0),
            flag("Temp", Node::Bool(false), 8, -1),
            flag("IsGet.Item_X", Node::Bool(false), 8, 0),
        ];
        let ints: Vec<Node> = vec![flag("Rupee", Node::I32(0), 8, 0), flag("PouchData.Item_X.StockNum", Node::I32(0), 32, 0)];
        let data: Node = Node::Map(vec![
            ("Bool".to_string(), Node::Arr(bools)),
            ("Enum".to_string(), Node::Arr(vec![quest("Step_Q", &["NotReady", "Ready", "Step01"])])),
            ("Int".to_string(), Node::Arr(ints)),
        ]);
        Byml { root: Node::Map(vec![("Data".to_string(), data)]), version: 2, big: false }
    }

    fn def(y: &Byml, ty: &str, name: &str) -> Node {
        let a: &[Node] = y.root.get("Data").unwrap().get(ty).unwrap().as_arr().unwrap();
        a.iter().find(|r| hash_of(r) == Some(key(name))).unwrap().get("DefaultValue").unwrap().clone()
    }

    #[test]
    fn each_kind_sets_only_its_default() {
        let mut y: Byml = sample();
        let before: Byml = y.clone();
        assert!(set_bool(&mut y, "Gate", true).unwrap());
        assert!(!set_bool(&mut y, "Gate", true).unwrap());
        assert!(set_int(&mut y, "Rupee", 42).unwrap());
        assert!(set_enum(&mut y, "Step_Q", "Step01").unwrap());
        assert_eq!(def(&y, "Bool", "Gate"), Node::Bool(true));
        assert_eq!(def(&y, "Int", "Rupee"), Node::I32(42));
        assert_eq!(def(&y, "Enum", "Step_Q"), Node::U32(key("Step01")));
        assert_eq!(def(&y, "Bool", "Scene"), def(&before, "Bool", "Scene"));
        assert_eq!(y.root.get("Data").unwrap().get("Enum").unwrap().as_arr().unwrap()[0].get("Values"), before.root.get("Data").unwrap().get("Enum").unwrap().as_arr().unwrap()[0].get("Values"));
    }

    #[test]
    fn a_grant_sets_the_first_time_flag_and_the_pouch_count() {
        let mut y: Byml = sample();
        assert!(grant(&mut y, "Item_X", 3).unwrap());
        assert_eq!(def(&y, "Bool", "IsGet.Item_X"), Node::Bool(true));
        assert_eq!(def(&y, "Int", "PouchData.Item_X.StockNum"), Node::I32(3));
        assert!(grant(&mut y, "Item_Y", 1).is_err());
    }

    #[test]
    fn unknown_names_wrong_types_and_bad_values_are_refused() {
        let mut y: Byml = sample();
        let e = |r: Res<bool>| r.err().unwrap().to_string();
        assert!(e(set_bool(&mut y, "Gaet", true)).contains("no flag named Gaet"));
        assert!(e(set_bool(&mut y, "Rupee", true)).contains("is a Int flag, not a Bool flag"));
        assert!(e(set_enum(&mut y, "Step_Q", "Step09")).contains("has no value Step09"));
        assert_eq!(y, sample());
    }

    #[test]
    fn a_flag_the_game_resets_during_play_or_never_saves_is_refused() {
        let mut y: Byml = sample();
        assert!(set_bool(&mut y, "Scene", true).err().unwrap().to_string().contains("reset mask 9"));
        assert!(set_bool(&mut y, "Temp", true).err().unwrap().to_string().contains("not saved"));
        assert!(allowed(8, 0) && allowed(24, 0) && allowed(32, 0));
        assert!(!allowed(9, 0) && !allowed(264, 0) && !allowed(16, 0) && !allowed(0, 0) && !allowed(8, 1));
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

    fn changed_rows(a: &Byml, b: &Byml) -> Vec<(String, u32)> {
        let mut out: Vec<(String, u32)> = Vec::new();
        for ((k, x), (_, z)) in tables(a).iter().zip(tables(b).iter()) {
            let (xa, za): (&[Node], &[Node]) = (x.as_arr().unwrap(), z.as_arr().unwrap());
            assert_eq!(xa.len(), za.len(), "{k}");
            for (r, s) in xa.iter().zip(za.iter()) {
                assert_eq!(r.get("Hash"), s.get("Hash"));
                if r != s {
                    out.push((k.clone(), hash_of(r).unwrap_or(0)));
                }
            }
        }
        out
    }

    #[test]
    fn the_real_flag_list_round_trips_and_only_the_named_rows_change() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read_zs(LIST).unwrap();
        let orig: Byml = Byml::parse(&raw).unwrap();
        assert_eq!(orig.write().unwrap(), raw);
        let b: presets::Book = presets::book().unwrap();
        let mut edits: Vec<Edit> = b.presets.iter().flat_map(|p| p.edits.clone()).collect();
        edits.extend(b.control.clone());
        edits.push(Edit::Enum("Step_Meet_DekuTree".to_string(), "Step03".to_string()));
        edits.push(Edit::Grant("Item_Ocarina_Fairy".to_string(), 1));
        let mut y: Byml = orig.clone();
        assert_eq!(apply(&mut y, &edits).unwrap(), edits.len());
        let out: Vec<u8> = y.write().unwrap();
        assert_eq!(out.len(), raw.len());
        let back: Byml = Byml::parse(&out).unwrap();
        assert_eq!(back.root, y.root);
        assert_eq!(back.write().unwrap(), out);
        let mut want: Vec<(String, u32)> = vec![
            ("Bool".to_string(), key("Npc_Hylia_Soldier_003_GateOpen")),
            ("Bool".to_string(), key("IsGet.Item_Ocarina_Fairy")),
            ("Int".to_string(), key("PlayerStatus.CurrentRupee")),
            ("Int".to_string(), key("PouchData.Item_Ocarina_Fairy.StockNum")),
            ("Enum".to_string(), key("Step_Meet_DekuTree")),
            ("Enum".to_string(), key("Step_Get_StoneOfWater")),
            ("Bool".to_string(), key("GaveFishToJabJab")),
            ("Bool".to_string(), key("JabJabOpenedMouth")),
            ("Bool".to_string(), key("HyruleField_ArriveEntrance")),
            ("Bool".to_string(), key("HavePlayedEvent.Dm_HrlField_0020")),
        ];
        let mut got: Vec<(String, u32)> = changed_rows(&orig, &back);
        want.sort();
        got.sort();
        assert_eq!(got, want);
        assert_eq!(def(&back, "Bool", "Npc_Hylia_Soldier_003_GateOpen"), Node::Bool(true));
        assert_eq!(def(&back, "Int", "PlayerStatus.CurrentRupee"), Node::I32(42));
        assert_eq!(def(&back, "Enum", "Step_Meet_DekuTree"), Node::U32(1871805950));
        let mut z: Byml = orig.clone();
        assert!(set_bool(&mut z, "GerudoFortress_TrainingGroundGateOpend", true).err().unwrap().to_string().contains("reset mask 9"));
        assert!(set_bool(&mut z, "IsGet.Item_Map_Kokiri", true).err().unwrap().to_string().contains("no flag named"));
        assert_eq!(z.write().unwrap(), raw);
    }

    #[test]
    fn the_stage_merges_into_an_earlier_flag_list_edit_and_writes_one_file() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut more: Vec<Out> = Vec::new();
        stage(&mut rf, &mut more, &[]).unwrap();
        assert!(more.is_empty());
        crate::pot_slice::rows::stage(&mut rf, &mut more, &[]).unwrap();
        assert!(more.is_empty());
        let raw: Vec<u8> = rf.read_zs(LIST).unwrap();
        let y: Byml = Byml::parse(&raw).unwrap();
        let first: u64 = y.root.get("Data").unwrap().get(crate::pots::TABLE).unwrap().as_arr().unwrap().iter().find_map(|r| crate::placement::hash_of(r).filter(|_| r.get("ExtraByte").is_some())).unwrap();
        crate::pot_slice::rows::stage(&mut rf, &mut more, &[first]).unwrap();
        let gate: Vec<Edit> = vec![Edit::Bool("Npc_Hylia_Soldier_003_GateOpen".to_string(), true)];
        stage(&mut rf, &mut more, &gate).unwrap();
        assert_eq!(more.len(), 1);
        assert_eq!(more[0].path, LIST);
        let both: Byml = Byml::parse(&rf.zs.decompress(&more[0].z).unwrap()).unwrap();
        assert_eq!(def(&both, "Bool", "Npc_Hylia_Soldier_003_GateOpen"), Node::Bool(true));
        let row: &Node = both.root.get("Data").unwrap().get(crate::pots::TABLE).unwrap().as_arr().unwrap().iter().find(|r| crate::placement::hash_of(r) == Some(first)).unwrap();
        assert_eq!(row.get("ResetTypeValue"), Some(&Node::I32(24)));
        let mut alone: Vec<Out> = Vec::new();
        stage(&mut rf, &mut alone, &gate).unwrap();
        assert_eq!(alone.len(), 1);
        assert_eq!(dict_id(&alone[0].z), dict_id(&rf.read(LIST).unwrap()));
        assert_eq!(alone[0].new, raw.len());
    }

    fn built(root: &std::path::Path, pairs: &[(&str, crate::settings::Value)], seed: &str) -> (Byml, Option<crate::bfevfl::Bf>, crate::settings::Settings, String) {
        let d: crate::slice::data::Data = crate::slice::data::Data::load().unwrap();
        let mut s: crate::settings::Settings = crate::settings::Settings::default();
        for (k, v) in pairs {
            s.set(k, v.clone()).unwrap();
        }
        let g: crate::slice::generate::Generated = crate::slice::generate::generate_data(&d, &s, seed, &crate::slice::generate::GenOpts::default()).unwrap();
        let back: crate::settings::Settings = g.seed_file.settings().unwrap().settings;
        let load: PB = PB::from(env!("CARGO_MANIFEST_DIR")).join("../../target/start_defaults_mods");
        let b: crate::slice::edit::Built = crate::slice::edit::build(&g.seed_file, root, &load, seed).unwrap();
        let out: PB = b.root.join("romfs");
        let mut rf: Romfs = Romfs::open(root).unwrap();
        let list: Byml = Byml::parse(&rf.zs.decompress(&std::fs::read(out.join(LIST)).unwrap()).unwrap()).unwrap();
        let f: PB = out.join(crate::intro_skip::FILE);
        let flow: Option<crate::bfevfl::Bf> = if f.is_file() { Some(crate::bfevfl::Bf::parse(&rf.zs.decompress(&std::fs::read(f).unwrap()).unwrap()).unwrap()) } else { None };
        (list, flow, back, b.report)
    }

    fn opening_steps(b: &crate::bfevfl::Bf) -> Vec<String> {
        let f: &crate::bfevfl::Flow = b.flows.iter().find(|x| x.name == crate::intro_skip::FLOW).unwrap();
        let act = |e: &crate::bfevfl::Event| -> Option<String> {
            let crate::bfevfl::Kind::Action { actor, act, .. } = &e.kind else {
                return None;
            };
            let a: &str = f.actors.get(*actor as usize)?.actions.get(*act as usize)?.as_str();
            let c: &crate::bfevfl::Cont = e.kind.params()?;
            match (a, c.get("ChallengeName"), c.get("StepName")) {
                (opening::ACT, Some(crate::bfevfl::Val::Str(q)), Some(crate::bfevfl::Val::Str(st))) if q == opening::QUEST => Some(st.clone()),
                _ => None,
            }
        };
        f.events.iter().filter_map(act).collect()
    }

    #[test]
    fn open_kakariko_seeds_generate_and_are_beatable_under_the_open_logic() {
        use crate::slice::generate::{generate_data, GenOpts, Generated};
        let d: crate::slice::data::Data = crate::slice::data::Data::load().unwrap();
        let base: crate::settings::Settings = crate::settings::Settings::default();
        let mut s: crate::settings::Settings = base.clone();
        s.set("open_kakariko", crate::settings::Value::s("open")).unwrap();
        let p: crate::slice::support::Plan = crate::slice::support::plan(&s).unwrap();
        let w: crate::world::World = crate::slice::logic::world_with(&d, &p.logic()).unwrap();
        for n in 0..4 {
            let seed: String = format!("kak {n}");
            let a: Generated = generate_data(&d, &base, &seed, &GenOpts::default()).unwrap();
            let b: Generated = generate_data(&d, &s, &seed, &GenOpts::default()).unwrap();
            assert!(b.report.contains("Death Mountain starts open"));
            assert!(!a.report.contains("Death Mountain starts open"));
            assert_ne!(a.seed_file.settings_string, b.seed_file.settings_string);
            let mut pl: crate::world::Placement = crate::slice::fill::vanilla_place(&w, &d).unwrap();
            for (loc, item) in &b.seed_file.placements {
                pl.put(w.loc(loc).unwrap(), w.item(item).unwrap());
            }
            assert!(crate::search::explore(&w, &pl, &[]).won(), "{seed}");
        }
    }

    #[test]
    fn open_kakariko_and_open_forest_round_trip_through_generate_and_build() {
        let Some(root) = real() else {
            return;
        };
        let raw: Byml = Romfs::open(&root).unwrap().read_byml(LIST).unwrap();
        let v: fn(&str) -> crate::settings::Value = crate::settings::Value::s;
        let (list, flow, back, report): (Byml, Option<crate::bfevfl::Bf>, crate::settings::Settings, String) = built(&root, &[("open_kakariko", v("open")), ("shuffle_boss_hearts", crate::settings::Value::Bool(true)), ("shuffle_pots", v("all"))], "defaults kak");
        assert_eq!(back.text("open_kakariko").unwrap(), "open");
        assert_eq!(def(&list, "Bool", "Npc_Hylia_Soldier_003_GateOpen"), Node::Bool(true));
        let rows: Vec<(String, u32)> = changed_rows(&raw, &list);
        assert!(rows.contains(&("Bool".to_string(), key("Npc_Hylia_Soldier_003_GateOpen"))));
        assert!(rows.iter().any(|(t, _)| t == crate::pots::TABLE), "the pot rows were lost");
        assert!(report.contains("Death Mountain starts open"));
        assert_eq!(opening_steps(&flow.unwrap()), vec![opening::FROM.to_string()]);
        let (list, flow, back, report): (Byml, Option<crate::bfevfl::Bf>, crate::settings::Settings, String) = built(&root, &[("open_forest", v("open")), ("shuffle_pots", v("all"))], "defaults forest");
        assert_eq!(back.text("open_forest").unwrap(), "open");
        assert_eq!(def(&list, "Bool", "Npc_Hylia_Soldier_003_GateOpen"), Node::Bool(false));
        assert_eq!(opening_steps(&flow.unwrap()), vec![opening::OPEN_STEP.to_string()]);
        assert!(report.contains("Meet_DekuTree Step03"));
        let (_, flow, _, _): (Byml, Option<crate::bfevfl::Bf>, crate::settings::Settings, String) = built(&root, &[("open_forest", v("open")), ("skip_intro", crate::settings::Value::Bool(false)), ("shuffle_pots", v("all"))], "defaults forest long");
        assert_eq!(opening_steps(&flow.unwrap()), vec![opening::OPEN_STEP.to_string()]);
    }
}
