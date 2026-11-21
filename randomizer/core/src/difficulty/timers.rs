use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::flow_locations::dict_id;
use crate::qol::Have;
use crate::romfs::Romfs;
use crate::staging::{Out, BANC_DICT};
use crate::zs;

pub const SETTING: &str = "shorter_timers";
pub const NUM: u16 = 147;
pub const DYNAMIC: &str = "Dynamic";
pub const ACTORS: &str = "Actors";
pub const GYAML: &str = "Gyaml";
pub const PERMANENT: f32 = 100.0;

pub const TORCH: &str = "Obj_CandleStick_TimeExtinguish_A_01";
pub const ROOM: &str = "DefaultDungeonRoomIndex";
pub const TRANSLATE: &str = "Translate";
pub const RUN_SPEED: f32 = 4.5;
pub const LIGHT_TIME: f32 = 1.0;
pub const MARGIN: f32 = 1.5;

pub struct Rule {
    pub gyaml: &'static str,
    pub key: &'static str,
    pub floor: f32,
    pub default: Option<f32>,
}

pub const RULES: [Rule; 3] = [
    Rule { gyaml: "Obj_StepSwitch_A_02", key: "CountTime", floor: 10.0, default: None },
    Rule { gyaml: "Obj_EyeSwitch_A_05", key: "LimitTime", floor: 6.0, default: None },
    Rule { gyaml: TORCH, key: "ExtinguishTime", floor: 4.0, default: Some(15.0) },
];

pub const MAPS: [&str; 5] = ["InsideTheDekuTree", "DodongoCavern", "FireTemple", "GerudoTrainingGround", "InsideGanonCastle"];

pub const NOTE: &str = "Timed floor switches, timed eye switches and the torches that burn out in the Deku Tree, Dodongo's Cavern, the Fire Temple, the Gerudo Training Ground and Ganon's Castle give you less time, but never less than a floor each puzzle still allows.";

pub fn file(map: &str) -> String {
    format!("Banc/{map}/{map}_Static.bcett.byml.zs")
}

pub fn files() -> Vec<String> {
    MAPS.iter().map(|m| file(m)).collect()
}

pub fn percent(choice: &str) -> Option<i64> {
    match choice {
        "three_quarters" => Some(75),
        "half" => Some(50),
        _ => None,
    }
}

pub fn scale(v: f32, pct: i64, floor: f32) -> f32 {
    let want: f32 = ((v * pct as f32 / 100.0) * 100.0).round() / 100.0;
    let least: f32 = if v < floor { v } else { floor };
    if want < least { least } else { want }
}

fn spot(a: &Node) -> Option<(f32, f32)> {
    let t: &[Node] = a.get(TRANSLATE)?.as_arr()?;
    match (t.first(), t.get(2)) {
        (Some(Node::F32(x)), Some(Node::F32(z))) => Some((*x, *z)),
        _ => None,
    }
}

fn room(a: &Node) -> Option<i32> {
    match a.get(DYNAMIC)?.get(ROOM)? {
        Node::I32(r) => Some(*r),
        _ => None,
    }
}

pub fn path_length(p: &[(f32, f32)]) -> f32 {
    let mut best: f32 = f32::MAX;
    for s in 0..p.len() {
        let mut left: Vec<(f32, f32)> = p.to_vec();
        let mut at: (f32, f32) = left.remove(s);
        let mut total: f32 = 0.0;
        while !left.is_empty() {
            let (i, d): (usize, f32) = left.iter().enumerate().map(|(i, q)| (i, ((q.0 - at.0).powi(2) + (q.1 - at.1).powi(2)).sqrt())).fold((0, f32::MAX), |m, c| if c.1 < m.1 { c } else { m });
            total += d;
            at = left.remove(i);
        }
        if total < best {
            best = total;
        }
    }
    if best == f32::MAX { 0.0 } else { best }
}

pub fn torch_floor(p: &[(f32, f32)]) -> f32 {
    let need: f32 = path_length(p) / RUN_SPEED + LIGHT_TIME * p.len() as f32 + MARGIN;
    (need * 2.0).ceil() / 2.0
}

