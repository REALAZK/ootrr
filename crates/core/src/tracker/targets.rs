use std::sync::OnceLock;

use crate::error::{Error, Res};
use crate::json::{self, Json};
use crate::murmur::murmur3_32;

pub const JSON: &str = include_str!("../../../../data/tracker_targets.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    Flag,
    Key,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Life {
    Permanent,
    Hourly,
    Daily,
    Scene,
    None,
}

impl Life {
    pub fn parse(s: &str) -> Option<Life> {
        match s {
            "permanent" => Some(Life::Permanent),
            "hourly" => Some(Life::Hourly),
            "daily" => Some(Life::Daily),
            "scene" => Some(Life::Scene),
            "none" => Some(Life::None),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Life::Permanent => "permanent",
            Life::Hourly => "hourly",
            Life::Daily => "daily",
            Life::Scene => "scene",
            Life::None => "none",
        }
    }

    pub fn volatile(self) -> bool {
        matches!(self, Life::Hourly | Life::Daily | Life::Scene)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub scene: Option<String>,
    pub area: Option<String>,
    pub map: Option<String>,
    pub room: Option<i64>,
    pub ootr: Option<String>,
    pub grade: Option<String>,
    pub how: How,
    pub flags: Vec<String>,
    pub keys: Vec<u64>,
    pub keys2: Vec<u64>,
    pub life: Life,
    pub weak: bool,
    pub anchor: Option<u64>,
    pub twin: Option<u64>,
}

pub const KINDS: [&str; 18] = [
    "chest",
    "skulltula",
    "heart_piece",
    "heart_container",
    "heart",
    "rupee",
    "silver_rupee",
    "pot",
    "flying_pot",
    "crate",
    "beehive",
    "key",
    "scrub",
    "shop",
    "gift",
    "cow",
    "pickup",
    "trap_chest",
];

pub const OFF_BY_DEFAULT: [&str; 4] = ["pot", "flying_pot", "crate", "shop"];

pub const DREAM: &str = "DreamWorld";

fn bad(m: String) -> Error {
    Error::Msg(format!("tracker table: {m}"))
}

fn opt_str(j: &Json, k: &str) -> Option<String> {
    j.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn strs(j: &Json, k: &str) -> Res<Vec<String>> {
    match j.get(k) {
        None => Ok(Vec::new()),
        Some(Json::Arr(a)) => a
            .iter()
            .map(|v| v.as_str().map(|s| s.to_string()).ok_or_else(|| bad(format!("{k} holds a non string"))))
            .collect(),
        Some(_) => Err(bad(format!("{k} is not a list"))),
    }
}

fn nums(j: &Json, k: &str) -> Res<Vec<u64>> {
    strs(j, k)?
        .iter()
        .map(|s| s.parse::<u64>().map_err(|_| bad(format!("{k} holds {s}"))))
        .collect()
}

fn one(j: &Json) -> Res<Target> {
    let id: String = opt_str(j, "id").ok_or_else(|| bad("a row has no id".to_string()))?;
    let how: How = match j.get("how").and_then(|v| v.as_str()) {
        Some("flag") => How::Flag,
        Some("key") => How::Key,
        Some("none") => How::None,
        _ => return Err(bad(format!("{id} has no detection rule"))),
    };
    let life: Life = j
        .get("life")
        .and_then(|v| v.as_str())
        .and_then(Life::parse)
        .ok_or_else(|| bad(format!("{id} has no life")))?;
    let anchor: Option<u64> = match opt_str(j, "anchor") {
        Some(s) => Some(s.parse::<u64>().map_err(|_| bad(format!("{id} has a bad anchor")))?),
        None => None,
    };
    let twin: Option<u64> = match opt_str(j, "twin") {
        Some(s) => Some(s.parse::<u64>().map_err(|_| bad(format!("{id} has a bad twin")))?),
        None => None,
    };
    let kind: String = opt_str(j, "kind").ok_or_else(|| bad(format!("{id} has no kind")))?;
    if !KINDS.contains(&kind.as_str()) {
        return Err(bad(format!("{id} has the unknown kind {kind}")));
    }
    Ok(Target {
        name: opt_str(j, "name").unwrap_or_else(|| id.clone()),
        kind,
        scene: opt_str(j, "scene"),
        area: opt_str(j, "area"),
        map: opt_str(j, "map"),
        room: j.get("room").and_then(|v| v.as_i64()),
        ootr: opt_str(j, "ootr"),
        grade: opt_str(j, "grade"),
        how,
        flags: strs(j, "flags")?,
        keys: nums(j, "keys")?,
        keys2: nums(j, "keys2")?,
        life,
        weak: j.get("weak").and_then(|v| v.as_bool()).unwrap_or(false),
        anchor,
        twin,
        id,
    })
}

pub fn parse(text: &str) -> Res<Vec<Target>> {
    let j: Json = json::parse(text)?;
    let rows: &[Json] = j
        .get("targets")
        .and_then(|v| v.as_arr())
        .ok_or_else(|| bad("no targets list".to_string()))?;
    let out: Vec<Target> = rows.iter().map(one).collect::<Res<Vec<Target>>>()?;
    let mut ids: Vec<&str> = out.iter().map(|t| t.id.as_str()).collect();
    ids.sort();
    if ids.windows(2).any(|w| w[0] == w[1]) {
        return Err(bad("two rows share an id".to_string()));
    }
    Ok(out)
}

static ALL: OnceLock<Result<Vec<Target>, String>> = OnceLock::new();

pub fn all() -> Res<&'static [Target]> {
    match ALL.get_or_init(|| parse(JSON).map_err(|e| e.to_string())) {
        Ok(v) => Ok(v),
        Err(m) => Err(Error::Msg(m.clone())),
    }
}

pub fn revision() -> u32 {
    murmur3_32(JSON.as_bytes(), 0)
}

impl Target {
    pub fn hash(&self) -> Option<u64> {
        if self.anchor.is_some() {
            return self.anchor;
        }
        if self.kind == "shop" || self.kind == "gift" {
            return None;
        }
        self.id.split_once(':').and_then(|(_, h)| h.parse::<u64>().ok())
    }

    pub fn keeper(&self) -> Option<&str> {
        if self.kind != "shop" {
            return None;
        }
        let rest: &str = self.id.strip_prefix("shop:")?;
        rest.rsplit_once('.').map(|(g, _)| g)
    }

    pub fn dream(&self) -> bool {
        self.scene.as_deref() == Some(DREAM)
    }

    pub fn label(&self) -> &str {
        match (&self.ootr, self.grade.as_deref()) {
            (Some(o), Some("CONFIRMED")) => o,
            _ => &self.name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn the_table_loads_with_every_kind() {
        let t: &[Target] = all().unwrap();
        assert_eq!(t.len(), 1324);
        let mut n: BTreeMap<&str, usize> = BTreeMap::new();
        for x in t {
            *n.entry(x.kind.as_str()).or_insert(0) += 1;
        }
        assert_eq!(n["pot"], 469);
        assert_eq!(n["chest"], 204);
        assert_eq!(n["shop"], 64);
        assert_eq!(n.len(), KINDS.len());
        assert_eq!(t.iter().filter(|x| x.how == How::None).count(), 28);
        assert_eq!(t.iter().filter(|x| x.weak).count(), 34);
        assert_eq!(t.iter().filter(|x| x.dream()).count(), 52);
    }

    #[test]
    fn the_table_holds_no_positions() {
        for k in ["\"pos\"", "\"x\"", "Translate", "\"stand"] {
            assert!(!JSON.contains(k), "{k}");
        }
    }

    #[test]
    fn anchors_come_from_ids_or_the_anchor_field() {
        let t: &[Target] = all().unwrap();
        let c: &Target = t.iter().find(|x| x.id == "chest:10905582354954996284").unwrap();
        assert_eq!(c.hash(), Some(10905582354954996284));
        let s: &Target = t.iter().find(|x| x.id == "shop:Npc_Kokiri_001_Shop_Child.0").unwrap();
        assert_eq!(s.hash(), None);
        assert_eq!(s.keeper(), Some("Npc_Kokiri_001_Shop_Child"));
        assert_eq!(t.iter().filter(|x| x.kind == "gift" && x.hash().is_some()).count(), 43);
    }

    #[test]
    fn bad_rows_are_refused() {
        assert!(parse("{\"targets\":[{\"id\":\"a\",\"kind\":\"chest\",\"how\":\"flag\"}]}").is_err());
        assert!(parse("{\"targets\":[{\"id\":\"a\",\"kind\":\"boat\",\"how\":\"flag\",\"life\":\"none\"}]}").is_err());
        let two: &str = "{\"targets\":[{\"id\":\"a\",\"kind\":\"chest\",\"how\":\"none\",\"life\":\"none\"},{\"id\":\"a\",\"kind\":\"chest\",\"how\":\"none\",\"life\":\"none\"}]}";
        assert!(parse(two).is_err());
    }
}
