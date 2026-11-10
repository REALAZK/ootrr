use std::collections::{BTreeMap as BM, BTreeSet as BS};

use crate::byml::{Byml, Node};
use crate::error::Res;
use crate::placement::{banc_files, hash_of};
use crate::romfs::Romfs;

pub const ENTRANCE: &str = "FieldAndDungeonEntranceEventRequestActor";
pub const ANCHOR: &str = "ChangeBancBeforehandAnchor";
pub const STARTPOS: &str = "Banc/BancBasedResource/StartPos.byml.zs";

const CORE: [&str; 7] = [
    "NextBancParamNameAdult",
    "NextBancParamNameChild",
    "NextStartPosName",
    "FirstEventName",
    "NextEventName",
    "InitEventName",
    "InitEventFlag",
];

const DUNGEONS: [&str; 12] = [
    "BottomOfTheWell",
    "DodongoCavern",
    "FireTemple",
    "ForestTemple",
    "GerudoTrainingGround",
    "IceCavern",
    "InsideGanonCastle",
    "InsideJabuJabuBelly",
    "InsideTheDekuTree",
    "ShadowTemple",
    "SpiritTemple",
    "WaterTemple",
];

const SPECIAL: [&str; 5] = [
    "SageRoom",
    "FinalBattleField",
    "CrumblingGanonCastle",
    "BossRushIntervalRoom",
    "EnemyMuseum",
];

const AREAS: [(&str, &str); 33] = [
    ("BottomOfTheWell", "Bottom of the Well"),
    ("DeathMountainCrater", "Death Mountain Crater"),
    ("DeathMountainTrail", "Death Mountain Trail"),
    ("DesertColossus", "Desert Colossus"),
    ("DodongoCavern", "Dodongo's Cavern"),
    ("FireTemple", "Fire Temple"),
    ("ForestTemple", "Forest Temple"),
    ("GanonCastle", "Outside Ganon's Castle"),
    ("GerudoFortress", "Gerudo's Fortress"),
    ("GerudoTrainingGround", "Gerudo Training Ground"),
    ("GerudoValley", "Gerudo Valley"),
    ("GoronCity", "Goron City"),
    ("Graveyard", "Graveyard"),
    ("HountedWasteland", "Haunted Wasteland"),
    ("HyruleCastle", "Hyrule Castle"),
    ("HyruleField", "Hyrule Field"),
    ("IceCavern", "Ice Cavern"),
    ("InsideGanonCastle", "Ganon's Castle"),
    ("InsideJabuJabuBelly", "Jabu Jabu's Belly"),
    ("InsideTheDekuTree", "Deku Tree"),
    ("KakarikoVillage", "Kakariko Village"),
    ("KokiriForest", "Kokiri Forest"),
    ("LostWood", "Lost Woods"),
    ("Market", "Market"),
    ("SacredForestMeadow", "Sacred Forest Meadow"),
    ("ShadowTemple", "Shadow Temple"),
    ("SpiritTemple", "Spirit Temple"),
    ("WaterTemple", "Water Temple"),
    ("ZoraDomain", "Zora's Domain"),
    ("ZoraFountain", "Zora's Fountain"),
    ("ZoraRiver", "Zora's River"),
    ("SageRoom", "Chamber of Sages"),
    ("FinalBattleField", "Ganon's Tower"),
];

