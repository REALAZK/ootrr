use crate::byml::{Byml, Node};
use crate::error::{Error, Res};

pub const DIR: &str = "Drop/DropSubTable/";
pub const FOLIAGE: &str = "Pack/FoliageDropParam.pack.zs";

pub const HEART: u32 = 1872319531;
pub const SHOTS: u32 = 1113099761;
pub const BOMB: u32 = 3237216321;
pub const NUT: u32 = 2211636230;
pub const STICK: u32 = 4192551701;
pub const NEEDED: u32 = 2121460961;
pub const ARROWS: u32 = 798871343;
pub const CHUS: u32 = 2565786591;
pub const AMMO: [u32; 6] = [SHOTS, BOMB, NUT, STICK, ARROWS, CHUS];

pub const BOSS_TABLES: [&str; 5] = ["DropTable_Enemy_RangedWeapon_01", "DropTable_Enemy_RangedWeapon_02", "DropTable_ResourceSupply_FinalBattle", "DropTable_ResourceSupply_Gohma", "Enemy_DungeonBoss_BongoBongo_00"];
pub const BOSS_TABLE_PREFIXES: [&str; 2] = ["Drop_ResourceSupply_BeforeBoss", "DropTable_Ganon_Remedies_"];
pub const PUZZLE_TABLES: [&str; 4] = ["DropTable_RangeWeapon_01", "DropTable_RangeWeapon_02", "DropTable_Obj_Bomb", "DropTable_Obj_DekuStick"];
pub const TABLE_EXT: &str = ".game__drop__DropSubTable.bgyml";

pub const HEART_ACTORS: [&str; 1] = ["Obj_Heart"];
pub const AMMO_ACTORS: [&str; 6] = ["Obj_Arrow", "Obj_Bomb", "Obj_DekuSeed", "Obj_DekuNut", "Obj_DekuStick", "Obj_Bombchu"];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cut {
    pub hearts: bool,
    pub ammo: bool,
}

impl Cut {
    pub fn any(&self) -> bool {
        self.hearts || self.ammo
    }
}

pub fn boss_table(path: &str) -> bool {
    let Some(stem) = path.strip_prefix(DIR).and_then(|p| p.strip_suffix(TABLE_EXT)) else {
        return false;
    };
    BOSS_TABLES.contains(&stem) || BOSS_TABLE_PREFIXES.iter().any(|p| stem.starts_with(p))
}

pub fn puzzle_table(path: &str) -> bool {
    let Some(stem) = path.strip_prefix(DIR).and_then(|p| p.strip_suffix(TABLE_EXT)) else {
        return false;
    };
    PUZZLE_TABLES.contains(&stem)
}

pub fn for_table(path: &str, c: Cut) -> Cut {
    if boss_table(path) || puzzle_table(path) { Cut { ammo: false, ..c } } else { c }
}

pub fn typed_ammo(n: &Node) -> bool {
    actor(n).is_empty() && AMMO.contains(&kind(n))
}

fn actor(n: &Node) -> &str {
    let s: &str = n.get("DropActorName").and_then(|x| x.as_str()).unwrap_or("");
    s.rsplit('/').next().unwrap_or("").split('.').next().unwrap_or("")
}

fn kind(n: &Node) -> u32 {
    match n.get("Type") {
        Some(Node::U32(v)) => *v,
        _ => 0,
    }
}

pub fn cut_item(n: &Node, c: Cut) -> bool {
    let a: &str = actor(n);
    let t: u32 = kind(n);
    let heart: bool = if a.is_empty() { t == HEART } else { HEART_ACTORS.contains(&a) };
    let ammo: bool = if a.is_empty() { AMMO.contains(&t) } else { AMMO_ACTORS.iter().any(|p| a.starts_with(p)) };
    (c.hearts && heart) || (c.ammo && ammo)
}

