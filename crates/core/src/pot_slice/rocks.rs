use std::path::Path as P;

use crate::catalog::{Catalog, Loc};
use crate::error::{Error as E, Res};
use crate::json::{self, Json};
use crate::logic_json::Json as Lj;
use crate::pot_slice::data::Kind;
use crate::pot_slice::scan::{finish, judge_all, PotScan};
use crate::romfs::Romfs;
use crate::slice::data::Data;

pub const CATALOG_JSON: &str = include_str!("../../../../data/rocks_catalog.json");
pub const SCAN_JSON: &str = include_str!("../../../../data/rocks_scan.json");
pub const KEY: &str = "rock_spots";
pub const ON: &str = "on";
pub const ROCK: &str = "Obj_LiftRock_A_01";
pub const GRASS: &str = "Obj_Plant_Grass_A_01";
pub const TABLE: &str = "Drop__DropTable";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spot {
    pub name: String,
    pub region: String,
    pub rule: String,
}

pub fn locs() -> Res<Vec<Loc>> {
    let c: Catalog = Catalog::parse(CATALOG_JSON)?;
    for l in &c.locs {
        if !Kind::of(&l.kind).is_some_and(Kind::wild) {
            return Err(E::Msg(format!("rock catalog: {} has the kind {}, which is not rock or grass", l.name, l.kind)));
        }
        if l.refs.len() != 1 || l.refs.iter().any(|r| r.files.len() != 1) {
            return Err(E::Msg(format!("rock catalog: {} must own exactly one actor in one file", l.name)));
        }
    }
    Ok(c.locs)
}

pub fn rule_of(k: Kind, age: &str) -> String {
    let who: &str = match age {
        "adult" => "is_adult",
        "child" => "is_child",
        _ => "",
    };
    match (k, who) {
        (Kind::Grass, "is_adult") => who.to_string(),
        (Kind::Grass, "") => "can_cut_shrubs".to_string(),
        (Kind::Grass, w) => format!("{w} and can_cut_shrubs"),
        (_, "") => "True".to_string(),
        (_, w) => w.to_string(),
    }
}

fn text<'a>(j: &'a Json, k: &str, at: &str) -> Res<&'a str> {
    j.get(k).and_then(|v| v.as_str()).ok_or_else(|| E::Msg(format!("rock catalog: {at} has no {k}")))
}

pub fn spots_of(t: &str) -> Res<Vec<Spot>> {
    let j: Json = json::parse(t)?;
    let rows: &[Json] = j.get("locations").and_then(|v| v.as_arr()).ok_or(E::Format("rock catalog: no locations"))?;
    let mut out: Vec<Spot> = Vec::with_capacity(rows.len());
    for r in rows {
        let name: &str = text(r, "name", "a row")?;
        let k: Kind = Kind::of(text(r, "kind", name)?).filter(|k| k.wild()).ok_or_else(|| E::Msg(format!("rock catalog: {name} is not rock or grass")))?;
        out.push(Spot { name: name.to_string(), region: text(r, "region", name)?.to_string(), rule: rule_of(k, text(r, "age", name)?) });
    }
    Ok(out)
}

pub fn spots() -> Res<Vec<Spot>> {
    spots_of(CATALOG_JSON)
}

pub fn wanted(over: &[(&str, &str)]) -> bool {
    over.iter().any(|(k, v)| *k == KEY && *v == ON)
}

fn named(fields: &[(String, Lj)], region: &str) -> bool {
    fields.iter().any(|(k, v)| k == "region_name" && v.as_str() == Some(region))
}

fn put(files: &mut [Lj], s: &Spot) -> Res<usize> {
    let mut n: usize = 0;
    for f in files.iter_mut() {
        let Lj::Arr(regions) = f else {
            continue;
        };
        for reg in regions.iter_mut() {
            let Lj::Obj(fields) = reg else {
                continue;
            };
            if !named(fields, &s.region) {
                continue;
            }
            let row: (String, Lj) = (s.name.clone(), Lj::Str(s.rule.clone()));
            match fields.iter_mut().find(|(k, _)| k == "locations") {
                Some((_, Lj::Obj(list))) => list.push(row),
                Some(_) => return Err(E::Msg(format!("rock spots: the locations of {} are not an object", s.region))),
                None => fields.push(("locations".to_string(), Lj::Obj(vec![row]))),
            }
            n += 1;
        }
    }
    Ok(n)
}

