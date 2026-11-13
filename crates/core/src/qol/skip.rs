use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::qol::{edit_plain, Have, Skip};
use crate::romfs::Romfs;
use crate::staging::Out;

pub const FILE: &str = "Event/EventSetting/EventSettingComponent/SkipSetting/DisableSkip.engine__event__SkipSetting.bgyml";
pub const LISTS: [&str; 2] = [
    "Event/EventSetting/EventSettingComponentList/Dm_Kokiri_0000.engine__event__EventSettingComponentList.bgyml",
    "Event/EventSetting/EventSettingComponentList/Dm_StaffCredit_0005.engine__event__EventSettingComponentList.bgyml",
];
pub const KEY: &str = "IsDisableSkip";
pub const COMPONENT: &str = "SkipSetting";
const TARGET: &str = "DisableSkip.engine__event__SkipSetting.bgyml";

pub const NOTE_STORY: &str = "The opening movie and the first credits movie can be skipped by the player. The other cutscenes were already skippable and the title screen demos stay locked. Experimental and untested in the game.";
pub const NOTE_ALL: &str = "Every cutscene lock is removed, including the six title screen demos. Experimental and untested in the game.";

pub fn flip(raw: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    match y.root.get(KEY) {
        Some(Node::Bool(true)) => {}
        _ => return Err(Error::Format("skip cutscenes: the skip file does not say IsDisableSkip true")),
    }
    y.root.set(KEY, Node::Bool(false));
    y.write()
}

pub fn unlock(raw: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    let c: &mut Node = y.root.get_mut("Components").ok_or(Error::Format("skip cutscenes: the event list has no Components"))?;
    let hit: bool = c.get(COMPONENT).and_then(|v| v.as_str()).is_some_and(|s| s.ends_with(TARGET));
    if !hit {
        return Err(Error::Format("skip cutscenes: the event list does not use the DisableSkip file"));
    }
    c.remove(COMPONENT);
    y.write()
}

