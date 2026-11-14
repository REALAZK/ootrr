use crate::settings::Settings;

pub const SETTING: &str = "shuffle_remake_chests";

pub const NOTE: &str = "Remake only chests and the newly paired dungeon and overworld chests are shuffled.";

pub const NEW: [&str; 10] = [
    "Dodongos Cavern Lobby Bombable Chest",
    "Dodongos Cavern Staircase Top Bombable Chest",
    "Dodongos Cavern Lower Lizalfos Chest",
    "Dodongos Cavern Upper Lizalfos Chest",
    "Fire Temple Scarecrow Ledge Second Chest",
    "Fire Temple Scarecrow Ledge Third Chest",
    "Gerudo Training Ground Heavy Block Fifth Chest",
    "Ice Cavern Push Block Room Chest",
    "Jabu Jabus Belly Basement 2 Octoroks Chest",
    "Water Temple Compass Room Chest",
];

pub const PAIRED: [&str; 29] = [
    "Dodongos Cavern End of Bridge Chest",
    "Dodongos Cavern Bomb Flower Platform Chest",
    "Fire Temple Scarecrow Chest",
    "Fire Temple Big Lava Room Blocked Door Chest",
    "Gerudo Training Ground Maze Path First Chest",
    "Gerudo Training Ground Maze Path Second Chest",
    "Gerudo Training Ground Maze Path Third Chest",
    "Gerudo Training Ground Maze Right Central Chest",
    "Gerudo Training Ground Maze Right Side Chest",
    "Gerudo Training Ground Near Scarecrow Chest",
    "Gerudo Training Ground Heavy Block First Chest",
    "Gerudo Training Ground Heavy Block Second Chest",
    "Gerudo Training Ground Heavy Block Third Chest",
    "Gerudo Training Ground Eye Statue Chest",
    "Gerudo Training Ground Freestanding Key",
    "Ganons Castle Light Trial First Left Chest",
    "Ganons Castle Light Trial Second Right Chest",
    "Ganons Castle Light Trial Third Left Chest",
    "Ganons Castle Light Trial Invisible Enemies Chest",
    "Ganons Castle Light Trial Lullaby Chest",
    "Shadow Temple Invisible Blades Visible Chest",
    "Shadow Temple Invisible Blades Invisible Chest",
    "Shadow Temple Falling Spikes Lower Chest",
    "Shadow Temple Wind Hint Chest",
    "Spirit Temple Child Climb North Chest",
    "Spirit Temple Statue Room Hand Chest",
    "Spirit Temple Near Four Armos Chest",
    "Water Temple Central Pillar Chest",
    "Water Temple Torches Chest",
];

pub const OFF: &str = "remake chest shuffle is off";

pub fn gated(name: &str) -> bool {
    NEW.contains(&name) || PAIRED.contains(&name) || crate::slice::remake_overworld::gated(name)
}

pub fn mode(s: &Settings, bad: &mut Vec<String>) -> bool {
    match s.text(SETTING) {
        Ok("off") => false,
        Ok("on") => true,
        Ok(v) => {
            bad.push(format!("{SETTING} is {v}, which is not a known choice"));
            false
        }
        Err(e) => {
            bad.push(e.to_string());
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::explore;
    use crate::settings::Value;
    use crate::slice::data::Data;
    use crate::slice::fill::vanilla_place;
    use crate::slice::logic::{world, world_with};
    use crate::slice::pick::pick;
    use crate::slice::support::{plan, Plan};
    use crate::world::{LocId, Placement, World};

    fn on() -> Settings {
        let mut s: Settings = Settings::default();
        s.set(SETTING, Value::s("on")).unwrap();
        s
    }

    #[test]
    fn every_gated_name_is_a_catalog_chest_and_a_logic_location() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        for n in NEW.iter().chain(PAIRED.iter()) {
            assert!(d.cat.by_name(n).is_some_and(|c| c.kind == "chest"), "{n}");
            assert!(w.loc(n).is_some(), "{n}");
        }
        for n in NEW {
            assert!(d.cat.by_name(n).is_some_and(|c| c.ev.iter().any(|e| e == "new")), "{n}");
        }
    }

    #[test]
    fn off_keeps_every_gated_chest_vanilla_and_on_frees_some() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let p: Plan = plan(&Settings::default()).unwrap();
        assert!(!p.remake);
        let off: Vec<String> = pick(&d, &w, &p).free.iter().map(|l| w.locs[*l].name.clone()).collect();
        assert!(off.iter().all(|n| !gated(n)));
        let q: Plan = plan(&on()).unwrap();
        assert!(q.remake);
        assert!(q.notes.iter().any(|n| n == NOTE));
        let got: Vec<String> = pick(&d, &w, &q).free.iter().map(|l| w.locs[*l].name.clone()).collect();
        assert!(got.iter().filter(|n| NEW.contains(&n.as_str())).count() >= 5);
        assert!(got.iter().any(|n| PAIRED.contains(&n.as_str())));
    }

    #[test]
    fn unknown_values_are_refused() {
        let mut s: Settings = Settings::default();
        assert!(s.set(SETTING, Value::s("maybe")).is_err() || plan(&s).is_err());
    }

    #[test]
    fn the_vanilla_world_with_the_new_chests_reaches_the_goal() {
        let d: Data = Data::load().unwrap();
        let w: World = world_with(&d, &[]).unwrap();
        let p: Placement = vanilla_place(&w, &d).unwrap();
        assert!(explore(&w, &p, &[]).won());
        for n in NEW {
            let l: LocId = w.loc(n).unwrap();
            assert!(explore(&w, &p, &[]).loc_access(l), "{n}");
        }
    }
}