fn torch_floors(acts: &[Node]) -> Vec<(i32, f32)> {
    let mut rooms: Vec<(i32, Vec<(f32, f32)>)> = Vec::new();
    for a in acts {
        if a.get(GYAML).and_then(|g| g.as_str()) != Some(TORCH) {
            continue;
        }
        let (Some(r), Some(p)) = (room(a), spot(a)) else {
            continue;
        };
        match rooms.iter_mut().find(|e| e.0 == r) {
            Some(e) => e.1.push(p),
            None => rooms.push((r, vec![p])),
        }
    }
    rooms.into_iter().map(|(r, p)| (r, torch_floor(&p))).collect()
}

pub fn shorten(y: &mut Byml, pct: i64) -> Res<usize> {
    let acts: &mut Vec<Node> = y.root.get_mut(ACTORS).and_then(|a| a.as_arr_mut()).ok_or(Error::Format("shorter timers: a map has no actor list"))?;
    let floors: Vec<(i32, f32)> = torch_floors(acts);
    let mut n: usize = 0;
    for a in acts.iter_mut() {
        let g: String = a.get(GYAML).and_then(|g| g.as_str()).unwrap_or("").to_string();
        let Some(r) = RULES.iter().find(|r| r.gyaml == g) else {
            continue;
        };
        let near: f32 = room(a).and_then(|k| floors.iter().find(|f| f.0 == k)).map_or(0.0, |f| f.1);
        let floor: f32 = if r.gyaml == TORCH && near > r.floor { near } else { r.floor };
        let Some(d) = a.get_mut(DYNAMIC) else {
            continue;
        };
        let now: Option<f32> = match d.get(r.key) {
            Some(Node::F32(v)) => Some(*v),
            Some(_) => None,
            None => r.default,
        };
        if let Some(v) = now
            && v < PERMANENT
        {
            let w: f32 = scale(v, pct, floor);
            if d.get(r.key).is_some() || w < v {
                d.set(r.key, Node::F32(w));
                n += 1;
            }
        }
    }
    Ok(n)
}

