use std::collections::{BTreeSet as BS, HashMap as HM};
use std::path::Path as P;

use crate::byml::{Byml, Node};
use crate::catalog::{Conf, Loc};
use crate::chest_slot::LIST;
use crate::error::Res;
use crate::json::Json;
use crate::placement::hash_of;
use crate::pot_slice::data::{merged, Kind};
use crate::pot_slice::pickup;
use crate::romfs::Romfs;
use crate::slice::banc::{actors, fingerprint};
use crate::slice::data::{Data, State};
use crate::slice::logic::world;
use crate::world::World;

pub const SUPPLY: &str = "Drop_ResourceSupply_BeforeBoss";

#[derive(Debug, Clone, PartialEq)]
pub struct Peek {
    pub gyaml: String,
    pub drop: String,
    pub table: String,
    pub presence: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PotScan {
    pub game: String,
    pub files: Vec<String>,
    pub rows: Vec<(String, State)>,
}

pub fn peek(y: &Byml, hash: u64) -> Vec<Peek> {
    let text = |a: &Node, k: &str| -> String {
        a.get("Dynamic").and_then(|d| d.get(k)).and_then(|v| v.as_str()).unwrap_or("").to_string()
    };
    actors(y)
        .iter()
        .filter(|a| hash_of(a) == Some(hash))
        .map(|a| Peek {
            gyaml: a.get("Gyaml").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            drop: text(a, "Drop__DropActor"),
            table: text(a, "Drop__DropTable"),
            presence: a.get("Presence").is_some(),
        })
        .collect()
}

pub fn resets(y: &Byml) -> HM<u64, i64> {
    let mut out: HM<u64, i64> = HM::new();
    let rows: &[Node] = y.root.get("Data").and_then(|d| d.get("Bool64bitKey")).and_then(|t| t.as_arr()).unwrap_or(&[]);
    for r in rows {
        let v: Option<i64> = match r.get("ResetTypeValue") {
            Some(Node::I32(x)) => Some(*x as i64),
            Some(Node::U32(x)) => Some(*x as i64),
            _ => None,
        };
        if let (Some(h), Some(v)) = (hash_of(r), v) {
            out.insert(h, v);
        }
    }
    out
}

pub fn judge(d: &Data, l: &Loc, k: Kind, found: &[Peek], missing: bool, rows: &[Option<i64>]) -> State {
    if missing || found.is_empty() {
        return State::Mismatch;
    }
    if found.iter().any(|f| !k.gyaml(&f.gyaml) || f.presence || f.table.starts_with(SUPPLY)) {
        return State::Special;
    }
    if rows.iter().any(|r| *r != Some(k.reset())) {
        return State::Special;
    }
    if found.iter().any(|f| f.drop != l.item) {
        return State::Mismatch;
    }
    if d.vanilla_of(&l.name).and_then(pickup::of).is_none() {
        return State::Unmapped;
    }
    if l.conf == Conf::Pool { State::Pooled } else { State::Eligible }
}

fn promote(d: &Data, locs: &[Loc], rows: &mut [(String, State)]) -> Res<()> {
    let w: World = world(d)?;
    let mut groups: HM<String, Vec<usize>> = HM::new();
    for (i, r) in rows.iter().enumerate() {
        if r.1 != State::Pooled {
            continue;
        }
        if let Some(pool) = locs.iter().find(|l| l.name == r.0).and_then(|l| l.pool.clone()) {
            groups.entry(pool).or_default().push(i);
        }
    }
    for (pool, ix) in groups {
        let members: usize = locs.iter().filter(|l| l.pool.as_deref() == Some(pool.as_str())).count();
        if members != ix.len() {
            continue;
        }
        let key = |i: usize| -> Option<(usize, String)> {
            let l: crate::world::LocId = w.loc(&rows[i].0)?;
            Some((w.locs[l].region, format!("{:?}", w.locs[l].rule.node)))
        };
        let first: Option<(usize, String)> = key(ix[0]);
        if first.is_some() && ix.iter().all(|i| key(*i) == first) {
            for i in ix {
                rows[i].1 = State::Eligible;
            }
        }
    }
    Ok(())
}

pub fn scan(root: &P) -> Res<PotScan> {
    let d: Data = Data::load()?;
    let locs: Vec<Loc> = merged(&d)?;
    let mut rf: Romfs = Romfs::open(root)?;
    let have_rows: HM<u64, i64> = resets(&rf.read_byml(LIST)?);
    let mut cache: HM<String, Byml> = HM::new();
    let mut rows: Vec<(String, State)> = Vec::new();
    for l in &locs {
        let Some(k) = Kind::of(&l.kind) else {
            continue;
        };
        let mut found: Vec<Peek> = Vec::new();
        let mut have: Vec<Option<i64>> = Vec::new();
        let mut missing: bool = false;
        for r in &l.refs {
            have.push(have_rows.get(&r.hash).copied());
            for f in &r.files {
                if !cache.contains_key(f) {
                    cache.insert(f.clone(), rf.read_byml(f)?);
                }
                let hits: Vec<Peek> = peek(&cache[f], r.hash);
                missing |= hits.is_empty();
                found.extend(hits);
            }
        }
        rows.push((l.name.clone(), judge(&d, l, k, &found, missing, &have)));
    }
    promote(&d, &locs, &mut rows)?;
    let mut files: BS<String> = BS::new();
    for (n, s) in &rows {
        if *s != State::Eligible {
            continue;
        }
        if let Some(l) = locs.iter().find(|l| l.name == *n) {
            for r in &l.refs {
                files.extend(r.files.iter().cloned());
            }
        }
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let list: Vec<String> = files.into_iter().collect();
    let game: String = fingerprint(&rf, &list)?;
    Ok(PotScan { game, files: list, rows })
}

pub fn to_json(s: &PotScan) -> Json {
    let rows: Vec<Json> = s
        .rows
        .iter()
        .map(|r| Json::obj().with("name", r.0.as_str().into()).with("state", r.1.name().into()))
        .collect();
    Json::obj()
        .with("format", 1i64.into())
        .with("game", s.game.as_str().into())
        .with("files", Json::strs(s.files.iter().map(|f| f.as_str())))
        .with("locations", Json::Arr(rows))
}

pub fn summary(s: &PotScan) -> String {
    let mut out: String = format!("game {}\nfiles {}\n", s.game, s.files.len());
    for st in [State::Eligible, State::Pooled, State::Pool, State::Special, State::Unmapped, State::Mismatch] {
        let n: usize = s.rows.iter().filter(|r| r.1 == st).count();
        out.push_str(&format!("{}: {}\n", st.name(), n));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Via;

    fn loc(item: &str, conf: Conf) -> Loc {
        Loc {
            name: "Pot".to_string(),
            area: "A".to_string(),
            kind: "pot".to_string(),
            vanilla: "Recovery Heart".to_string(),
            item: item.to_string(),
            conf,
            via: Via::Drop,
            folder: "F".to_string(),
            age: "both".to_string(),
            pool: None,
            refs: Vec::new(),
            ev: Vec::new(),
        }
    }

    fn pk(g: &str, drop: &str, table: &str, presence: bool) -> Peek {
        Peek { gyaml: g.to_string(), drop: drop.to_string(), table: table.to_string(), presence }
    }

    fn data() -> Data {
        Data::new(crate::slice::data::ITEMS_JSON, crate::slice::data::VANILLA_JSON, crate::slice::data::SLICE_JSON, crate::slice::data::CATALOG_JSON).unwrap()
    }

    #[test]
    fn the_judge_refuses_every_unsafe_pot_for_its_own_reason() {
        let d: Data = data();
        let mut l: Loc = loc("", Conf::Forced);
        l.name = "KF Know it All House Pot 1".to_string();
        let ok: Vec<Peek> = vec![pk("Obj_Pot_A_01", "", "", false)];
        let row: Vec<Option<i64>> = vec![Some(88)];
        assert_eq!(judge(&d, &l, Kind::Pot, &ok, false, &row), State::Eligible);
        assert_eq!(judge(&d, &l, Kind::Pot, &ok, true, &row), State::Mismatch);
        assert_eq!(judge(&d, &l, Kind::Pot, &[], false, &row), State::Mismatch);
        assert_eq!(judge(&d, &l, Kind::Pot, &[pk("Obj_Pot_A_06_Dream", "", "", false)], false, &row), State::Special);
        assert_eq!(judge(&d, &l, Kind::Pot, &[pk("Obj_Pot_A_01", "", "", true)], false, &row), State::Special);
        assert_eq!(judge(&d, &l, Kind::Pot, &[pk("Obj_Pot_A_01", "", "Drop_ResourceSupply_BeforeBoss_HeartMagic", false)], false, &row), State::Special);
        assert_eq!(judge(&d, &l, Kind::Pot, &ok, false, &[None]), State::Special);
        assert_eq!(judge(&d, &l, Kind::Pot, &ok, false, &[Some(24)]), State::Special);
        assert_eq!(judge(&d, &l, Kind::Pot, &[pk("Obj_Pot_A_01", "Obj_Heart", "", false)], false, &row), State::Mismatch);
        assert_eq!(judge(&d, &l, Kind::Flying, &ok, false, &[Some(280)]), State::Special);
        l.conf = Conf::Pool;
        assert_eq!(judge(&d, &l, Kind::Pot, &ok, false, &row), State::Pooled);
        let mut n: Loc = loc("", Conf::Forced);
        n.name = "Nothing Pot".to_string();
        assert_eq!(judge(&d, &n, Kind::Pot, &ok, false, &row), State::Unmapped);
    }

    #[test]
    fn rows_are_read_from_the_flag_list() {
        let row = |h: u64, t: i32| Node::Map(vec![("Hash".to_string(), Node::U64(h)), ("ResetTypeValue".to_string(), Node::I32(t))]);
        let data: Node = Node::Map(vec![("Bool64bitKey".to_string(), Node::Arr(vec![row(1, 88), row(2, 280)]))]);
        let y: Byml = Byml { root: Node::Map(vec![("Data".to_string(), data)]), version: 2, big: false };
        let m: HM<u64, i64> = resets(&y);
        assert_eq!((m[&1], m[&2], m.len()), (88, 280, 2));
    }

    #[test]
    fn the_scan_json_round_trips_its_states() {
        let s: PotScan = PotScan {
            game: "g".to_string(),
            files: vec!["f".to_string()],
            rows: vec![("A".to_string(), State::Eligible), ("B".to_string(), State::Special)],
        };
        let t: String = to_json(&s).pretty();
        let (g, f, r): (String, Vec<String>, Vec<(String, State)>) = crate::pot_slice::data::parse_scan(&t).unwrap();
        assert_eq!((g, f, r), (s.game.clone(), s.files.clone(), s.rows.clone()));
        assert!(summary(&s).contains("eligible: 1"));
    }
}
