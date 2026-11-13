use crate::error::{Error as E, Res};
use crate::json::{self, Json};
use crate::pickup_pack::{default_name, Layers, Spec, Style};

pub const JSON: &str = include_str!("../../../../data/pickup_packs.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Built,
    DryOk,
    Shipped,
    Blocked,
}

impl Status {
    pub fn parse(s: &str) -> Option<Status> {
        match s {
            "built" => Some(Status::Built),
            "dry_ok" => Some(Status::DryOk),
            "shipped" => Some(Status::Shipped),
            "blocked" => Some(Status::Blocked),
            _ => None,
        }
    }

    pub fn ready(self) -> bool {
        matches!(self, Status::Built | Status::DryOk)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    pub args: Vec<String>,
    pub status: Status,
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub group: String,
    pub item: String,
    pub pack: Option<String>,
    pub args: Vec<String>,
    pub status: Status,
    pub identity: Option<String>,
    pub tier: Option<String>,
    pub flag: Option<String>,
    pub notes: String,
    pub variants: Vec<Variant>,
}

fn text(j: &Json, k: &str) -> Res<String> {
    j.get(k).and_then(|v| v.as_str()).map(|s| s.to_string()).ok_or_else(|| E::Msg(format!("pickup pack catalog: missing text field {k}")))
}

fn opt(j: &Json, k: &str) -> Option<String> {
    j.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn status(j: &Json) -> Res<Status> {
    let s: String = text(j, "status")?;
    Status::parse(&s).ok_or_else(|| E::Msg(format!("pickup pack catalog: {s} is not a status")))
}

fn args(j: &Json) -> Res<Vec<String>> {
    let a: &[Json] = j.get("args").and_then(|v| v.as_arr()).ok_or(E::Format("pickup pack catalog: missing args"))?;
    a.iter().map(|v| v.as_str().map(|s| s.to_string()).ok_or(E::Format("pickup pack catalog: an argument must be text"))).collect()
}

fn variant(j: &Json) -> Res<Variant> {
    Ok(Variant { args: args(j)?, status: status(j)?, notes: text(j, "notes")? })
}

fn entry(j: &Json) -> Res<Entry> {
    let mut variants: Vec<Variant> = Vec::new();
    for v in j.get("variants").and_then(|v| v.as_arr()).unwrap_or(&[]) {
        variants.push(variant(v)?);
    }
    Ok(Entry {
        group: text(j, "group")?,
        item: text(j, "item")?,
        pack: opt(j, "pack"),
        args: args(j)?,
        status: status(j)?,
        identity: opt(j, "identity"),
        tier: opt(j, "tier"),
        flag: opt(j, "flag"),
        notes: text(j, "notes")?,
        variants,
    })
}

pub fn parse(t: &str) -> Res<Vec<Entry>> {
    let j: Json = json::parse(t)?;
    let a: &[Json] = j.get("packs").and_then(|v| v.as_arr()).ok_or(E::Format("pickup pack catalog: missing packs"))?;
    let mut out: Vec<Entry> = Vec::new();
    for p in a {
        let e: Entry = entry(p)?;
        if out.iter().any(|x| x.item == e.item) {
            return Err(E::Msg(format!("pickup pack catalog: {} is listed twice", e.item)));
        }
        let ready: bool = e.status.ready() || e.variants.iter().any(|v| v.status.ready());
        if ready && (e.pack.is_none() || e.identity.is_none()) {
            return Err(E::Msg(format!("pickup pack catalog: {} can be built but names no pack or no identity", e.item)));
        }
        out.push(e);
    }
    Ok(out)
}

pub fn load() -> Res<Vec<Entry>> {
    parse(JSON)
}

pub fn spec(item: &str, a: &[String]) -> Res<(Spec, Layers)> {
    let mut s: Spec = Spec { item: item.to_string(), name: default_name(item), style: Style::Touch, heap: None };
    let mut l: Layers = Layers::default();
    let mut i: usize = 0;
    while i < a.len() {
        let v: Option<&str> = a.get(i + 1).map(|x| x.as_str());
        let step: usize = match (a[i].as_str(), v) {
            ("--press", _) => {
                s.style = Style::Press;
                1
            }
            ("--ammo", _) => {
                l.ammo = true;
                1
            }
            ("--magic", _) => {
                l.magic = true;
                1
            }
            ("--host", Some(_)) => 2,
            ("--name", Some(v)) => {
                s.name = v.to_string();
                2
            }
            ("--tag", Some(v)) => {
                if !l.tags.iter().any(|t| t == v) {
                    l.tags.push(v.to_string());
                }
                2
            }
            ("--heap", Some(v)) => {
                s.heap = Some(v.parse().map_err(|_| E::Msg(format!("pickup pack catalog: {v} is not a heap size")))?);
                2
            }
            (x, _) => return Err(E::Msg(format!("pickup pack catalog: {x} is not a pack option or lacks its value"))),
        };
        i += step;
    }
    Ok((s, l))
}

impl Entry {
    pub fn builds(&self) -> Vec<(&[String], Status)> {
        let mut out: Vec<(&[String], Status)> = vec![(self.args.as_slice(), self.status)];
        out.extend(self.variants.iter().map(|v| (v.args.as_slice(), v.status)));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byml::Byml;
    use crate::modpack::Host;
    use crate::pickup_pack::actor::{pouch_file, POUCH_ACTOR};
    use crate::pickup_pack::pack_of;
    use crate::pickup_pack::write::{run_with, Done, Opts};
    use crate::sarc::{self, Entry as Packed};
    use std::path::{Path as P, PathBuf as PB};

    fn real() -> Option<PB> {
        let p: PB = std::env::var("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|_| PB::from("C:/o/nup dump/updates/131072/romfs"));
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Some(p)
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    fn strs(x: &[&str]) -> Vec<String> {
        x.iter().map(|s| s.to_string()).collect()
    }

    fn written(d: &Done, name: &str) -> String {
        let es: Vec<Packed> = sarc::parse(&d.files[0].raw).unwrap();
        let y: Byml = Byml::parse(&sarc::get(&es, &pouch_file(name)).unwrap().data).unwrap();
        let v: &str = y.root.get(POUCH_ACTOR).and_then(|v| v.as_str()).unwrap();
        let file: &str = v.rsplit('/').next().unwrap();
        file.split(".engine__actor__").next().unwrap().to_string()
    }

    fn same(a: &Done, b: &Done) -> bool {
        a.heap == b.heap && a.report == b.report && a.files.len() == b.files.len() && a.files.iter().zip(b.files.iter()).all(|(x, y)| x.path == y.path && x.raw == y.raw && x.z == y.z)
    }

    #[test]
    fn the_catalog_loads_and_every_entry_is_well_formed() {
        let all: Vec<Entry> = load().unwrap();
        assert!(all.len() >= 60, "{}", all.len());
        for e in &all {
            assert!(["A", "B", "C", "D", "E"].contains(&e.group.as_str()), "{}", e.item);
            for (a, st) in e.builds() {
                if !st.ready() {
                    continue;
                }
                let (s, _): (Spec, Layers) = spec(&e.item, a).unwrap();
                assert_eq!(Some(&s.name), e.pack.as_ref(), "{}", e.item);
                assert!(s.item.starts_with("Item_"), "{}", e.item);
            }
        }
        let n: usize = all.iter().filter(|e| e.status == Status::Built).count();
        assert!(n >= 50, "{n}");
        let goron: &Entry = all.iter().find(|e| e.item == "Item_Gauntlet_Goron").unwrap();
        assert_eq!(goron.flag.as_deref(), Some("in_game_first"));
        assert_eq!(goron.identity.as_deref(), Some("Item_Gauntlet_Goron"));
    }

    #[test]
    fn the_options_of_an_entry_become_a_spec_and_its_layers() {
        let (s, l): (Spec, Layers) = spec("Item_Hookshot", &strs(&["--heap", "199928", "--host", "hookshot"])).unwrap();
        assert_eq!(s, Spec { item: "Item_Hookshot".to_string(), name: "Obj_Hookshot".to_string(), style: Style::Touch, heap: Some(199928) });
        assert!(l.plain());
        let (s, l): (Spec, Layers) = spec("Item_Bottle_Letter", &strs(&["--name", "Obj_Bottle_LetterLand", "--tag", "LetterBottle", "--tag", "LetterBottle"])).unwrap();
        assert_eq!(s.name, "Obj_Bottle_LetterLand");
        assert_eq!(l, Layers { tags: strs(&["LetterBottle"]), ammo: false, magic: false });
        let (s, l): (Spec, Layers) = spec("Item_Range_Bow", &strs(&["--ammo", "--magic", "--press"])).unwrap();
        assert_eq!(s.style, Style::Press);
        assert!(l.ammo && l.magic);
        assert!(spec("Item_Hookshot", &strs(&["--place", "7"])).is_err());
        assert!(spec("Item_Hookshot", &strs(&["--heap"])).is_err());
        assert!(spec("Item_Hookshot", &strs(&["--heap", "big"])).is_err());
    }

    #[test]
    fn a_broken_catalog_is_refused() {
        let one: &str = r#"{"group":"A","item":"Item_X","pack":"Obj_X","args":[],"status":"built","identity":"Item_X","notes":""}"#;
        assert_eq!(parse(&format!(r#"{{"packs":[{one}]}}"#)).unwrap().len(), 1);
        assert!(parse(&format!(r#"{{"packs":[{one},{one}]}}"#)).is_err());
        assert!(parse(r#"{"packs":[{"group":"A","item":"Item_X","pack":null,"args":[],"status":"built","identity":"Item_X","notes":""}]}"#).is_err());
        assert!(parse(r#"{"packs":[{"group":"A","item":"Item_X","pack":"Obj_X","args":[],"status":"blocked_engine","identity":"Item_X","notes":""}]}"#).is_err());
        assert!(parse(r#"{"packs":[{"group":"A","item":"Item_X","pack":"Obj_X","args":[7],"status":"built","identity":"Item_X","notes":""}]}"#).is_err());
        assert!(parse(r#"{"version":1}"#).is_err());
        assert_eq!(parse(r#"{"packs":[{"group":"E","item":"Milk","pack":null,"args":[],"status":"blocked","identity":null,"notes":"n"}]}"#).unwrap()[0].pack, None);
    }

    #[test]
    fn every_ready_entry_builds_twice_to_the_same_bytes_with_its_identity() {
        let Some(root) = real() else {
            return;
        };
        let o: Opts = Opts { host: Host::Hookshot, base: None, rst_from: None, places: Vec::new(), dry: true };
        let mut n: usize = 0;
        for e in load().unwrap() {
            for (a, st) in e.builds() {
                if !st.ready() {
                    continue;
                }
                let (s, l): (Spec, Layers) = spec(&e.item, a).unwrap();
                let d: Done = run_with(&root, P::new("unused"), "unused", &s, &o, &l).unwrap_or_else(|x| panic!("{} {a:?}: {x}", e.item));
                assert!(d.root.is_none(), "{}", e.item);
                assert_eq!(d.files[0].path, pack_of(&s.name), "{}", e.item);
                assert_eq!(Some(written(&d, &s.name)), e.identity, "{} {a:?}", e.item);
                assert!(d.report.contains("stays collected once taken"), "{}", e.item);
                let again: Done = run_with(&root, P::new("unused"), "unused", &s, &o, &l).unwrap();
                assert!(same(&d, &again), "{} {a:?}", e.item);
                n += 1;
            }
        }
        assert!(n >= 60, "{n}");
    }

    #[test]
    fn a_shipped_entry_is_in_the_game_and_a_ready_one_is_not() {
        let Some(root) = real() else {
            return;
        };
        for e in load().unwrap() {
            let Some(p) = e.pack.as_ref() else {
                continue;
            };
            let there: bool = root.join(pack_of(p)).is_file();
            match e.status {
                Status::Shipped => assert!(there, "{p}"),
                Status::Built | Status::DryOk => assert!(!there, "{p}"),
                Status::Blocked => {}
            }
        }
    }
}
