use crate::error::{Error as E, Res};
use crate::settings::{Settings, Value};
use crate::slice::dungeon_items as di;
use crate::slice::pickups as pu;
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
    Overworld,
    Regional,
    Start,
}

impl Keep {
    pub fn roams(self) -> bool {
        matches!(self, Keep::AnyDungeon | Keep::Anywhere | Keep::Overworld | Keep::Regional)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forest {
    Closed,
    ClosedDeku,
    Open,
}

/// Whether King Zora blocks the way to Zora's Fountain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fountain {
    Closed,
    Adult,
    Open,
}

pub const MIDO_WALL: &str = "KokiriForest_Npc_Kokiri_Mido_Child_Blocker";
pub const EXIT_WALL: &str = "KokiriForest_Npc_Kokiri_007_Child_Blocker_00";
pub const DOOR_OPENED: &str = "Market_Child_TimeTempleDoor_Opened";

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
    pub extra: bool,
    pub trade: bool,
    pub scrubs: bool,
    pub scrubs_b: bool,
    pub hearts: bool,
    pub world: Mode,
    pub songs: Songs,
    pub hints: bool,
    pub fast: bool,
    pub intro: bool,
    pub start: Reward,
    pub forest: Forest,
    pub door: bool,
    pub kakariko: bool,
    pub fountain: Fountain,
    pub gear: Vec<&'static str>,
    pub pouch: Vec<&'static str>,
    pub excluded: Vec<String>,
    pub notes: Vec<String>,
    pub drops: crate::pot_slice::Drops,
    pub skulls: crate::slice::skulls::Mode,
    pub remake: bool,
}

const GEAR: &[(&str, &str, &str)] = &[
    ("kokiri_sword", "Kokiri Sword", "Weapon_Sword_Kokiri"),
    ("deku_shield", "Deku Shield", "Weapon_Shield_Deku"),
    ("slingshot", "Slingshot", "Weapon_Range_SlingShot"),
];

