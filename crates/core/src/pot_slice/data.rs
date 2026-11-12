use std::collections::{HashMap as HM, HashSet as HS};

use crate::catalog::{Catalog, Loc};
use crate::error::{Error as E, Res};
use crate::json::{self, Json};
use crate::pot_slice::pickup;
use crate::slice::data::{Data, State};

pub const CATALOG_JSON: &str = include_str!("../../../../data/pots_catalog.json");
pub const SCAN_JSON: &str = include_str!("../../../../data/pots_scan.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Pot,
    Crate,
    Flying,
}

impl Kind {
    pub fn of(k: &str) -> Option<Kind> {
        match k {
            "pot" => Some(Kind::Pot),
            "crate" => Some(Kind::Crate),
            "flying_pot" => Some(Kind::Flying),
            _ => None,
        }
    }

    pub fn reset(self) -> i64 {
        match self {
            Kind::Flying => 280,
            _ => 88,
        }
    }

    pub fn gyaml(self, g: &str) -> bool {
        match self {
            Kind::Pot => g.strip_prefix("Obj_Pot_A_0").is_some_and(|t| matches!(t, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8")),
            Kind::Crate => matches!(g, "Obj_BrokenBox_A_01" | "Obj_BrokenBox_A_03"),
            Kind::Flying => matches!(g, "Enemy_FlyingPot_SpiritTemple" | "Enemy_FlyingPot_ShadowTemple" | "Enemy_FlyingPot_IceCavern"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PotData {
    pub locs: Vec<Loc>,
    pub ix: HM<String, usize>,
    pub state: HM<String, State>,
    pub only: HS<String>,
    pub game: String,
    pub files: Vec<String>,
}

impl PotData {
    pub fn has(&self, name: &str) -> bool {
        self.ix.contains_key(name)
    }

    pub fn get(&self, name: &str) -> Option<&Loc> {
        self.ix.get(name).map(|i| &self.locs[*i])
    }

    pub fn kind_of(&self, name: &str) -> Option<Kind> {
        self.get(name).and_then(|l| Kind::of(&l.kind))
    }

    pub fn state_of(&self, name: &str) -> Option<State> {
        self.state.get(name).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.locs.is_empty()
    }
}

fn names(j: &Json, k: &str) -> Vec<String> {
    j.get(k).and_then(|v| v.as_arr()).unwrap_or(&[]).iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
}

pub fn merged(d: &Data) -> Res<Vec<Loc>> {
    let extra: Catalog = Catalog::parse(CATALOG_JSON)?;
    let retire: Vec<String> = names(&json::parse(CATALOG_JSON)?, "retire");
    let mut out: Vec<Loc> = Vec::new();
    for l in &d.cat.locs {
        let mine: bool = Kind::of(&l.kind).is_some();
        if mine && !retire.contains(&l.name) && extra.by_name(&l.name).is_none() {
            out.push(l.clone());
        }
    }
    for l in &extra.locs {
        if Kind::of(&l.kind).is_none() {
            return Err(E::Msg(format!("pot catalog: {} has the unknown kind {}", l.name, l.kind)));
        }
        out.push(l.clone());
    }
    Ok(out)
}

fn pot_only(d: &Data, locs: &[Loc]) -> HS<String> {
    let mut held: HS<String> = HS::new();
    for l in d.cat.locs.iter().filter(|l| matches!(l.kind.as_str(), "bossheart" | "scrub")) {
        if let Some(v) = d.vanilla_of(&l.name) {
            held.insert(v.to_string());
        }
    }
    for r in d.scan.rows.iter().filter(|r| matches!(r.state, State::Eligible | State::Event)) {
        if let Some(v) = d.vanilla_of(&r.name) {
            held.insert(v.to_string());
        }
    }
    for l in &d.flow.locs {
        if let Some(v) = d.vanilla_of(&l.name) {
            held.insert(v.to_string());
        }
    }
    let mut out: HS<String> = HS::new();
    for l in locs {
        if let Some(v) = d.vanilla_of(&l.name).filter(|v| pickup::of(v).is_some() && !held.contains(*v)) {
            out.insert(v.to_string());
        }
    }
    out
}

pub fn parse_scan(t: &str) -> Res<(String, Vec<String>, Vec<(String, State)>)> {
    let j: Json = json::parse(t)?;
    if j.get("format").and_then(|v| v.as_i64()) != Some(1) {
        return Err(E::Format("pot scan: unsupported format"));
    }
    let game: String = j.get("game").and_then(|v| v.as_str()).ok_or(E::Format("pot scan: no game"))?.to_string();
    let files: Vec<String> = names(&j, "files");
    let mut rows: Vec<(String, State)> = Vec::new();
    for r in j.get("locations").and_then(|v| v.as_arr()).ok_or(E::Format("pot scan: no locations"))? {
        let n: &str = r.get("name").and_then(|v| v.as_str()).ok_or(E::Format("pot scan: a row has no name"))?;
        let s: &str = r.get("state").and_then(|v| v.as_str()).ok_or(E::Format("pot scan: a row has no state"))?;
        let st: State = State::from_name(s).ok_or_else(|| E::Msg(format!("pot scan: unknown state {s}")))?;
        rows.push((n.to_string(), st));
    }
    Ok((game, files, rows))
}

impl PotData {
    pub fn load(d: &Data) -> Res<PotData> {
        let locs: Vec<Loc> = merged(d)?;
        let (game, files, rows): (String, Vec<String>, Vec<(String, State)>) = parse_scan(SCAN_JSON)?;
        let mut ix: HM<String, usize> = HM::new();
        for (i, l) in locs.iter().enumerate() {
            if ix.insert(l.name.clone(), i).is_some() {
                return Err(E::Msg(format!("pot catalog: {} is listed twice", l.name)));
            }
        }
        let only: HS<String> = pot_only(d, &locs);
        Ok(PotData { locs, ix, state: rows.into_iter().collect(), only, game, files })
    }
}

impl Data {
    pub fn with_pots(mut self) -> Res<Data> {
        self.pots = PotData::load(&self)?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_and_their_actor_types() {
        assert_eq!(Kind::of("flying_pot"), Some(Kind::Flying));
        assert_eq!(Kind::of("chest"), None);
        assert!(Kind::Pot.gyaml("Obj_Pot_A_03"));
        assert!(!Kind::Pot.gyaml("Obj_Pot_A_09"));
        assert!(!Kind::Pot.gyaml("Obj_Pot_A_03_Dream"));
        assert!(!Kind::Pot.gyaml("Obj_Pot_A_07_Respawn"));
        assert!(Kind::Crate.gyaml("Obj_BrokenBox_A_03"));
        assert!(!Kind::Crate.gyaml("Obj_BrokenBox_A_02"));
        assert!(Kind::Flying.gyaml("Enemy_FlyingPot_IceCavern"));
        assert!(!Kind::Flying.gyaml("Enemy_FlyingPot_Dictionary"));
        assert_eq!((Kind::Pot.reset(), Kind::Flying.reset()), (88, 280));
    }

    #[test]
    fn the_shipped_pot_data_is_consistent() {
        let d: Data = Data::load().unwrap();
        let p: &PotData = &d.pots;
        assert!(p.locs.len() > 100);
        assert_eq!(p.locs.len(), p.ix.len());
        let mut owner: HM<(String, u64), String> = HM::new();
        for l in &p.locs {
            for r in &l.refs {
                let prev: Option<String> = owner.insert((l.folder.clone(), r.hash), l.name.clone());
                assert!(prev.is_none(), "{} and {:?} share hash {}", l.name, prev, r.hash);
            }
            assert!(d.vanilla_of(&l.name).is_some(), "{} has no vanilla item", l.name);
        }
        for (n, s) in &p.state {
            assert!(p.has(n), "{n} is scanned but not in the catalog");
            assert!(*s != State::Eligible || p.get(n).is_some());
        }
        for l in &p.locs {
            assert!(p.state_of(&l.name).is_some(), "{} was never scanned", l.name);
        }
        assert!(p.state.values().filter(|s| **s == State::Eligible).count() > 50);
    }

    #[test]
    fn pot_only_items_are_the_junk_that_no_chest_holds() {
        let d: Data = Data::load().unwrap();
        assert!(!d.pots.only.contains("Deku Nuts (5)"));
        assert!(d.pots.only.contains("Fairy Drop"));
        assert!(!d.pots.only.contains("Rupees (5)"));
        assert!(d.pots.only.contains("Recovery Heart"));
    }

    #[test]
    fn a_synthetic_data_set_has_no_pots() {
        let p: PotData = PotData::default();
        assert!(p.is_empty() && !p.has("x") && p.kind_of("x").is_none());
    }
}
