use std::collections::{BTreeMap as BM, BTreeSet as BS};
use std::fs;
use std::path::{Path as P, PathBuf as PB};

use crate::byml::{Byml, Node};
use crate::colors::{self, Colors};
use crate::error::{Error as E, Res};
use crate::rng::Rng;
use crate::romfs::Romfs;
use crate::sfx::{self, Sfx};
use crate::rsizetable::{self, Rst};
use crate::zs;

pub const BGM_FILE: &str = "Sound/Bgm/bgm.Product.111.byml.zs";
const STREAM: &str = "Sound/Resource/Stream";
const BGM_DICT: u32 = 1;
const JINGLES: [&str; 8] = [
    "BGM_Appear",
    "BGM_FinaleJingle",
    "BGM_GameOver",
    "BGM_ItemGet_FromZelda",
    "BGM_MedalGet",
    "BGM_SpiritStoneGet",
    "BGM_SleepInBed",
    "BGM_SleepInBedToEnemyMuseum",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Music {
    Off,
    Grouped,
    Anywhere,
}

impl Music {
    pub fn parse(s: &str) -> Option<Music> {
        match s {
            "off" => Some(Music::Off),
            "grouped" => Some(Music::Grouped),
            "anywhere" => Some(Music::Anywhere),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Music::Off => "off",
            Music::Grouped => "grouped",
            Music::Anywhere => "anywhere",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cosmetics {
    pub music: Music,
    pub sfx: Sfx,
    pub colors: Colors,
    pub seed: String,
}

impl Default for Cosmetics {
    fn default() -> Cosmetics {
        Cosmetics { music: Music::Off, sfx: Sfx::Off, colors: Colors::Off, seed: String::new() }
    }
}

impl Cosmetics {
    pub fn any(&self) -> bool {
        self.music != Music::Off || self.sfx != Sfx::Off || self.colors != Colors::Off
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Theme,
    Boss,
    Battle,
    Jingle,
}

fn kind(name: &str) -> Option<Kind> {
    if name.starts_with("BGM_Dm_") {
        return None;
    }
    if JINGLES.contains(&name) || name.contains("Jingle") {
        return Some(Kind::Jingle);
    }
    if name.starts_with("BGM_DungeonBoss") || name == "BGM_MiddleBoss" {
        return Some(Kind::Boss);
    }
    if name == "BGM_EnemyBattle" || name == "BGM_FishingBattle" {
        return Some(Kind::Battle);
    }
    Some(Kind::Theme)
}

fn asset_key(e: &Node, have: &dyn Fn(&str) -> bool) -> Option<String> {
    let nodes: &[Node] = e.get("Node")?.as_arr()?;
    let mut keys: Vec<&str> = Vec::new();
    for n in nodes {
        match n.get("Type").and_then(|t| t.as_str()) {
            Some("Fork") => return None,
            Some("Asset") => keys.push(n.get("Key")?.as_str()?),
            _ => {}
        }
    }
    if keys.len() == 1 {
        return Some(keys[0].to_string());
    }
    let side = |k: &&str| ["Intro", "Outro", "Pad", "Roar", "Demo"].iter().any(|w| k.contains(w));
    keys.iter().find(|k| !side(k) && have(k)).map(|k| k.to_string())
}

fn events(y: &Byml, have: &dyn Fn(&str) -> bool) -> Vec<(String, String, Kind)> {
    let mut out: Vec<(String, String, Kind)> = Vec::new();
    let Some(Node::Map(all)) = y.root.get("BGM") else {
        return out;
    };
    for (name, e) in all {
        let (Some(k), Some(key)) = (kind(name), asset_key(e, have)) else {
            continue;
        };
        if have(&key) {
            out.push((name.clone(), key, k));
        }
    }
    out
}

fn stems(e: &Node, have: &dyn Fn(&str) -> bool) -> Option<Vec<String>> {
    let nodes: &[Node] = e.get("Node")?.as_arr()?;
    let mut fork: bool = false;
    let mut keys: BS<String> = BS::new();
    for n in nodes {
        match n.get("Type").and_then(|t| t.as_str()) {
            Some("Fork") => fork = true,
            Some("Asset") => {
                keys.insert(n.get("Key")?.as_str()?.to_string());
            }
            _ => {}
        }
    }
    (fork && keys.len() > 1 && keys.iter().all(|k| have(k))).then(|| keys.into_iter().collect())
}

fn layered(y: &Byml, have: &dyn Fn(&str) -> bool) -> Vec<(String, Vec<String>, Kind)> {
    let mut out: Vec<(String, Vec<String>, Kind)> = Vec::new();
    let Some(Node::Map(all)) = y.root.get("BGM") else {
        return out;
    };
    for (name, e) in all {
        if let (Some(k), Some(set)) = (kind(name), stems(e, have)) {
            out.push((name.clone(), set, k));
        }
    }
    out
}

fn plan_layers(ev: &[(String, Vec<String>, Kind)], m: Music, rng: &mut Rng) -> BM<String, BM<String, String>> {
    let mut by: BM<u8, BS<Vec<String>>> = BM::new();
    for (_, set, k) in ev {
        let slot: u8 = match (m, k) {
            (Music::Anywhere, Kind::Jingle) => 9,
            (Music::Anywhere, _) => 0,
            (_, k) => *k as u8,
        };
        by.entry(slot).or_default().insert(set.clone());
    }
    let mut swap: BM<Vec<String>, Vec<String>> = BM::new();
    for pool in by.into_values() {
        let from: Vec<Vec<String>> = pool.into_iter().collect();
        let mut to: Vec<Vec<String>> = from.clone();
        rng.shuffle(&mut to);
        swap.extend(from.into_iter().zip(to));
    }
    let mut out: BM<String, BM<String, String>> = BM::new();
    for (name, set, _) in ev {
        let Some(new) = swap.get(set).filter(|n| *n != set) else {
            continue;
        };
        let map: BM<String, String> = set.iter().enumerate().map(|(i, k)| (k.clone(), new[i % new.len()].clone())).collect();
        out.insert(name.clone(), map);
    }
    out
}

fn rewrite_layers(y: &mut Byml, to: &BM<String, BM<String, String>>) -> usize {
    let mut n: usize = 0;
    let Some(Node::Map(all)) = y.root.get_mut("BGM") else {
        return n;
    };
    for (name, e) in all.iter_mut() {
        let Some(map) = to.get(name) else {
            continue;
        };
        let Some(nodes) = e.get_mut("Node").and_then(|x| x.as_arr_mut()) else {
            continue;
        };
        for node in nodes.iter_mut() {
            if node.get("Type").and_then(|t| t.as_str()) != Some("Asset") {
                continue;
            }
            let Some(new) = node.get("Key").and_then(|k| k.as_str()).and_then(|k| map.get(k)).cloned() else {
                continue;
            };
            if node.set("Key", Node::Str(new)) {
                n += 1;
            }
        }
    }
    n
}

fn pools(ev: &[(String, String, Kind)], m: Music) -> Vec<BS<String>> {
    let mut by: BM<u8, BS<String>> = BM::new();
    for (_, key, k) in ev {
        let slot: u8 = match (m, k) {
            (Music::Anywhere, Kind::Jingle) => 9,
            (Music::Anywhere, _) => 0,
            (_, k) => *k as u8,
        };
        by.entry(slot).or_default().insert(key.clone());
    }
    by.into_values().collect()
}

fn plan(ev: &[(String, String, Kind)], m: Music, rng: &mut Rng) -> BM<String, String> {
    let mut swap: BM<String, String> = BM::new();
    for pool in pools(ev, m) {
        let from: Vec<String> = pool.iter().cloned().collect();
        let mut to: Vec<String> = from.clone();
        rng.shuffle(&mut to);
        for (a, b) in from.into_iter().zip(to) {
            swap.insert(a, b);
        }
    }
    let mut out: BM<String, String> = BM::new();
    for (name, key, _) in ev {
        if let Some(n) = swap.get(key).filter(|n| *n != key) {
            out.insert(name.clone(), n.clone());
        }
    }
    out
}

fn rewrite(y: &mut Byml, to: &BM<String, String>) -> usize {
    let mut n: usize = 0;
    let Some(Node::Map(all)) = y.root.get_mut("BGM") else {
        return n;
    };
    for (name, e) in all.iter_mut() {
        let Some(new) = to.get(name) else {
            continue;
        };
        let Some(nodes) = e.get_mut("Node").and_then(|x| x.as_arr_mut()) else {
            continue;
        };
        for node in nodes.iter_mut() {
            if node.get("Type").and_then(|t| t.as_str()) == Some("Asset") && node.set("Key", Node::Str(new.clone())) {
                n += 1;
            }
        }
    }
    n
}

pub fn shuffle_music(y: &mut Byml, m: Music, seed: &str, have: &dyn Fn(&str) -> bool) -> usize {
    if m == Music::Off {
        return 0;
    }
    let ev: Vec<(String, String, Kind)> = events(y, have);
    let mut rng: Rng = Rng::from_text(&format!("{seed} music {}", m.id()));
    let to: BM<String, String> = plan(&ev, m, &mut rng);
    let lay: Vec<(String, Vec<String>, Kind)> = layered(y, have);
    let sets: BM<String, BM<String, String>> = plan_layers(&lay, m, &mut rng);
    rewrite(y, &to) + rewrite_layers(y, &sets)
}

pub fn apply(mod_root: &P, romfs: &P, c: &Cosmetics, seed: &str) -> Res<Vec<String>> {
    let mut notes: Vec<String> = apply_music(mod_root, romfs, c, seed)?;
    let text: &str = if c.seed.trim().is_empty() { seed } else { c.seed.trim() };
    notes.extend(sfx::apply(mod_root, romfs, c.sfx, text)?);
    notes.extend(colors::apply(mod_root, romfs, c.colors, text)?);
    Ok(notes)
}

fn apply_music(mod_root: &P, romfs: &P, c: &Cosmetics, seed: &str) -> Res<Vec<String>> {
    let mut notes: Vec<String> = Vec::new();
    if c.music == Music::Off {
        return Ok(notes);
    }
    let mut rf: Romfs = Romfs::open(romfs)?;
    let mut y: Byml = rf.read_byml(BGM_FILE)?;
    let top: PB = rf.root.join(STREAM);
    let have = |k: &str| top.join(format!("{k}.bwav")).is_file();
    let text: &str = if c.seed.trim().is_empty() { seed } else { c.seed.trim() };
    let n: usize = shuffle_music(&mut y, c.music, text, &have);
    if n == 0 {
        return Err(E::Format("the music table has no track this shuffle can move"));
    }
    let old_len: u64 = rf.read_zs(BGM_FILE)?.len() as u64;
    let raw: Vec<u8> = y.write()?;
    let z: Vec<u8> = zs::compress_raw(&raw, Some(BGM_DICT))?;
    let out: PB = mod_root.join("romfs").join(BGM_FILE);
    if let Some(d) = out.parent() {
        fs::create_dir_all(d)?;
    }
    fs::write(&out, z)?;
    if raw.len() as u64 > old_len {
        let tp: PB = mod_root.join("romfs").join(rsizetable::TABLE_PATH);
        let mut t: Rst = if tp.is_file() {
            Rst::parse(&rf.zs.decompress(&fs::read(&tp)?)?)?
        } else {
            rsizetable::load(&mut rf)?
        };
        t.grow(BGM_FILE, old_len, raw.len() as u64, 0x400)?;
        if let Some(d) = tp.parent() {
            fs::create_dir_all(d)?;
        }
        fs::write(&tp, t.to_zs()?)?;
    }
    notes.push(format!(
        "Music is shuffled {}. {n} music tracks moved. Cutscene music keeps its own tracks. Themes built from stacked layers trade whole layer sets. Area themes that change with the time of day play one track all day.",
        if c.music == Music::Grouped { "within each kind of track" } else { "across all tracks" }
    ));
    Ok(notes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(name: &str, key: &str) -> (String, Node) {
        let asset: Node = Node::Map(vec![("Key".to_string(), Node::Str(key.to_string())), ("Type".to_string(), Node::Str("Asset".to_string()))]);
        let start: Node = Node::Map(vec![("Type".to_string(), Node::Str("Start".to_string()))]);
        (name.to_string(), Node::Map(vec![("Node".to_string(), Node::Arr(vec![start, asset]))]))
    }

    fn table(rows: &[(&str, &str)]) -> Byml {
        let all: Vec<(String, Node)> = rows.iter().map(|r| ev(r.0, r.1)).collect();
        Byml { root: Node::Map(vec![("BGM".to_string(), Node::Map(all))]), version: 7, big: false }
    }

    fn keys(y: &Byml) -> BM<String, String> {
        let mut out: BM<String, String> = BM::new();
        if let Some(Node::Map(all)) = y.root.get("BGM") {
            for (n, e) in all {
                out.insert(n.clone(), asset_key(e, &|_| true).unwrap());
            }
        }
        out
    }

    const ROWS: [(&str, &str); 8] = [
        ("BGM_ForestTemple", "BGM_ForestTemple"),
        ("BGM_ShadowTemple", "BGM_ShadowTemple"),
        ("BGM_WaterTemple", "BGM_WaterTemple"),
        ("BGM_GoronCity", "BGM_GoronCity"),
        ("BGM_EnemyBattle", "BGM_EnemyBattle"),
        ("BGM_MiddleBoss", "BGM_MiddleBoss"),
        ("BGM_MedalGet", "BGM_MedalGet"),
        ("BGM_GameOver", "BGM_GameOver"),
    ];

    #[test]
    fn grouped_music_only_trades_tracks_of_the_same_kind() {
        let mut y: Byml = table(&ROWS);
        let n: usize = (0..40).map(|i| shuffle_music(&mut y, Music::Grouped, &format!("s{i}"), &|_| true)).find(|n| *n > 0).unwrap_or(0);
        assert!(n > 0);
        let k: BM<String, String> = keys(&y);
        for jingle in ["BGM_MedalGet", "BGM_GameOver"] {
            assert!(k[jingle] == "BGM_MedalGet" || k[jingle] == "BGM_GameOver", "{jingle}");
        }
        assert_eq!(k["BGM_EnemyBattle"], "BGM_EnemyBattle");
        assert_eq!(k["BGM_MiddleBoss"], "BGM_MiddleBoss");
        let mut themes: Vec<&String> = ["BGM_ForestTemple", "BGM_ShadowTemple", "BGM_WaterTemple", "BGM_GoronCity"].iter().map(|n| &k[*n]).collect();
        themes.sort();
        assert_eq!(themes, vec!["BGM_ForestTemple", "BGM_GoronCity", "BGM_ShadowTemple", "BGM_WaterTemple"]);
    }

    #[test]
    fn anywhere_music_mixes_boss_battle_and_themes_but_not_jingles() {
        let mut moved: bool = false;
        for s in 0..20 {
            let mut y: Byml = table(&ROWS);
            shuffle_music(&mut y, Music::Anywhere, &format!("seed {s}"), &|_| true);
            let k: BM<String, String> = keys(&y);
            assert!(k["BGM_MedalGet"] == "BGM_MedalGet" || k["BGM_MedalGet"] == "BGM_GameOver");
            moved |= k["BGM_EnemyBattle"] != "BGM_EnemyBattle";
        }
        assert!(moved);
    }

    #[test]
    fn the_same_seed_gives_the_same_music_and_off_changes_nothing() {
        let mut a: Byml = table(&ROWS);
        let mut b: Byml = table(&ROWS);
        shuffle_music(&mut a, Music::Grouped, "same", &|_| true);
        shuffle_music(&mut b, Music::Grouped, "same", &|_| true);
        assert_eq!(a, b);
        let mut c: Byml = table(&ROWS);
        assert_eq!(shuffle_music(&mut c, Music::Off, "same", &|_| true), 0);
        assert_eq!(c, table(&ROWS));
    }

    #[test]
    fn cutscene_events_and_tracks_without_a_stream_file_are_left_alone() {
        let rows: [(&str, &str); 4] = [("BGM_Dm_HrlField_0030", "BGM_Kaepora"), ("BGM_ForestTemple", "BGM_ForestTemple"), ("BGM_ShadowTemple", "BGM_ShadowTemple"), ("BGM_WaterTemple", "BGM_WaterTemple")];
        let mut y: Byml = table(&rows);
        shuffle_music(&mut y, Music::Grouped, "s", &|k| k != "BGM_WaterTemple");
        let k: BM<String, String> = keys(&y);
        assert_eq!(k["BGM_Dm_HrlField_0030"], "BGM_Kaepora");
        assert_eq!(k["BGM_WaterTemple"], "BGM_WaterTemple");
    }

    #[test]
    fn events_that_share_a_track_keep_sharing_it() {
        let rows: [(&str, &str); 4] = [("BGM_DungeonBoss", "BGM_DungeonBoss"), ("BGM_DungeonBoss_KotakeKoume", "BGM_DungeonBoss"), ("BGM_MiddleBoss", "BGM_MiddleBoss"), ("BGM_DungeonBoss_Dinasour", "BGM_DungeonBoss_Dinasour")];
        for s in 0..10 {
            let mut y: Byml = table(&rows);
            shuffle_music(&mut y, Music::Grouped, &format!("s{s}"), &|_| true);
            let k: BM<String, String> = keys(&y);
            assert_eq!(k["BGM_DungeonBoss"], k["BGM_DungeonBoss_KotakeKoume"]);
        }
    }

    #[test]
    fn the_modes_parse_and_round_trip_their_ids() {
        for m in [Music::Off, Music::Grouped, Music::Anywhere] {
            assert_eq!(Music::parse(m.id()), Some(m));
        }
        assert_eq!(Music::parse("loud"), None);
        assert!(!Cosmetics::default().any());
    }

    fn real() -> Option<PB> {
        let p: PB = std::env::var("OOTRR_ROMFS").map(PB::from).ok()?;
        p.join("Pack").join("ZsDic.pack.zs").is_file().then_some(p)
    }

    #[test]
    fn the_real_music_table_shuffles_and_the_written_file_reads_back() {
        let Some(root) = real() else {
            return;
        };
        let out: PB = std::env::temp_dir().join(format!("ootrr_music_{}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        let c: Cosmetics = Cosmetics { music: Music::Grouped, sfx: Sfx::Off, colors: Colors::Off, seed: String::new() };
        let notes: Vec<String> = apply(&out, &root, &c, "cos seed").unwrap();
        assert!(notes[0].contains("music tracks moved"), "{notes:?}");
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let old: Byml = rf.read_byml(BGM_FILE).unwrap();
        let bytes: Vec<u8> = rf.zs.decompress(&fs::read(out.join("romfs").join(BGM_FILE)).unwrap()).unwrap();
        let new: Byml = Byml::parse(&bytes).unwrap();
        let (a, b): (BM<String, String>, BM<String, String>) = (keys_any(&old), keys_any(&new));
        assert_eq!(a.len(), b.len());
        assert!(a.iter().filter(|(k, v)| b[*k] != **v).count() >= 5);
        let mut x: Vec<&String> = a.values().collect();
        let mut z: Vec<&String> = b.values().collect();
        x.sort();
        z.sort();
        assert_eq!(x.len(), z.len());
        assert_eq!(a.iter().filter(|(k, _)| k.starts_with("BGM_Dm_")).count(), b.iter().filter(|(k, v)| k.starts_with("BGM_Dm_") && a[*k] == **v).count());
        let _ = fs::remove_dir_all(&out);
    }

    fn keys_any(y: &Byml) -> BM<String, String> {
        let mut out: BM<String, String> = BM::new();
        if let Some(Node::Map(all)) = y.root.get("BGM") {
            for (n, e) in all {
                if let Some(k) = asset_key(e, &|_| true) {
                    out.insert(n.clone(), k);
                }
            }
        }
        out
    }

    fn layered_ev(name: &str, keys: &[&str]) -> (String, Node) {
        let mut nodes: Vec<Node> = vec![Node::Map(vec![("Type".to_string(), Node::Str("Fork".to_string()))])];
        for k in keys {
            nodes.push(Node::Map(vec![("Key".to_string(), Node::Str(k.to_string())), ("Type".to_string(), Node::Str("Asset".to_string()))]));
        }
        (name.to_string(), Node::Map(vec![("Node".to_string(), Node::Arr(nodes))]))
    }

    #[test]
    fn layered_themes_trade_whole_stem_sets() {
        let fresh = || {
            let all: Vec<(String, Node)> = vec![layered_ev("BGM_A", &["a1", "a2", "a3"]), layered_ev("BGM_B", &["b1", "b2"])];
            Byml { root: Node::Map(vec![("BGM".to_string(), Node::Map(all))]), version: 7, big: false }
        };
        let moved = (0..40).find_map(|i| {
            let mut y: Byml = fresh();
            (shuffle_music(&mut y, Music::Grouped, &format!("s{i}"), &|_| true) > 0).then_some(y)
        });
        let y: Byml = moved.unwrap();
        let a: Vec<String> = stems(y.root.get("BGM").unwrap().get("BGM_A").unwrap(), &|_| true).unwrap();
        assert!(a.iter().all(|k| k.starts_with('b')));
    }
}
