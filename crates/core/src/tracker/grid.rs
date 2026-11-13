use crate::json::Json;
use crate::murmur::murmur3_32;
use crate::tracker::peek::Peek;

pub const MAX_COUNT: u32 = 999;
pub const QUARTERS: u32 = 4;
pub const MAGIC_ONE: u32 = 240;
pub const MAGIC_TWO: u32 = 480;
pub const REWARD_TAGS: [&str; 10] = ["DEKU", "DC", "JABU", "FOREST", "FIRE", "WATER", "SHADOW", "SPIRIT", "FREE", "?"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    Own(&'static [&'static str]),
    Any(&'static [&'static str]),
    Count(&'static str),
    Bag { item: &'static str, bags: &'static [&'static str], ammo: &'static str },
    Flag(&'static str),
    Hearts,
    Magic,
    Keys(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub id: &'static str,
    pub label: &'static str,
    pub tag: &'static str,
    pub sec: &'static str,
    pub group: &'static str,
    pub slot: u8,
    pub rule: Rule,
    pub icons: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sec {
    pub id: &'static str,
    pub title: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct St {
    pub lv: u32,
    pub n: Option<u32>,
    pub part: Option<u32>,
}

pub const SECS: [Sec; 7] = [
    Sec { id: "tools", title: "Tools" },
    Sec { id: "spells", title: "Spells and Ocarina" },
    Sec { id: "bottles", title: "Bottles" },
    Sec { id: "gear", title: "Gear" },
    Sec { id: "hearts", title: "Hearts and Tokens" },
    Sec { id: "songs", title: "Songs" },
    Sec { id: "dungeons", title: "Dungeons" },
];

pub const ROW_SEC: &str = "dungeons";
pub const ROW_SLOTS: u8 = 3;
pub const ROWS: [&str; 12] = ["Deku Tree", "Dodongo", "Jabu-Jabu", "Forest", "Fire", "Water", "Shadow", "Spirit", "Ganon", "Well", "Training", "Free"];

const fn c(id: &'static str, label: &'static str, tag: &'static str, sec: &'static str, group: &'static str, slot: u8, rule: Rule, icons: &'static [&'static str]) -> Cell {
    Cell { id, label, tag, sec, group, slot, rule, icons }
}

const QUIVERS: &[&str] = &["Item_Quiver_Small", "Item_Quiver_Medium", "Item_Quiver_Large"];
const SEED_BAGS: &[&str] = &["Item_BulletBag_Small", "Item_BulletBag_Medium", "Item_BulletBag_Large"];
const BOMB_BAGS: &[&str] = &["Item_BombBag_Small", "Item_BombBag_Medium", "Item_BombBag_Large"];
const WALLETS: &[&str] = &["Item_Wallet_Small", "Item_Wallet_Medium", "Item_Wallet_Large"];
const GLOVES: &[&str] = &["Armor_Child_Gauntlet_Goron", "Armor_Adult_Gauntlet_Silver", "Armor_Adult_Gauntlet_Gold"];
const KEY: &[&str] = &["Item_Dungeon_SmallKey"];
const BOSS: &[&str] = &["Item_Dungeon_BossKey"];
const BOTTLE: &[&str] = &["Item_Bottle"];

pub const CELLS: [Cell; 76] = [
    c("slingshot", "Fairy Slingshot", "", "tools", "", 0, Rule::Bag { item: "Weapon_Range_SlingShot", bags: SEED_BAGS, ammo: "Item_DekuSeed" }, &["Weapon_Range_SlingShot"]),
    c("boomerang", "Boomerang", "", "tools", "", 0, Rule::Own(&["Item_Boomerang"]), &["Item_Boomerang"]),
    c("sticks", "Deku Sticks", "", "tools", "", 0, Rule::Count("Item_DekuStick"), &["Item_DekuStick"]),
    c("nuts", "Deku Nuts", "", "tools", "", 0, Rule::Count("Item_DekuNut"), &["Item_DekuNut"]),
    c("bombs", "Bombs", "", "tools", "", 0, Rule::Bag { item: "Item_Bomb", bags: BOMB_BAGS, ammo: "Item_Bomb" }, &["Item_Bomb"]),
    c("bombchus", "Bombchus", "", "tools", "", 0, Rule::Count("Item_Bombchu"), &["Item_Bombchu"]),
    c("bow", "Fairy Bow", "", "tools", "", 0, Rule::Bag { item: "Weapon_Range_Bow", bags: QUIVERS, ammo: "Item_Arrow_Normal" }, &["Weapon_Range_Bow"]),
    c("fire_arrow", "Fire Arrow", "", "tools", "", 0, Rule::Own(&["Item_Arrow_Fire"]), &["Item_Arrow_Fire_PouchIcon"]),
    c("ice_arrow", "Ice Arrow", "", "tools", "", 0, Rule::Own(&["Item_Arrow_Ice"]), &["Item_Arrow_Ice_PouchIcon"]),
    c("light_arrow", "Light Arrow", "", "tools", "", 0, Rule::Own(&["Item_Arrow_Light"]), &["Item_Arrow_Light_PouchIcon"]),
    c("hookshot", "Hookshot", "", "tools", "", 0, Rule::Own(&["Item_Hookshot", "Item_Longshot"]), &["Item_Hookshot", "Item_Longshot"]),
    c("hammer", "Megaton Hammer", "", "tools", "", 0, Rule::Own(&["Item_Hammer"]), &["Item_Hammer_PouchIcon"]),
    c("lens", "Lens of Truth", "", "tools", "", 0, Rule::Own(&["Armor_GlassTruth"]), &["Armor_GlassTruth"]),
    c("beans", "Magic Beans", "", "tools", "", 0, Rule::Count("Item_MagicBean"), &["Item_MagicBean"]),
    c("ocarina", "Ocarina", "", "spells", "", 0, Rule::Own(&["Item_Ocarina_Fairy", "Item_Ocarina_Time"]), &["Item_Ocarina_Fairy", "Item_Ocarina_Time"]),
    c("magic", "Magic Meter", "", "spells", "", 0, Rule::Magic, &["Item_Bottle_GreenPotion"]),
    c("dins_fire", "Din's Fire", "", "spells", "", 0, Rule::Own(&["Item_DinFire"]), &["Item_DinFire"]),
    c("farores_wind", "Farore's Wind", "", "spells", "", 0, Rule::Own(&["Item_FaroreWind"]), &["Item_FaroreWind"]),
    c("nayrus_love", "Nayru's Love", "", "spells", "", 0, Rule::Own(&["Item_NayruLove"]), &["Item_NayruLove"]),
    c("bottle_1", "Bottle 1", "", "bottles", "", 0, Rule::Own(&["BottleA"]), BOTTLE),
    c("bottle_2", "Bottle 2", "", "bottles", "", 0, Rule::Own(&["BottleB"]), BOTTLE),
    c("bottle_3", "Bottle 3", "", "bottles", "", 0, Rule::Own(&["BottleC"]), BOTTLE),
    c("bottle_4", "Bottle 4", "", "bottles", "", 0, Rule::Own(&["BottleD"]), BOTTLE),
    c("bottle_5", "Bottle 5", "", "bottles", "", 0, Rule::Own(&["BottleE"]), BOTTLE),
    c("kokiri_sword", "Kokiri Sword", "", "gear", "", 0, Rule::Own(&["Weapon_Sword_Kokiri"]), &["Weapon_Sword_Kokiri"]),
    c("master_sword", "Master Sword", "", "gear", "", 0, Rule::Own(&["Weapon_Sword_Master"]), &["Weapon_Sword_Master"]),
    c("biggoron_sword", "Biggoron's Sword", "", "gear", "", 0, Rule::Own(&["Weapon_Sword_Giant", "Weapon_Sword_Biggoron"]), &["Weapon_Sword_Giant", "Weapon_Sword_Biggoron"]),
    c("deku_shield", "Deku Shield", "", "gear", "", 0, Rule::Own(&["Weapon_Shield_Deku"]), &["Weapon_Shield_Deku"]),
    c("hylian_shield", "Hylian Shield", "", "gear", "", 0, Rule::Own(&["Weapon_Shield_Hylian"]), &["Weapon_Shield_Hylian"]),
    c("mirror_shield", "Mirror Shield", "", "gear", "", 0, Rule::Own(&["Weapon_Shield_Mirror"]), &["Weapon_Shield_Mirror"]),
    c("kokiri_tunic", "Kokiri Tunic", "", "gear", "", 0, Rule::Any(&["Obj_Armor_Child_Normal", "Armor_Adult_Normal"]), &["Armor_Adult_Normal"]),
    c("goron_tunic", "Goron Tunic", "", "gear", "", 0, Rule::Own(&["Armor_Adult_Goron"]), &["Armor_Adult_Goron"]),
    c("zora_tunic", "Zora Tunic", "", "gear", "", 0, Rule::Own(&["Armor_Adult_Zora"]), &["Armor_Adult_Zora"]),
    c("iron_boots", "Iron Boots", "", "gear", "", 0, Rule::Own(&["Armor_Adult_HeavyBoot"]), &["Armor_Adult_HeavyBoot"]),
    c("hover_boots", "Hover Boots", "", "gear", "", 0, Rule::Own(&["Armor_Adult_HoverBoot"]), &["Armor_Adult_HoverBoot"]),
    c("strength", "Strength", "", "gear", "", 0, Rule::Own(GLOVES), GLOVES),
    c("scale", "Scale", "", "gear", "", 0, Rule::Own(&["Item_Scale_Silver", "Item_Scale_Gold"]), &["Item_Scale_Silver", "Item_Scale_Gold"]),
    c("wallet", "Wallet", "", "gear", "", 0, Rule::Own(WALLETS), WALLETS),
    c("gerudo_card", "Gerudo Card", "", "gear", "", 0, Rule::Own(&["Item_Token_Gerudo"]), &["Item_Token_Gerudo"]),
    c("hearts", "Hearts", "", "hearts", "", 0, Rule::Hearts, &["Obj_Heart_Piece"]),
    c("skulltulas", "Gold Skulltula Tokens", "", "hearts", "", 0, Rule::Count("Item_Token_Gold"), &["Item_Token_Gold"]),
    c("zeldas_lullaby", "Zelda's Lullaby", "", "songs", "", 0, Rule::Flag("IsGet.Item_Melody_ZeldaLullaby"), &["Item_Melody_ZeldaLullaby"]),
    c("eponas_song", "Epona's Song", "", "songs", "", 0, Rule::Flag("IsGet.Item_Melody_EponaSong"), &["Item_Melody_EponaSong"]),
    c("sarias_song", "Saria's Song", "", "songs", "", 0, Rule::Flag("IsGet.Item_Melody_SariaSong"), &["Item_Melody_SariaSong"]),
    c("suns_song", "Sun's Song", "", "songs", "", 0, Rule::Flag("IsGet.Item_Melody_SunSong"), &["Item_Melody_SunSong"]),
    c("song_of_time", "Song of Time", "", "songs", "", 0, Rule::Flag("IsGet.Item_Melody_TimeSong"), &["Item_Melody_TimeSong"]),
    c("song_of_storms", "Song of Storms", "", "songs", "", 0, Rule::Flag("IsGet.Item_Melody_StormSong"), &["Item_Melody_StormSong"]),
    c("minuet", "Minuet of Forest", "", "songs", "", 0, Rule::Flag("IsGet.Item_Melody_ForestMinuet"), &["Item_Melody_ForestMinuet"]),
    c("bolero", "Bolero of Fire", "", "songs", "", 0, Rule::Flag("IsGet.Item_Melody_FireBolero"), &["Item_Melody_FireBolero"]),
    c("serenade", "Serenade of Water", "", "songs", "", 0, Rule::Flag("IsGet.Item_Melody_WaterSerenade"), &["Item_Melody_WaterSerenade"]),
    c("requiem", "Requiem of Spirit", "", "songs", "", 0, Rule::Flag("IsGet.Item_Melody_SpiritRequiem"), &["Item_Melody_SpiritRequiem"]),
    c("nocturne", "Nocturne of Shadow", "", "songs", "", 0, Rule::Flag("IsGet.Item_Melody_ShadowNocturne"), &["Item_Melody_ShadowNocturne"]),
    c("prelude", "Prelude of Light", "", "songs", "", 0, Rule::Flag("IsGet.Item_Melody_LightPrelude"), &["Item_Melody_LightPrelude"]),
    c("kokiri_emerald", "Kokiri's Emerald", "DEKU", "dungeons", "Deku Tree", 1, Rule::Own(&["Item_Stone_Forest"]), &["Item_Stone_Forest"]),
    c("goron_ruby", "Goron's Ruby", "DC", "dungeons", "Dodongo", 1, Rule::Own(&["Item_Stone_Fire"]), &["Item_Stone_Fire"]),
    c("zora_sapphire", "Zora's Sapphire", "JABU", "dungeons", "Jabu-Jabu", 1, Rule::Own(&["Item_Stone_Water"]), &["Item_Stone_Water"]),
    c("forest_medallion", "Forest Medallion", "FOREST", "dungeons", "Forest", 1, Rule::Own(&["Item_Medal_Forest"]), &["Item_Medal_Forest"]),
    c("forest_keys", "Forest Temple small keys", "", "dungeons", "Forest", 2, Rule::Keys("ForestTemple"), KEY),
    c("forest_boss_key", "Forest Temple boss key", "", "dungeons", "Forest", 3, Rule::Flag("DungeonInfo.ForestTemple.IsGetBossKey"), BOSS),
    c("fire_medallion", "Fire Medallion", "FIRE", "dungeons", "Fire", 1, Rule::Own(&["Item_Medal_Fire"]), &["Item_Medal_Fire"]),
    c("fire_keys", "Fire Temple small keys", "", "dungeons", "Fire", 2, Rule::Keys("FireTemple"), KEY),
    c("fire_boss_key", "Fire Temple boss key", "", "dungeons", "Fire", 3, Rule::Flag("DungeonInfo.FireTemple.IsGetBossKey"), BOSS),
    c("water_medallion", "Water Medallion", "WATER", "dungeons", "Water", 1, Rule::Own(&["Item_Medal_Water"]), &["Item_Medal_Water"]),
    c("water_keys", "Water Temple small keys", "", "dungeons", "Water", 2, Rule::Keys("WaterTemple"), KEY),
    c("water_boss_key", "Water Temple boss key", "", "dungeons", "Water", 3, Rule::Flag("DungeonInfo.WaterTemple.IsGetBossKey"), BOSS),
    c("shadow_medallion", "Shadow Medallion", "SHADOW", "dungeons", "Shadow", 1, Rule::Own(&["Item_Medal_Shadow"]), &["Item_Medal_Shadow"]),
    c("shadow_keys", "Shadow Temple small keys", "", "dungeons", "Shadow", 2, Rule::Keys("ShadowTemple"), KEY),
    c("shadow_boss_key", "Shadow Temple boss key", "", "dungeons", "Shadow", 3, Rule::Flag("DungeonInfo.ShadowTemple.IsGetBossKey"), BOSS),
    c("spirit_medallion", "Spirit Medallion", "SPIRIT", "dungeons", "Spirit", 1, Rule::Own(&["Item_Medal_Spirit"]), &["Item_Medal_Spirit"]),
    c("spirit_keys", "Spirit Temple small keys", "", "dungeons", "Spirit", 2, Rule::Keys("SpiritTemple"), KEY),
    c("spirit_boss_key", "Spirit Temple boss key", "", "dungeons", "Spirit", 3, Rule::Flag("DungeonInfo.SpiritTemple.IsGetBossKey"), BOSS),
    c("ganon_keys", "Ganon's Castle small keys", "", "dungeons", "Ganon", 2, Rule::Keys("InsideGanonCastle"), KEY),
    c("ganon_boss_key", "Ganon's Castle boss key", "", "dungeons", "Ganon", 3, Rule::Flag("DungeonInfo.InsideGanonCastle.IsGetBossKey"), BOSS),
    c("botw_keys", "Bottom of the Well small keys", "", "dungeons", "Well", 2, Rule::Keys("BottomOfTheWell"), KEY),
    c("gtg_keys", "Gerudo Training Ground small keys", "", "dungeons", "Training", 2, Rule::Keys("GerudoTrainingGround"), KEY),
    c("light_medallion", "Light Medallion", "FREE", "dungeons", "Free", 1, Rule::Own(&["Item_Medal_Light"]), &["Item_Medal_Light"]),
];

fn is_get(a: &str) -> String {
    format!("IsGet.{a}")
}

fn stock_name(a: &str) -> String {
    format!("PouchData.{a}.StockNum")
}

fn keys_name(d: &str) -> String {
    format!("DungeonInfo.{d}.SmallKeyNum")
}

fn h(n: &str) -> u32 {
    murmur3_32(n.as_bytes(), 0)
}

fn int(p: &Peek, n: &str) -> u32 {
    let v: i32 = p.val(h(n)).unwrap_or(0) as i32;
    (v.max(0) as u32).min(MAX_COUNT)
}

fn stock(p: &Peek, a: &str) -> u32 {
    int(p, &stock_name(a))
}

fn held(p: &Peek, a: &str) -> bool {
    p.on(h(&is_get(a))) || stock(p, a) > 0
}

fn tier(p: &Peek, list: &[&str]) -> u32 {
    list.iter().rposition(|a| held(p, a)).map(|i| i as u32 + 1).unwrap_or(0)
}

impl Cell {
    pub fn max(&self) -> u32 {
        match self.rule {
            Rule::Own(l) => l.len() as u32,
            Rule::Bag { bags, .. } => bags.len() as u32,
            Rule::Magic => 2,
            _ => 1,
        }
    }

    pub fn counted(&self) -> bool {
        matches!(self.rule, Rule::Count(_) | Rule::Bag { .. } | Rule::Hearts | Rule::Keys(_))
    }

    pub fn reward(&self) -> bool {
        self.slot == 1
    }

    pub fn flags(&self) -> Vec<Vec<String>> {
        let both = |a: &str| vec![is_get(a), stock_name(a)];
        match self.rule {
            Rule::Own(l) | Rule::Any(l) => l.iter().map(|a| both(a)).collect(),
            Rule::Count(a) => vec![both(a)],
            Rule::Bag { item, bags, ammo } => {
                let mut out: Vec<Vec<String>> = vec![both(item), vec![stock_name(ammo)]];
                out.extend(bags.iter().map(|a| both(a)));
                out
            }
            Rule::Flag(n) => vec![vec![n.to_string()]],
            Rule::Hearts => vec![vec!["PlayerStatus.MaxLife".to_string()], vec![stock_name("Obj_Heart_Piece")]],
            Rule::Magic => vec![vec!["PlayerStatus.MPMax".to_string()], vec![is_get("Item_FairyBless_MP")], vec![is_get("Item_FairyBless_MPDouble")]],
            Rule::Keys(d) => vec![vec![keys_name(d)]],
        }
    }

    pub fn read(&self, p: &Peek) -> St {
        match self.rule {
            Rule::Own(l) => St { lv: tier(p, l), ..St::default() },
            Rule::Any(l) => St { lv: l.iter().any(|a| held(p, a)) as u32, ..St::default() },
            Rule::Count(a) => St { lv: held(p, a) as u32, n: Some(stock(p, a)), part: None },
            Rule::Bag { item, bags, ammo } => St { lv: tier(p, bags).max(held(p, item) as u32), n: Some(stock(p, ammo)), part: None },
            Rule::Flag(n) => St { lv: p.on(h(n)) as u32, ..St::default() },
            Rule::Hearts => {
                let n: u32 = int(p, "PlayerStatus.MaxLife") / QUARTERS;
                St { lv: (n > 0) as u32, n: Some(n), part: Some(stock(p, "Obj_Heart_Piece").min(QUARTERS - 1)) }
            }
            Rule::Magic => {
                let mp: u32 = int(p, "PlayerStatus.MPMax");
                let lv: u32 = if mp >= MAGIC_TWO || p.on(h(&is_get("Item_FairyBless_MPDouble"))) {
                    2
                } else if mp >= MAGIC_ONE || p.on(h(&is_get("Item_FairyBless_MP"))) {
                    1
                } else {
                    0
                };
                St { lv, ..St::default() }
            }
            Rule::Keys(d) => {
                let n: u32 = int(p, &keys_name(d));
                St { lv: (n > 0) as u32, n: Some(n), part: None }
            }
        }
    }
}

pub fn read(p: &Peek) -> Vec<St> {
    CELLS.iter().map(|c| c.read(p)).collect()
}

pub fn layout_json() -> Json {
    let secs: Vec<Json> = SECS.iter().map(|s| Json::obj().with("id", s.id.into()).with("title", s.title.into())).collect();
    let cells: Vec<Json> = CELLS
        .iter()
        .map(|c| {
            Json::obj()
                .with("id", c.id.into())
                .with("label", c.label.into())
                .with("tag", c.tag.into())
                .with("sec", c.sec.into())
                .with("group", c.group.into())
                .with("slot", Json::Int(c.slot as i64))
                .with("max", Json::Int(c.max() as i64))
                .with("count", c.counted().into())
                .with("reward", c.reward().into())
                .with("icons", Json::strs(c.icons.iter().copied()))
        })
        .collect();
    Json::obj()
        .with("sections", Json::Arr(secs))
        .with("rows", Json::strs(ROWS.iter().copied()))
        .with("row_slots", Json::Int(ROW_SLOTS as i64))
        .with("cells", Json::Arr(cells))
        .with("reward_tags", Json::strs(REWARD_TAGS.iter().copied()))
}

pub fn state_json(st: &[St]) -> Json {
    let num = |v: Option<u32>| v.map(|n| Json::Int(n as i64)).unwrap_or(Json::Null);
    Json::Obj(CELLS.iter().zip(st).map(|(c, s)| (c.id.to_string(), Json::obj().with("lv", Json::Int(s.lv as i64)).with("n", num(s.n)).with("part", num(s.part)))).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::fixture::{Cell as Raw, raw};
    use crate::tracker::icon_map;
    use crate::tracker::peek::{Lim, peek_with};
    use std::collections::HashSet;

    const FLAGS: &str = include_str!("../../../../data/flags.json");

    fn cell(id: &str) -> (usize, &'static Cell) {
        CELLS.iter().enumerate().find(|c| c.1.id == id).unwrap()
    }

    fn save(rows: &[(&str, u32)]) -> Peek {
        let cells: Vec<(u32, Raw)> = rows.iter().map(|(n, v)| (h(n), Raw::Inline(*v))).collect();
        peek_with(&raw(&cells, &[]), &Lim::ANY).unwrap()
    }

    #[test]
    fn ids_are_unique_and_every_icon_is_mapped() {
        let ids: HashSet<&str> = CELLS.iter().map(|c| c.id).collect();
        assert_eq!(ids.len(), CELLS.len());
        let known: HashSet<&str> = icon_map::all().iter().map(|s| s.id).collect();
        let mut used: HashSet<&str> = HashSet::new();
        for c in &CELLS {
            assert!(!c.icons.is_empty() && !c.label.is_empty(), "{}", c.id);
            assert!(c.icons.len() == 1 || c.icons.len() as u32 == c.max(), "{}", c.id);
            for i in c.icons {
                assert!(known.contains(i), "{} uses {i}", c.id);
                used.insert(i);
            }
        }
        assert_eq!(used, known);
    }

    #[test]
    fn every_cell_sits_in_a_section_and_rows_fill_their_slots_once() {
        for s in &SECS {
            assert!(CELLS.iter().any(|c| c.sec == s.id), "{} is empty", s.id);
            assert!(!s.title.is_empty());
        }
        let mut seen: HashSet<(&str, u8)> = HashSet::new();
        for c in &CELLS {
            assert!(SECS.iter().any(|s| s.id == c.sec), "{}", c.id);
            if c.sec == ROW_SEC {
                assert!(ROWS.contains(&c.group), "{}", c.id);
                assert!((1..=ROW_SLOTS).contains(&c.slot), "{}", c.id);
                assert!(seen.insert((c.group, c.slot)), "{} shares a slot", c.id);
                let want: u8 = match c.rule {
                    Rule::Keys(_) => 2,
                    Rule::Flag(_) => 3,
                    _ => 1,
                };
                assert_eq!(c.slot, want, "{}", c.id);
            } else {
                assert!(c.group.is_empty() && c.slot == 0, "{}", c.id);
            }
        }
        for r in ROWS {
            assert!(seen.iter().any(|x| x.0 == r), "{r} has no tile");
        }
        let order: Vec<&str> = SECS.iter().map(|s| s.id).collect();
        let mut last: usize = 0;
        for c in &CELLS {
            let at: usize = order.iter().position(|s| *s == c.sec).unwrap();
            assert!(at >= last, "{} is out of section order", c.id);
            last = at;
        }
    }

    #[test]
    fn the_sections_hold_the_expected_tiles() {
        let n = |sec: &str| CELLS.iter().filter(|c| c.sec == sec).count();
        assert_eq!((n("tools"), n("spells"), n("bottles"), n("gear"), n("hearts"), n("songs"), n("dungeons")), (14, 5, 5, 15, 2, 12, 23));
        let tags: Vec<&str> = CELLS.iter().filter(|c| c.reward()).map(|c| c.tag).collect();
        assert_eq!(tags, vec!["DEKU", "DC", "JABU", "FOREST", "FIRE", "WATER", "SHADOW", "SPIRIT", "FREE"]);
        assert!(tags.iter().all(|t| REWARD_TAGS.contains(t)));
        let small: Vec<&str> = CELLS.iter().filter(|c| matches!(c.rule, Rule::Keys(_))).map(|c| c.group).collect();
        assert_eq!(small, vec!["Forest", "Fire", "Water", "Shadow", "Spirit", "Ganon", "Well", "Training"]);
        let songs: Vec<&str> = CELLS.iter().filter(|c| c.sec == "songs").map(|c| c.id).collect();
        assert_eq!(&songs[..3], &["zeldas_lullaby", "eponas_song", "sarias_song"]);
    }

    #[test]
    fn every_rule_reads_a_flag_the_game_list_has() {
        for c in &CELLS {
            for group in c.flags() {
                assert!(group.iter().any(|n| FLAGS.contains(&format!("\"{n}\""))), "{} needs one of {group:?}", c.id);
            }
        }
    }

    #[test]
    fn an_empty_save_shows_nothing_owned() {
        let st: Vec<St> = read(&save(&[]));
        assert_eq!(st.len(), CELLS.len());
        assert!(st.iter().all(|s| s.lv == 0));
        assert_eq!(st[cell("sticks").0].n, Some(0));
        assert_eq!(st[cell("lens").0].n, None);
    }

    #[test]
    fn counts_tiers_and_flags_come_from_the_save() {
        let p: Peek = save(&[
            ("IsGet.Item_DekuStick", 1),
            ("PouchData.Item_DekuStick.StockNum", 10),
            ("PouchData.Item_Hookshot.StockNum", 1),
            ("IsGet.Item_Longshot", 1),
            ("IsGet.Weapon_Range_Bow", 1),
            ("PouchData.Item_Quiver_Small.StockNum", 1),
            ("PouchData.Item_Quiver_Medium.StockNum", 1),
            ("PouchData.Item_Arrow_Normal.StockNum", 37),
            ("IsGet.Item_Bomb", 1),
            ("IsGet.Weapon_Range_SlingShot", 1),
            ("PouchData.BottleC.StockNum", 1),
            ("IsGet.Item_Melody_SunSong", 1),
            ("PlayerStatus.MaxLife", 68),
            ("PouchData.Obj_Heart_Piece.StockNum", 3),
            ("PlayerStatus.MPMax", 240),
            ("DungeonInfo.FireTemple.SmallKeyNum", 8),
            ("DungeonInfo.WaterTemple.IsGetBossKey", 1),
            ("PouchData.Item_Token_Gold.StockNum", 91),
            ("PouchData.Item_Wallet_Small.StockNum", 1),
            ("IsGet.Obj_Armor_Child_Normal", 1),
            ("PouchData.Item_Bombchu.StockNum", 0xffff_fff0),
        ]);
        let st: Vec<St> = read(&p);
        let s = |id: &str| st[cell(id).0];
        assert_eq!(s("sticks"), St { lv: 1, n: Some(10), part: None });
        assert_eq!(s("hookshot").lv, 2);
        assert_eq!(s("bow"), St { lv: 2, n: Some(37), part: None });
        assert_eq!(s("bombs"), St { lv: 1, n: Some(0), part: None });
        assert_eq!(s("slingshot"), St { lv: 1, n: Some(0), part: None });
        assert_eq!((s("bottle_3").lv, s("bottle_2").lv), (1, 0));
        assert_eq!((s("suns_song").lv, s("sarias_song").lv), (1, 0));
        assert_eq!(s("hearts"), St { lv: 1, n: Some(17), part: Some(3) });
        assert_eq!(s("magic").lv, 1);
        assert_eq!(s("fire_keys"), St { lv: 1, n: Some(8), part: None });
        assert_eq!((s("water_boss_key").lv, s("fire_boss_key").lv), (1, 0));
        assert_eq!(s("skulltulas"), St { lv: 1, n: Some(91), part: None });
        assert_eq!((s("wallet").lv, s("kokiri_tunic").lv, s("strength").lv), (1, 1, 0));
        assert_eq!(s("bombchus"), St { lv: 0, n: Some(0), part: None });
    }

    #[test]
    fn double_magic_comes_from_the_meter_or_the_gift_flag() {
        let (i, _): (usize, &Cell) = cell("magic");
        assert_eq!(read(&save(&[("PlayerStatus.MPMax", 480)]))[i].lv, 2);
        assert_eq!(read(&save(&[("IsGet.Item_FairyBless_MPDouble", 1)]))[i].lv, 2);
        assert_eq!(read(&save(&[("IsGet.Item_FairyBless_MP", 1)]))[i].lv, 1);
    }

    #[test]
    fn the_json_carries_layout_and_state() {
        let l: Json = layout_json();
        assert_eq!(l.get("cells").and_then(|x| x.as_arr()).map(|a| a.len()), Some(76));
        assert_eq!(l.get("sections").and_then(|x| x.as_arr()).map(|a| a.len()), Some(7));
        assert_eq!(l.get("rows").and_then(|x| x.as_arr()).map(|a| a.len()), Some(12));
        let t: String = l.to_string();
        assert!(t.contains("\"id\":\"hookshot\"") && t.contains("\"icons\":[\"Item_Hookshot\",\"Item_Longshot\"]"));
        assert!(!t.contains("IsGet") && !t.contains("PouchData"));
        assert!(t.contains("\"id\":\"fire_keys\",\"label\":\"Fire Temple small keys\",\"tag\":\"\",\"sec\":\"dungeons\",\"group\":\"Fire\",\"slot\":2"));
        let s: String = state_json(&read(&save(&[("PlayerStatus.MaxLife", 12)]))).to_string();
        assert!(s.contains("\"hearts\":{\"lv\":1,\"n\":3,\"part\":0}"));
        assert!(s.contains("\"lens\":{\"lv\":0,\"n\":null,\"part\":null}"));
    }

    #[test]
    fn a_real_save_gives_sane_values_when_one_is_given() {
        let Ok(d) = std::env::var("OOTRR_TEST_SAVES") else {
            return;
        };
        let mut n: usize = 0;
        let mut stack: Vec<std::path::PathBuf> = vec![d.into()];
        while let Some(dir) = stack.pop() {
            for e in std::fs::read_dir(&dir).unwrap().flatten() {
                let p: std::path::PathBuf = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.file_name().is_some_and(|f| f == "progress.sav") {
                    let pk: Peek = crate::tracker::peek::peek(&std::fs::read(&p).unwrap()).unwrap();
                    let st: Vec<St> = read(&pk);
                    let s = |id: &str| st[cell(id).0];
                    assert!((3..=20).contains(&s("hearts").n.unwrap()), "{}", p.display());
                    assert!(s("skulltulas").n.unwrap() <= 100, "{}", p.display());
                    assert!(s("wallet").lv >= 1, "{}", p.display());
                    for (c, v) in CELLS.iter().zip(&st) {
                        assert!(v.lv <= c.max(), "{} {}", c.id, p.display());
                    }
                    n += 1;
                }
            }
        }
        assert!(n > 0);
    }
}
