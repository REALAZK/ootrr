use std::collections::BTreeMap as BM;

use crate::settings::Settings;
use crate::start_items::table::{group, opt, refused, song, Bag, Group, Recipe, Strip};

pub const HEARTS: &str = "starting_hearts";
pub const ITEMS: &str = "starting_items";
pub const SONGS: &str = "starting_songs";
pub const VANILLA_HEARTS: i64 = 3;
pub const MP_UNIT: i32 = 240;
pub const SCALE_DEPTH: [f32; 2] = [6.0, 9.0];
pub const LIFT: [&str; 3] = ["Low", "Middle", "High"];
pub const STRENGTH: [&str; 3] = ["Armor_Child_Gauntlet_Goron", "Armor_Adult_Gauntlet_Silver", "Armor_Adult_Gauntlet_Gold"];
pub const SCALES: [&str; 2] = ["Item_Scale_Silver", "Item_Scale_Gold"];
pub const WALLETS: [&str; 2] = ["Item_Wallet_Medium", "Item_Wallet_Large"];
pub const MAGIC: [&str; 2] = ["Item_FairyBless_MP", "Item_FairyBless_MPDouble"];
pub const OCARINAS: [&str; 2] = ["Item_Ocarina_Fairy", "Item_Ocarina_Time"];
pub const BOTTLES: [&str; 4] = ["BottleA", "BottleB", "BottleC", "BottleD"];

