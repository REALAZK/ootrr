use std::collections::HashSet as Set;

use crate::flags::{FlagList, hash_name, hex};
use crate::flags_harvest::Evidence;
use crate::json::Json;

const DUNGEONS: [&str; 14] = [
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

const SONGS: [&str; 13] = [
    "ZeldaLullaby",
    "EponaSong",
    "SariaSong",
    "SunSong",
    "TimeSong",
    "StormSong",
    "ForestMinuet",
    "FireBolero",
    "WaterSerenade",
    "SpiritRequiem",
    "ShadowNocturne",
    "LightPrelude",
    "ScarecrowSong",
];

const LIST_FILE: &str = "gamedata:GameData/GameDataList.Product.110.byml.zs";
const RANK: [&str; 8] = [
    "flow",
    "ainb",
    "param",
    "eventparam",
    "banc",
    "aoc",
    "pack",
    "rsdb",
];

const MEDALS: [&str; 6] = ["Forest", "Fire", "Water", "Shadow", "Spirit", "Light"];
const STONES: [&str; 3] = ["Forest", "Fire", "Water"];
const SAGES: [&str; 5] = ["Forest", "Fire", "Water", "Shadow", "Spirit"];
const BOSSES: [&str; 10] = [
    "Gohma",
    "Dodongo",
    "Barinade",
    "Volvagia",
    "Morpha",
    "BongoBongo",
    "Twinrova",
    "PhantomGanon",
    "Ganondorf",
    "Ganon",
];
const BOTTLES: [&str; 13] = [
    "Bottle",
    "RedPotion",
    "GreenPotion",
    "BluePotion",
    "BlueFire",
    "Fish",
    "Letter",
    "BigPoe",
    "Poe",
    "Bug",
    "Fairy",
    "Milk",
    "HalfMilk",
];
const TRADE: [&str; 15] = [
    "ZeldaLetter",
    "WeirdEgg",
    "AlarmCucco",
    "PocketEgg",
    "PocketCucco",
    "CojiroCucco",
    "OddMushroom",
    "OddPowder",
    "PoacherSaw",
    "BrokenSword",
    "Prescription",
    "EyeballFrog",
    "EyeDrop",
    "ClaimCheck",
    "Bottle_Letter",
];
const GATES: [&str; 9] = [
    "Npc_Hylia_Soldier_003_GateOpen",
    "Npc_Hylia_Soldier_003_GateClose",
    "GerudoFortress_TrainingGroundGateOpend",
    "Gerudo_phantomdesert_gate",
    "GanonCastle_IsOpenMainGate",
    "HrlCastle_GateOpen",
    "IsHyruleFieldBridgeClose",
    "ShadowTemple_Gimmick_Open_MainGate_Adult",
    "ShadowTemple_Gimmick_Open_MainGate_Child",
];
const FOREST: [&str; 5] = [
    "Step_Meet_DekuTree",
    "Meet_DekuTree_IsCompleted_Exp",
    "Step_Get_StoneOfForest",
    "Get_StoneOfForest_IsCompleted_Exp",
    "InsideTheDekuTree_Heart_Container_Obtained",
];

pub struct Concept {
    pub id: String,
    pub what: String,
    pub conf: &'static str,
    pub basis: String,
    pub flags: Vec<String>,
}

fn c(id: &str, what: &str, conf: &'static str, basis: &str, flags: Vec<String>) -> Concept {
    Concept {
        id: id.to_string(),
        what: what.to_string(),
        conf,
        basis: basis.to_string(),
        flags,
    }
}

fn each(fmt: &str, items: &[&str]) -> Vec<String> {
    items.iter().map(|i| fmt.replace("{}", i)).collect()
}

fn own(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

pub fn concepts() -> Vec<Concept> {
    let mut stone_steps: Vec<String> = each("Step_Get_StoneOf{}", &STONES);
    stone_steps.extend(each("Get_StoneOf{}_IsCompleted_Exp", &STONES));
    let mut sage_steps: Vec<String> = each("Step_SageOf{}", &SAGES);
    sage_steps.extend(each("SageOf{}_IsCompleted_Exp", &SAGES));
    sage_steps.push("Step_SageOfLight".to_string());
    let mut boss: Vec<String> = each("HaveDefeatedEnemy_Enemy_DungeonBoss_{}", &BOSSES);
    boss.extend(each("DefeatedEnemyNum.Enemy_DungeonBoss_{}", &BOSSES));
    let mut door: Vec<String> = own(&["Step_DoorOfTime", "DoorOfTime_AfterAwakeInpa"]);
    door.push("DoorOfTime_IsCompleted_Exp".to_string());
    for s in [
        "Ready", "Step01", "Step02", "Step03", "Step04", "Step05", "Step06",
    ] {
        door.push(format!("DoorOfTime_IsAfter_{s}_Exp"));
    }
    door.push("DoorOfTime_IsAfterStep03_AND_SageOfForest_NotCompleted_Exp".to_string());
    let mut bottles: Vec<String> = each("IsGet.Item_Bottle_{}", &BOTTLES[1..]);
    bottles.push("IsGet.Item_Bottle".to_string());
    bottles.extend(each(
        "PouchData.Bottle{}.StockNum",
        &["A", "B", "C", "D", "E"],
    ));
    bottles.push("Shop_SelectBottleContentItem".to_string());
    bottles.push("Shop_SelectBottleAssignableItem".to_string());
    let mut trade: Vec<String> = each("IsGet.Item_{}", &TRADE[..14]);
    trade.extend(each("PouchData.Item_{}.StockNum", &TRADE[..14]));
    trade.push("Step_SubEvent_GetSwordBigGoron".to_string());
    trade.push("GetSwordBigGoron_IsAfterReady_AND_AfterGetBrokenSword".to_string());
    vec![
        c(
            "dungeon_boss_defeated",
            "boss defeated per dungeon",
            "medium",
            "boss kill counters and derived have-defeated expressions keyed by boss actor name",
            boss,
        ),
        c(
            "dungeon_reward_stones",
            "spiritual stone dungeon completion",
            "high",
            "quest step enums end in Complete and the matching IsCompleted expression is a flow query",
            stone_steps,
        ),
        c(
            "dungeon_reward_medallions",
            "temple dungeon completion through the sage quests",
            "high",
            "quest step enums end in Complete and the matching IsCompleted expression is a flow query",
            sage_steps,
        ),
        c(
            "small_key_counts",
            "small keys held per dungeon",
            "high",
            "Int member named SmallKeyNum inside the per dungeon DungeonInfo struct",
            each("DungeonInfo.{}.SmallKeyNum", &DUNGEONS),
        ),
        c(
            "boss_keys",
            "boss key obtained per dungeon",
            "high",
            "Bool member named IsGetBossKey inside DungeonInfo",
            each("DungeonInfo.{}.IsGetBossKey", &DUNGEONS),
        ),
        c(
            "dungeon_maps",
            "map obtained per dungeon",
            "high",
            "Bool member named IsGetMap inside DungeonInfo",
            each("DungeonInfo.{}.IsGetMap", &DUNGEONS),
        ),
        c(
            "dungeon_compasses",
            "compass obtained per dungeon",
            "high",
            "Bool member named IsGetCompass inside DungeonInfo",
            each("DungeonInfo.{}.IsGetCompass", &DUNGEONS),
        ),
        c(
            "songs_known",
            "ocarina songs known",
            "high",
            "IsGet flags per Item_Melody actor, queried by the song teaching flows",
            each("IsGet.Item_Melody_{}", &SONGS),
        ),
        c(
            "medallions",
            "medallions held",
            "high",
            "IsGet flags per Item_Medal actor",
            each("IsGet.Item_Medal_{}", &MEDALS),
        ),
        c(
            "medallion_stock",
            "medallions held as pouch stock",
            "medium",
            "PouchData stock per Item_Medal actor",
            each("PouchData.Item_Medal_{}.StockNum", &MEDALS),
        ),
        c(
            "stones",
            "spiritual stones held",
            "high",
            "IsGet flags per Item_Stone actor",
            each("IsGet.Item_Stone_{}", &STONES),
        ),
        c(
            "stone_stock",
            "spiritual stones held as pouch stock",
            "medium",
            "PouchData stock per Item_Stone actor",
            each("PouchData.Item_Stone_{}.StockNum", &STONES),
        ),
        c(
            "door_of_time",
            "Door of Time opening progress",
            "high",
            "quest step enum named after the door with its IsAfter expressions",
            door,
        ),
        c(
            "forest_openness",
            "Kokiri Forest and Deku Tree progression",
            "low",
            "Deku Tree meeting quest and the forest stone quest, no flag named for the forest exit itself",
            own(&FOREST),
        ),
        c(
            "gate_openness",
            "gates and bridges that open or close",
            "low",
            "names state the gate but the mapping to OoTR gates such as Kakariko is a naming guess",
            own(&GATES),
        ),
        c(
            "bottle_contents",
            "bottle contents and bottle slots",
            "medium",
            "13 content flags match the 13 content choices of the bottle enum, slots are the Bottle A to E pouch entries, the per slot content flag is not found",
            bottles,
        ),
        c(
            "trade_quest_steps",
            "adult trade sequence and the Biggoron quest",
            "high",
            "IsGet and pouch stock per trade item plus the Biggoron quest step enum",
            trade,
        ),
        c(
            "keys_in_minigames",
            "treasure chest minigame small keys",
            "low",
            "MiniGame_SmallKeyNum is a minigame counter and not a dungeon key count",
            own(&["MiniGame_SmallKeyNum"]),
        ),
    ]
}

pub fn wanted() -> Set<String> {
    let mut out: Set<String> = Set::new();
    for k in concepts() {
        for f in k.flags {
            out.insert(f);
        }
    }
    out
}

pub struct Row {
    pub concept: String,
    pub name: String,
    pub hash: u64,
    pub found: bool,
    pub ty: String,
    pub how: String,
    pub files: Vec<String>,
}

fn ranked(v: Option<&Vec<(String, String)>>) -> Vec<String> {
    let mut x: Vec<(usize, String)> = Vec::new();
    for (c, p) in v.map(|v| v.as_slice()).unwrap_or(&[]) {
        let r: usize = RANK.iter().position(|k| k == c).unwrap_or(RANK.len());
        x.push((r, format!("{c}:{p}")));
    }
    x.sort();
    x.into_iter().map(|t| t.1).collect()
}

pub fn rows(l: &FlagList, ev: &Evidence) -> Vec<(Concept, Vec<Row>)> {
    let mut out: Vec<(Concept, Vec<Row>)> = Vec::new();
    for k in concepts() {
        let mut rs: Vec<Row> = Vec::new();
        for n in &k.flags {
            let h: u64 = hash_name(n) as u64;
            let f: Option<&crate::flags::Flag> = l.by_hash(h);
            let mut files: Vec<String> = ranked(ev.get(n));
            if files.is_empty() && f.is_some() {
                files.push(LIST_FILE.to_string());
            }
            rs.push(Row {
                concept: k.id.clone(),
                name: n.clone(),
                hash: h,
                found: f.is_some(),
                ty: f.map(|x| x.ty.clone()).unwrap_or_default(),
                how: f.and_then(|x| x.how.clone()).unwrap_or_default(),
                files,
            });
        }
        out.push((k, rs));
    }
    out
}

pub fn status(rs: &[Row], conf: &str) -> &'static str {
    let n: usize = rs.iter().filter(|r| r.found).count();
    if conf == "low" || n == 0 {
        "GUESS"
    } else if n == rs.len() {
        "VERIFIED"
    } else {
        "PARTIAL"
    }
}

pub fn to_json(l: &FlagList, ev: &Evidence) -> Json {
    let mut arr: Vec<Json> = Vec::new();
    for (k, rs) in rows(l, ev) {
        let st: &str = status(&rs, k.conf);
        let found: Vec<Json> = rs
            .iter()
            .filter(|r| r.found)
            .map(|r| Json::Str(hex(r.hash, &r.ty)))
            .collect();
        let missing: Vec<Json> = rs
            .iter()
            .filter(|r| !r.found)
            .map(|r| Json::Str(r.name.clone()))
            .collect();
        let o: Json = Json::obj()
            .with("id", Json::Str(k.id))
            .with("status", Json::Str(st.to_string()))
            .with("confidence", Json::Str(k.conf.to_string()))
            .with("flags", Json::Arr(found))
            .with("missing", Json::Arr(missing));
        arr.push(o);
    }
    Json::Arr(arr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::{Body, Flag};

    fn flag(n: &str) -> Flag {
        Flag {
            hash: hash_name(n) as u64,
            ty: "Int".to_string(),
            def: Json::Int(0),
            reset: 0,
            save: 0,
            extra: None,
            size: None,
            body: Body::Plain,
            name: None,
            how: None,
        }
    }

    #[test]
    fn every_concept_has_flags_and_unique_ids() {
        let k: Vec<Concept> = concepts();
        let mut ids: Set<&str> = Set::new();
        for x in &k {
            assert!(!x.flags.is_empty());
            assert!(ids.insert(x.id.as_str()));
        }
        assert!(wanted().contains("DungeonInfo.ForestTemple.SmallKeyNum"));
    }

    #[test]
    fn found_and_missing_split() {
        let l: FlagList = FlagList::from_flags(vec![flag("DungeonInfo.ForestTemple.SmallKeyNum")]);
        let r: Vec<(Concept, Vec<Row>)> = rows(&l, &Evidence::new());
        let keys: &(Concept, Vec<Row>) = r
            .iter()
            .find(|x| x.0.id == "small_key_counts")
            .unwrap_or(&r[0]);
        assert_eq!(keys.1.iter().filter(|x| x.found).count(), 1);
        assert_eq!(status(&keys.1, "high"), "PARTIAL");
    }
}
