use std::collections::{HashMap as HM, HashSet as HS};

use crate::catalog::Catalog;
use crate::flow_locations::Data as FlowData;
use crate::error::{Error as E, Res};
use crate::json::{self, Json};

pub const ITEMS_JSON: &str = include_str!("../../../../data/items.json");
pub const VANILLA_JSON: &str = include_str!("../../../../data/logic/vanilla.json");
pub const SLICE_JSON: &str = include_str!("../../../../data/slice.json");
pub const CATALOG_JSON: &str = include_str!("../../../../data/locations.json");
pub const REMAKE_JSON: &str = include_str!("../../../../data/remake_vanilla.json");
pub const EVENTS_JSON: &str = include_str!("../../../../data/event_chests.json");

pub const HELD: [u64; 1] = [6859289214373847284];

pub const SEP: &str = " @ ";

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub name: String,
    pub kind: String,
    pub adv: bool,
    pub actor: Option<String>,
    pub conf: String,
    pub dungeon: Option<String>,
    pub tiers: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Eligible,
    Pooled,
    Pool,
    Event,
    Check,
    Special,
    Unmapped,
    Mismatch,
}

impl State {
    pub fn name(&self) -> &'static str {
        match self {
            State::Eligible => "eligible",
            State::Pooled => "pooled",
            State::Pool => "pool",
            State::Event => "event",
            State::Check => "check",
            State::Special => "special",
            State::Unmapped => "unmapped",
            State::Mismatch => "mismatch",
        }
    }

    pub fn from_name(s: &str) -> Option<State> {
        [
            State::Eligible,
            State::Pooled,
            State::Pool,
            State::Event,
            State::Check,
            State::Special,
            State::Unmapped,
            State::Mismatch,
        ]
        .into_iter()
        .find(|x| x.name() == s)
    }

    pub fn why(&self) -> &'static str {
        match self {
            State::Eligible => "shuffled",
            State::Pooled => "verified chest in a pool whose chests are not equivalent in logic",
            State::Pool => "match is a guess inside a pool",
            State::Event => "chest runs a story event that can grant its own item",
            State::Check => "chest checks or substitutes its item",
            State::Special => "not a plain treasure chest",
            State::Unmapped => "vanilla item has no remake actor yet",
            State::Mismatch => "remake chest holds something else than the vanilla item",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub name: String,
    pub state: State,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scan {
    pub game: String,
    pub files: Vec<String>,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ev {
    Strip,
    Keep,
}

#[derive(Debug, Clone, Default)]
pub struct Events {
    pub strip: HS<u64>,
    pub keep: HS<u64>,
}

#[derive(Debug, Clone)]
pub struct Data {
    pub flow: FlowData,
    pub remake: HM<String, String>,
    pub events: Events,
    pub items: Vec<Item>,
    pub item_ix: HM<String, usize>,
    pub vanilla: HM<String, String>,
    pub cat: Catalog,
    pub scan: Scan,
}

fn text(j: &Json, k: &str) -> Res<String> {
    j.get(k)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| E::Msg(format!("data file: field {k} is missing or not text")))
}

fn list<'a>(j: &'a Json, k: &str) -> Res<&'a [Json]> {
    j.get(k)
        .and_then(|v| v.as_arr())
        .ok_or_else(|| E::Msg(format!("data file: field {k} is missing or not a list")))
}