pub fn apply_with(files: &mut [Lj], spots: &[Spot]) -> Res<usize> {
    for s in spots {
        match put(files, s)? {
            1 => {}
            n => return Err(E::Msg(format!("rock spots: {} found {n} regions named {} instead of one", s.name, s.region))),
        }
    }
    Ok(spots.len())
}

pub fn apply(files: &mut [Lj]) -> Res<usize> {
    apply_with(files, &spots()?)
}

pub fn scan(root: &P) -> Res<PotScan> {
    let d: Data = Data::load()?;
    let ls: Vec<Loc> = locs()?;
    let mut rf: Romfs = Romfs::open(root)?;
    let rows: Vec<(String, crate::slice::data::State)> = judge_all(&d, &mut rf, &ls)?;
    finish(&rf, &ls, rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap as BM, HashSet as HS};

    use crate::logic_items::ItemId;
    use crate::pot_slice::data::PotData;
    use crate::search::{explore, Search};
    use crate::settings::{Settings, Value};
    use crate::slice::data::State;
    use crate::slice::fill::vanilla_place;
    use crate::slice::generate::{allowed, generate_data, GenOpts, Generated};
    use crate::slice::logic::{world, world_with};
    use crate::slice::support::{plan, Plan};
    use crate::world::{LocId, Placement, World};

    fn with(pairs: &[(&str, &str)]) -> Settings {
        let mut s: Settings = Settings::default();
        for (k, v) in pairs {
            s.set(k, Value::s(v)).unwrap();
        }
        s
    }

    fn on_world(d: &Data) -> World {
        world_with(d, &[(KEY, ON)]).unwrap()
    }

    #[test]
    fn rules_follow_the_kind_and_the_age() {
        assert_eq!(rule_of(Kind::Rock, "child"), "is_child");
        assert_eq!(rule_of(Kind::Rock, "adult"), "is_adult");
        assert_eq!(rule_of(Kind::Rock, "both"), "True");
        assert_eq!(rule_of(Kind::Grass, "child"), "is_child and can_cut_shrubs");
        assert_eq!(rule_of(Kind::Grass, "adult"), "is_adult");
        assert_eq!(rule_of(Kind::Grass, "both"), "can_cut_shrubs");
    }

    #[test]
    fn a_spot_must_land_in_exactly_one_region() {
        let mut f: Vec<Lj> = vec![crate::logic_json::parse("[{\"region_name\":\"R\",\"locations\":{\"A\":\"True\"}},{\"region_name\":\"S\"}]").unwrap()];
        let a: Spot = Spot { name: "R Rock 1".to_string(), region: "R".to_string(), rule: "is_child".to_string() };
        let b: Spot = Spot { name: "S Grass 1".to_string(), region: "S".to_string(), rule: "can_cut_shrubs".to_string() };
        assert_eq!(apply_with(&mut f, &[a, b]).unwrap(), 2);
        let want: Lj = crate::logic_json::parse("[{\"region_name\":\"R\",\"locations\":{\"A\":\"True\",\"R Rock 1\":\"is_child\"}},{\"region_name\":\"S\",\"locations\":{\"S Grass 1\":\"can_cut_shrubs\"}}]").unwrap();
        assert_eq!(f[0], want);
        let gone: Spot = Spot { name: "X".to_string(), region: "Nowhere".to_string(), rule: "True".to_string() };
        assert!(apply_with(&mut f, &[gone]).is_err());
    }

    #[test]
    fn the_catalog_counts_are_pinned() {
        let ls: Vec<Loc> = locs().unwrap();
        let mut n: BM<(String, String, String), usize> = BM::new();
        for l in &ls {
            *n.entry((l.kind.clone(), l.area.clone(), l.age.clone())).or_insert(0) += 1;
        }
        let rock = |a: &str, g: &str| n.get(&("rock".to_string(), a.to_string(), g.to_string())).copied().unwrap_or(0);
        let grass = |a: &str, g: &str| n.get(&("grass".to_string(), a.to_string(), g.to_string())).copied().unwrap_or(0);
        assert_eq!(ls.iter().filter(|l| l.kind == "rock").count(), 146);
        assert_eq!(ls.iter().filter(|l| l.kind == "grass").count(), 92);
        assert_eq!((rock("Kokiri Forest", "child"), rock("Kokiri Forest", "adult")), (19, 18));
        assert_eq!((rock("Death Mountain Crater", "child"), rock("Death Mountain Crater", "adult")), (14, 17));
        assert_eq!((rock("Death Mountain Trail", "child"), rock("Death Mountain Trail", "adult")), (6, 5));
        assert_eq!((rock("Desert Colossus", "child"), rock("Desert Colossus", "adult")), (13, 13));
        assert_eq!((rock("Gerudo Valley", "child"), rock("Gerudo Valley", "adult")), (3, 7));
        assert_eq!((rock("Hyrule Field", "child"), rock("Hyrule Field", "adult")), (9, 6));
        assert_eq!((rock("Zora River", "child"), rock("Zora River", "adult")), (8, 8));
        assert_eq!((grass("Deku Tree", "both"), grass("Bottom of the Well", "both")), (18, 9));
        assert_eq!((grass("Hyrule Field", "child"), grass("Hyrule Field", "adult")), (11, 18));
        assert_eq!((grass("Kokiri Forest", "child"), grass("Sacred Forest Meadow", "adult")), (8, 7));
        assert!(ls.iter().all(|l| l.ev == ["census"] && l.conf == crate::catalog::Conf::Forced));
        let hashes: HS<u64> = ls.iter().map(|l| l.refs[0].hash).collect();
        assert_eq!(hashes.len(), ls.len());
    }

    #[test]
    fn every_spot_is_scanned_and_only_the_boss_door_grass_is_held_back() {
        let d: Data = Data::load().unwrap();
        let p: &PotData = &d.pots;
        let ls: Vec<Loc> = locs().unwrap();
        let mut held: Vec<&str> = Vec::new();
        for l in &ls {
            let s: State = p.state_of(&l.name).unwrap_or_else(|| panic!("{} was never scanned", l.name));
            assert!(d.vanilla_of(&l.name).is_some_and(|v| crate::pot_slice::pickup::of(v).is_some()), "{}", l.name);
            if s != State::Eligible {
                held.push(&l.name);
            }
        }
        assert_eq!(held, vec!["Deku Tree Grass 8"]);
        assert!(p.rock_game.starts_with("oot-remake:"));
        assert!(!p.rock_files.is_empty());
    }

    #[test]
    fn the_world_has_no_rock_spot_until_the_key_is_set() {
        let d: Data = Data::load().unwrap();
        let off: World = world(&d).unwrap();
        let on: World = on_world(&d);
        let sp: Vec<Spot> = spots().unwrap();
        assert!(sp.iter().all(|s| off.loc(&s.name).is_none()));
        assert_eq!(on.real.len(), off.real.len() + sp.len());
        for s in &sp {
            let l: LocId = on.loc(&s.name).unwrap_or_else(|| panic!("{}", s.name));
            assert_eq!(on.regions[on.locs[l].region].name, s.region, "{}", s.name);
        }
        assert!(!wanted(&plan(&Settings::default()).unwrap().logic()));
        assert!(wanted(&plan(&with(&[("shuffle_rocks", "all")])).unwrap().logic()));
        assert!(wanted(&plan(&with(&[("shuffle_grass", "dungeons")])).unwrap().logic()));
    }

    #[test]
    fn the_vanilla_world_reaches_every_rock_and_the_age_and_cut_rules_bind() {
        let d: Data = Data::load().unwrap();
        let w: World = on_world(&d);
        let v: Placement = vanilla_place(&w, &d).unwrap();
        let s: Search = explore(&w, &v, &[w.item("Progressive Strength Upgrade").unwrap(); 3]);
        assert!(s.won());
        for sp in spots().unwrap() {
            assert!(s.loc_access(w.loc(&sp.name).unwrap()), "{}", sp.name);
        }
        let mut bare: Placement = v.clone();
        for l in &w.real {
            bare.item[*l] = None;
        }
        let b: Search = explore(&w, &bare, &[]);
        assert!(b.loc_access(w.loc("KF Child Rock 1").unwrap()));
        assert!(!b.loc_access(w.loc("KF Adult Rock 1").unwrap()));
        assert!(!b.loc_access(w.loc("KF Child Grass 1").unwrap()));
        let sword: ItemId = w.item("Kokiri Sword").unwrap();
        let c: Search = explore(&w, &bare, &[sword]);
        assert!(c.loc_access(w.loc("KF Child Grass 1").unwrap()));
    }

    #[test]
    fn rocks_and_grass_take_any_item_with_a_pickup_or_a_kit() {
        let d: Data = Data::load().unwrap();
        let w: World = on_world(&d);
        let p: Plan = plan(&with(&[("shuffle_rocks", "all"), ("shuffle_grass", "all")])).unwrap();
        let ok = |item: &str, loc: &str| allowed(&d, &w, &p, w.item(item).unwrap(), w.loc(loc).unwrap());
        for loc in ["KF Child Rock 1", "Deku Tree Grass 1"] {
            assert!(ok("Rupees (20)", loc), "{loc}");
            assert!(ok("Recovery Heart", loc), "{loc}");
            assert!(ok("Bombs (5)", loc), "{loc}");
            assert!(ok("Progressive Hookshot", loc), "{loc}");
            assert!(ok("Fire Arrows", loc), "{loc}");
            assert!(ok("Gold Skulltula Token", loc), "{loc}");
            assert!(ok("Piece of Heart", loc), "{loc}");
            assert!(ok("Megaton Hammer", loc), "{loc}");
            assert!(ok("Progressive Strength Upgrade", loc), "{loc}");
            assert!(!ok("Small Key (Forest Temple)", loc), "{loc}");
        }
    }

    fn beatable(d: &Data, s: &Settings, g: &Generated) -> bool {
        let p: Plan = plan(s).unwrap();
        let w: World = world_with(d, &p.logic()).unwrap();
        let mut pl: Placement = vanilla_place(&w, d).unwrap();
        for (loc, item) in &g.seed_file.placements {
            pl.put(w.loc(loc).unwrap(), w.item(item).unwrap());
        }
        explore(&w, &pl, &[]).won()
    }

    #[test]
    fn rock_and_grass_seeds_are_beatable_and_keep_to_the_rules() {
        let d: Data = Data::load().unwrap();
        let mut moved: usize = 0;
        for (rocks, grass, pots) in [("all", "all", "off"), ("overworld", "dungeons", "all"), ("off", "all", "off")] {
            let s: Settings = with(&[("shuffle_rocks", rocks), ("shuffle_grass", grass), ("shuffle_pots", pots)]);
            let p: Plan = plan(&s).unwrap();
            let w: World = world_with(&d, &p.logic()).unwrap();
            for n in 0..2 {
                let g: Generated = generate_data(&d, &s, &format!("rocks {rocks} {grass} {n}"), &GenOpts::default()).unwrap();
                assert!(beatable(&d, &s, &g), "{rocks} {grass} {n}");
                let mut here: usize = 0;
                for (loc, item) in &g.seed_file.placements {
                    let Some(k) = d.pots.kind_of(loc).filter(|k| k.wild()) else {
                        continue;
                    };
                    here += 1;
                    assert!(crate::pot_slice::rules::at(&d, loc, item) == Some(true) || d.vanilla_of(loc) == Some(item.as_str()), "{loc} holds {item}");
                    let dungeon: bool = w.regions[w.locs[w.loc(loc).unwrap()].region].dungeon.is_some();
                    let m: &str = if k == Kind::Rock { rocks } else { grass };
                    assert!(m == "all" || (m == "dungeons") == dungeon, "{loc} under {m}");
                    if d.vanilla_of(loc) != Some(item.as_str()) {
                        moved += 1;
                    }
                }
                assert!(here > 5, "{rocks} {grass} {here}");
            }
        }
        assert!(moved > 10, "{moved}");
    }

    #[test]
    fn the_default_settings_never_touch_a_rock() {
        let d: Data = Data::load().unwrap();
        let g: Generated = generate_data(&d, &Settings::default(), "default rocks", &GenOpts::default()).unwrap();
        assert!(g.seed_file.placements.keys().all(|k| !d.pots.kind_of(k).is_some_and(Kind::wild)));
    }

    fn real() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var("OOTRR_ROMFS").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::PathBuf::from("C:/o/nup dump/updates/131072/romfs"));
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Some(p)
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    #[test]
    fn a_real_edit_puts_the_pickup_on_the_actor_and_makes_its_row_permanent() {
        use crate::byml::{Byml, Node};
        use crate::pot_slice::targets::{targets, Give};
        use crate::settings::seed::SeedFile;
        use crate::slice::edit::Edited;
        use crate::staging::Out;
        let Some(root) = real() else {
            return;
        };
        let d: Data = Data::load().unwrap();
        let want: [(&str, &str, &str); 4] = [
            ("KF Child Rock 18", "Rupees (50)", "Obj_Rupee_Purple"),
            ("KF Child Grass 5", "Rupees (20)", "Obj_Rupee_Red"),
            ("DMT Adult Rock 2", "Recovery Heart", "Obj_Heart"),
            ("Colossus Child Rock 10", "Bombs (5)", "Obj_Bomb"),
        ];
        let m: BM<String, String> = want.iter().map(|(l, i, _)| (l.to_string(), i.to_string())).collect();
        let seed: SeedFile = SeedFile::new("s", &Settings::default(), "g", m).unwrap();
        let ts: Vec<Give> = targets(&d, &seed).unwrap();
        assert_eq!(ts.len(), 4);
        assert!(ts.iter().all(|t| t.kind.wild() && !t.kit));
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut edits: Vec<Edited> = Vec::new();
        assert_eq!(crate::pot_slice::apply::apply(&mut rf, &d, &ts, &mut edits).unwrap(), 4);
        for (loc, _, actor) in want {
            let t: &Give = ts.iter().find(|t| t.name == loc).unwrap();
            let e: &Edited = edits.iter().find(|e| e.file == t.files[0]).unwrap();
            let y: Byml = Byml::parse(&e.bytes).unwrap();
            let p: Vec<crate::pot_slice::scan::Peek> = crate::pot_slice::scan::peek(&y, t.hash);
            assert_eq!(p.len(), 1, "{loc}");
            assert_eq!((p[0].drop.as_str(), p[0].table.as_str()), (actor, ""), "{loc}");
        }
        let mut more: Vec<Out> = Vec::new();
        let hs: Vec<u64> = crate::pot_slice::apply::hashes(&ts);
        crate::pot_slice::rows::stage(&mut rf, &mut more, &hs).unwrap();
        let y: Byml = Byml::parse(&rf.zs.decompress(&more[0].z).unwrap()).unwrap();
        let rows: &[Node] = y.root.get("Data").and_then(|x| x.get(crate::pots::TABLE)).and_then(|x| x.as_arr()).unwrap();
        for h in hs {
            let r: &Node = rows.iter().find(|r| crate::placement::hash_of(r) == Some(h)).unwrap();
            assert!(crate::pots::permanent(crate::pots::saved(r)), "{h}");
            assert!(r.get("ExtraByte").is_none(), "{h}");
        }
    }

    #[test]
    fn the_shipped_scan_matches_the_dump() {
        let Some(root) = real() else {
            return;
        };
        let s: PotScan = scan(&root).unwrap();
        let (game, files, rows): (String, Vec<String>, Vec<(String, State)>) = crate::pot_slice::data::parse_scan(SCAN_JSON).unwrap();
        assert_eq!((s.game, s.files, s.rows), (game, files, rows));
    }
}
