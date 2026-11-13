use std::collections::BTreeMap as BM;
use std::f32::consts::PI;

use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::placement::{banc_files, hash_of};
use crate::romfs::Romfs;

pub const DIST: f32 = 3.0;
pub const LIFT: f32 = 0.5;
pub const NAME_MAX: usize = 63;
pub const CHEST: &str = "chest";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stand {
    pub dist: f32,
    pub lift: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stands(pub BM<String, Stand>);

impl Stands {
    pub fn new() -> Stands {
        let mut m: BM<String, Stand> = BM::new();
        m.insert(CHEST.to_string(), Stand { dist: DIST, lift: LIFT });
        m.insert("freestanding".to_string(), Stand { dist: 0.0, lift: 0.1 });
        m.insert("pot".to_string(), Stand { dist: 1.5, lift: LIFT });
        Stands(m)
    }

    pub fn parse(&mut self, s: &str) -> Res<()> {
        for part in s.split(',').filter(|x| !x.is_empty()) {
            let (k, v): (&str, &str) = part.split_once('=').ok_or(Error::Format("teleport: a stand setting needs kind=dist:lift"))?;
            let (d, l): (&str, &str) = v.split_once(':').unwrap_or((v, "0.5"));
            let num = |x: &str| -> Res<f32> { x.parse::<f32>().map_err(|_| Error::Format("teleport: bad number in a stand setting")) };
            self.0.insert(k.to_string(), Stand { dist: num(d)?, lift: num(l)? });
        }
        Ok(())
    }

    pub fn get(&self, kind: &str) -> Res<Stand> {
        self.0.get(kind).copied().ok_or_else(|| Error::Msg(format!("teleport: no stand setting for the kind {kind}")))
    }
}

impl Default for Stands {
    fn default() -> Stands {
        Stands::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dest {
    pub id: String,
    pub banc: String,
    pub pos: [f32; 3],
    pub yaw: f32,
    pub label: String,
    pub kind: String,
}

pub fn start_name(d: &Dest) -> String {
    format!("TP_{}", d.id)
}

pub fn stand(t: [f32; 3], ry: f32, s: Stand) -> ([f32; 3], f32) {
    let p: [f32; 3] = [t[0] + ry.sin() * s.dist, t[1] + s.lift, t[2] + ry.cos() * s.dist];
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
            kind: f.get(7).filter(|s| !s.is_empty()).map(|s| s.to_string()).unwrap_or_else(|| CHEST.to_string()),
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
    let mut out: String = String::from("# id\tbanc\tx\ty\tz\tyaw\tlabel\tkind\n");
    for d in v {
        out.push_str(&format!("{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n", d.id, d.banc, d.pos[0], d.pos[1], d.pos[2], d.yaw, d.label, d.kind));
    }
    out
}

fn floats(n: Option<&Node>) -> Vec<f32> {
    let Some(Node::Arr(a)) = n else {
        return Vec::new();
    };
    a.iter().filter_map(|x| if let Node::F32(v) = x { Some(*v) } else { None }).collect()
}

type Want = BM<(String, u64), String>;

fn from_file(y: &Byml, banc: &str, want: Option<&Want>, st: &Stands, out: &mut Vec<Dest>) -> Res<()> {
    let Some(l) = y.root.get("Actors").and_then(|x| x.as_arr()) else {
        return Ok(());
    };
    for a in l {
        let g: &str = a.get("Gyaml").and_then(|v| v.as_str()).unwrap_or("");
        let Some(h) = hash_of(a) else {
            continue;
        };
        let kind: Option<&str> = match want {
            Some(w) => w.get(&(banc.to_string(), h)).map(|k| k.as_str()),
            None => g.starts_with("Obj_TBox").then_some(CHEST),
        };
        let Some(kind) = kind else {
            continue;
        };
        let t: Vec<f32> = floats(a.get("Translate"));
        if t.len() < 3 {
            continue;
        }
        let r: Vec<f32> = floats(a.get("Rotate"));
        let ry: f32 = r.get(1).copied().unwrap_or(0.0);
        let (p, face): ([f32; 3], f32) = stand([t[0], t[1], t[2]], ry, st.get(kind)?);
        out.push(Dest { id: h.to_string(), banc: banc.to_string(), pos: p, yaw: face, label: g.to_string(), kind: kind.to_string() });
    }
    Ok(())
}

pub fn chests(rf: &mut Romfs, st: &Stands) -> Res<Vec<Dest>> {
    scan(rf, None, st)
}

pub fn picks(rf: &mut Romfs, text: &str, st: &Stands) -> Res<Vec<Dest>> {
    let mut w: Want = Want::new();
    for l in text.lines().filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() < 2 {
            return Err(Error::Format("teleport: a pick line needs banc and hash"));
        }
        let h: u64 = f[1].trim().parse::<u64>().map_err(|_| Error::Format("teleport: bad hash in a pick line"))?;
        let k: &str = f.get(2).map(|s| s.trim()).filter(|s| !s.is_empty()).unwrap_or(CHEST);
        st.get(k)?;
        w.insert((f[0].to_string(), h), k.to_string());
    }
    scan(rf, Some(&w), st)
}

fn scan(rf: &mut Romfs, want: Option<&Want>, st: &Stands) -> Res<Vec<Dest>> {
    let mut out: Vec<Dest> = Vec::new();
    for f in banc_files(rf)? {
        let Some(b) = banc_of(&f) else {
            continue;
        };
        let y: Byml = rf.read_byml(&f)?;
        from_file(&y, &b, want, st, &mut out)?;
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
        let (p, f): ([f32; 3], f32) = stand([10.0, 5.0, -2.0], 0.0, Stand { dist: DIST, lift: LIFT });
        assert!((p[0] - 10.0).abs() < 1e-4);
        assert!((p[1] - 5.5).abs() < 1e-4);
        assert!((p[2] - 1.0).abs() < 1e-4);
        assert!((f - PI).abs() < 1e-4);
    }

    #[test]
    fn freestanding_stands_on_the_item() {
        let st: Stands = Stands::new();
        let (p, _): ([f32; 3], f32) = stand([10.0, 5.0, -2.0], 1.3, st.get("freestanding").unwrap());
        assert!((p[0] - 10.0).abs() < 1e-5 && (p[2] + 2.0).abs() < 1e-5);
        assert!((p[1] - 5.1).abs() < 1e-5);
        assert!(st.get("nothing").is_err());
    }

    #[test]
    fn stand_settings_parse_and_override() {
        let mut st: Stands = Stands::new();
        st.parse("chest=2.5:0.4,pot=1").unwrap();
        assert_eq!(st.get("chest").unwrap(), Stand { dist: 2.5, lift: 0.4 });
        assert_eq!(st.get("pot").unwrap(), Stand { dist: 1.0, lift: 0.5 });
        assert!(st.parse("chest").is_err());
        assert!(st.parse("chest=x").is_err());
    }

    #[test]
    fn banc_names_come_from_file_names() {
        assert_eq!(banc_of("Banc/FireTemple/FireTemple_Static.bcett.byml.zs").as_deref(), Some("FireTemple"));
        assert_eq!(banc_of("Banc/KokiriForest/KokiriForest_Adult_Dynamic.bcett.byml.zs").as_deref(), Some("KokiriForest_Adult"));
        assert_eq!(banc_of("Banc/BancBasedResource/StartPos.byml.zs"), None);
    }

    #[test]
    fn text_round_trips() {
        let v: Vec<Dest> = vec![Dest { id: "FIRE_01".to_string(), banc: "FireTemple".to_string(), pos: [1.5, 2.0, -3.25], yaw: 0.5, label: "green rupee".to_string(), kind: "freestanding".to_string() }];
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
