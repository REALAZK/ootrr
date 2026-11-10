use crate::error::{Error as E, Res};
use crate::settings::{Settings, Value};

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
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub chests: Mode,
    pub small: Keep,
    pub boss: Keep,
    pub map: Keep,
    pub ganon: Keep,
    pub rewards: bool,
    pub excluded: Vec<String>,
    pub notes: Vec<String>,
}

const UNCHANGED: [&str; 26] = [
    "logic_rules",
    "starting_age",
    "open_forest",
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
    "starting_items",
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

fn keep(s: &Settings, id: &str, bad: &mut Vec<String>) -> Keep {
    match s.text(id) {
        Ok("vanilla") => Keep::Vanilla,
        Ok("dungeon") => Keep::Dungeon,
        Ok(v) => {
            bad.push(format!("{id} is {v}, this slice supports only vanilla and dungeon"));
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

pub fn plan(s: &Settings) -> Res<Plan> {
    let def: Settings = Settings::default();
    let mut bad: Vec<String> = Vec::new();
    let mut noted: Vec<String> = Vec::new();
    for id in UNCHANGED {
        match (s.get(id), def.get(id)) {
            (Some(a), Some(b)) if a == b => noted.push(format!("{id}={}", show(a))),
            (Some(a), _) => bad.push(format!(
                "{id} is {}, this slice does not change the game and accepts only the default",
                show(a)
            )),
            _ => bad.push(format!("no setting named {id}")),
        }
    }
    for id in OFF_ONLY {
        match s.text(id) {
            Ok("off") => {}
            Ok(v) => bad.push(format!("{id} is {v}, this slice shuffles chests only and accepts only off")),
            Err(e) => bad.push(e.to_string()),
        }
    }
    match s.text("shuffle_hideoutkeys") {
        Ok("vanilla") => {}
        Ok(v) => bad.push(format!("shuffle_hideoutkeys is {v}, this slice accepts only vanilla")),
        Err(e) => bad.push(e.to_string()),
    }
    let chests: Mode = chest_mode(s, &mut bad);
    let small: Keep = keep(s, "shuffle_smallkeys", &mut bad);
    let boss: Keep = keep(s, "shuffle_bosskeys", &mut bad);
    let map: Keep = keep(s, "shuffle_mapcompass", &mut bad);
    let ganon: Keep = keep(s, "shuffle_ganon_bosskey", &mut bad);
    let rewards: bool = match s.text("shuffle_rewards") {
        Ok("off") => false,
        Ok("supported") => true,
        Ok(v) => {
            bad.push(format!("shuffle_rewards is {v}, this slice accepts off or supported"));
            false
        }
        Err(e) => {
            bad.push(e.to_string());
            false
        }
    };
    let excluded: Vec<String> = s.items("disabled_locations").map(|v| v.to_vec()).unwrap_or_default();
    if !bad.is_empty() {
        let lines: Vec<String> = bad.iter().map(|b| format!("  {b}")).collect();
        return Err(E::Msg(format!(
            "these settings are not supported by the chest shuffle slice:\n{}",
            lines.join("\n")
        )));
    }
    let mut notes: Vec<String> = Vec::new();
    if !noted.is_empty() {
        notes.push(format!(
            "Not applied by this slice, the game plays as unmodified: {}",
            noted.join(", ")
        ));
    }
    Ok(Plan { chests, small, boss, map, ganon, rewards, excluded, notes })
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
        assert_eq!(p.notes.len(), 1);
        assert!(p.notes[0].contains("bridge=medallions"));
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
        s.set("shuffle_smallkeys", Value::s("keysanity")).unwrap();
        s.set("bridge", Value::s("open")).unwrap();
        s.set("starting_hearts", Value::Int(5)).unwrap();
        let e: String = plan(&s).err().unwrap().to_string();
        assert!(e.contains("shuffle_pots is all"));
        assert!(e.contains("shuffle_smallkeys is keysanity"));
        assert!(e.contains("bridge is open"));
        assert!(e.contains("starting_hearts is 5"));
    }

    #[test]
    fn excluded_locations_pass_through() {
        let mut s: Settings = Settings::default();
        s.set("disabled_locations", Value::set(&["ZD Chest"])).unwrap();
        assert_eq!(plan(&s).unwrap().excluded, vec!["ZD Chest".to_string()]);
    }
}
