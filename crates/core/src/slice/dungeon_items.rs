use std::collections::BTreeMap as BM;

use crate::byml::Byml;
use crate::dungeon_credit::flag;
use crate::dungeon_grant::{mark, struct_of, Grants, Kind, Marks, KINDS};
use crate::error::{Error as E, Res};
use crate::settings::seed::SeedFile;
use crate::slice::data::{Data, Item, State};
use crate::slice::edit::Edited;
use crate::slice::support::{Keep, Plan};
use crate::world::{LocId, World};

pub const BOSS_OK: [&str; 7] = ["remove", "vanilla", "dungeon", "regional", "overworld", "any_dungeon", "keysanity"];
pub const MAP_OK: [&str; 7] = ["remove", "startwith", "vanilla", "dungeon", "regional", "any_dungeon", "keysanity"];
pub const GANON_OK: [&str; 7] = BOSS_OK;

pub const BOSS_DUNGEONS: [&str; 5] = ["ForestTemple", "FireTemple", "WaterTemple", "ShadowTemple", "SpiritTemple"];
pub const GANON: &str = "InsideGanonCastle";
pub const MAP_DUNGEONS: [&str; 10] = [
    "InsideTheDekuTree",
    "DodongoCavern",
    "InsideJabuJabuBelly",
    "ForestTemple",
    "FireTemple",
    "WaterTemple",
    "ShadowTemple",
    "SpiritTemple",
    "BottomOfTheWell",
    "IceCavern",
];

pub const GROUPS: [(&str, &[&str]); 6] = [
    ("forest", &["KokiriForest", "LostWood", "SacredForestMeadow", "InsideTheDekuTree", "ForestTemple"]),
    ("mountain", &["DeathMountainTrail", "DeathMountainCrater", "GoronCity", "DodongoCavern", "FireTemple"]),
    ("water", &["ZoraRiver", "ZoraDomain", "ZoraFountain", "InsideJabuJabuBelly", "IceCavern", "WaterTemple"]),
    ("kakariko", &["KakarikoVillage", "Graveyard", "BottomOfTheWell", "ShadowTemple"]),
    ("gerudo", &["GerudoValley", "GerudoFortress", "HountedWasteland", "DesertColossus", "SpiritTemple", "GerudoTrainingGround"]),
    ("castle", &["HyruleField", "Market", "HyruleCastle", "GanonCastle", "InsideGanonCastle"]),
];

pub const ROAM_BOSS: &str = "Boss Keys can be found outside their own dungeon. The chest sets the boss key of the dungeon it belongs to and says which. Experimental and untested in the game.";
pub const START_BOSS: &str = "Boss Keys are removed and you start with every boss door key, experimental and untested in the game.";
pub const ROAM_MAP: &str = "Maps and Compasses can be found outside their own dungeon. The chest sets the map or compass of the dungeon it belongs to and says which. Experimental and untested in the game.";
pub const START_MAP: &str = "You start with every Map and Compass. The Map and Compass items left in the world do nothing, experimental and untested in the game.";

pub fn keep_value(v: &str) -> Keep {
    match v {
        "vanilla" => Keep::Vanilla,
        "dungeon" => Keep::Dungeon,
        "any_dungeon" => Keep::AnyDungeon,
        "keysanity" => Keep::Anywhere,
        "overworld" => Keep::Overworld,
        "regional" => Keep::Regional,
        _ => Keep::Start,
    }
}

pub fn listed(ok: &[&str]) -> String {
    match ok.split_last() {
        Some((l, rest)) if !rest.is_empty() => format!("{} and {l}", rest.join(", ")),
        Some((l, _)) => l.to_string(),
        None => String::new(),
    }
}

pub fn keep_of(p: &Plan, kind: &str) -> Option<Keep> {
    match kind {
        "BossKey" => Some(p.boss),
        "GanonBossKey" => Some(p.ganon),
        "Map" | "Compass" => Some(p.map),
        _ => None,
    }
}