const UNCHANGED: [&str; 21] = [
    "logic_rules",
    "starting_age",
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

const OFF_ONLY: [&str; 1] = ["shuffle_scrubs"];

pub fn show(v: &Value) -> String {
    match v {
        Value::Bool(b) => b.to_string(),
        Value::Str(s) => s.clone(),
        Value::Int(n) => n.to_string(),
        Value::Set(x) => format!("[{}]", x.join(", ")),
    }
}

const SMALL_OK: [&str; 4] = ["vanilla", "dungeon", "any_dungeon", "keysanity"];

fn keep(s: &Settings, id: &str, ok: &[&str], bad: &mut Vec<String>) -> Keep {
    match s.text(id) {
        Ok(v) if ok.contains(&v) => di::keep_value(v),
        Ok(v) => {
            bad.push(format!("{id} is {v}, which this build cannot place yet (only {} work)", di::listed(ok)));
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
        let mut out: Vec<(String, bool)> = match self.forest {
            Forest::Closed => Vec::new(),
            Forest::ClosedDeku => vec![(EXIT_WALL.to_string(), false)],
            Forest::Open => vec![(MIDO_WALL.to_string(), false), (EXIT_WALL.to_string(), false)],
        };
        if self.door {
            out.push((DOOR_OPENED.to_string(), true));
        }
        out.extend(di::start_flags(self));
        out
    }

    pub fn logic(&self) -> Vec<(&'static str, &'static str)> {
        let mut out: Vec<(&'static str, &'static str)> = match self.forest {
            Forest::Closed => Vec::new(),
            Forest::ClosedDeku => vec![("open_forest", "closed_deku")],
            Forest::Open => vec![("open_forest", "open")],
        };
        if self.door {
            out.push(("open_door_of_time", "open"));
        }
        if self.kakariko {
            out.push(("open_kakariko", "open"));
            out.push(("skip_child_zelda", "true"));
        }
        match self.fountain {
            Fountain::Closed => {}
            Fountain::Adult => out.push(("zora_fountain", "adult")),
            Fountain::Open => out.push(("zora_fountain", "open")),
        }
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
    let world: Mode = pu::mode(s, &mut bad);
    let drops: crate::pot_slice::Drops = crate::pot_slice::plan::read(s, &mut bad);
    let small: Keep = keep(s, "shuffle_smallkeys", &SMALL_OK, &mut bad);
    let boss: Keep = keep(s, "shuffle_bosskeys", &di::BOSS_OK, &mut bad);
    let map: Keep = keep(s, "shuffle_mapcompass", &di::MAP_OK, &mut bad);
    let ganon: Keep = keep(s, "shuffle_ganon_bosskey", &di::GANON_OK, &mut bad);
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
    let extra: bool = rewards
        && s.flag(crate::flow_extra::SETTING).unwrap_or_else(|e| {
            bad.push(e.to_string());
            false
        });
    let trade: bool = rewards
        && s.flag(crate::flow_extra::trade::SETTING).unwrap_or_else(|e| {
            bad.push(e.to_string());
            false
        });
    let scrubs: bool = crate::scrubs_a::read(s, rewards, &mut bad);
    let scrubs_b: bool = crate::scrubs_b::read(s, rewards, &mut bad);
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
    let door: bool = s.flag("open_door_of_time").unwrap_or_else(|e| {
        bad.push(e.to_string());
        false
    });
    let kakariko: bool = match s.text("open_kakariko") {
        Ok("open") => true,
        Ok("zelda") => false,
        Ok(v) => {
            bad.push(format!("open_kakariko is {v}, which this build cannot apply yet (only open and zelda work)"));
            false
        }
        Err(e) => {
            bad.push(e.to_string());
            false
        }
    };
    let fountain: Fountain = match s.text("zora_fountain") {
        Ok("closed") => Fountain::Closed,
        Ok("adult") => Fountain::Adult,
        Ok("open") => Fountain::Open,
        Ok(v) => {
            bad.push(format!("zora_fountain is {v}, which is not a known choice"));
            Fountain::Closed
        }
        Err(e) => {
            bad.push(e.to_string());
            Fountain::Closed
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
    // Settings that would hand out the same check twice.
    // The schema disables the open gate behind a closed forest (the page greys it out and a
    // settings string decodes it as zelda); a value set directly falls back the same way.
    let kakariko: bool = kakariko && forest != Forest::Closed;
    if kakariko && chosen.iter().any(|i| i == "zeldas_letter") {
        bad.push("starting_items includes zeldas_letter while open_kakariko is open, which already hands out that check at new game; drop one of them".to_string());
    }
    if kakariko && s.items("starting_songs").is_ok_and(|v| v.iter().any(|x| x == "zeldas_lullaby")) {
        bad.push("starting_songs includes zeldas_lullaby while open_kakariko is open, which already hands out that check at new game; drop one of them".to_string());
    }
    let skulls: crate::slice::skulls::Mode = crate::slice::skulls::mode(s, &mut bad);
    let remake: bool = crate::slice::remake_chests::mode(s, &mut bad);
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
        (world != Mode::Off, pu::NOTE),
        (fast, "Chest and item pickup scenes are sped up."),
        (intro, "The new game intro is skipped. The opening movie and the wake up scene are bypassed, and the harmless first-visit scenes of the areas are marked as played, experimental and untested in the game."),
        (start != Reward::Off, "You start the game with one random Stone or Medallion. Experimental and untested in the game."),
        (!pouch.is_empty(), "The ticked starting gear is added to the pouch when a new game starts. A copy can still be found in the world."),
        (small.roams(), "Small keys can be found in other dungeons or outside them. A chest key is counted for the dungeon it belongs to and the chest says which. Experimental and untested in the game."),
        (forest != Forest::Closed, "Kokiri Forest starts open. The blocker walls are switched off when a new game starts, experimental and untested in the game."),
        (kakariko, "The Kakariko gate starts open and the castle visit is skipped. A new game sets the castle quest complete, the Door of Time quest past Impa's send off, the Fire Stone quest past the gate and the gate guard's flag, and gives the items of Zelda's Letter and the Song from Impa. Experimental and untested in the game."),
        (fountain == Fountain::Open, "Zora's Fountain starts open. A new game sets the Water Stone quest to the state after Ruto's Letter was shown, so King Zora already stands aside, and marks his moving scene as played, which removes the wall in front of the passage. Ruto's Letter can still be found and stays a bottle. Experimental and untested in the game."),
        (fountain == Fountain::Adult, "Zora's Fountain counts as open for the adult in the logic only. The game is not changed: the adult Zora's Domain has no wall or quest check in front of the passage in the game data."),
        (door, "The Door of Time starts open. The Temple of Time logic is edited so the door is always open and the Master Sword can be drawn without the Door of Time quest, and the door's opened flag is set when a new game starts, so the Ocarina and the Song of Time are not needed to reach the Master Sword. The quest itself is left alone, so Zelda's escape and the Song of Time stay checks, and the adult pedestal works before the Forest Sage so you can return to childhood at any time. Experimental and untested in the game."),
        (hints, "Gossip stones give hints. Their text is replaced in every language, experimental and untested in the game."),
        (extra, "Kak 20 Gold Skulltula Reward, DMT Biggoron and GC Medigoron are shuffled with structural event edits. Experimental and untested in the game."),
        (trade, "The ten adult trade quest hand ins each give one extra shuffled item the first time. The trade chain itself stays vanilla. Experimental and untested in the game."),
    ] {
        if on {
            notes.push(text.to_string());
        }
    }
    notes.extend(di::notes(boss, ganon, map));
    notes.extend(drops.notes());
    if skulls != crate::slice::skulls::Mode::Off {
        notes.push(crate::slice::skulls::NOTE.to_string());
    }
    if scrubs {
        notes.push(crate::scrubs_a::NOTE.to_string());
    }
    if scrubs_b {
        notes.push(crate::scrubs_b::NOTE.to_string());
    }
    if remake {
        notes.push(crate::slice::remake_chests::NOTE.to_string());
    }
    Ok(Plan { chests, small, boss, map, ganon, rewards, extra, trade, scrubs, scrubs_b, hearts, world, songs, hints, fast, intro, start, forest, door, kakariko, fountain, gear, pouch, excluded, notes, drops, skulls, remake })
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
        assert_eq!(p.notes.len(), 5);
        assert!(p.notes.iter().any(|n| n.contains("bridge=medallions")));
        assert!(p.notes.iter().any(|n| n.contains("Kokiri Forest starts open")));
        assert!(p.notes.iter().any(|n| n.contains("intro is skipped")));
        assert!(p.notes.iter().any(|n| n.contains("Door of Time starts open")));
        assert!(p.notes.iter().any(|n| n.contains("Kakariko gate starts open")));
    }

    #[test]
    fn the_kakariko_setting_becomes_logic_values_and_a_note() {
        let mut s: Settings = Settings::default();
        s.set("open_door_of_time", Value::Bool(false)).unwrap();
        s.set("open_forest", Value::s("closed_deku")).unwrap();
        let p: Plan = plan(&s).unwrap();
        assert!(p.kakariko);
        assert_eq!(p.logic(), vec![("open_forest", "closed_deku"), ("open_kakariko", "open"), ("skip_child_zelda", "true")]);
        assert_eq!(p.start_flags(), vec![(EXIT_WALL.to_string(), false)]);
        s.set("open_kakariko", Value::s("zelda")).unwrap();
        let p: Plan = plan(&s).unwrap();
        assert!(!p.kakariko);
        assert_eq!(p.logic(), vec![("open_forest", "closed_deku")]);
        assert!(!p.notes.iter().any(|n| n.contains("Kakariko")));
        s.set("open_kakariko", Value::s("closed")).unwrap();
        assert!(plan(&s).err().unwrap().to_string().contains("open_kakariko is closed"));
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
        s.set("shuffle_scrubs", Value::s("low")).unwrap();
        s.set("shuffle_smallkeys", Value::s("remove")).unwrap();
        s.set("bridge", Value::s("open")).unwrap();
        s.set("starting_hearts", Value::Int(5)).unwrap();
        let e: String = plan(&s).err().unwrap().to_string();
        assert!(e.contains("shuffle_scrubs is low"));
        assert!(e.contains("shuffle_smallkeys is remove"));
        assert!(e.contains("bridge is open"));
        assert!(e.contains("starting_hearts is 5"));
    }

    #[test]
    fn small_keys_can_roam_and_the_logic_is_told_to_count_them_strictly() {
        let mut s: Settings = Settings::default();
        s.set("open_door_of_time", Value::Bool(false)).unwrap();
        s.set("open_kakariko", Value::s("zelda")).unwrap();
        s.set("open_forest", Value::s("closed")).unwrap();
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
        s.set("shuffle_ganon_bosskey", Value::s("medallions")).unwrap();
        assert!(plan(&s).err().unwrap().to_string().contains("shuffle_ganon_bosskey is medallions"));
    }

    #[test]
    fn the_door_of_time_setting_becomes_a_start_flag_and_a_logic_value() {
        let mut s: Settings = Settings::default();
        s.set("open_kakariko", Value::s("zelda")).unwrap();
        s.set("open_forest", Value::s("closed")).unwrap();
        let p: Plan = plan(&s).unwrap();
        assert_eq!(p.start_flags(), vec![(DOOR_OPENED.to_string(), true)]);
        assert_eq!(p.logic(), vec![("open_door_of_time", "open")]);
        assert!(p.notes.iter().any(|n| n.contains("Door of Time starts open")));
        s.set("open_door_of_time", Value::Bool(false)).unwrap();
        let p: Plan = plan(&s).unwrap();
        assert!(p.start_flags().is_empty());
        assert!(p.logic().is_empty());
        assert!(!p.notes.iter().any(|n| n.contains("Door of Time")));
    }

    #[test]
    fn a_closed_forest_turns_the_open_kakariko_gate_back_to_the_letter() {
        let mut s: Settings = Settings::default();
        assert!(plan(&s).unwrap().kakariko);
        s.set("open_forest", Value::s("closed")).unwrap();
        let p: Plan = plan(&s).unwrap();
        assert!(!p.kakariko);
        assert!(!p.logic().iter().any(|(k, _)| *k == "open_kakariko"));
        let back: Settings = Settings::decode(&s.encode().unwrap()).unwrap().settings;
        assert_eq!(back.get("open_kakariko"), Some(&Value::s("zelda")));
    }

    #[test]
    fn starting_with_zeldas_letter_next_to_an_open_kakariko_is_refused() {
        let mut s: Settings = Settings::default();
        s.set("starting_items", Value::set(&["zeldas_letter"])).unwrap();
        let e: String = plan(&s).unwrap_err().to_string();
        assert!(e.contains("zeldas_letter while open_kakariko"), "{e}");
    }

    #[test]
    fn the_forest_setting_becomes_start_flags_and_a_logic_value() {
        let mut s: Settings = Settings::default();
        s.set("open_door_of_time", Value::Bool(false)).unwrap();
        s.set("open_kakariko", Value::s("zelda")).unwrap();
        s.set("open_forest", Value::s("closed")).unwrap();
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
