use super::model::Save;
use super::text::hash_of;
use super::val::Val;
use crate::error::{Error, Res};
use crate::flags::hash_name;

pub type Op = (String, Val);

pub const NAMES: [&str; 9] = [
    "songs",
    "stones",
    "dungeons",
    "keys",
    "items",
    "stats",
    "adult",
    "everything",
    "bottles",
];

pub const EVERYTHING: [&str; 7] = [
    "songs", "stones", "dungeons", "keys", "items", "stats", "bottles",
];

pub const SONG_NAMES: [&str; 13] = [
    "ZeldaLullaby",
    "EponaSong",
    "SariaSong",
    "SunSong",
    "TimeSong",
    "StormSong",
    "ForestMinuet",
    "FireBolero",
    "WaterSerenade",
    "ShadowNocturne",
    "SpiritRequiem",
    "LightPrelude",
    "ScarecrowSong",
];

pub const STONE_NAMES: [&str; 3] = ["Forest", "Fire", "Water"];

pub const MEDAL_NAMES: [&str; 6] = ["Forest", "Fire", "Water", "Shadow", "Spirit", "Light"];

pub const BOSS_NAMES: [&str; 8] = [
    "Gohma",
    "Dodongo",
    "Barinade",
    "Volvagia",
    "Morpha",
    "BongoBongo",
    "Twinrova",
    "PhantomGanon",
];

pub const DUNGEON_NAMES: [&str; 14] = [
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
    "GerudoTrainingGround",
    "InsideGanonCastle",
    "GerudoFortress",
    "FinalBattleField",
];

const KEYS: [(&str, i32); 9] = [
    ("ForestTemple", 5),
    ("FireTemple", 8),
    ("WaterTemple", 6),
    ("ShadowTemple", 5),
    ("SpiritTemple", 5),
    ("BottomOfTheWell", 3),
    ("GerudoTrainingGround", 9),
    ("GerudoFortress", 4),
    ("InsideGanonCastle", 2),
];

const BOSS_KEYS: [&str; 6] = [
    "ForestTemple",
    "FireTemple",
    "WaterTemple",
    "ShadowTemple",
    "SpiritTemple",
    "InsideGanonCastle",
];

