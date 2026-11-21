use crate::byml::{Byml, Node};
use crate::error::{Error as E, Res};
use crate::slice::edit::Edited;
use crate::slice::skull_edit::SKULL;

pub const SETTING: &str = "skulls_by_day";
pub const NUM: u16 = 203;
pub const KEY: &str = "gold_skulls_ignore_daytime";
pub const ON: &str = "true";
pub const FLAG: &str = "Enemy_Skulltula_Gold_Nocturnal";
pub const NIGHT: &str = "Logic/Module/Gimmick_Nighttime_Skulltula_Gold_Appear/Gimmick_Nighttime_Skulltula_Gold_Appear_05a9.logic.module.ain";
pub const ALL_DAY: &str = "Logic/Module/Gimmick_AllDay_Skulltula/Gimmick_AllDay_Skulltula_115c.logic.module.ain";
pub const RENAMES: [(&str, &str); 2] = [("Enemy_Skulltula_Gold_9a42", "Enemy_Skulltula_Gold_e916"), ("Land_SkulltulaGold_SpiderWeb_A_01_310c", "Land_SkulltulaGold_SpiderWeb_A_01_a32a")];
pub const NOTE: &str = "Skulltulas by day: the Gold Skulltulas that only come out at night are out at any hour, and the logic no longer waits for night. Tested in the game in nearly every scene.";

fn has(raw: &[u8], s: &str) -> bool {
    raw.windows(s.len()).any(|w| w == s.as_bytes())
}

fn named(raw: &[u8]) -> bool {
    has(raw, FLAG) || has(raw, NIGHT)
}

fn renamed(id: &str) -> Option<&'static str> {
    RENAMES.iter().find(|(a, _)| *a == id).map(|(_, b)| *b)
}

pub fn regroup(y: &mut Byml) -> Res<usize> {
    let Some(groups) = y.root.get_mut("AiGroups").and_then(|a| a.as_arr_mut()) else {
        return Ok(0);
    };
    let mut n: usize = 0;
    for g in groups.iter_mut().filter(|g| g.get("Logic").and_then(|l| l.as_str()) == Some(NIGHT)) {
        let refs: &mut Vec<Node> = g.get_mut("References").and_then(|r| r.as_arr_mut()).ok_or(E::Format("skulltulas by day: a night group has no references"))?;
        for r in refs.iter_mut() {
            let id: &str = r.get("Id").and_then(|v| v.as_str()).ok_or(E::Format("skulltulas by day: a night group reference has no id"))?;
            let to: &str = renamed(id).ok_or(E::Format("skulltulas by day: a night group holds another actor, the game files differ"))?;
            r.set("Id", Node::Str(to.to_string()));
            r.set("Path", Node::Str(to.to_string()));
        }
        g.set("Logic", Node::Str(ALL_DAY.to_string()));
        n += 1;
    }
    Ok(n)
}

pub fn wake(y: &mut Byml) -> usize {
    let Some(list) = y.root.get_mut("Actors").and_then(|a| a.as_arr_mut()) else {
        return 0;
    };
    let mut n: usize = 0;
    for a in list.iter_mut().filter(|a| a.get("Gyaml").and_then(|g| g.as_str()) == Some(SKULL)) {
        let Some(d) = a.get_mut("Dynamic") else {
            continue;
        };
        if !d.remove(FLAG) {
            continue;
        }
        n += 1;
        if matches!(d, Node::Map(m) if m.is_empty()) {
            a.remove("Dynamic");
        }
    }
    n
}

pub fn apply(file: &str, raw: &[u8], prev: Option<Edited>, on: bool, woke: &mut usize) -> Res<Option<Edited>> {
    let src: &[u8] = prev.as_ref().map(|e| e.bytes.as_slice()).unwrap_or(raw);
    if !on || !named(src) {
        return Ok(prev);
    }
    let mut y: Byml = Byml::parse(src)?;
    let n: usize = wake(&mut y) + regroup(&mut y)?;
    if n == 0 {
        return Ok(prev);
    }
    *woke += n;
    let bytes: Vec<u8> = y.write()?;
    Ok(Some(match prev {
        Some(mut e) => {
            e.bytes = bytes;
            e
        }
        None => Edited {
            file: file.to_string(),
            old_len: raw.len(),
            bytes,
            hits: Vec::new(),
            used: Vec::new(),
            hearts: Vec::new(),
            wraps: Vec::new(),
            touched: Vec::new(),
        },
    }))
}

