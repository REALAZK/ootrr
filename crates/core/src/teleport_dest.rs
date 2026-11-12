use std::collections::BTreeSet as BS;
use std::f32::consts::PI;

use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::placement::{banc_files, hash_of};
use crate::romfs::Romfs;

pub const DIST: f32 = 3.0;
pub const LIFT: f32 = 0.5;
pub const NAME_MAX: usize = 63;

#[derive(Clone, Debug, PartialEq)]
pub struct Dest {
    pub id: String,
    pub banc: String,
    pub pos: [f32; 3],
    pub yaw: f32,
    pub label: String,
}

pub fn start_name(d: &Dest) -> String {
    format!("TP_{}", d.id)
}

pub fn stand(t: [f32; 3], ry: f32) -> ([f32; 3], f32) {
    let p: [f32; 3] = [t[0] + ry.sin() * DIST, t[1] + LIFT, t[2] + ry.cos() * DIST];
    (p, ry + PI)
}

pub fn banc_of(file: &str) -> Option<String> {
    let n: &str = file.rsplit('/').next()?;
    let s: &str = n.strip_suffix(".bcett.byml.zs")?;
    let b: &str = s.strip_suffix("_Static").or_else(|| s.strip_suffix("_Dynamic"))?;
    Some(b.to_string())
}

pub fn parse(text: &str) -> Res<Vec<Dest>> {
    let mut out: Vec<Dest> = Vec::new();
    for l in text.lines() {
        let t: &str = l.trim_end();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = t.split('\t').collect();
        if f.len() < 6 {
            return Err(Error::Format("teleport: a destination line needs id, banc, x, y, z, yaw"));
        }
        let num = |s: &str| -> Res<f32> { s.parse::<f32>().map_err(|_| Error::Format("teleport: bad number in a destination line")) };
        let d: Dest = Dest {
            id: f[0].to_string(),
            banc: f[1].to_string(),
            pos: [num(f[2])?, num(f[3])?, num(f[4])?],
            yaw: num(f[5])?,
            label: f.get(6).map(|s| s.to_string()).unwrap_or_default(),
        };
        if start_name(&d).len() > NAME_MAX {
            return Err(Error::Msg(format!("teleport: id {} makes a start position name longer than {NAME_MAX}", d.id)));
        }
        if out.iter().any(|o| o.id == d.id && o.banc == d.banc) {
            return Err(Error::Msg(format!("teleport: duplicate id {} in {}", d.id, d.banc)));
        }
        out.push(d);
    }
    Ok(out)
}

pub fn to_text(v: &[Dest]) -> String {
    let mut out: String = String::from("# id\tbanc\tx\ty\tz\tyaw\tlabel\n");
    for d in v {
        out.push_str(&format!("{}\t{}\t{}\t{}\t{}\t{}\t{}\n", d.id, d.banc, d.pos[0], d.pos[1], d.pos[2], d.yaw, d.label));
    }
    out
}

fn floats(n: Option<&Node>) -> Vec<f32> {
    let Some(Node::Arr(a)) = n else {
        return Vec::new();
    };
    a.iter().filter_map(|x| if let Node::F32(v) = x { Some(*v) } else { None }).collect()
}

fn from_file(y: &Byml, banc: &str, want: Option<&BS<(String, u64)>>, out: &mut Vec<Dest>) {
    let Some(l) = y.root.get("Actors").and_then(|x| x.as_arr()) else {
        return;
    };
    for a in l {
        let g: &str = a.get("Gyaml").and_then(|v| v.as_str()).unwrap_or("");
        let Some(h) = hash_of(a) else {
            continue;
        };
        let keep: bool = match want {
            Some(w) => w.contains(&(banc.to_string(), h)),
            None => g.starts_with("Obj_TBox"),
        };
        if !keep {
            continue;
        }
        let t: Vec<f32> = floats(a.get("Translate"));
        if t.len() < 3 {
            continue;
        }
        let r: Vec<f32> = floats(a.get("Rotate"));
        let ry: f32 = r.get(1).copied().unwrap_or(0.0);
        let (p, face): ([f32; 3], f32) = stand([t[0], t[1], t[2]], ry);
        out.push(Dest { id: h.to_string(), banc: banc.to_string(), pos: p, yaw: face, label: g.to_string() });
    }
}

pub fn chests(rf: &mut Romfs) -> Res<Vec<Dest>> {
    scan(rf, None)
}

pub fn picks(rf: &mut Romfs, text: &str) -> Res<Vec<Dest>> {
    let mut w: BS<(String, u64)> = BS::new();
    for l in text.lines().filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let (b, h): (&str, &str) = l.split_once('\t').ok_or(Error::Format("teleport: a pick line needs banc and hash"))?;
        w.insert((b.to_string(), h.trim().parse::<u64>().map_err(|_| Error::Format("teleport: bad hash in a pick line"))?));
    }
    scan(rf, Some(&w))
}

fn scan(rf: &mut Romfs, want: Option<&BS<(String, u64)>>) -> Res<Vec<Dest>> {
    let mut out: Vec<Dest> = Vec::new();
    for f in banc_files(rf)? {
        let Some(b) = banc_of(&f) else {
            continue;
        };
        let y: Byml = rf.read_byml(&f)?;
        from_file(&y, &b, want, &mut out);
    }
    out.sort_by(|a, b| (a.banc.as_str(), a.id.len(), a.id.as_str()).cmp(&(b.banc.as_str(), b.id.len(), b.id.as_str())));
    out.dedup_by(|a, b| a.banc == b.banc && a.id == b.id);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stands_in_front_and_faces_back() {
        let (p, f): ([f32; 3], f32) = stand([10.0, 5.0, -2.0], 0.0);
        assert!((p[0] - 10.0).abs() < 1e-4);
        assert!((p[1] - 5.5).abs() < 1e-4);
        assert!((p[2] - 1.0).abs() < 1e-4);
        assert!((f - PI).abs() < 1e-4);
    }

    #[test]
    fn banc_names_come_from_file_names() {
        assert_eq!(banc_of("Banc/FireTemple/FireTemple_Static.bcett.byml.zs").as_deref(), Some("FireTemple"));
        assert_eq!(banc_of("Banc/KokiriForest/KokiriForest_Adult_Dynamic.bcett.byml.zs").as_deref(), Some("KokiriForest_Adult"));
        assert_eq!(banc_of("Banc/BancBasedResource/StartPos.byml.zs"), None);
    }

    #[test]
    fn text_round_trips() {
        let v: Vec<Dest> = vec![Dest { id: "FIRE_01".to_string(), banc: "FireTemple".to_string(), pos: [1.5, 2.0, -3.25], yaw: 0.5, label: "green rupee".to_string() }];
        assert_eq!(parse(&to_text(&v)).unwrap(), v);
    }

    #[test]
    fn rejects_duplicates_and_long_names() {
        assert!(parse("A\tFireTemple\t0\t0\t0\t0\nA\tFireTemple\t0\t0\t0\t0\n").is_err());
        let long: String = "X".repeat(70);
        assert!(parse(&format!("{long}\tFireTemple\t0\t0\t0\t0\n")).is_err());
        assert!(parse("A\tFireTemple\t0\t0\n").is_err());
    }
}
