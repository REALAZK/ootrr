use crate::error::{Error as E, Res};
use crate::settings::{Settings, Value};
use crate::start_reward::Reward;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Off,
    Overworld,
    Dungeons,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keep {
    Vanilla,
    Dungeon,
    AnyDungeon,
    Anywhere,
}

impl Keep {
    pub fn roams(self) -> bool {
        matches!(self, Keep::AnyDungeon | Keep::Anywhere)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forest {
    Closed,
    ClosedDeku,
    Open,
}

pub const MIDO_WALL: &str = "KokiriForest_Npc_Kokiri_Mido_Child_Blocker";
pub const EXIT_WALL: &str = "KokiriForest_Npc_Kokiri_007_Child_Blocker_00";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Songs {
    Off,
    Own,
    Anywhere,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub chests: Mode,
    pub small: Keep,
    pub boss: Keep,
    pub map: Keep,
    pub ganon: Keep,
    pub rewards: bool,
    pub hearts: bool,
    pub songs: Songs,
    pub hints: bool,
    pub fast: bool,
    pub intro: bool,
    pub start: Reward,
    pub forest: Forest,
    pub gear: Vec<&'static str>,
    pub pouch: Vec<&'static str>,
    pub excluded: Vec<String>,
    pub notes: Vec<String>,
}

/// Setting id, logic item name, pouch actor the new-game flow pushes.
const GEAR: &[(&str, &str, &str)] = &[
    ("kokiri_sword", "Kokiri Sword", "Weapon_Sword_Kokiri"),
    ("deku_shield", "Deku Shield", "Weapon_Shield_Deku"),
    ("slingshot", "Slingshot", "Weapon_Range_SlingShot"),
];

const UNCHANGED: [&str; 24] = [
    "logic_rules",
    "starting_age",
    "open_kakariko",
    "open_door_of_time",
    "zora_fountain",
    "gerudo_fortress",
    "bridge",
    "bridge_stones",
    "bridge_medallions",
    "bridge_rewards",
    "bridge_tokens",
    "bridge_hearts",
    "trials_random",
    "trials",
    "triforce_hunt",
    "triforce_count_per_world",
    "triforce_goal_per_world",
    "starting_hearts",
    "starting_songs",
    "ganon_bosskey_stones",
    "ganon_bosskey_medallions",
    "ganon_bosskey_rewards",
    "ganon_bosskey_tokens",
    "ganon_bosskey_hearts",
];

const OFF_ONLY: [&str; 4] = ["shuffle_pots", "shuffle_crates", "shuffle_scrubs", "shuffle_freestanding_items"];

pub fn show(v: &Value) -> String {
    match v {
        Value::Bool(b) => b.to_string(),
        Value::Str(s) => s.clone(),
        Value::Int(n) => n.to_string(),
        Value::Set(x) => format!("[{}]", x.join(", ")),
    }
}

fn keep(s: &Settings, id: &str, roam: bool, bad: &mut Vec<String>) -> Keep {
    match s.text(id) {
        Ok("vanilla") => Keep::Vanilla,
        Ok("dungeon") => Keep::Dungeon,
        Ok("any_dungeon") if roam => Keep::AnyDungeon,
        Ok("keysanity") if roam => Keep::Anywhere,
        Ok(v) => {
            let ok: &str = if roam { "vanilla, dungeon, any_dungeon and keysanity" } else { "vanilla and dungeon" };
            bad.push(format!("{id} is {v}, which this build cannot place yet (only {ok} work)"));
            Keep::Vanilla
        }
        Err(e) => {
            bad.push(e.to_string());
            Keep::Vanilla
        }
    }
}

fn chest_mode(s: &Settings, bad: &mut Vec<String>) -> Mode {
    match s.text("shuffle_chests") {
        Ok("off") => Mode::Off,
        Ok("overworld") => Mode::Overworld,
        Ok("dungeons") => Mode::Dungeons,
        Ok("all") => Mode::All,
        Ok(v) => {
            bad.push(format!("shuffle_chests is {v}, which is not a known choice"));
            Mode::Off
        }
        Err(e) => {
            bad.push(e.to_string());
            Mode::Off
        }
    }
}

pub const KEY_LAYOUTS: [&str; 3] = ["fire_key_layout", "water_key_layout", "spirit_key_layout"];

impl Plan {
    pub fn keys_roam(&self) -> bool {
        self.small.roams()
    }

    pub fn start_flags(&self) -> Vec<(String, bool)> {
        match self.forest {
            Forest::Closed => Vec::new(),
            Forest::ClosedDeku => vec![(EXIT_WALL.to_string(), false)],
            Forest::Open => vec![(MIDO_WALL.to_string(), false), (EXIT_WALL.to_string(), false)],
        }
    }

    pub fn logic(&self) -> Vec<(&'static str, &'static str)> {
        let mut out: Vec<(&'static str, &'static str)> = match self.forest {
            Forest::Closed => Vec::new(),
            Forest::ClosedDeku => vec![("open_forest", "closed_deku")],
            Forest::Open => vec![("open_forest", "open")],
        };
        if self.keys_roam() {
            out.extend(KEY_LAYOUTS.iter().map(|k| (*k, "shuffled")));
        }
        out
    }
}

pub fn plan(s: &Settings) -> Res<Plan> {
    let def: Settings = Settings::default();
    let mut bad: Vec<String> = Vec::new();
    let mut noted: Vec<String> = Vec::new();
    for id in UNCHANGED {
        match (s.get(id), def.get(id)) {
            (Some(a), Some(b)) if a == b => noted.push(format!("{id}={}", show(a))),
            (Some(a), _) => bad.push(format!(
                "{id} is {}, which this build cannot apply yet (only the default works)",
                show(a)
            )),
            _ => bad.push(format!("no setting named {id}")),
        }
    }
    for id in OFF_ONLY {
        match s.text(id) {
            Ok("off") => {}
            Ok(v) => bad.push(format!("{id} is {v}, which this build cannot shuffle yet (only off works)")),
            Err(e) => bad.push(e.to_string()),
        }
    }
    match s.text("shuffle_hideoutkeys") {
        Ok("vanilla") => {}
        Ok(v) => bad.push(format!("shuffle_hideoutkeys is {v}, which this build cannot place yet (only vanilla works)")),
        Err(e) => bad.push(e.to_string()),
    }
    let chests: Mode = chest_mode(s, &mut bad);
    let small: Keep = keep(s, "shuffle_smallkeys", true, &mut bad);
    let boss: Keep = keep(s, "shuffle_bosskeys", false, &mut bad);
    let map: Keep = keep(s, "shuffle_mapcompass", false, &mut bad);
    let ganon: Keep = keep(s, "shuffle_ganon_bosskey", false, &mut bad);
    let rewards: bool = match s.text("shuffle_rewards") {
        Ok("off") => false,
        Ok("supported") => true,
        Ok(v) => {
            bad.push(format!("shuffle_rewards is {v}, which this build cannot apply (only off or supported work)"));
            false
        }
        Err(e) => {
            bad.push(e.to_string());
            false
        }
    };
    let hearts: bool = s.flag("shuffle_boss_hearts").unwrap_or_else(|e| {
        bad.push(e.to_string());
        false
    });
    let hints: bool = s.flag("gossip_hints").unwrap_or_else(|e| {
        bad.push(e.to_string());
        false
    });
    let fast: bool = s.flag("faster_chests").unwrap_or_else(|e| {
        bad.push(e.to_string());
        false
    });
    let intro: bool = s.flag("skip_intro").unwrap_or_else(|e| {
        bad.push(e.to_string());
        false
    });
    let start: Reward = match s.text("start_reward") {
        Ok("off") => Reward::Off,
        Ok("stone") => Reward::Stone,
        Ok("medal") => Reward::Medal,
        Ok("any") => Reward::Any,
        Ok(v) => {
            bad.push(format!("start_reward is {v}, which is not a known choice"));
            Reward::Off
        }
        Err(e) => {
            bad.push(e.to_string());
            Reward::Off
        }
    };
    let forest: Forest = match s.text("open_forest") {
        Ok("closed") => Forest::Closed,
        Ok("closed_deku") => Forest::ClosedDeku,
        Ok("open") => Forest::Open,
        Ok(v) => {
            bad.push(format!("open_forest is {v}, which is not a known choice"));
            Forest::Closed
        }
        Err(e) => {
            bad.push(e.to_string());
            Forest::Closed
        }
    };
    let songs: Songs = match s.text("shuffle_songs") {
        Ok("off") => Songs::Off,
        Ok("own") => Songs::Own,
        Ok("anywhere") => Songs::Anywhere,
        Ok(v) => {
            bad.push(format!("shuffle_songs is {v}, which is not a known choice"));
            Songs::Off
        }
        Err(e) => {
            bad.push(e.to_string());
            Songs::Off
        }
    };
    let chosen: Vec<String> = s.items("starting_items").map(|v| v.to_vec()).unwrap_or_default();
    let mut gear: Vec<&'static str> = Vec::new();
    let mut pouch: Vec<&'static str> = Vec::new();
    for id in &chosen {
        match GEAR.iter().find(|(k, _, _)| *k == id) {
            Some((_, logic, actor)) => {
                gear.push(*logic);
                pouch.push(*actor);
            }
            None => bad.push(format!(
                "starting_items includes {id}, which this build cannot give yet (kokiri_sword, deku_shield and slingshot work)"
            )),
        }
    }
    let excluded: Vec<String> = s.items("disabled_locations").map(|v| v.to_vec()).unwrap_or_default();
    if !bad.is_empty() {
        let lines: Vec<String> = bad.iter().map(|b| format!("  {b}")).collect();
        return Err(E::Msg(format!(
            "This build cannot apply these settings yet:\n{}",
            lines.join("\n")
        )));
    }
    let mut notes: Vec<String> = Vec::new();
    if !noted.is_empty() {
        notes.push(format!(
            "Not applied by this build, the game plays as unmodified: {}",
            noted.join(", ")
        ));
    }
    for (on, text) in [
        (hearts, "Boss Heart Containers are shuffled. Each becomes a chest, experimental and untested in the game."),
        (songs != Songs::Off, "Songs are shuffled. The song lessons are edited, experimental and untested in the game."),
        (fast, "Chest and item pickup scenes are sped up."),
        (intro, "The new game intro is skipped. The opening movie and the wake up scene are bypassed, experimental and untested in the game."),
        (start != Reward::Off, "You start the game with one random Stone or Medallion. Experimental and untested in the game."),
        (!pouch.is_empty(), "The ticked starting gear is added to the pouch when a new game starts. A copy can still be found in the world."),
        (small.roams(), "Small keys can be found in other dungeons or outside them. A chest key is counted for the dungeon it belongs to and the chest says which. Experimental and untested in the game."),
        (forest != Forest::Closed, "Kokiri Forest starts open. The blocker walls are switched off when a new game starts, experimental and untested in the game."),
        (hints, "Gossip stones give hints. Their text is replaced in every language, experimental and untested in the game."),
    ] {
        if on {
            notes.push(text.to_string());
        }
    }
    Ok(Plan { chests, small, boss, map, ganon, rewards, hearts, songs, hints, fast, intro, start, forest, gear, pouch, excluded, notes })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_accepted_and_noted() {
        let p: Plan = plan(&Settings::default()).unwrap();
        assert_eq!(p.chests, Mode::All);
        assert_eq!(p.small, Keep::Dungeon);
        assert_eq!(p.map, Keep::Dungeon);
        assert!(p.intro);
        assert_eq!(p.notes.len(), 2);
        assert!(p.notes.iter().any(|n| n.contains("bridge=medallions")));
        assert!(p.notes.iter().any(|n| n.contains("intro is skipped")));
    }

    #[test]
    fn vanilla_keys_and_chest_modes_are_honored() {
        let mut s: Settings = Settings::default();
        s.set("shuffle_smallkeys", Value::s("vanilla")).unwrap();
        s.set("shuffle_chests", Value::s("dungeons")).unwrap();
        let p: Plan = plan(&s).unwrap();
        assert_eq!(p.small, Keep::Vanilla);
        assert_eq!(p.chests, Mode::Dungeons);
    }

    #[test]
    fn unsupported_values_are_named_not_ignored() {
        let mut s: Settings = Settings::default();
        s.set("shuffle_pots", Value::s("all")).unwrap();
        s.set("shuffle_smallkeys", Value::s("remove")).unwrap();
        s.set("bridge", Value::s("open")).unwrap();
        s.set("starting_hearts", Value::Int(5)).unwrap();
        let e: String = plan(&s).err().unwrap().to_string();
        assert!(e.contains("shuffle_pots is all"));
        assert!(e.contains("shuffle_smallkeys is remove"));
        assert!(e.contains("bridge is open"));
        assert!(e.contains("starting_hearts is 5"));
    }

    #[test]
    fn small_keys_can_roam_and_the_logic_is_told_to_count_them_strictly() {
        let mut s: Settings = Settings::default();
        assert!(!plan(&s).unwrap().keys_roam());
        assert!(plan(&s).unwrap().logic().is_empty());
        for v in ["any_dungeon", "keysanity"] {
            s.set("shuffle_smallkeys", Value::s(v)).unwrap();
            let p: Plan = plan(&s).unwrap();
            assert!(p.keys_roam());
            assert_eq!(p.logic().len(), 3);
            assert!(p.logic().iter().all(|(k, v)| k.ends_with("_key_layout") && *v == "shuffled"));
            assert!(p.notes.iter().any(|n| n.contains("Small keys can be found")));
        }
        s.set("shuffle_smallkeys", Value::s("overworld")).unwrap();
        assert!(plan(&s).is_err());
        s.set("shuffle_smallkeys", Value::s("dungeon")).unwrap();
        s.set("shuffle_bosskeys", Value::s("keysanity")).unwrap();
        assert!(plan(&s).err().unwrap().to_string().contains("shuffle_bosskeys is keysanity"));
    }

    #[test]
    fn the_forest_setting_becomes_start_flags_and_a_logic_value() {
        let mut s: Settings = Settings::default();
        assert!(plan(&s).unwrap().start_flags().is_empty());
        s.set("open_forest", Value::s("closed_deku")).unwrap();
        let p: Plan = plan(&s).unwrap();
        assert_eq!(p.start_flags(), vec![(EXIT_WALL.to_string(), false)]);
        assert_eq!(p.logic(), vec![("open_forest", "closed_deku")]);
        s.set("open_forest", Value::s("open")).unwrap();
        let p: Plan = plan(&s).unwrap();
        assert_eq!(p.start_flags().len(), 2);
        assert!(p.notes.iter().any(|n| n.contains("Kokiri Forest starts open")));
    }

    #[test]
    fn starting_sword_shield_and_slingshot_are_accepted() {
        let mut s: Settings = Settings::default();
        s.set("starting_items", Value::set(&["slingshot", "kokiri_sword", "deku_shield"])).unwrap();
        let p: Plan = plan(&s).unwrap();
        assert_eq!(p.gear, vec!["Slingshot", "Kokiri Sword", "Deku Shield"]);
        assert_eq!(p.pouch, vec!["Weapon_Range_SlingShot", "Weapon_Sword_Kokiri", "Weapon_Shield_Deku"]);
        s.set("starting_items", Value::set(&["ocarina"])).unwrap();
        assert!(plan(&s).err().unwrap().to_string().contains("ocarina"));
    }

    fn excluded_locations_pass_through() {
        let mut s: Settings = Settings::default();
        s.set("disabled_locations", Value::set(&["ZD Chest"])).unwrap();
        assert_eq!(plan(&s).unwrap().excluded, vec!["ZD Chest".to_string()]);
    }
}