pub fn stage(rf: &Romfs, mode: Skip, have: Have) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    match mode {
        Skip::Off => {}
        Skip::All => outs.push(edit_plain(rf, have, FILE, flip)?),
        Skip::Story => {
            for l in LISTS {
                outs.push(edit_plain(rf, have, l, unlock)?);
            }
        }
    }
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qol::Opts;

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("C:/o/nup dump/updates/131072/romfs"));
        if p.is_dir() {
            Some(p)
        } else {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            None
        }
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    fn list(skip: bool) -> Byml {
        let mut c: Vec<(String, Node)> = vec![("EventPlayEnvSetting".to_string(), Node::Str("E".to_string()))];
        if skip {
            c.push((COMPONENT.to_string(), Node::Str(format!("?Event/EventSetting/EventSettingComponent/SkipSetting/{TARGET}"))));
        }
        Byml {
            root: Node::Map(vec![("$parent".to_string(), Node::Str("P".to_string())), ("Components".to_string(), Node::Map(c))]),
            version: 7,
            big: false,
        }
    }

    #[test]
    fn the_flip_changes_only_the_bool_and_refuses_a_second_run() {
        let y: Byml = Byml { root: Node::Map(vec![(KEY.to_string(), Node::Bool(true))]), version: 7, big: false };
        let raw: Vec<u8> = y.write().unwrap();
        let out: Vec<u8> = flip(&raw).unwrap();
        assert_eq!(out.len(), raw.len());
        let back: Byml = Byml::parse(&out).unwrap();
        assert_eq!(back.root.get(KEY), Some(&Node::Bool(false)));
        assert!(flip(&out).is_err());
    }

    #[test]
    fn unlocking_a_list_removes_only_the_skip_component() {
        let raw: Vec<u8> = list(true).write().unwrap();
        let out: Vec<u8> = unlock(&raw).unwrap();
        let back: Byml = Byml::parse(&out).unwrap();
        let c: &Node = back.root.get("Components").unwrap();
        assert!(c.get(COMPONENT).is_none());
        assert!(c.get("EventPlayEnvSetting").is_some());
        assert!(back.root.get("$parent").is_some());
        assert!(unlock(&out).is_err());
        let mut other: Byml = list(true);
        other.root.get_mut("Components").unwrap().set(COMPONENT, Node::Str("Other.bgyml".to_string()));
        assert!(unlock(&other.write().unwrap()).is_err());
    }

    #[test]
    fn the_dump_file_is_the_one_shared_lock_and_flips_to_false() {
        let Some(root) = dump() else {
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read(FILE).unwrap();
        assert_eq!(raw.len(), 56);
        let y: Byml = Byml::parse(&raw).unwrap();
        assert_eq!(y.root.get(KEY), Some(&Node::Bool(true)));
        assert_eq!(y.write().unwrap(), raw);
        let out: Vec<u8> = flip(&raw).unwrap();
        assert_eq!(out.len(), raw.len());
        let n: usize = raw.iter().zip(&out).filter(|(a, b)| a != b).count();
        assert_eq!(n, 1);
        assert_eq!(Byml::parse(&out).unwrap().root.get(KEY), Some(&Node::Bool(false)));
    }

    #[test]
    fn exactly_eight_events_use_the_lock_and_the_two_story_lists_are_among_them() {
        let Some(root) = dump() else {
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        let dir: &str = "Event/EventSetting/EventSettingComponentList";
        let mut names: Vec<String> = rf.list(dir).unwrap();
        names.sort();
        let suffix: &str = ".engine__event__EventSettingComponentList.bgyml";
        let mut parent: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
        let mut direct: Vec<String> = Vec::new();
        for n in &names {
            let Some(id) = n.rsplit('/').next().and_then(|f| f.strip_suffix(suffix)) else {
                continue;
            };
            let y: Byml = Byml::parse(&rf.read(n).unwrap()).unwrap();
            let p: String = y.root.get("$parent").and_then(|v| v.as_str()).unwrap_or("").rsplit('/').next().unwrap_or("").to_string();
            parent.insert(id.to_string(), p.strip_suffix(".engine__event__EventSettingComponentList.gyml").unwrap_or("").to_string());
            let lock: bool = y.root.get("Components").and_then(|c| c.get(COMPONENT)).and_then(|v| v.as_str()).is_some_and(|s| s.ends_with(TARGET));
            if lock {
                direct.push(id.to_string());
            }
        }
        direct.sort();
        assert_eq!(direct, vec!["Dm_Kokiri_0000".to_string(), "Dm_StaffCredit_0005".to_string(), "Dm_Title_0005".to_string()]);
        let kids: Vec<&String> = parent.iter().filter(|(_, p)| direct.contains(p)).map(|(k, _)| k).collect();
        assert_eq!(kids.len(), 5, "{kids:?}");
        assert!(kids.iter().all(|k| k.starts_with("Dm_Title_")));
        assert_eq!(direct.len() + kids.len(), 8);
    }

    #[test]
    fn staged_outputs_parse_and_leave_the_title_lists_alone() {
        let Some(root) = dump() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let story: Vec<Out> = stage(&rf, Skip::Story, &none).unwrap();
        assert_eq!(story.len(), 2);
        assert!(story.iter().all(|o| o.new < o.old));
        assert!(story.iter().all(|o| o.path != FILE && !o.path.contains("Dm_Title")));
        for o in &story {
            let y: Byml = Byml::parse(&o.z).unwrap();
            assert!(y.root.get("Components").unwrap().get(COMPONENT).is_none());
        }
        let all: Vec<Out> = stage(&rf, Skip::All, &none).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].path, FILE);
        assert_eq!(all[0].old, all[0].new);
        assert!(stage(&rf, Skip::Off, &none).unwrap().is_empty());
        let both: Vec<Out> = crate::qol::stage(&mut rf, &Opts { skip: Skip::Story, ..Opts::default() }, &none).unwrap();
        assert_eq!(both.len(), 2);
    }
}