fn opt(j: &Json, k: &str) -> Option<String> {
    j.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

pub fn parse_items(t: &str) -> Res<Vec<Item>> {
    let j: Json = json::parse(t)?;
    let mut out: Vec<Item> = Vec::new();
    for e in list(&j, "items")? {
        out.push(Item {
            name: text(e, "name")?,
            kind: text(e, "kind")?,
            adv: e.get("adv").and_then(|v| v.as_bool()).unwrap_or(false),
            actor: opt(e, "actor"),
            conf: opt(e, "conf").unwrap_or_default(),
            dungeon: opt(e, "dungeon"),
            tiers: e
                .get("tiers")
                .and_then(|v| v.as_arr())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
        });
    }
    Ok(out)
}

pub fn parse_vanilla(t: &str) -> Res<HM<String, String>> {
    let j: Json = json::parse(t)?;
    let o: &[(String, Json)] = j
        .get("locations")
        .and_then(|v| v.as_obj())
        .ok_or(E::Format("vanilla file has no locations"))?;
    let mut out: HM<String, String> = HM::new();
    for (k, v) in o {
        let s: &str = v.as_str().ok_or(E::Format("vanilla item must be text"))?;
        out.insert(k.clone(), s.to_string());
    }
    Ok(out)
}

pub fn parse_events(t: &str) -> Res<Events> {
    let j: Json = json::parse(t)?;
    let hashes = |k: &str| -> Res<Vec<u64>> {
        let mut out: Vec<u64> = Vec::new();
        for h in j.get(k).and_then(|v| v.as_arr()).unwrap_or(&[]) {
            let s: &str = h.as_str().ok_or(E::Format("event file: hash must be text"))?;
            out.push(s.parse().map_err(|_| E::Format("event file: bad hash"))?);
        }
        Ok(out)
    };
    let mut ev: Events = Events::default();
    for h in hashes("strip_event")?.into_iter().chain(hashes("strip_event_untested")?) {
        if !HELD.contains(&h) {
            ev.strip.insert(h);
        }
    }
    for h in hashes("keep_event")? {
        if !HELD.contains(&h) {
            ev.keep.insert(h);
        }
    }
    Ok(ev)
}

pub fn parse_scan(t: &str) -> Res<Scan> {
    let j: Json = json::parse(t)?;
    if j.get("format").and_then(|v| v.as_i64()) != Some(1) {
        return Err(E::Format("slice file: unsupported format"));
    }
    let mut files: Vec<String> = Vec::new();
    for f in list(&j, "files")? {
        files.push(f.as_str().ok_or(E::Format("slice file: file must be text"))?.to_string());
    }
    let mut rows: Vec<Row> = Vec::new();
    for r in list(&j, "chests")? {
        let s: String = text(r, "state")?;
        let state: State = State::from_name(&s).ok_or_else(|| E::Msg(format!("slice file: unknown state {s}")))?;
        rows.push(Row { name: text(r, "name")?, state });
    }
    Ok(Scan { game: text(&j, "game")?, files, rows })
}

impl Data {
    pub fn new(items: &str, vanilla: &str, scan: &str, cat: &str) -> Res<Data> {
        let items: Vec<Item> = parse_items(items)?;
        let mut item_ix: HM<String, usize> = HM::new();
        for (i, it) in items.iter().enumerate() {
            if item_ix.insert(it.name.clone(), i).is_some() {
                return Err(E::Msg(format!("item {} is listed twice", it.name)));
            }
        }
        Ok(Data {
            flow: FlowData { locs: Vec::new(), accept: Vec::new(), gaps: Vec::new(), loose: Vec::new() },
            remake: HM::new(),
            events: Events::default(),
            items,
            item_ix,
            vanilla: parse_vanilla(vanilla)?,
            cat: Catalog::parse(cat)?,
            scan: parse_scan(scan)?,
        })
    }

    pub fn with_events(mut self, events: &str) -> Res<Data> {
        self.events = parse_events(events)?;
        Ok(self)
    }

    pub fn with_flow(mut self) -> Res<Data> {
        self.flow = FlowData::load()?;
        Ok(self)
    }

    pub fn flow_ok(&self, name: &str) -> bool {
        let Some(l) = self.flow.by_name(name) else {
            return false;
        };
        if !l.supported || l.conf == "guess" || l.edits.is_empty() {
            return false;
        }
        let Some(actor) = l.item.as_deref() else {
            return false;
        };
        if !self.flow.usable(actor) {
            return false;
        }
        match self.vanilla_of(name).and_then(|v| self.item(v)) {
            Some(i) => (i.actor.as_deref() == Some(actor) || i.tiers.iter().any(|t| t == actor)) && self.flow_accepts(i),
            None => false,
        }
    }

    pub fn flow_accepts(&self, item: &Item) -> bool {
        let Some(a) = item.actor.as_deref() else {
            return false;
        };
        self.flow.usable(a) && item.tiers.iter().all(|t| self.flow.usable(t))
    }

    pub fn with_remake(mut self, remake: &str) -> Res<Data> {
        self.remake = parse_vanilla(remake)?;
        Ok(self)
    }

    pub fn load() -> Res<Data> {
        Data::new(ITEMS_JSON, VANILLA_JSON, SLICE_JSON, CATALOG_JSON)?.with_events(EVENTS_JSON)?.with_remake(REMAKE_JSON)?.with_flow()
    }

    pub fn event_mode(&self, hash: u64) -> Option<Ev> {
        if self.events.strip.contains(&hash) {
            Some(Ev::Strip)
        } else if self.events.keep.contains(&hash) {
            Some(Ev::Keep)
        } else {
            None
        }
    }

    pub fn event_ok(&self, name: &str) -> bool {
        match self.cat.by_name(name) {
            Some(l) if !l.refs.is_empty() => l.refs.iter().all(|r| self.event_mode(r.hash).is_some()),
            _ => false,
        }
    }

    pub fn item(&self, name: &str) -> Option<&Item> {
        self.item_ix.get(name).map(|i| &self.items[*i])
    }

    pub fn vanilla_of(&self, loc: &str) -> Option<&str> {
        let base: &str = loc.split(SEP).next().unwrap_or(loc);
        self.remake.get(base).or_else(|| self.vanilla.get(base)).map(|s| s.as_str())
    }

    pub fn state_of(&self, name: &str) -> Option<State> {
        self.scan.rows.iter().find(|r| r.name == name).map(|r| r.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_data_loads_and_agrees_with_itself() {
        let d: Data = Data::load().unwrap();
        assert!(d.items.len() > 250);
        assert!(d.item("Piece of Heart").is_some_and(|i| i.actor.as_deref() == Some("Obj_Heart_Piece")));
        assert!(d.item("Map (Ice Cavern)").is_some_and(|i| i.dungeon.as_deref() == Some("Ice Cavern")));
        assert_eq!(d.vanilla_of("Ganon"), Some("Triforce"));
        for r in &d.scan.rows {
            assert!(d.cat.by_name(&r.name).is_some(), "{} is not in the catalog", r.name);
            if r.state == State::Eligible {
                let v: &str = d.vanilla_of(&r.name).unwrap_or("");
                assert!(d.item(v).is_some_and(|i| i.actor.is_some()), "{} has an unmapped vanilla item", r.name);
            }
        }
        assert!(d.scan.game.starts_with("oot-remake:"));
    }

    #[test]
    fn state_names_round_trip() {
        for s in [State::Eligible, State::Pool, State::Event, State::Check, State::Special, State::Unmapped, State::Mismatch] {
            assert_eq!(State::from_name(s.name()), Some(s));
            assert!(!s.why().is_empty());
        }
        assert_eq!(State::from_name("nope"), None);
    }

    #[test]
    fn broken_data_is_refused() {
        assert!(parse_items("{\"items\":[{\"kind\":\"x\"}]}").is_err());
        assert!(parse_scan("{\"format\":2}").is_err());
        assert!(parse_vanilla("{}").is_err());
        let dup: &str = "{\"items\":[{\"name\":\"A\",\"kind\":\"x\"},{\"name\":\"A\",\"kind\":\"x\"}]}";
        let e: Res<Data> = Data::new(dup, VANILLA_JSON, SLICE_JSON, CATALOG_JSON);
        assert!(e.is_err());
    }
}