const MAPPED: [&str; 10] = [
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

const GEAR: [&str; 27] = [
    "Weapon_Sword_Kokiri",
    "Weapon_Sword_Master",
    "Weapon_Sword_Biggoron",
    "Weapon_Shield_Deku",
    "Weapon_Shield_Hylian",
    "Weapon_Shield_Mirror",
    "Armor_Adult_Goron",
    "Armor_Adult_Zora",
    "Armor_Adult_HeavyBoot",
    "Armor_Adult_HoverBoot",
    "Armor_Child_Gauntlet_Goron",
    "Armor_Adult_Gauntlet_Silver",
    "Armor_Adult_Gauntlet_Gold",
    "Armor_GlassTruth",
    "Item_Scale_Silver",
    "Item_Scale_Gold",
    "Item_Wallet_Large",
    "Armor_Child_Mask_Keaton",
    "Armor_Child_Mask_Skull",
    "Armor_Child_Mask_Spooky",
    "Armor_Child_Mask_Bunny",
    "Armor_Child_Mask_Goron",
    "Armor_Child_Mask_Zora",
    "Armor_Child_Mask_Gerudo",
    "Armor_Child_Mask_Truth",
    "Item_AgonyStone",
    "Item_FairyBless_MPDouble",
];

const TOOLS: [&str; 14] = [
    "Weapon_Range_Bow",
    "Weapon_Range_SlingShot",
    "Item_Boomerang",
    "Item_Hookshot",
    "Item_Longshot",
    "Item_Hammer",
    "Item_Arrow_Fire",
    "Item_Arrow_Ice",
    "Item_Arrow_Light",
    "Item_DinFire",
    "Item_FaroreWind",
    "Item_NayruLove",
    "Item_Quiver_Large",
    "Item_BulletBag_Large",
];

const AMMO: [(&str, i32); 10] = [
    ("Item_DekuStick", 30),
    ("Item_StickBag_Large", 1),
    ("Item_DekuNut", 40),
    ("Item_NutBag_Large", 1),
    ("Item_Bomb", 40),
    ("Item_BombBag_Large", 1),
    ("Item_Arrow_Normal", 50),
    ("Item_DekuSeed", 50),
    ("Item_Bombchu", 20),
    ("Item_MagicBean", 10),
];

const LOWER_TIERS: [&str; 13] = [
    "Item_FairyBless_MP",
    "Item_Ocarina_Fairy",
    "Item_Quiver_Small",
    "Item_Quiver_Medium",
    "Item_BulletBag_Small",
    "Item_BulletBag_Medium",
    "Item_BombBag_Small",
    "Item_BombBag_Medium",
    "Item_NutBag_Small",
    "Item_NutBag_Medium",
    "Item_StickBag_Small",
    "Item_StickBag_Medium",
    "Item_Wallet_Small",
];

const MID_WALLET: &str = "Item_Wallet_Medium";

const MAX_STOCK: [(&str, i32); 6] = [
    ("0xe753508b", 40),
    ("0x735b00f7", 40),
    ("0x457616b0", 30),
    ("0xae8cc742", 50),
    ("0x418db791", 50),
    ("0xe0010a50", 1000),
];

const BOTTLES: [&str; 4] = ["BottleA", "BottleB", "BottleC", "BottleD"];

fn on(n: String) -> Op {
    (n, Val::Bool(true))
}

fn enm(n: &str, v: &str) -> Op {
    (n.to_string(), Val::Enum(hash_name(v)))
}

fn int(n: &str, v: i32) -> Op {
    (n.to_string(), Val::Int(v))
}

fn got(a: &str, n: i32, out: &mut Vec<Op>) {
    out.push(on(format!("IsGet.{a}")));
    out.push(int(&format!("PouchData.{a}.StockNum"), n));
}

fn songs() -> Vec<Op> {
    let mut out: Vec<Op> = Vec::new();
    got("Item_Ocarina_Time", 1, &mut out);
    out.push(on("HavePreparedToPlayOcarina".to_string()));
    for s in SONG_NAMES {
        out.push(on(format!("IsGet.Item_Melody_{s}")));
    }
    out
}

fn stones() -> Vec<Op> {
    let mut out: Vec<Op> = Vec::new();
    for s in STONE_NAMES {
        got(&format!("Item_Stone_{s}"), 1, &mut out);
    }
    for m in MEDAL_NAMES {
        got(&format!("Item_Medal_{m}"), 1, &mut out);
    }
    out
}

fn dungeons() -> Vec<Op> {
    let mut out: Vec<Op> = Vec::new();
    for b in BOSS_NAMES {
        out.push(int(&format!("DefeatedEnemyNum.Enemy_DungeonBoss_{b}"), 1));
    }
    for s in STONE_NAMES {
        out.push(enm(&format!("Step_Get_StoneOf{s}"), "Complete"));
    }
    for s in MEDAL_NAMES {
        out.push(enm(&format!("Step_SageOf{s}"), "Complete"));
    }
    out
}

fn keys() -> Vec<Op> {
    let mut out: Vec<Op> = Vec::new();
    for (d, n) in KEYS {
        out.push(int(&format!("DungeonInfo.{d}.SmallKeyNum"), n));
    }
    for d in BOSS_KEYS {
        out.push(on(format!("DungeonInfo.{d}.IsGetBossKey")));
    }
    for d in MAPPED {
        out.push(on(format!("DungeonInfo.{d}.IsGetMap")));
        out.push(on(format!("DungeonInfo.{d}.IsGetCompass")));
    }
    out
}

fn items() -> Vec<Op> {
    let mut out: Vec<Op> = Vec::new();
    for g in GEAR.iter().chain(TOOLS.iter()) {
        got(g, 1, &mut out);
    }
    for (a, n) in AMMO {
        got(a, n, &mut out);
    }
    for t in LOWER_TIERS.iter().chain([MID_WALLET].iter()) {
        out.push(on(format!("IsGet.{t}")));
        out.push(int(&format!("PouchData.{t}.StockNum"), 0));
    }
    for (h, n) in MAX_STOCK {
        out.push(int(h, n));
    }
    out.push(int("PlayerStatus.MPMax", 480));
    out.push(int("0x76b18200", 480));
    out.push(enm("PlayerStatus.LiftGrade", "High"));
    out.push(("PlayerStatus.DiveDepth".to_string(), Val::Float(9.0)));
    out
}

fn bottles() -> Vec<Op> {
    let mut out: Vec<Op> = Vec::new();
    out.push(on("IsGet.Item_Bottle".to_string()));
    for b in BOTTLES {
        out.push(int(&format!("PouchData.{b}.StockNum"), 1));
    }
    out
}

fn stats() -> Vec<Op> {
    vec![
        int("PlayerStatus.MaxLife", 80),
        int("PlayerStatus.Life", 80),
        int("PlayerStatus.CurrentRupee", 500),
    ]
}

fn adult() -> Vec<Op> {
    vec![
        ("Sequence_IsChildhood".to_string(), Val::Bool(false)),
        enm("Step_DoorOfTime", "Complete"),
    ]
}

pub fn ops(name: &str) -> Res<Vec<Op>> {
    Ok(match name {
        "songs" => songs(),
        "stones" => stones(),
        "dungeons" => dungeons(),
        "keys" => keys(),
        "items" => items(),
        "bottles" => bottles(),
        "stats" => stats(),
        "adult" => adult(),
        "everything" => {
            let mut out: Vec<Op> = Vec::new();
            for n in EVERYTHING {
                out.extend(ops(n)?);
            }
            out
        }
        _ => return Err(Error::Msg(format!("save: unknown preset {name}"))),
    })
}

pub fn ops_of(names: &[&str]) -> Res<Vec<Op>> {
    let mut out: Vec<Op> = Vec::new();
    for n in names {
        out.extend(ops(n)?);
    }
    Ok(out)
}

#[derive(Debug, Default, PartialEq)]
pub struct Applied {
    pub changed: usize,
    pub same: usize,
    pub missing: Vec<String>,
}

pub fn apply(s: &mut Save, ops: &[Op]) -> Res<Applied> {
    let mut out: Applied = Applied::default();
    for (n, v) in ops {
        let h: u32 = hash_of(n);
        let Some(r) = s.row_mut(h) else {
            out.missing.push(n.clone());
            continue;
        };
        if r.get() == *v {
            out.same += 1;
            continue;
        }
        r.put(v).map_err(|e| Error::Msg(format!("{n}: {e}")))?;
        out.changed += 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::fixture::{flag_list, h, sample};
    use crate::save::read::parse;

    #[test]
    fn every_preset_builds_and_everything_is_the_union() {
        let mut n: usize = 0;
        for p in EVERYTHING {
            n += ops(p).unwrap().len();
        }
        assert_eq!(ops("everything").unwrap().len(), n);
        for p in NAMES {
            assert!(!ops(p).unwrap().is_empty());
        }
        assert!(ops("nothing").is_err());
    }

    #[test]
    fn no_preset_repeats_a_flag() {
        for p in NAMES {
            let o: Vec<Op> = ops(p).unwrap();
            let mut seen: Vec<&str> = o.iter().map(|x| x.0.as_str()).collect();
            seen.sort();
            seen.dedup();
            assert_eq!(seen.len(), o.len(), "{p}");
        }
    }

    #[test]
    fn apply_counts_changes() {
        let mut s: Save = parse(&sample(), &flag_list()).unwrap();
        let o: Vec<Op> = vec![
            ("IsGet.Test_A".to_string(), Val::Bool(true)),
            ("IsGet.Test_B".to_string(), Val::Bool(true)),
            ("Absent.Flag".to_string(), Val::Bool(true)),
            (
                format!("0x{:08x}", h("PouchData.Test_A.StockNum")),
                Val::Int(9),
            ),
        ];
        let r: Applied = apply(&mut s, &o).unwrap();
        assert_eq!(r.changed, 2);
        assert_eq!(r.same, 1);
        assert_eq!(r.missing, vec!["Absent.Flag".to_string()]);
        let again: Applied = apply(&mut s, &o).unwrap();
        assert_eq!(again.changed, 0);
        assert_eq!(again.same, 3);
    }

    #[test]
    fn apply_reports_the_flag_on_a_type_clash() {
        let mut s: Save = parse(&sample(), &flag_list()).unwrap();
        let o: Vec<Op> = vec![("IsGet.Test_A".to_string(), Val::Int(1))];
        let e = apply(&mut s, &o).unwrap_err();
        assert!(e.to_string().contains("IsGet.Test_A"));
    }

    #[test]
    fn placing_the_player_rewrites_the_position_flags() {
        let mut s: Save = parse(&sample(), &flag_list()).unwrap();
        let o: Vec<Op> = crate::save::pos::ops("Market_Adult", [7.0, 8.0, 9.0], 1.0);
        let r: Applied = apply(&mut s, &o).unwrap();
        assert_eq!(r.changed, 3);
        assert_eq!(r.missing.len(), 5);
        assert_eq!(
            s.get(h("Sequence_CurrentBanc")),
            Some(Val::Str("Market_Adult".to_string()))
        );
        assert_eq!(
            s.get(h("PlayerStatus.SavePos")),
            Some(Val::Vec3([7.0, 8.0, 9.0]))
        );
    }
}