pub fn free(k: Keep) -> bool {
    !matches!(k, Keep::Vanilla | Keep::Dungeon)
}

pub fn credited(k: Keep) -> bool {
    !matches!(k, Keep::Vanilla)
}

pub fn handles(p: &Plan, kind: &str) -> bool {
    keep_of(p, kind).is_some_and(free)
}

pub fn exempt(p: &Plan, kind: &str) -> bool {
    (p.keys_roam() && kind == "SmallKey") || handles(p, kind)
}

pub fn group(folder: &str) -> Option<&'static str> {
    GROUPS.iter().find(|(_, f)| f.contains(&folder)).map(|(g, _)| *g)
}

fn near(d: &Data, i: &Item, name: &str) -> bool {
    let home: Option<&str> = i.dungeon.as_deref().and_then(struct_of).and_then(group);
    let at: Option<&str> = d.cat.by_name(name).map(|c| c.folder.as_str()).and_then(group);
    home.is_some() && home == at
}

pub fn allowed(d: &Data, w: &World, p: &Plan, i: &Item, l: LocId) -> bool {
    let Some(k) = keep_of(p, &i.kind) else {
        return false;
    };
    let name: &str = &w.locs[l].name;
    let here: Option<&str> = w.regions[w.locs[l].region].dungeon.as_deref();
    let spot: bool = match k {
        Keep::AnyDungeon => here.is_some(),
        Keep::Overworld => here.is_none(),
        Keep::Regional => here == i.dungeon.as_deref() || near(d, i, name),
        Keep::Anywhere | Keep::Start => true,
        Keep::Vanilla | Keep::Dungeon => false,
    };
    spot
        && i.dungeon.is_some()
        && d.flow.by_name(name).is_none()
        && !d.special(name)
        && chest_ok(d, name)
        && d.cat.by_name(name).is_some_and(|c| c.kind == "chest")
}

pub fn chest_ok(d: &Data, name: &str) -> bool {
    match d.state_of(name) {
        Some(State::Eligible) => !d.keeps_event(name),
        Some(State::Event) => d.event_ok(name) && !d.keeps_event(name),
        _ => false,
    }
}

pub fn blocked(d: &Data, p: &Plan, kind: &str, name: &str) -> bool {
    Kind::of(kind).is_some_and(|k| keeps(p, k).into_iter().any(credited)) && d.keeps_event(name)
}

pub fn start_items(d: &Data, p: &Plan) -> Vec<String> {
    d.items
        .iter()
        .filter(|i| i.dungeon.is_some() && matches!(i.kind.as_str(), "BossKey" | "GanonBossKey") && keep_of(p, &i.kind) == Some(Keep::Start))
        .map(|i| i.name.clone())
        .collect()
}

pub fn start_flags(p: &Plan) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    if p.boss == Keep::Start {
        out.extend(BOSS_DUNGEONS.iter().map(|d| (flag(d, Kind::Boss.member()), true)));
    }
    if p.ganon == Keep::Start {
        out.push((flag(GANON, Kind::Boss.member()), true));
    }
    if p.map == Keep::Start {
        for d in MAP_DUNGEONS {
            out.push((flag(d, Kind::Map.member()), true));
            out.push((flag(d, Kind::Compass.member()), true));
        }
    }
    out
}

pub fn notes(boss: Keep, ganon: Keep, map: Keep) -> Vec<String> {
    let roam = |k: Keep| free(k) && k != Keep::Start;
    let mut out: Vec<String> = Vec::new();
    for (on, t) in [
        (roam(boss) || roam(ganon), ROAM_BOSS),
        (boss == Keep::Start || ganon == Keep::Start, START_BOSS),
        (roam(map), ROAM_MAP),
        (map == Keep::Start, START_MAP),
    ] {
        if on {
            out.push(t.to_string());
        }
    }
    out
}