#[derive(Debug, Clone, PartialEq)]
pub struct Ent {
    pub file: String,
    pub map: String,
    pub age: String,
    pub hash: u64,
    pub name: String,
    pub adult: String,
    pub child: String,
    pub start: String,
    pub first: String,
    pub next: String,
    pub init_ev: String,
    pub init_flag: String,
    pub other: Vec<(String, String)>,
    pub pos: Vec<f32>,
    pub links: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Anc {
    pub file: String,
    pub hash: u64,
    pub pos: Vec<f32>,
    pub radius: Option<f32>,
    pub links: Vec<u64>,
    pub strs: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sp {
    pub map: String,
    pub name: String,
    pub trans: Vec<f32>,
    pub rot: Vec<f32>,
}

#[derive(Debug, Default)]
pub struct Scan {
    pub ents: Vec<Ent>,
    pub ancs: Vec<Anc>,
    pub sps: Vec<Sp>,
    pub params: BS<String>,
    pub banc_keys: BM<String, usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Cat {
    Dungeon,
    Overworld,
    Interior,
    Boss,
    Other,
}

impl Cat {
    pub fn name(self) -> &'static str {
        match self {
            Cat::Dungeon => "dungeon",
            Cat::Overworld => "overworld",
            Cat::Interior => "interior",
            Cat::Boss => "boss",
            Cat::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum St {
    Ok,
    NoParam,
    NoStart,
    Unset,
}

impl St {
    pub fn name(self) -> &'static str {
        match self {
            St::Ok => "ok",
            St::NoParam => "no_param",
            St::NoStart => "no_start",
            St::Unset => "unset",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Row {
    pub i: usize,
    pub cat: Cat,
    pub src_area: String,
    pub dst_area: String,
    pub ages: Vec<&'static str>,
    pub st_adult: St,
    pub st_child: St,
    pub rets: Vec<(&'static str, u64)>,
    pub weak_rets: Vec<(&'static str, u64)>,
    pub anchors: Vec<u64>,
    pub no_dest: bool,
}

#[derive(Debug, Clone)]
pub struct Broken {
    pub i: usize,
    pub field: &'static str,
    pub param: String,
    pub start: String,
    pub st: St,
    pub why: &'static str,
}

#[derive(Debug, Default)]
pub struct Analysis {
    pub rows: Vec<Row>,
    pub broken: Vec<Broken>,
    pub loose: Vec<usize>,
    pub reach: BM<String, BS<&'static str>>,
}

pub type StartIdx = BM<String, BS<String>>;

pub fn stem(f: &str) -> String {
    let b: &str = f.rsplit('/').next().unwrap_or(f);
    let b: &str = b.split('.').next().unwrap_or(b);
    let b: &str = b.strip_suffix("_Static").or_else(|| b.strip_suffix("_Dynamic")).unwrap_or(b);
    b.to_string()
}

pub fn age_of(map: &str) -> &'static str {
    if map.ends_with("_Adult") || map.contains("_Adult_") {
        "adult"
    } else if map.ends_with("_Child") || map.contains("_Child_") {
        "child"
    } else {
        "both"
    }
}

pub fn is_alt(map: &str) -> bool {
    map.ends_with("_Alternative")
}

pub fn base(map: &str) -> String {
    let m: &str = map.strip_suffix("_Alternative").unwrap_or(map);
    let m: &str = m.strip_suffix("_Adult").or_else(|| m.strip_suffix("_Child")).unwrap_or(m);
    m.to_string()
}

pub fn kind_of(b: &str) -> Cat {
    if DUNGEONS.contains(&b) {
        Cat::Dungeon
    } else if b.ends_with("Stage") {
        Cat::Boss
    } else if SPECIAL.contains(&b) || b.starts_with("Demo_") {
        Cat::Other
    } else {
        Cat::Overworld
    }
}

pub fn area_of(b: &str, hint: &str) -> String {
    let lake: bool = ["LakeHylia", "ZoraLakeHylia", "OutsideOfWaterTemple", "WaterTemple"]
        .iter()
        .any(|p| hint.starts_with(p));
    if b == "HyruleField" && lake {
        return "Lake Hylia".to_string();
    }
    match AREAS.iter().find(|a| a.0 == b) {
        Some(a) => a.1.to_string(),
        None => b.to_string(),
    }
}

fn txt(n: &Node, k: &str) -> String {
    n.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn floats(n: Option<&Node>) -> Vec<f32> {
    n.and_then(|t| t.as_arr())
        .map(|t| t.iter().filter_map(|x| if let Node::F32(v) = x { Some(*v) } else { None }).collect())
        .unwrap_or_default()
}

fn link_dsts(a: &Node) -> Vec<u64> {
    let mut out: Vec<u64> = Vec::new();
    if let Some(l) = a.get("Links").and_then(|x| x.as_arr()) {
        for k in l {
            match k.get("Dst") {
                Some(Node::U64(x)) => out.push(*x),
                Some(Node::I64(x)) => out.push(*x as u64),
                _ => {}
            }
        }
    }
    out
}

fn show(n: &Node) -> String {
    match n {
        Node::Str(s) => s.clone(),
        Node::Bool(b) => b.to_string(),
        Node::I32(x) => x.to_string(),
        Node::U32(x) => x.to_string(),
        Node::F32(x) => x.to_string(),
        o => format!("{:?}", o),
    }
}

fn read_ent(f: &str, a: &Node) -> Ent {
    let d: Option<&Node> = a.get("Dynamic");
    let g = |k: &str| -> String { d.map(|x| txt(x, k)).unwrap_or_default() };
    let mut other: Vec<(String, String)> = Vec::new();
    if let Some(Node::Map(m)) = d {
        for (k, v) in m {
            if !CORE.contains(&k.as_str()) {
                other.push((k.clone(), show(v)));
            }
        }
    }
    let map: String = stem(f);
    Ent {
        file: f.to_string(),
        age: age_of(&map).to_string(),
        map,
        hash: hash_of(a).unwrap_or(0),
        name: txt(a, "Name"),
        adult: g("NextBancParamNameAdult"),
        child: g("NextBancParamNameChild"),
        start: g("NextStartPosName"),
        first: g("FirstEventName"),
        next: g("NextEventName"),
        init_ev: g("InitEventName"),
        init_flag: g("InitEventFlag"),
        other,
        pos: floats(a.get("Translate")),
        links: link_dsts(a),
    }
}

fn read_anc(f: &str, a: &Node) -> Anc {
    let mut strs: Vec<(String, String)> = Vec::new();
    let mut radius: Option<f32> = None;
    if let Some(Node::Map(m)) = a.get("Dynamic") {
        for (k, v) in m {
            match v {
                Node::F32(x) if k == "LoadRadius" => radius = Some(*x),
                o => strs.push((k.clone(), show(o))),
            }
        }
    }
    Anc {
        file: f.to_string(),
        hash: hash_of(a).unwrap_or(0),
        pos: floats(a.get("Translate")),
        radius,
        links: link_dsts(a),
        strs,
    }
}

pub fn read_startpos(y: &Byml) -> Vec<Sp> {
    let mut out: Vec<Sp> = Vec::new();
    let Node::Map(top) = &y.root else {
        return out;
    };
    for (m, v) in top {
        let Node::Map(inner) = v else {
            continue;
        };
        for (n, p) in inner {
            out.push(Sp {
                map: m.clone(),
                name: n.clone(),
                trans: floats(p.get("Trans")),
                rot: floats(p.get("Rot")),
            });
        }
    }
    out
}

pub fn scan(rf: &mut Romfs) -> Res<Scan> {
    let mut s: Scan = Scan::default();
    for f in banc_files(rf)? {
        let y: Byml = rf.read_byml(&f)?;
        let Some(l) = y.root.get("Actors").and_then(|x| x.as_arr()) else {
            continue;
        };
        for a in l {
            let gy: String = txt(a, "Gyaml");
            if let Some(Node::Map(m)) = a.get("Dynamic") {
                for (k, _) in m {
                    if k.contains("Banc") || k.contains("StartPos") {
                        *s.banc_keys.entry(format!("{gy}.{k}")).or_insert(0) += 1;
                    }
                }
            }
            if gy == ENTRANCE {
                s.ents.push(read_ent(&f, a));
            } else if gy == ANCHOR {
                s.ancs.push(read_anc(&f, a));
            }
        }
    }
    for f in rf.list("Banc/GameBancParam")? {
        let b: &str = f.rsplit('/').next().unwrap_or(&f);
        if let Some(n) = b.split('.').next() {
            s.params.insert(n.to_string());
        }
    }
    let y: Byml = rf.read_byml(STARTPOS)?;
    s.sps = read_startpos(&y);
    Ok(s)
}

pub fn start_index(s: &Scan) -> StartIdx {
    let mut out: StartIdx = BM::new();
    for p in &s.sps {
        out.entry(p.map.clone()).or_default().insert(p.name.clone());
    }
    out
}

pub fn resolve(s: &Scan, ix: &StartIdx, param: &str, start: &str) -> St {
    if param.is_empty() && start.is_empty() {
        return St::Unset;
    }
    if !s.params.contains(param) {
        return St::NoParam;
    }
    match ix.get(param) {
        Some(set) if set.contains(start) => St::Ok,
        _ => St::NoStart,
    }
}

pub fn ages_of(age: &str) -> Vec<&'static str> {
    match age {
        "adult" => vec!["adult"],
        "child" => vec!["child"],
        _ => vec!["adult", "child"],
    }
}

pub fn field<'a>(e: &'a Ent, age: &str) -> &'a str {
    if age == "adult" { &e.adult } else { &e.child }
}

fn primary(e: &Ent) -> &str {
    match e.age.as_str() {
        "child" => &e.child,
        _ => {
            if e.adult.is_empty() {
                &e.child
            } else {
                &e.adult
            }
        }
    }
}

pub fn analyze(s: &Scan) -> Analysis {
    let ix: StartIdx = start_index(s);
    let mut an: Analysis = Analysis::default();
    for e in &s.ents {
        for x in ages_of(&e.age) {
            let p: &str = field(e, x);
            if !p.is_empty() {
                an.reach.entry(p.to_string()).or_default().insert(x);
            }
        }
    }
    for (i, e) in s.ents.iter().enumerate() {
        let no_dest: bool = e.adult.is_empty() && e.child.is_empty() && e.start.is_empty();
        let sb: String = base(&e.map);
        let dp: &str = primary(e);
        let db: String = base(dp);
        let cat: Cat = if no_dest {
            kind_of(&sb)
        } else {
            let a: Cat = kind_of(&sb);
            let b: Cat = kind_of(&db);
            if a == Cat::Dungeon || b == Cat::Dungeon {
                Cat::Dungeon
            } else if a == Cat::Boss || b == Cat::Boss {
                Cat::Boss
            } else if a == Cat::Other || b == Cat::Other {
                Cat::Other
            } else {
                Cat::Overworld
            }
        };
        let st_adult: St = resolve(s, &ix, &e.adult, &e.start);
        let st_child: St = resolve(s, &ix, &e.child, &e.start);
        let ages: Vec<&'static str> = ages_of(&e.age);
        let mut rets: Vec<(&'static str, u64)> = Vec::new();
        let mut weak: Vec<(&'static str, u64)> = Vec::new();
        if !no_dest && !e.name.is_empty() {
            for x in &ages {
                let p: &str = field(e, x);
                for r in &s.ents {
                    if r.map != p || r.hash == e.hash {
                        continue;
                    }
                    if r.name == e.start && r.start == e.name {
                        rets.push((x, r.hash));
                    } else if r.start == e.name && !e.name.is_empty() {
                        weak.push((x, r.hash));
                    }
                }
            }
        }
        let anchors: Vec<u64> = s.ancs.iter().filter(|a| a.links.contains(&e.hash)).map(|a| a.hash).collect();
        an.rows.push(Row {
            i,
            cat,
            src_area: area_of(&sb, &e.name),
            dst_area: area_of(&db, &e.start),
            ages,
            st_adult,
            st_child,
            rets,
            weak_rets: weak,
            anchors,
            no_dest,
        });
    }
    for r in &an.rows {
        let e: &Ent = &s.ents[r.i];
        for (fld, st) in [("adult", r.st_adult), ("child", r.st_child)] {
            if st == St::Ok || st == St::Unset {
                continue;
            }
            let why: &'static str = if !r.ages.contains(&fld) {
                "file_age"
            } else if an.reach.get(&e.map).is_some_and(|a| !a.contains(fld)) {
                "map_age"
            } else {
                "live"
            };
            an.broken.push(Broken {
                i: r.i,
                field: if fld == "adult" { "adult" } else { "child" },
                param: field(e, fld).to_string(),
                start: e.start.clone(),
                st,
                why,
            });
        }
    }
    for (i, a) in s.ancs.iter().enumerate() {
        if a.strs.iter().any(|k| k.0 == "NextBancParamName") {
            an.loose.push(i);
        }
    }
    an
}

pub fn verify(s: &Scan, an: &Analysis) -> Vec<String> {
    let mut bad: Vec<String> = Vec::new();
    let hashes: BS<u64> = s.ents.iter().map(|e| e.hash).collect();
    if hashes.len() != s.ents.len() {
        bad.push("entrance hashes are not unique".to_string());
    }
    let listed: BS<(usize, &str)> = an.broken.iter().map(|b| (b.i, b.field)).collect();
    for r in &an.rows {
        let e: &Ent = &s.ents[r.i];
        for (fld, st) in [("adult", r.st_adult), ("child", r.st_child)] {
            let named: bool = !field(e, fld).is_empty();
            if named && st != St::Ok && !listed.contains(&(r.i, fld)) {
                bad.push(format!("{} {} {} unresolved and not listed", e.file, e.name, fld));
            }
        }
        for (_, h) in r.rets.iter().chain(r.weak_rets.iter()) {
            if !hashes.contains(h) {
                bad.push(format!("{} return {} is not an entrance", e.name, h));
            }
        }
    }
    bad
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_strips_age_and_variant() {
        assert_eq!(base("KakarikoVillage_Adult_Alternative"), "KakarikoVillage");
        assert_eq!(base("LostWood_Child"), "LostWood");
        assert_eq!(base("FireTemple"), "FireTemple");
    }

    #[test]
    fn ages_follow_file_names() {
        assert_eq!(age_of("LostWood_Adult"), "adult");
        assert_eq!(age_of("ZoraDomain_Adult_Alternative"), "adult");
        assert_eq!(age_of("LostWood_Child"), "child");
        assert_eq!(age_of("IceCavern"), "both");
        assert_eq!(stem("Banc/LostWood/LostWood_Adult_Static.bcett.byml.zs"), "LostWood_Adult");
    }

    #[test]
    fn kinds() {
        assert_eq!(kind_of("ShadowTemple"), Cat::Dungeon);
        assert_eq!(kind_of("GohmaStage"), Cat::Boss);
        assert_eq!(kind_of("SageRoom"), Cat::Other);
        assert_eq!(kind_of("HyruleField"), Cat::Overworld);
    }

    #[test]
    fn lake_hylia_split() {
        assert_eq!(area_of("HyruleField", "LakeHylia_FromGerudoValley"), "Lake Hylia");
        assert_eq!(area_of("HyruleField", "HyruleField_FrontOfMarket"), "Hyrule Field");
        assert_eq!(area_of("InsideTheDekuTree", ""), "Deku Tree");
    }

    #[test]
    fn resolve_states() {
        let mut s: Scan = Scan::default();
        s.params.insert("A".to_string());
        s.sps.push(Sp { map: "A".to_string(), name: "x".to_string(), trans: vec![], rot: vec![] });
        let ix: StartIdx = start_index(&s);
        assert_eq!(resolve(&s, &ix, "A", "x"), St::Ok);
        assert_eq!(resolve(&s, &ix, "A", "y"), St::NoStart);
        assert_eq!(resolve(&s, &ix, "B", "x"), St::NoParam);
        assert_eq!(resolve(&s, &ix, "", ""), St::Unset);
    }
}