fn trim_needed(n: &mut Node) -> bool {
    if kind(n) != NEEDED {
        return false;
    }
    let Some(list) = n.get_mut("MostDepletedResource").and_then(|x| x.as_arr_mut()) else {
        return false;
    };
    let was: usize = list.len();
    list.retain(|x| !matches!(x, Node::U32(v) if AMMO.contains(v)));
    list.len() != was
}

pub fn edit(raw: &[u8], c: Cut) -> Res<Option<Vec<u8>>> {
    if !c.any() {
        return Ok(None);
    }
    let mut y: Byml = Byml::parse(raw)?;
    let Some(items) = y.root.get_mut("Items").and_then(|x| x.as_arr_mut()) else {
        return Ok(None);
    };
    let mut hit: bool = false;
    let before: usize = items.len();
    if c.ammo {
        items.retain(|it| !typed_ammo(it));
        hit = items.len() != before;
    }
    for it in items.iter_mut() {
        if cut_item(it, c) {
            if !matches!(it.get("DropProbability"), Some(Node::I32(0))) {
                it.set("DropProbability", Node::I32(0));
                hit = true;
            }
        } else if c.ammo && trim_needed(it) {
            hit = true;
        }
    }
    if !hit {
        return Ok(None);
    }
    let out: Vec<u8> = y.write()?;
    if out.len() > raw.len() {
        return Err(Error::Format("difficulty: a drop table grew"));
    }
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(actor: &str, t: u32, p: i32, needed: &[u32]) -> Node {
        let a: String = if actor.is_empty() { String::new() } else { format!("Work/Actor/{actor}.engine__actor__ActorParam.gyml") };
        Node::Map(vec![
            ("DropActorName".to_string(), Node::Str(a)),
            ("DropProbability".to_string(), Node::I32(p)),
            ("MostDepletedResource".to_string(), Node::Arr(needed.iter().map(|x| Node::U32(*x)).collect())),
            ("Type".to_string(), Node::U32(t)),
        ])
    }

    fn table(items: Vec<Node>) -> Vec<u8> {
        Byml { root: Node::Map(vec![("Items".to_string(), Node::Arr(items))]), version: 7, big: false }.write().unwrap()
    }

    fn odds(raw: &[u8]) -> Vec<i32> {
        let y: Byml = Byml::parse(raw).unwrap();
        y.root.get("Items").unwrap().as_arr().unwrap().iter().map(|i| if let Some(Node::I32(v)) = i.get("DropProbability") { *v } else { -1 }).collect()
    }

    const RUPEE: u32 = 995674644;
    const NAMED: u32 = 126134492;
    const MAGIC: u32 = 832898412;

    #[test]
    fn hearts_and_ammo_are_cut_by_type_and_by_actor() {
        let raw: Vec<u8> = table(vec![
            item("Obj_Rupee_Green", RUPEE, 40, &[]),
            item("", HEART, 10, &[]),
            item("Obj_Heart", NAMED, 10, &[]),
            item("", SHOTS, 10, &[]),
            item("Obj_DekuSeed", NAMED, 10, &[]),
            item("Obj_ArrowBundle_Medium", 798871343, 10, &[]),
            item("", MAGIC, 10, &[]),
        ]);
        assert!(edit(&raw, Cut::default()).unwrap().is_none());
        assert_eq!(odds(&edit(&raw, Cut { hearts: true, ammo: false }).unwrap().unwrap()), vec![40, 0, 0, 10, 10, 10, 10]);
        assert_eq!(odds(&edit(&raw, Cut { hearts: false, ammo: true }).unwrap().unwrap()), vec![40, 10, 10, 0, 0, 10]);
        assert_eq!(odds(&edit(&raw, Cut { hearts: true, ammo: true }).unwrap().unwrap()), vec![40, 0, 0, 0, 0, 10]);
    }

    #[test]
    fn the_most_needed_drop_keeps_only_magic_without_ammo() {
        let raw: Vec<u8> = table(vec![item("", NEEDED, 10, &[MAGIC, SHOTS, NUT, BOMB])]);
        assert!(edit(&raw, Cut { hearts: true, ammo: false }).unwrap().is_none());
        let y: Byml = Byml::parse(&edit(&raw, Cut { hearts: false, ammo: true }).unwrap().unwrap()).unwrap();
        let it: &Node = &y.root.get("Items").unwrap().as_arr().unwrap()[0];
        assert_eq!(it.get("MostDepletedResource"), Some(&Node::Arr(vec![Node::U32(MAGIC)])));
        assert_eq!(it.get("DropProbability"), Some(&Node::I32(10)));
    }

    #[test]
    fn a_table_with_nothing_to_cut_is_left_alone() {
        let raw: Vec<u8> = table(vec![item("Obj_Rupee_Blue", RUPEE, 100, &[]), item("Obj_Spirit_Poe", NAMED, 100, &[]), item("", HEART, 0, &[])]);
        assert!(edit(&raw, Cut { hearts: true, ammo: true }).unwrap().is_none());
    }

    #[test]
    fn boss_and_puzzle_tables_keep_their_ammo_and_every_other_table_loses_it() {
        let dir: &str = "Drop/DropSubTable/";
        let ext: &str = ".game__drop__DropSubTable.bgyml";
        let all: Cut = Cut { hearts: true, ammo: true };
        for t in ["DropTable_Enemy_RangedWeapon_01", "DropTable_Enemy_RangedWeapon_02", "DropTable_ResourceSupply_FinalBattle", "DropTable_ResourceSupply_Gohma", "Enemy_DungeonBoss_BongoBongo_00", "DropTable_Ganon_Remedies_00", "DropTable_Ganon_Remedies_01", "DropTable_Ganon_Remedies_02", "Drop_ResourceSupply_BeforeBoss_NoBomb"] {
            let path: String = format!("{dir}{t}{ext}");
            assert!(boss_table(&path), "{t}");
            assert!(!for_table(&path, all).ammo && for_table(&path, all).hearts, "{t}");
        }
        for t in PUZZLE_TABLES {
            let path: String = format!("{dir}{t}{ext}");
            assert!(puzzle_table(&path) && !boss_table(&path), "{t}");
            assert!(!for_table(&path, all).ammo, "{t}");
        }
        for t in ["DropTable_ResourceSupply_00", "DropTable_Enemy_RangedWeapon_00", "DropTable_Obj_Bomb_01", "DropTable_Obj_DekuNut", "DropTable_Enemy_None", "DropTable_LowLevelEnemy_01", "Obj_Tree_None"] {
            let path: String = format!("{dir}{t}{ext}");
            assert!(!boss_table(&path) && !puzzle_table(&path), "{t}");
            assert!(for_table(&path, all).ammo, "{t}");
        }
        assert!(!boss_table("Life/DropTable_Enemy_RangedWeapon_02.bgyml"));
        let raw: Vec<u8> = table(vec![item("Obj_DekuSeed", NAMED, 80, &[]), item("Obj_Heart", NAMED, 20, &[]), item("", SHOTS, 0, &[])]);
        let kept: Cut = for_table(&format!("{dir}DropTable_Enemy_RangedWeapon_02{ext}"), all);
        assert_eq!(odds(&edit(&raw, kept).unwrap().unwrap()), vec![80, 0, 0]);
    }

    #[test]
    fn ordinary_tables_lose_the_ammo_entries_the_low_supply_boost_feeds() {
        let raw: Vec<u8> = table(vec![item("", SHOTS, 0, &[]), item("", NUT, 0, &[]), item("", BOMB, 30, &[]), item("Obj_DekuSeed", NAMED, 10, &[]), item("", MAGIC, 0, &[])]);
        let out: Vec<u8> = edit(&raw, Cut { hearts: false, ammo: true }).unwrap().unwrap();
        assert_eq!(odds(&out), vec![0, 0]);
        assert!(edit(&out, Cut { hearts: false, ammo: true }).unwrap().is_none());
        let boss: Vec<u8> = table(vec![item("", ARROWS, 0, &[]), item("", MAGIC, 0, &[])]);
        assert!(edit(&boss, for_table(&format!("{DIR}DropTable_Ganon_Remedies_02{TABLE_EXT}"), Cut { hearts: false, ammo: true })).unwrap().is_none());
    }

    #[test]
    fn the_game_keeps_ammo_only_in_the_boss_and_puzzle_tables() {
        let Some(p) = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from) else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: crate::romfs::Romfs = crate::romfs::Romfs::open(&p).unwrap();
        let mut packs: Vec<String> = rf.list("Pack/Actor").unwrap();
        packs.push(FOLIAGE.to_string());
        let ammo_left = |es: &[crate::sarc::Entry], name: &str| -> bool {
            let Some(e) = crate::sarc::get(es, name) else {
                return false;
            };
            let y: Byml = Byml::parse(&e.data).unwrap();
            let Some(items) = y.root.get("Items").and_then(|x| x.as_arr()) else {
                return false;
            };
            items.iter().any(|it| {
                let live: bool = typed_ammo(it) || !matches!(it.get("DropProbability"), Some(Node::I32(0)));
                let listed: bool = kind(it) == NEEDED && it.get("MostDepletedResource").and_then(|x| x.as_arr()).is_some_and(|l| l.iter().any(|v| matches!(v, Node::U32(t) if AMMO.contains(t))));
                (live && cut_item(it, Cut { hearts: false, ammo: true })) || listed
            })
        };
        let mut kept: Vec<String> = Vec::new();
        let mut ordinary: Vec<String> = Vec::new();
        let mut typed: Vec<String> = Vec::new();
        for f in packs.iter().filter(|f| f.ends_with(".pack.zs")) {
            let raw: Vec<u8> = rf.zs.decompress(&rf.read(f).unwrap()).unwrap();
            let mut es: Vec<crate::sarc::Entry> = crate::sarc::parse(&raw).unwrap();
            for e in es.iter_mut().filter(|e| e.name.starts_with(DIR)) {
                if let Some(d) = edit(&e.data, for_table(&e.name, Cut { hearts: false, ammo: true })).unwrap() {
                    e.data = d;
                }
            }
            for e in es.iter().filter(|e| e.name.starts_with(DIR)) {
                let stem: String = e.name.trim_start_matches(DIR).trim_end_matches(TABLE_EXT).to_string();
                let kept_table: bool = boss_table(&e.name) || puzzle_table(&e.name);
                if ammo_left(&es, &e.name) {
                    if kept_table { kept.push(stem.clone()) } else { ordinary.push(format!("{stem} in {f}")) }
                }
                if !kept_table {
                    let y: Byml = Byml::parse(&e.data).unwrap();
                    if y.root.get("Items").and_then(|x| x.as_arr()).is_some_and(|l| l.iter().any(typed_ammo)) {
                        typed.push(format!("{stem} in {f}"));
                    }
                }
            }
        }
        assert!(ordinary.is_empty(), "ammo still drops from {ordinary:?}");
        assert!(typed.is_empty(), "the low-supply boost would still feed {typed:?}");
        kept.sort();
        kept.dedup();
        for t in ["DropTable_Enemy_RangedWeapon_02", "DropTable_ResourceSupply_FinalBattle", "DropTable_Enemy_RangedWeapon_01", "DropTable_ResourceSupply_Gohma", "Enemy_DungeonBoss_BongoBongo_00", "DropTable_Ganon_Remedies_02", "DropTable_RangeWeapon_01", "DropTable_RangeWeapon_02", "DropTable_Obj_Bomb", "DropTable_Obj_DekuStick"] {
            assert!(kept.contains(&t.to_string()), "{t} keeps its ammo: {kept:?}");
        }
        for t in ["DropTable_ResourceSupply_00", "DropTable_Enemy_RangedWeapon_00", "DropTable_Obj_DekuNut", "DropTable_Enemy_None", "DropTable_LowLevelEnemy_01", "Obj_Tree_None"] {
            assert!(!kept.contains(&t.to_string()), "{t}");
        }
    }
}