#[derive(Debug, Clone, PartialEq)]
pub enum Set {
    Bool(String, bool),
    Int(String, i32),
    Enum(String, String),
    Str(String, String),
    Float(String, f32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Take {
    pub logic: &'static str,
    pub count: u32,
    pub strip: Strip,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Kit {
    pub take: Vec<Take>,
    pub sets: Vec<Set>,
    pub pushes: Vec<&'static str>,
    pub hearts: Option<i32>,
    pub magic: Option<i32>,
    pub notes: Vec<String>,
}

impl Kit {
    pub fn any(&self) -> bool {
        !self.sets.is_empty() || !self.pushes.is_empty() || self.hearts.is_some() || self.magic.is_some()
    }

    pub fn module(&self) -> bool {
        !self.pushes.is_empty() || self.hearts.is_some() || self.magic.is_some()
    }

    pub fn logic(&self) -> Vec<(&'static str, u32)> {
        self.take.iter().filter(|t| t.count > 0).map(|t| (t.logic, t.count)).collect()
    }
}

pub const NOTE: &str = "The chosen starting items are written into the new game defaults (GameDataList) and the game start module, and one copy of each is taken out of the item pool and replaced by Rupee (1).";

fn grant(out: &mut Vec<Set>, actor: &str, n: i32) {
    out.push(Set::Bool(format!("IsGet.{actor}"), true));
    out.push(Set::Int(format!("PouchData.{actor}.StockNum"), n));
}

fn bag(out: &mut Vec<Set>, b: &Bag, tier: u32) {
    let t: usize = (tier as usize).clamp(1, 3);
    if !b.tool.is_empty() {
        grant(out, b.tool, 1);
    }
    for a in &b.tiers[..t] {
        grant(out, a, 1);
    }
    let cap: i32 = b.caps[t - 1];
    grant(out, b.ammo, cap);
    out.push(Set::Int(b.max.to_string(), cap));
    out.push(Set::Str(format!("CurrentExpansionActor_{}", b.kind), b.tiers[t - 1].to_string()));
}

fn recipe(k: &mut Kit, g: &Group, tier: u32) {
    let t: usize = tier as usize;
    match g.recipe {
        Recipe::Pushed => {}
        Recipe::Grant(actors) => {
            for a in actors.iter().take(t.max(1)) {
                grant(&mut k.sets, a, 1);
            }
        }
        Recipe::Bag(b) => bag(&mut k.sets, &b, tier),
        Recipe::Strength => {
            for a in STRENGTH.iter().take(t) {
                grant(&mut k.sets, a, 1);
            }
            k.sets.push(Set::Enum("PlayerStatus.LiftGrade".to_string(), LIFT[t.clamp(1, 3) - 1].to_string()));
        }
        Recipe::Scale => {
            for a in SCALES.iter().take(t) {
                grant(&mut k.sets, a, 1);
            }
            k.sets.push(Set::Float("PlayerStatus.DiveDepth".to_string(), SCALE_DEPTH[t.clamp(1, 2) - 1]));
        }
        Recipe::Wallet => k.pushes.extend(WALLETS.iter().take(t)),
        Recipe::Magic => {
            for a in MAGIC.iter().take(t) {
                grant(&mut k.sets, a, 1);
            }
            k.magic = Some(MP_UNIT * t.clamp(1, 2) as i32);
        }
        Recipe::Ocarina => {
            for a in OCARINAS.iter().take(t) {
                grant(&mut k.sets, a, 1);
            }
            k.sets.push(Set::Str("PlayerStatus.EquipOcarina".to_string(), OCARINAS[t.clamp(1, 2) - 1].to_string()));
            k.sets.push(Set::Bool("HavePreparedToPlayOcarina".to_string(), true));
        }
        Recipe::Bottles => {
            k.sets.push(Set::Bool("IsGet.Item_Bottle".to_string(), true));
            for b in BOTTLES.iter().take(t) {
                k.sets.push(Set::Int(format!("PouchData.{b}.StockNum"), 1));
                k.sets.push(Set::Bool(format!("PlayerStatus.AssignableItem.BottleInfo.{b}.IsValid"), true));
            }
        }
        Recipe::Count(actor, n) => grant(&mut k.sets, actor, n),
    }
}

fn take_count(g: &Group, tier: u32) -> u32 {
    match g.recipe {
        Recipe::Count(_, n) if g.logic == "Magic Bean" => n as u32,
        _ => tier.saturating_sub(g.offset),
    }
}

pub fn read(s: &Settings, bad: &mut Vec<String>) -> Kit {
    let mut k: Kit = Kit::default();
    let chosen: Vec<String> = s.items(ITEMS).map(|v| v.to_vec()).unwrap_or_default();
    let mut tiers: BM<&'static str, u32> = BM::new();
    for id in &chosen {
        if let Some(why) = refused(id) {
            bad.push(format!("starting_items includes {id}, which this build cannot give: {why}"));
            continue;
        }
        match opt(id) {
            Some(o) => {
                let t: &mut u32 = tiers.entry(o.group).or_insert(0);
                *t = (*t).max(o.tier);
            }
            None => bad.push(format!("starting_items includes {id}, which this build does not know")),
        }
    }
    for (gid, tier) in &tiers {
        let Some(g) = group(gid) else {
            bad.push(format!("starting item group {gid} is missing from the table"));
            continue;
        };
        recipe(&mut k, g, *tier);
        k.take.push(Take { logic: g.logic, count: take_count(g, *tier), strip: g.strip });
    }
    if tiers.contains_key("bombchus") && !tiers.contains_key("bombs") {
        bad.push("starting_items includes bombchus without a bomb bag. The remake only lets you use Bombchus once a Bomb Bag and its ammo maximum are set, so tick a bomb bag as well".to_string());
    }
    let songs: Vec<String> = s.items(SONGS).map(|v| v.to_vec()).unwrap_or_default();
    for id in &songs {
        match song(id) {
            Some((_, logic, flag)) => {
                k.sets.push(Set::Bool(flag.to_string(), true));
                k.take.push(Take { logic, count: 1, strip: Strip::Unique });
            }
            None => bad.push(format!("starting_songs includes {id}, which this build does not know")),
        }
    }
    match s.int(HEARTS) {
        Ok(n) if n == VANILLA_HEARTS => {}
        Ok(n) => k.hearts = Some(n as i32),
        Err(e) => bad.push(e.to_string()),
    }
    if !k.take.is_empty() || k.hearts.is_some() {
        k.notes.push(NOTE.to_string());
    }
    if let Some(n) = k.hearts {
        k.notes.push(format!("You start with {n} hearts. The start module sets the heart containers and the current life."));
    }
    k
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Value;

    fn with(items: &[&str], songs: &[&str], hearts: i64) -> Settings {
        let mut s: Settings = Settings::default();
        s.set(ITEMS, Value::set(items)).unwrap();
        s.set(SONGS, Value::set(songs)).unwrap();
        s.set(HEARTS, Value::Int(hearts)).unwrap();
        s
    }

    fn find<'a>(k: &'a Kit, name: &str) -> Option<&'a Set> {
        k.sets.iter().find(|x| match x {
            Set::Bool(n, _) | Set::Int(n, _) | Set::Enum(n, _) | Set::Str(n, _) | Set::Float(n, _) => n == name,
        })
    }

    #[test]
    fn nothing_chosen_gives_an_empty_kit() {
        let mut bad: Vec<String> = Vec::new();
        let k: Kit = read(&Settings::default(), &mut bad);
        assert!(bad.is_empty());
        assert_eq!(k, Kit::default());
        assert!(!k.any());
    }

    #[test]
    fn the_highest_tier_wins_and_counts_as_the_logic_count() {
        let mut bad: Vec<String> = Vec::new();
        let k: Kit = read(&with(&["hookshot", "longshot", "big_bomb_bag", "golden_gauntlets", "giant_wallet", "double_magic", "big_stick_bag"], &[], 3), &mut bad);
        assert!(bad.is_empty(), "{bad:?}");
        let l: Vec<(&str, u32)> = k.logic();
        for want in [("Progressive Hookshot", 2), ("Bomb Bag", 2), ("Progressive Strength Upgrade", 3), ("Progressive Wallet", 2), ("Magic Meter", 2), ("Deku Stick Capacity", 1)] {
            assert!(l.contains(&want), "{want:?} {l:?}");
        }
        assert_eq!(find(&k, "0xe753508b"), Some(&Set::Int("0xe753508b".to_string(), 30)));
        assert_eq!(find(&k, "PouchData.Item_Bomb.StockNum"), Some(&Set::Int("PouchData.Item_Bomb.StockNum".to_string(), 30)));
        assert_eq!(find(&k, "CurrentExpansionActor_Bomb"), Some(&Set::Str("CurrentExpansionActor_Bomb".to_string(), "Item_BombBag_Medium".to_string())));
        assert!(find(&k, "IsGet.Item_BombBag_Small").is_some() && find(&k, "IsGet.Item_BombBag_Large").is_none());
        assert_eq!(find(&k, "PlayerStatus.LiftGrade"), Some(&Set::Enum("PlayerStatus.LiftGrade".to_string(), "High".to_string())));
        assert_eq!(k.pushes, vec!["Item_Wallet_Medium", "Item_Wallet_Large"]);
        assert_eq!(k.magic, Some(480));
        assert!(find(&k, "IsGet.Item_Longshot").is_some());
        assert_eq!(find(&k, "0x457616b0"), Some(&Set::Int("0x457616b0".to_string(), 20)));
    }

    #[test]
    fn songs_bottles_ocarina_and_hearts_are_read() {
        let mut bad: Vec<String> = Vec::new();
        let k: Kit = read(&with(&["ocarina", "two_bottles", "magic_beans"], &["song_of_time", "eponas_song"], 7), &mut bad);
        assert!(bad.is_empty(), "{bad:?}");
        assert_eq!(k.hearts, Some(7));
        assert_eq!(find(&k, "PlayerStatus.EquipOcarina"), Some(&Set::Str("PlayerStatus.EquipOcarina".to_string(), "Item_Ocarina_Fairy".to_string())));
        assert!(find(&k, "HavePreparedToPlayOcarina").is_some());
        assert!(find(&k, "PouchData.BottleB.StockNum").is_some() && find(&k, "PouchData.BottleC.StockNum").is_none());
        assert!(find(&k, "PlayerStatus.AssignableItem.BottleInfo.BottleA.IsValid").is_some());
        assert!(find(&k, "IsGet.Item_Melody_TimeSong").is_some());
        let l: Vec<(&str, u32)> = k.logic();
        assert!(l.contains(&("Bottle", 2)) && l.contains(&("Magic Bean", 10)) && l.contains(&("Song of Time", 1)) && l.contains(&("Ocarina", 1)));
        assert_eq!(k.notes.len(), 2);
    }

    #[test]
    fn refused_and_inconsistent_choices_are_named() {
        let mut bad: Vec<String> = Vec::new();
        read(&with(&["zeldas_letter", "bombchus"], &[], 3), &mut bad);
        assert_eq!(bad.len(), 2, "{bad:?}");
        assert!(bad[0].contains("zeldas_letter") && bad[0].contains("trade slot"));
        assert!(bad[1].contains("bombchus without a bomb bag"));
        let mut ok: Vec<String> = Vec::new();
        let k: Kit = read(&with(&["bombchus", "bomb_bag"], &[], 3), &mut ok);
        assert!(ok.is_empty());
        assert!(k.logic().contains(&("Bombchus", 1)));
    }

    #[test]
    fn a_settings_string_from_the_stack_before_this_change_still_reads_the_same() {
        let old: &str = "OOTRR1-CAAQAJAEAMAR4HZGAEAUOAIBJAAQCAIX";
        let l: crate::settings::Loaded = Settings::decode(old).unwrap();
        assert!(l.warnings.is_empty(), "{:?}", l.warnings);
        let s: Settings = l.settings;
        assert_eq!(s.items(ITEMS).unwrap(), ["slingshot", "kokiri_sword", "deku_shield"]);
        assert_eq!(s.text("shuffle_mapcompass").unwrap(), "remove");
        assert!(s.flag(crate::qol::ANJU).unwrap() && s.flag(crate::qol::POE).unwrap());
        assert_eq!(s.int(HEARTS).unwrap(), 3);
        assert_eq!(s.encode().unwrap(), old);
        let p: crate::slice::support::Plan = crate::slice::support::plan(&s).unwrap();
        assert_eq!(p.pouch, vec!["Weapon_Range_SlingShot", "Weapon_Sword_Kokiri", "Weapon_Shield_Deku"]);
        assert_eq!(p.kit.logic().len(), 3);
    }

    #[test]
    fn the_three_pushed_items_only_change_the_logic_and_the_pool() {
        let mut bad: Vec<String> = Vec::new();
        let k: Kit = read(&with(&["kokiri_sword", "deku_shield", "slingshot"], &[], 3), &mut bad);
        assert!(bad.is_empty());
        assert!(find(&k, "IsGet.Weapon_Sword_Kokiri").is_none());
        assert!(find(&k, "IsGet.Item_BulletBag_Small").is_some() && find(&k, "IsGet.Weapon_Range_SlingShot").is_none());
        assert_eq!(find(&k, "0xae8cc742"), Some(&Set::Int("0xae8cc742".to_string(), 30)));
        assert_eq!(k.logic().len(), 3);
    }
}