pub fn stage(rf: &mut Romfs, have: Have, pct: i64) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    for f in files() {
        let z: Vec<u8> = match have(&f) {
            Some(b) => b,
            None => rf.read(&f)?,
        };
        let dict: u32 = dict_id(&z).unwrap_or(BANC_DICT);
        let raw: Vec<u8> = rf.zs.decompress(&z)?;
        let mut y: Byml = Byml::parse(&raw)?;
        if shorten(&mut y, pct)? == 0 {
            return Err(Error::Msg(format!("shorter timers: {f} holds no timer, the game files may have changed")));
        }
        let bytes: Vec<u8> = y.write()?;
        outs.push(Out { path: f, old: raw.len(), new: bytes.len(), z: zs::compress_raw(&bytes, Some(dict))? });
    }
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(g: &str, key: &str, v: f32) -> Node {
        Node::Map(vec![(GYAML.to_string(), Node::Str(g.to_string())), (DYNAMIC.to_string(), Node::Map(vec![(key.to_string(), Node::F32(v))]))])
    }

    fn value(a: &Node, key: &str) -> f32 {
        match a.get(DYNAMIC).and_then(|d| d.get(key)) {
            Some(Node::F32(v)) => *v,
            _ => panic!("no {key}"),
        }
    }

    fn map(list: Vec<Node>) -> Byml {
        Byml { root: Node::Map(vec![(ACTORS.to_string(), Node::Arr(list))]), version: 2, big: false }
    }

    #[test]
    fn scaling_halves_and_keeps_a_floor() {
        assert_eq!(scale(20.0, 50, 10.0), 10.0);
        assert_eq!(scale(20.2, 50, 10.0), 10.1);
        assert_eq!(scale(19.0, 75, 10.0), 14.25);
        assert_eq!(scale(12.95, 50, 10.0), 10.0);
        assert_eq!(scale(8.0, 50, 6.0), 6.0);
        assert_eq!(scale(8.0, 75, 6.0), 6.0);
        assert_eq!(scale(4.0, 50, 6.0), 4.0);
        assert_eq!(scale(1.0, 50, 6.0), 1.0);
        assert_eq!(scale(80.0, 50, 6.0), 40.0);
    }

    #[test]
    fn a_timer_never_grows_and_never_drops_below_the_least_it_allows() {
        for v in [0.5f32, 1.0, 3.0, 6.0, 8.0, 10.0, 12.95, 20.2, 22.0, 80.0] {
            for pct in [75, 50] {
                for floor in [6.0f32, 10.0] {
                    let s: f32 = scale(v, pct, floor);
                    assert!(s <= v + 0.001, "{v} {pct} {floor}");
                    assert!(s >= v.min(floor) - 0.001, "{v} {pct} {floor}");
                }
            }
        }
    }

    #[test]
    fn only_listed_actors_and_keys_change() {
        let mut y: Byml = map(vec![actor("Obj_StepSwitch_A_02", "CountTime", 20.0), actor("Obj_StepSwitch_A_02", "CountTime", 9999.0), actor("Obj_StepSwitch_A_02", "Other", 20.0), actor("Obj_Other", "CountTime", 20.0), actor("Obj_EyeSwitch_A_05", "LimitTime", 10.0)]);
        assert_eq!(shorten(&mut y, 50).unwrap(), 2);
        let l: &[Node] = y.root.get(ACTORS).unwrap().as_arr().unwrap();
        assert_eq!(value(&l[0], "CountTime"), 10.0);
        assert_eq!(value(&l[1], "CountTime"), 9999.0);
        assert_eq!(value(&l[2], "Other"), 20.0);
        assert_eq!(value(&l[3], "CountTime"), 20.0);
        assert_eq!(value(&l[4], "LimitTime"), 6.0);
        assert!(shorten(&mut Byml { root: Node::Map(vec![]), version: 2, big: false }, 50).is_err());
    }

    #[test]
    fn the_choice_maps_to_a_percent() {
        assert_eq!(percent("off"), None);
        assert_eq!(percent("three_quarters"), Some(75));
        assert_eq!(percent("half"), Some(50));
        assert_eq!(files().len(), MAPS.len());
    }

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = ootrr_game::test_romfs::path();
        if p.join("Banc").is_dir() {
            Some(p)
        } else {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            None
        }
    }

    fn timers(y: &Byml) -> Vec<(String, String, Option<f32>)> {
        let mut out: Vec<(String, String, Option<f32>)> = Vec::new();
        for a in y.root.get(ACTORS).and_then(|a| a.as_arr()).unwrap_or(&[]) {
            let g: &str = a.get(GYAML).and_then(|g| g.as_str()).unwrap_or("");
            for r in RULES.iter().filter(|r| r.gyaml == g) {
                let v: Option<f32> = match a.get(DYNAMIC).and_then(|d| d.get(r.key)) {
                    Some(Node::F32(v)) => Some(*v),
                    _ => None,
                };
                out.push((g.to_string(), r.key.to_string(), v));
            }
        }
        out
    }

    #[test]
    fn a_torch_room_floor_grows_with_the_walk_between_torches() {
        assert_eq!(path_length(&[(0.0, 0.0)]), 0.0);
        assert_eq!(path_length(&[(0.0, 0.0), (3.0, 4.0)]), 5.0);
        assert_eq!(path_length(&[(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)]), 20.0);
        assert!(torch_floor(&[(0.0, 0.0)]) < torch_floor(&[(0.0, 0.0), (30.0, 0.0)]));
    }

    #[test]
    fn a_torch_without_a_timer_gets_the_scaled_default() {
        let t = |x: f32, with: bool, room: i32| {
            let mut d: Vec<(String, Node)> = vec![(ROOM.to_string(), Node::I32(room))];
            if with {
                d.push(("ExtinguishTime".to_string(), Node::F32(22.0)));
            }
            Node::Map(vec![(GYAML.to_string(), Node::Str(TORCH.to_string())), (DYNAMIC.to_string(), Node::Map(d)), (TRANSLATE.to_string(), Node::Arr(vec![Node::F32(x), Node::F32(0.0), Node::F32(0.0)]))])
        };
        let mut y: Byml = map(vec![t(0.0, false, 2), t(3.0, false, 2), t(500.0, true, 3)]);
        assert_eq!(shorten(&mut y, 50).unwrap(), 3);
        let l: &[Node] = y.root.get(ACTORS).unwrap().as_arr().unwrap();
        assert_eq!(value(&l[0], "ExtinguishTime"), 7.5);
        assert_eq!(value(&l[2], "ExtinguishTime"), 11.0);
        let mut far: Byml = map(vec![t(0.0, false, 2), t(300.0, false, 2)]);
        shorten(&mut far, 50).unwrap();
        let f: &[Node] = far.root.get(ACTORS).unwrap().as_arr().unwrap();
        assert!(f[0].get(DYNAMIC).unwrap().get("ExtinguishTime").is_none());
    }

    #[test]
    fn the_game_maps_hold_the_timers_and_fifty_percent_respects_the_floors() {
        let Some(root) = dump() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let outs: Vec<Out> = stage(&mut rf, &|_: &str| None, 50).unwrap();
        assert_eq!(outs.len(), MAPS.len());
        let mut changed: usize = 0;
        let mut total: usize = 0;
        for o in &outs {
            let old: Byml = rf.read_byml(&o.path).unwrap();
            let new: Byml = Byml::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap();
            let (a, b): (Vec<(String, String, Option<f32>)>, Vec<(String, String, Option<f32>)>) = (timers(&old), timers(&new));
            assert_eq!(a.len(), b.len(), "{}", o.path);
            for (x, y) in a.iter().zip(&b) {
                let r: &Rule = RULES.iter().find(|r| r.gyaml == x.0).unwrap();
                let was: f32 = x.2.or(r.default).unwrap_or(0.0);
                let now: f32 = y.2.or(r.default).unwrap_or(0.0);
                assert!(now <= was && now >= was.min(r.floor) - 0.001, "{} {} {} -> {}", o.path, x.0, was, now);
                if was < PERMANENT && was > 0.0 {
                    total += 1;
                    if now < was {
                        changed += 1;
                    }
                }
            }
            let mut o2: Byml = old.clone();
            let mut n2: Byml = new.clone();
            for y in [&mut o2, &mut n2] {
                for a in y.root.get_mut(ACTORS).and_then(|a| a.as_arr_mut()).unwrap().iter_mut() {
                    let g: String = a.get(GYAML).and_then(|g| g.as_str()).unwrap_or("").to_string();
                    if let Some(r) = RULES.iter().find(|r| r.gyaml == g)
                        && let Some(d) = a.get_mut(DYNAMIC)
                    {
                        d.remove(r.key);
                    }
                }
            }
            assert_eq!(o2, n2, "{}: only the timers may differ", o.path);
        }
        assert!(total >= 17 && changed >= 12, "{total} {changed}");
    }

    #[test]
    fn the_setting_reads_and_defaults_off() {
        use crate::settings::{Settings, Value};
        let mut bad: Vec<String> = Vec::new();
        let o: crate::difficulty::Opts = crate::difficulty::read(&Settings::default(), &mut bad);
        assert_eq!(o.timers, crate::difficulty::NORMAL);
        assert!(!o.any());
        let mut s: Settings = Settings::default();
        s.set(SETTING, Value::s("half")).unwrap();
        let back: Settings = Settings::decode(&s.encode().unwrap()).unwrap().settings;
        let o: crate::difficulty::Opts = crate::difficulty::read(&back, &mut bad);
        assert!(bad.is_empty() && o.timers == 50 && o.any() && !o.packs());
        assert!(o.notes().contains(&NOTE.to_string()));
    }
}