fn keeps(p: &Plan, k: Kind) -> Vec<Keep> {
    match k {
        Kind::Boss => vec![p.boss, p.ganon],
        Kind::Map | Kind::Compass => vec![p.map],
    }
}

pub fn grants(d: &Data, seed: &SeedFile, p: &Plan) -> Res<Grants> {
    let skip: Vec<Kind> = KINDS.iter().copied().filter(|k| keeps(p, *k).into_iter().any(credited)).collect();
    let sweep: Vec<Kind> = KINDS.iter().copied().filter(|k| keeps(p, *k).into_iter().any(|x| credited(x) && x != Keep::Start)).collect();
    let mut credits: BM<u64, (Kind, &'static str)> = BM::new();
    for (loc, item) in &seed.placements {
        let Some(i) = d.item(item) else {
            continue;
        };
        let Some(k) = Kind::of(&i.kind).filter(|k| sweep.contains(k)) else {
            continue;
        };
        let s: &'static str = i
            .dungeon
            .as_deref()
            .and_then(struct_of)
            .ok_or_else(|| E::Msg(format!("{item} at {loc} belongs to no dungeon this build can credit")))?;
        let l: &crate::catalog::Loc = d
            .cat
            .by_name(loc)
            .filter(|c| c.kind == "chest")
            .ok_or_else(|| E::Msg(format!("{item} at {loc} is not in a chest, this build can only hand out dungeon items from chests")))?;
        for r in &l.refs {
            if d.events.keep.contains(&r.hash) {
                return Err(E::Msg(format!("{item} at {loc} would replace a story event that has to stay")));
            }
            credits.insert(r.hash, (k, s));
        }
    }
    Ok(Grants { skip, sweep, credits })
}

pub fn apply(file: &str, raw: &[u8], prev: Option<Edited>, g: &Grants, m: &mut Marks) -> Res<Option<Edited>> {
    if !g.active() {
        return Ok(prev);
    }
    let mut y: Byml = match &prev {
        Some(e) => Byml::parse(&e.bytes)?,
        None => Byml::parse(raw)?,
    };
    if mark(&mut y, file, g, m)? == 0 {
        return Ok(prev);
    }
    let bytes: Vec<u8> = y.write()?;
    Ok(Some(match prev {
        Some(mut e) => {
            e.bytes = bytes;
            e
        }
        None => Edited {
            file: file.to_string(),
            old_len: raw.len(),
            bytes,
            hits: Vec::new(),
            used: Vec::new(),
            hearts: Vec::new(),
            wraps: Vec::new(),
            touched: Vec::new(),
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::explore;
    use crate::settings::{Settings, Value};
    use crate::slice::fill::vanilla_place;
    use crate::slice::generate::{generate_data, GenOpts, Generated};
    use crate::slice::logic::world_with;
    use crate::slice::support::plan;
    use crate::world::Placement;

    fn with(pairs: &[(&str, &str)]) -> Settings {
        let mut s: Settings = Settings::default();
        for (k, v) in pairs {
            s.set(k, Value::s(v)).unwrap();
        }
        s
    }

    fn full(d: &Data, s: &Settings, g: &Generated) -> (World, Placement) {
        let p: Plan = plan(s).unwrap();
        let mut w: World = world_with(d, &p.logic()).unwrap();
        for n in start_items(d, &p) {
            let i: crate::logic_items::ItemId = w.item(&n).unwrap();
            w.start.push((i, 1));
        }
        let mut pl: Placement = vanilla_place(&w, d).unwrap();
        for (loc, item) in &g.seed_file.placements {
            pl.put(w.loc(loc).unwrap(), w.item(item).unwrap());
        }
        (w, pl)
    }

    fn counts(w: &World, pl: &Placement) -> BM<String, usize> {
        let mut out: BM<String, usize> = BM::new();
        for l in &w.real {
            if let Some(i) = pl.get(*l) {
                *out.entry(w.items.names[i].clone()).or_insert(0) += 1;
            }
        }
        out
    }

    fn dungeon_items(c: &BM<String, usize>) -> BM<String, usize> {
        c.iter()
            .filter(|(k, _)| k.starts_with("Boss Key (") || k.starts_with("Map (") || k.starts_with("Compass (") || k.starts_with("Small Key ("))
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    #[test]
    fn the_new_values_are_accepted_and_the_goal_values_are_refused() {
        for v in BOSS_OK {
            assert!(plan(&with(&[("shuffle_bosskeys", v)])).is_ok(), "{v}");
            assert!(plan(&with(&[("shuffle_ganon_bosskey", v)])).is_ok(), "{v}");
        }
        for v in MAP_OK {
            assert!(plan(&with(&[("shuffle_mapcompass", v)])).is_ok(), "{v}");
        }
        for v in ["stones", "medallions", "dungeons", "tokens", "hearts"] {
            let e: String = plan(&with(&[("shuffle_ganon_bosskey", v)])).err().unwrap().to_string();
            assert!(e.contains(&format!("shuffle_ganon_bosskey is {v}")), "{e}");
        }
        assert!(Settings::default().set("shuffle_bosskeys", Value::s("startwith")).is_err());
        assert!(plan(&with(&[("shuffle_mapcompass", "overworld")])).err().unwrap().to_string().contains("shuffle_mapcompass is overworld"));
        assert_eq!(listed(&["a", "b", "c"]), "a, b and c");
        assert_eq!(listed(&["a"]), "a");
    }

    #[test]
    fn defaults_add_no_flags_or_notes_but_credit_every_boss_key_map_and_compass_chest() {
        let p: Plan = plan(&Settings::default()).unwrap();
        assert!(start_flags(&p).is_empty());
        assert!(notes(p.boss, p.ganon, p.map).is_empty());
        assert!(!handles(&p, "BossKey") && !handles(&p, "Map") && !handles(&p, "GanonBossKey"));
        let d: Data = Data::load().unwrap();
        let g: Generated = generate_data(&d, &Settings::default(), "grant default", &GenOpts::default()).unwrap();
        let gr: Grants = grants(&d, &g.seed_file, &p).unwrap();
        assert!(gr.active());
        assert_eq!(gr.skip.len(), KINDS.len());
        assert_eq!(gr.sweep.len(), KINDS.len());
        assert!(!gr.credits.is_empty());
        assert!(start_items(&d, &p).is_empty());
    }

    #[test]
    fn start_and_remove_set_the_flags_of_every_dungeon() {
        let p: Plan = plan(&with(&[("shuffle_mapcompass", "startwith"), ("shuffle_bosskeys", "remove"), ("shuffle_ganon_bosskey", "remove")])).unwrap();
        let f: Vec<(String, bool)> = start_flags(&p);
        assert_eq!(f.len(), 5 + 1 + 20);
        assert!(f.iter().all(|(_, v)| *v));
        assert!(f.iter().any(|(n, _)| n == "DungeonInfo.InsideGanonCastle.IsGetBossKey"));
        assert!(f.iter().any(|(n, _)| n == "DungeonInfo.IceCavern.IsGetCompass"));
        // 27 dungeon and door flags plus the two forest walls of the open forest default.
        assert!(p.start_flags().len() == 29);
        let d: Data = Data::load().unwrap();
        assert_eq!(start_items(&d, &p).len(), 6);
        let q: Plan = plan(&with(&[("shuffle_mapcompass", "remove")])).unwrap();
        assert_eq!(start_flags(&q).len(), 20);
    }

    #[test]
    fn every_folder_with_a_chest_is_in_one_region_group() {
        let d: Data = Data::load().unwrap();
        for l in d.cat.locs.iter().filter(|l| l.kind == "chest") {
            assert!(group(&l.folder).is_some(), "{} in {}", l.name, l.folder);
        }
        let n: usize = GROUPS.iter().map(|(_, f)| f.len()).sum();
        let mut all: Vec<&str> = GROUPS.iter().flat_map(|(_, f)| f.iter().copied()).collect();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), n);
        for d in MAP_DUNGEONS.iter().chain(BOSS_DUNGEONS.iter()).chain([GANON].iter()) {
            assert!(group(d).is_some(), "{d}");
        }
    }

    fn check(d: &Data, s: &Settings, tag: &str, n: usize) -> usize {
        let p: Plan = plan(s).unwrap();
        let base: Generated = generate_data(d, &Settings::default(), "count base", &GenOpts::default()).unwrap();
        let (bw, bp): (World, Placement) = full(d, &Settings::default(), &base);
        let want: BM<String, usize> = dungeon_items(&counts(&bw, &bp));
        let mut away: usize = 0;
        for k in 0..n {
            let g: Generated = generate_data(d, s, &format!("{tag} {k}"), &GenOpts::default()).unwrap();
            let (w, pl): (World, Placement) = full(d, s, &g);
            assert!(explore(&w, &pl, &[]).won(), "{tag} {k} is not beatable");
            assert_eq!(dungeon_items(&counts(&w, &pl)), want, "{tag} {k}");
            for (loc, item) in &g.seed_file.placements {
                let Some(i) = d.item(item) else {
                    continue;
                };
                if !handles(&p, &i.kind) {
                    continue;
                }
                let l: LocId = w.loc(loc).unwrap();
                assert!(allowed(d, &w, &p, i, l), "{item} at {loc}");
                let here: Option<&str> = w.regions[w.locs[l].region].dungeon.as_deref();
                match keep_of(&p, &i.kind) {
                    Some(Keep::AnyDungeon) => assert!(here.is_some(), "{item} at {loc}"),
                    Some(Keep::Overworld) => assert!(here.is_none(), "{item} at {loc}"),
                    _ => {}
                }
                if here != i.dungeon.as_deref() {
                    away += 1;
                }
            }
            let gr: Grants = grants(d, &g.seed_file, &p).unwrap();
            for (loc, item) in &g.seed_file.placements {
                let Some(i) = d.item(item).filter(|i| Kind::of(&i.kind).is_some_and(|k| gr.sweep.contains(&k))) else {
                    continue;
                };
                for r in &d.cat.by_name(loc).unwrap().refs {
                    assert_eq!(gr.credits.get(&r.hash).map(|c| c.1), i.dungeon.as_deref().and_then(struct_of), "{item} at {loc}");
                }
            }
        }
        away
    }

    #[test]
    fn roaming_boss_keys_stay_one_per_dungeon_and_every_seed_is_beatable() {
        let d: Data = Data::load().unwrap();
        let mut away: usize = 0;
        for v in ["any_dungeon", "keysanity", "regional", "overworld"] {
            away += check(&d, &with(&[("shuffle_bosskeys", v)]), &format!("boss {v}"), 3);
        }
        assert!(away > 4, "{away}");
    }

    #[test]
    fn removed_boss_keys_still_generate() {
        let d: Data = Data::load().unwrap();
        check(&d, &with(&[("shuffle_bosskeys", "remove"), ("shuffle_ganon_bosskey", "remove")]), "boss remove", 3);
    }

    #[test]
    fn ganons_boss_key_can_roam() {
        let d: Data = Data::load().unwrap();
        let mut away: usize = 0;
        for v in ["any_dungeon", "keysanity", "regional", "overworld"] {
            away += check(&d, &with(&[("shuffle_ganon_bosskey", v)]), &format!("ganon {v}"), 3);
        }
        assert!(away > 2, "{away}");
    }

    #[test]
    fn maps_and_compasses_keep_their_counts_in_every_mode() {
        let d: Data = Data::load().unwrap();
        let mut away: usize = 0;
        for v in ["startwith", "remove", "any_dungeon", "keysanity", "regional"] {
            away += check(&d, &with(&[("shuffle_mapcompass", v)]), &format!("map {v}"), 2);
        }
        assert!(away > 4, "{away}");
    }

    #[test]
    fn small_key_counts_hold_with_roaming_boss_keys() {
        let d: Data = Data::load().unwrap();
        for v in ["any_dungeon", "keysanity"] {
            let s: Settings = with(&[("shuffle_smallkeys", v), ("shuffle_bosskeys", v), ("shuffle_ganon_bosskey", v), ("shuffle_mapcompass", v)]);
            check(&d, &s, &format!("all {v}"), 3);
        }
    }

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("C:/o/nup dump/updates/131072/romfs"));
        if p.is_dir() {
            Some(p)
        } else {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            None
        }
    }

    fn actor(y: &Byml, hash: u64) -> Option<&crate::byml::Node> {
        y.root.get("Actors")?.as_arr()?.iter().find(|a| crate::placement::hash_of(a) == Some(hash) && a.get("Dynamic").is_some_and(|d| d.get(crate::dungeon_credit::DROP).is_some()))
    }

    fn dy<'a>(a: &'a crate::byml::Node, k: &str) -> &'a str {
        a.get("Dynamic").and_then(|d| d.get(k)).and_then(|v| v.as_str()).unwrap_or("")
    }

    fn built_and_checked(root: &std::path::Path, s: &Settings, seed: &str, name: &str) -> (usize, usize) {
        use crate::bfevfl::{Bf, Val};
        use crate::progressive::{Size, BIG_FLOW, SETTING_TABLE, SMALL_FLOW};
        let d: Data = Data::load().unwrap();
        let p: Plan = plan(s).unwrap();
        let g: Generated = generate_data(&d, s, seed, &GenOpts::default()).unwrap();
        let gr: Grants = grants(&d, &g.seed_file, &p).unwrap();
        let load: std::path::PathBuf = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/dungeon_items_mods");
        let b: crate::slice::edit::Built = crate::slice::edit::build(&g.seed_file, root, &load, name).unwrap();
        let out: std::path::PathBuf = b.root.join("romfs");
        let mut rf: crate::romfs::Romfs = crate::romfs::Romfs::open(root).unwrap();
        let mut away: usize = 0;
        for (loc, item) in &g.seed_file.placements {
            let Some(i) = d.item(item).filter(|i| Kind::of(&i.kind).is_some_and(|k| gr.sweep.contains(&k))) else {
                continue;
            };
            let k: Kind = Kind::of(&i.kind).unwrap();
            let want: &str = i.dungeon.as_deref().and_then(struct_of).unwrap();
            for r in &d.cat.by_name(loc).unwrap().refs {
                for f in &r.files {
                    let old: Byml = Byml::parse(&rf.read_zs(f).unwrap()).unwrap();
                    let new: Byml = Byml::parse(&rf.zs.decompress(&std::fs::read(out.join(f)).unwrap()).unwrap()).unwrap();
                    let a: &crate::byml::Node = actor(&new, r.hash).unwrap();
                    let v: &crate::byml::Node = actor(&old, r.hash).unwrap();
                    let size: Size = Size::of_gyaml(a.get("Gyaml").and_then(|x| x.as_str()).unwrap_or(""));
                    assert_eq!(dy(a, crate::dungeon_credit::DROP), k.actor(), "{loc}");
                    assert_eq!(dy(a, "GetEventName"), crate::dungeon_grant::event_name(k, want, size), "{loc}");
                    assert_ne!(dy(v, "GetEventName"), dy(a, "GetEventName"), "{loc}");
                    if crate::dungeon_grant::scene_of(f) != Some(want) {
                        away += 1;
                    }
                }
            }
        }
        let mut entries: usize = 0;
        if gr.active() && !gr.sweep.is_empty() {
            let big: Bf = Bf::parse(&rf.zs.decompress(&std::fs::read(out.join(BIG_FLOW)).unwrap()).unwrap()).unwrap();
            let small: Bf = Bf::parse(&rf.zs.decompress(&std::fs::read(out.join(SMALL_FLOW)).unwrap()).unwrap()).unwrap();
            let table: Byml = Byml::parse(&rf.zs.decompress(&std::fs::read(out.join(SETTING_TABLE)).unwrap()).unwrap()).unwrap();
            for (loc, item) in &g.seed_file.placements {
                let Some(i) = d.item(item).filter(|i| Kind::of(&i.kind).is_some_and(|k| gr.sweep.contains(&k))) else {
                    continue;
                };
                let k: Kind = Kind::of(&i.kind).unwrap();
                let want: &str = i.dungeon.as_deref().and_then(struct_of).unwrap();
                for bf in [&big, &small] {
                    let f: &crate::bfevfl::Flow = &bf.flows[0];
                    let e: &crate::bfevfl::Entry = f.entries.iter().find(|e| e.name == crate::dungeon_grant::entry_name(k, want)).unwrap_or_else(|| panic!("{loc} {item}"));
                    let flag_name: Option<&Val> = f.events[e.main as usize].kind.params().and_then(|c| c.get("GameDataName"));
                    assert_eq!(flag_name, Some(&Val::Str(format!("DungeonInfo.{want}.{}", k.member()))));
                    entries += 1;
                }
                for size in [Size::Small, Size::Big] {
                    let id: String = crate::dungeon_grant::event_name(k, want, size);
                    let row: &crate::byml::Node = table.root.as_arr().unwrap().iter().find(|r| r.get("__RowId").and_then(|x| x.as_str()) == Some(id.as_str())).unwrap();
                    assert_eq!(row.get("EntryPointName").and_then(|x| x.as_str()), Some(crate::dungeon_grant::entry_name(k, want).as_str()));
                }
            }
        }
        (away, entries)
    }

    #[test]
    fn built_mods_give_each_roaming_chest_the_key_of_its_own_dungeon() {
        let Some(root) = dump() else {
            return;
        };
        let s: Settings = with(&[("shuffle_smallkeys", "keysanity"), ("shuffle_bosskeys", "keysanity"), ("shuffle_ganon_bosskey", "keysanity"), ("shuffle_mapcompass", "any_dungeon")]);
        let (away, entries): (usize, usize) = built_and_checked(&root, &s, "dungeon items build", "OOTRR Dungeon Items Test");
        assert!(away > 4, "{away}");
        assert!(entries > 10, "{entries}");
        let t: Settings = with(&[("shuffle_bosskeys", "remove"), ("shuffle_ganon_bosskey", "regional"), ("shuffle_mapcompass", "startwith")]);
        built_and_checked(&root, &t, "dungeon items start", "OOTRR Dungeon Items Start Test");
        let o: Settings = with(&[("shuffle_bosskeys", "overworld"), ("shuffle_ganon_bosskey", "overworld"), ("shuffle_mapcompass", "keysanity")]);
        let (out, _): (usize, usize) = built_and_checked(&root, &o, "dungeon items overworld", "OOTRR Dungeon Items Overworld Test");
        assert!(out >= 6, "{out}");
    }

    #[test]
    fn a_default_build_credits_every_boss_key_map_and_compass_chest() {
        let Some(root) = dump() else {
            return;
        };
        let (_, entries): (usize, usize) = built_and_checked(&root, &Settings::default(), "default credit", "OOTRR Default Credit Test");
        assert!(entries > 10, "{entries}");
    }

    #[test]
    fn the_keysanity_preset_generates() {
        let d: Data = Data::load().unwrap();
        let pr = crate::settings::presets::builtin().unwrap().into_iter().find(|p| p.id == "keysanity").unwrap();
        let s: Settings = pr.apply(&Settings::default()).unwrap();
        check(&d, &s, "preset keysanity", 3);
    }
}