pub fn all_woke(on: bool, woke: usize) -> Res<()> {
    if on && woke == 0 {
        return Err(E::Format("skulltulas by day: no night Gold Skulltula was found in the scenes, the game files differ"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(gyaml: &str, dynamic: Option<Vec<(&str, Node)>>) -> Node {
        let mut m: Vec<(String, Node)> = vec![("Gyaml".to_string(), Node::Str(gyaml.to_string()))];
        if let Some(d) = dynamic {
            m.push(("Dynamic".to_string(), Node::Map(d.into_iter().map(|(k, v)| (k.to_string(), v)).collect())));
        }
        Node::Map(m)
    }

    fn raw(list: Vec<Node>) -> Vec<u8> {
        Byml { root: Node::Map(vec![("Actors".to_string(), Node::Arr(list))]), version: 7, big: false }.write().unwrap()
    }

    #[test]
    fn night_skulltulas_lose_the_night_flag_and_keep_the_rest() {
        let b: Vec<u8> = raw(vec![
            actor(SKULL, Some(vec![(FLAG, Node::Bool(true))])),
            actor(SKULL, Some(vec![(FLAG, Node::Bool(true)), ("Enemy_Skulltula_Gold_AppearPopOut", Node::Bool(true))])),
            actor(SKULL, None),
            actor("Enemy_Guay", Some(vec![(FLAG, Node::Bool(true))])),
        ]);
        let mut woke: usize = 0;
        assert!(apply("Banc/A/a.bcett.byml.zs", &b, None, false, &mut woke).unwrap().is_none());
        let e: Edited = apply("Banc/A/a.bcett.byml.zs", &b, None, true, &mut woke).unwrap().unwrap();
        assert_eq!(woke, 2);
        let y: Byml = Byml::parse(&e.bytes).unwrap();
        let a: &[Node] = y.root.get("Actors").unwrap().as_arr().unwrap();
        assert!(a[0].get("Dynamic").is_none());
        assert!(a[1].get("Dynamic").unwrap().get(FLAG).is_none());
        assert_eq!(a[1].get("Dynamic").unwrap().get("Enemy_Skulltula_Gold_AppearPopOut"), Some(&Node::Bool(true)));
        assert!(a[2].get("Dynamic").is_none());
        assert_eq!(a[3].get("Dynamic").unwrap().get(FLAG), Some(&Node::Bool(true)));
        assert!(all_woke(true, woke).is_ok() && all_woke(true, 0).is_err() && all_woke(false, 0).is_ok());
    }

    fn group(logic: &str, ids: &[(&str, u64)]) -> Node {
        let refs: Vec<Node> = ids
            .iter()
            .map(|(id, h)| Node::Map(vec![("Id".to_string(), Node::Str(id.to_string())), ("Path".to_string(), Node::Str(id.to_string())), ("Reference".to_string(), Node::U64(*h))]))
            .collect();
        Node::Map(vec![("Hash".to_string(), Node::U64(7)), ("Logic".to_string(), Node::Str(logic.to_string())), ("References".to_string(), Node::Arr(refs))])
    }

    #[test]
    fn night_groups_run_the_all_day_module_on_the_same_actors() {
        let night: Node = group(NIGHT, &[("Enemy_Skulltula_Gold_9a42", 11), ("Land_SkulltulaGold_SpiderWeb_A_01_310c", 12)]);
        let other: Node = group("Logic/Module/Gimmick_SwayingTree_Skulltula_Gold_00/x.logic.module.ain", &[("Enemy_Skulltula_Gold_c079", 13)]);
        let y0: Byml = Byml { root: Node::Map(vec![("Actors".to_string(), Node::Arr(Vec::new())), ("AiGroups".to_string(), Node::Arr(vec![night, other.clone()]))]), version: 7, big: false };
        let b: Vec<u8> = y0.write().unwrap();
        let mut woke: usize = 0;
        let e: Edited = apply("Banc/A/a.bcett.byml.zs", &b, None, true, &mut woke).unwrap().unwrap();
        assert_eq!(woke, 1);
        let y: Byml = Byml::parse(&e.bytes).unwrap();
        let gs: &[Node] = y.root.get("AiGroups").unwrap().as_arr().unwrap();
        assert_eq!(gs[0].get("Logic").and_then(|v| v.as_str()), Some(ALL_DAY));
        let rs: &[Node] = gs[0].get("References").unwrap().as_arr().unwrap();
        assert_eq!(rs[0].get("Id").and_then(|v| v.as_str()), Some("Enemy_Skulltula_Gold_e916"));
        assert_eq!(rs[0].get("Path").and_then(|v| v.as_str()), Some("Enemy_Skulltula_Gold_e916"));
        assert_eq!(rs[0].get("Reference"), Some(&Node::U64(11)));
        assert_eq!(rs[1].get("Id").and_then(|v| v.as_str()), Some("Land_SkulltulaGold_SpiderWeb_A_01_a32a"));
        assert_eq!(gs[1], other);
        let odd: Node = group(NIGHT, &[("Enemy_Skulltula_Gold_ffff", 1)]);
        let b2: Vec<u8> = Byml { root: Node::Map(vec![("AiGroups".to_string(), Node::Arr(vec![odd]))]), version: 7, big: false }.write().unwrap();
        assert!(apply("Banc/A/a.bcett.byml.zs", &b2, None, true, &mut woke).is_err());
    }

    #[test]
    fn every_night_skulltula_in_the_game_is_woken() {
        let p: std::path::PathBuf = match std::env::var("OOTRR_ROMFS") {
            Ok(v) => std::path::PathBuf::from(v),
            Err(_) => return,
        };
        if !p.is_dir() {
            return;
        }
        let mut rf: crate::romfs::Romfs = crate::romfs::Romfs::open(&p).unwrap();
        let mut groups: usize = 0;
        let mut flags: usize = 0;
        for f in crate::placement::banc_files(&rf).unwrap() {
            let raw: Vec<u8> = rf.read_zs(&f).unwrap();
            if !named(&raw) {
                continue;
            }
            let mut y: Byml = Byml::parse(&raw).unwrap();
            flags += wake(&mut y);
            groups += regroup(&mut y).unwrap();
            let b: Vec<u8> = y.write().unwrap();
            assert!(!has(&b, FLAG) && !has(&b, NIGHT), "{f}");
        }
        assert_eq!(groups, 30);
        assert!(flags >= groups, "{flags}");
    }

    #[test]
    fn the_setting_turns_on_the_daytime_logic_key() {
        let mut s: crate::settings::Settings = crate::settings::Settings::default();
        let off: crate::slice::support::Plan = crate::slice::support::plan(&s).unwrap();
        assert!(!off.day_skulls && !off.logic().contains(&(KEY, ON)));
        s.set(SETTING, crate::settings::Value::Bool(true)).unwrap();
        let on: crate::slice::support::Plan = crate::slice::support::plan(&s).unwrap();
        assert!(on.day_skulls && on.logic().contains(&(KEY, ON)));
        assert!(on.notes.iter().any(|n| n == NOTE));
    }

    #[test]
    fn a_file_without_night_skulltulas_is_left_alone() {
        let b: Vec<u8> = raw(vec![actor(SKULL, None)]);
        let mut woke: usize = 0;
        assert!(apply("Banc/A/a.bcett.byml.zs", &b, None, true, &mut woke).unwrap().is_none());
        assert_eq!(woke, 0);
    }
}
