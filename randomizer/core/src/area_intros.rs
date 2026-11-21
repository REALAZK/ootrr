pub const SETTING: &str = "skip_area_intros";
pub const NUM: u16 = 149;
pub const NOTE: &str = "The short scene that shows off an area the first time you enter it is skipped, for the overworld areas and the dungeon entrances that only show the place. Scenes that start or advance a quest keep playing.";

pub const PLAYED: &str = "HavePlayedEvent.";
pub const VISIT: &str = "IsVisitLocation.";

pub const INTROS: [(&str, Option<&str>); 25] = [
    ("Dm_DeathMtTrail_0000", Some("DeathMountainTrail")),
    ("Dm_DeathMtCrater_0000", Some("DeathMountainCrater")),
    ("Dm_DeathMtCrater_0000_FromTop", None),
    ("Dm_DodongoCave_0000", Some("DodongoCavern")),
    ("Dm_FireTemple_0000", None),
    ("Dm_GanonCastle_0000", Some("GanonCastle")),
    ("Dm_InsideGanonCastle_0000", None),
    ("Dm_GerudoValley_0000", Some("GerudoValley")),
    ("Dm_Gerudo_0000", Some("GerudoFortress")),
    ("Dm_Goron_0000", Some("GoronCity")),
    ("Dm_Graveyard_0000", Some("Graveyard")),
    ("Dm_HrlCastle_0000", Some("HyruleCastle")),
    ("Dm_Kakariko_0000", Some("KakarikoVillage")),
    ("Dm_LakeHrl_0000", Some("LakeHylia")),
    ("Dm_SacredForest_0000", None),
    ("Dm_ZoraFountain_0000", Some("ZoraFountain")),
    ("Goto_FireTemple", Some("FireTemple")),
    ("Goto_GerudoTrainingGround", Some("GerudoTrainingGround")),
    ("Goto_IceCave", Some("IceCavern")),
    ("Goto_KakarikoWells", Some("BottomOfTheWell")),
    ("Goto_SacredForest", Some("SacredForestMeadow")),
    ("Goto_ShadowTemple", Some("ShadowTemple")),
    ("Goto_SpritTemple", Some("SpiritTemple")),
    ("Goto_WaterTemple", Some("WaterTemple")),
    ("Goto_ZoraRiver", Some("ZoraRiver")),
];

pub fn start_flags() -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    for (scene, area) in INTROS {
        out.push((format!("{PLAYED}{scene}"), true));
        if let Some(a) = area {
            let n: String = format!("{VISIT}{a}");
            if !out.iter().any(|(f, _)| *f == n) {
                out.push((n, true));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> String {
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/flags.json")).unwrap()
    }

    #[test]
    fn the_flag_list_is_every_intro_and_each_area_once() {
        let f: Vec<(String, bool)> = start_flags();
        assert!(f.iter().all(|(_, v)| *v));
        let played: usize = f.iter().filter(|(n, _)| n.starts_with(PLAYED)).count();
        let seen: usize = f.iter().filter(|(n, _)| n.starts_with(VISIT)).count();
        assert_eq!(played, INTROS.len());
        assert_eq!(seen, 21);
        assert_eq!(f.len(), 46);
        let mut names: Vec<&String> = f.iter().map(|(n, _)| n).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), f.len());
        assert!(f.contains(&("HavePlayedEvent.Dm_Kakariko_0000".to_string(), true)));
        assert!(f.contains(&("IsVisitLocation.IceCavern".to_string(), true)));
        assert!(f.contains(&("HavePlayedEvent.Goto_SpritTemple".to_string(), true)));
    }

    #[test]
    fn every_flag_is_a_game_save_bool() {
        let r: String = rows();
        for (n, _) in start_flags() {
            let row: &str = r.split('\n').find(|l| l.contains(&format!("\"name\":\"{n}\""))).unwrap_or_else(|| panic!("{n} is not a game flag"));
            assert!(row.contains("\"type\":\"Bool\"") && row.contains("\"save\":0"), "{n}: {row}");
        }
    }

    #[test]
    fn no_flag_is_one_the_story_skip_sets_or_clears() {
        let mut theirs: Vec<String> = Vec::new();
        theirs.extend(crate::story_skip::DEKU_FLAGS.iter().map(|n| n.to_string()));
        theirs.extend(crate::story_skip::TEMPLE_FLAGS.iter().map(|n| n.to_string()));
        theirs.extend(crate::story_skip::OWL_FLAGS.iter().map(|n| n.to_string()));
        theirs.extend(crate::start_state::KAKARIKO_FLAGS.iter().map(|(n, _)| n.to_string()));
        for (n, _) in start_flags() {
            assert!(!theirs.contains(&n), "{n}");
        }
        for keep in ["Dm_TimeTemple_0000", "Dm_ZoraRiver_0000", "Dm_HrlField_0000", "Dm_Kokiri_0045", "Dm_Deku_0000", "Dm_Zora_0000", "Dm_DesertColossus_0000", "Dm_ForestTemple_0000", "Dm_LonLon_0010", "Dm_WaterTemple_0000"] {
            assert!(!INTROS.iter().any(|(s, _)| *s == keep), "{keep}");
        }
    }

    #[test]
    fn every_scene_the_intro_skip_marks_is_here_unless_the_story_skip_owns_it() {
        for n in crate::start_state::FIRST_VISIT {
            let theirs: bool = crate::story_skip::TEMPLE_FLAGS.contains(&format!("{PLAYED}{n}").as_str()) || n == "Dm_ZoraRiver_0000";
            assert!(theirs || INTROS.iter().any(|(s, _)| *s == n), "{n}");
        }
    }

    #[test]
    fn every_dungeon_entrance_intro_is_a_real_event_flow() {
        let p: std::path::PathBuf = ootrr_game::test_romfs::path();
        if !p.is_dir() {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            return;
        }
        let mut rf: crate::romfs::Romfs = crate::romfs::Romfs::open(&p).unwrap();
        for (scene, _) in INTROS {
            if scene.starts_with("Goto_") {
                let flow: &str = if scene == "Goto_SpritTemple" { "Goto_SpiritTemple" } else { scene };
                rf.read_zs(&format!("Event/EventFlow/{flow}.bfevfl.zs")).unwrap_or_else(|e| panic!("{flow}: {e}"));
            }
        }
    }
}
