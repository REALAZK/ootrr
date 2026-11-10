use crate::error::{Error as E, Res};
use crate::json::{self, Json};

pub const JSON: &str = include_str!("../../../../data/flow_locations.json");

#[derive(Clone, Debug, PartialEq)]
pub struct Edit {
    pub file: String,
    pub index: usize,
    pub event: String,
    pub param: String,
    pub role: String,
    pub via: String,
    pub item: String,
    pub entries: Vec<String>,
    pub also: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Loc {
    pub name: String,
    pub kind: String,
    pub cat: String,
    pub vanilla: String,
    pub item: Option<String>,
    pub conf: String,
    pub supported: bool,
    pub why: String,
    pub edits: Vec<Edit>,
    pub grant_flags: Vec<String>,
    pub bound_flags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Gap {
    pub name: String,
    pub kind: String,
    pub vanilla: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Loose {
    pub file: String,
    pub index: usize,
    pub event: String,
    pub role: String,
    pub item: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Data {
    pub locs: Vec<Loc>,
    pub accept: Vec<String>,
    pub gaps: Vec<Gap>,
    pub loose: Vec<Loose>,
}

fn text(j: &Json, k: &str) -> Res<String> {
    match j.get(k).and_then(|v| v.as_str()) {
        Some(s) => Ok(s.to_string()),
        None => Err(E::Msg(format!("flow_locations json: missing text field {k}"))),
    }
}

fn list(j: &Json, k: &str) -> Res<Vec<String>> {
    let a: &[Json] = j
        .get(k)
        .and_then(|v| v.as_arr())
        .ok_or_else(|| E::Msg(format!("flow_locations json: missing list field {k}")))?;
    let mut out: Vec<String> = Vec::new();
    for x in a {
        let s: &str = x
            .as_str()
            .ok_or_else(|| E::Msg(format!("flow_locations json: {k} holds a value that is not text")))?;
        out.push(s.to_string());
    }
    Ok(out)
}

fn count(j: &Json, k: &str) -> Res<usize> {
    let n: i64 = j
        .get(k)
        .and_then(|v| v.as_i64())
        .ok_or_else(|| E::Msg(format!("flow_locations json: missing number field {k}")))?;
    usize::try_from(n).map_err(|_| E::Msg(format!("flow_locations json: {k} is negative")))
}

fn arr<'a>(j: &'a Json, k: &str) -> Res<&'a [Json]> {
    j.get(k)
        .and_then(|v| v.as_arr())
        .ok_or_else(|| E::Msg(format!("flow_locations json: missing array {k}")))
}

fn edit(j: &Json) -> Res<Edit> {
    Ok(Edit {
        file: text(j, "file")?,
        index: count(j, "index")?,
        event: text(j, "event")?,
        param: text(j, "param")?,
        role: text(j, "role")?,
        via: text(j, "via")?,
        item: text(j, "item")?,
        entries: list(j, "entries")?,
        also: list(j, "also")?,
    })
}

fn loc(j: &Json) -> Res<Loc> {
    let mut edits: Vec<Edit> = Vec::new();
    for e in arr(j, "edits")? {
        edits.push(edit(e)?);
    }
    let sup: String = text(j, "support")?;
    if sup != "supported" && sup != "unsupported" {
        return Err(E::Msg(format!("flow_locations json: bad support value {sup}")));
    }
    Ok(Loc {
        name: text(j, "name")?,
        kind: text(j, "type")?,
        cat: text(j, "cat")?,
        vanilla: text(j, "vanilla")?,
        item: j.get("item").and_then(|v| v.as_str()).map(|s| s.to_string()),
        conf: text(j, "conf")?,
        supported: sup == "supported",
        why: text(j, "why")?,
        edits,
        grant_flags: list(j, "grant_flags")?,
        bound_flags: list(j, "bound_flags")?,
    })
}

fn gap(j: &Json) -> Res<Gap> {
    Ok(Gap {
        name: text(j, "name")?,
        kind: text(j, "type")?,
        vanilla: j.get("vanilla").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        reason: text(j, "reason")?,
    })
}

fn loose(j: &Json) -> Res<Loose> {
    Ok(Loose {
        file: text(j, "file")?,
        index: count(j, "index")?,
        event: text(j, "event")?,
        role: text(j, "role")?,
        item: text(j, "item")?,
        reason: text(j, "reason")?,
    })
}

impl Data {
    pub fn load() -> Res<Data> {
        Data::parse(JSON)
    }

    pub fn parse(t: &str) -> Res<Data> {
        let j: Json = json::parse(t)?;
        let mut locs: Vec<Loc> = Vec::new();
        for x in arr(&j, "locations")? {
            locs.push(loc(x)?);
        }
        let mut gaps: Vec<Gap> = Vec::new();
        for x in arr(&j, "unmatched_oot")? {
            gaps.push(gap(x)?);
        }
        let mut lo: Vec<Loose> = Vec::new();
        for x in arr(&j, "unmatched_flow")? {
            lo.push(loose(x)?);
        }
        Ok(Data { locs, accept: list(&j, "accept")?, gaps, loose: lo })
    }

    pub fn by_name(&self, n: &str) -> Option<&Loc> {
        self.locs.iter().find(|l| l.name == n)
    }

    pub fn usable(&self, actor: &str) -> bool {
        self.accept.iter().any(|a| a == actor)
    }
}

pub fn path_of(file: &str) -> String {
    format!("Event/EventFlow/{file}.zs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_json_parses_and_names_are_unique() {
        let d: Data = Data::load().unwrap();
        assert!(!d.locs.is_empty());
        for (i, l) in d.locs.iter().enumerate() {
            assert!(d.locs[i + 1..].iter().all(|m| m.name != l.name), "{}", l.name);
            assert!(!l.edits.is_empty(), "{}", l.name);
        }
        for g in &d.gaps {
            assert!(d.by_name(&g.name).is_none(), "{}", g.name);
        }
    }

    #[test]
    fn supported_locations_are_plain_single_item_grants() {
        let d: Data = Data::load().unwrap();
        for l in d.locs.iter().filter(|l| l.supported) {
            let item: &str = &l.edits[0].item;
            for e in &l.edits {
                assert_eq!(e.item, item, "{}", l.name);
                assert!(e.role == "grant" || e.role == "display", "{}", l.name);
                assert!(!e.via.contains("Bottle"), "{}", l.name);
                assert!(e.file.ends_with(".bfevfl"), "{}", l.name);
            }
            assert!(d.usable(item), "{}", l.name);
            assert!(l.edits.iter().any(|e| e.role == "grant"), "{}", l.name);
            assert!(l.bound_flags.is_empty(), "{}", l.name);
        }
    }

    #[test]
    fn unsupported_locations_say_why() {
        let d: Data = Data::load().unwrap();
        for l in d.locs.iter().filter(|l| !l.supported) {
            assert!(!l.why.is_empty(), "{}", l.name);
        }
    }

    #[test]
    fn parse_rejects_a_bad_support_value() {
        let t: String = JSON.replacen("\"support\": \"supported\"", "\"support\": \"maybe\"", 1);
        assert!(Data::parse(&t).is_err());
    }

    #[test]
    fn paths_point_into_the_event_flow_folder() {
        assert_eq!(path_of("Animal_Cow.bfevfl"), "Event/EventFlow/Animal_Cow.bfevfl.zs");
    }
}
