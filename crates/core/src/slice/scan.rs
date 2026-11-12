use std::collections::{BTreeSet as BS, HashMap as HM};
use std::path::Path as P;

use crate::byml::Byml;
use crate::catalog::{Conf, Loc, Via};
use crate::error::Res;
use crate::json::Json;
use crate::romfs::Romfs;
use crate::slice::banc::{find, fingerprint, Found};
use crate::slice::data::{Data, Row, Scan, State};
use crate::slice::logic::world;
use crate::slice::pickups as pu;
use crate::world::World;

fn judge(d: &Data, l: &Loc, found: &[Found], missing: bool) -> State {
    let s: State = judge_plain(d, l, found, missing);
    if l.conf == Conf::Pool {
        if s == State::Eligible || (s == State::Event && d.event_ok(&l.name)) { State::Pooled } else { State::Pool }
    } else {
        s
    }
}

pub fn rewritable(f: &Found) -> bool {
    f.gyaml.starts_with("Obj_TBox_A_06") && f.sub && f.drop.is_some()
}

fn judge_plain(d: &Data, l: &Loc, found: &[Found], missing: bool) -> State {
    if missing || found.is_empty() {
        return State::Mismatch;
    }
    if found.iter().any(|f| f.event) {
        return State::Event;
    }
    if found.iter().any(|f| f.check && !rewritable(f)) {
        return State::Check;
    }
    if found.iter().any(|f| !f.gyaml.starts_with("Obj_TBox")) {
        return State::Special;
    }
    let actor: Option<&str> = d
        .vanilla_of(&l.name)
        .and_then(|v| d.item(v))
        .and_then(|i| i.actor.as_deref());
    let Some(actor) = actor else {
        return State::Unmapped;
    };
    if found.iter().any(|f| f.drop.as_deref() != Some(l.item.as_str())) || actor != l.item {
        return State::Mismatch;
    }
    State::Eligible
}

fn promote_pools(d: &Data, rows: &mut [Row]) -> Res<()> {
    let w: World = world(d)?;
    let mut groups: HM<String, Vec<usize>> = HM::new();
    for (i, r) in rows.iter().enumerate() {
        if r.state != State::Pooled {
            continue;
        }
        if let Some(pool) = d.cat.by_name(&r.name).and_then(|l| l.pool.clone()) {
            groups.entry(pool).or_default().push(i);
        }
    }
    for (pool, ix) in groups {
        let members: usize = d.cat.locs.iter().filter(|l| l.pool.as_deref() == Some(pool.as_str())).count();
        if members != ix.len() {
            continue;
        }
        let key = |i: usize| -> Option<(usize, String)> {
            let l = w.loc(&rows[i].name)?;
            Some((w.locs[l].region, format!("{:?}", w.locs[l].rule.node)))
        };
        let first = key(ix[0]);
        if first.is_some() && ix.iter().all(|i| key(*i) == first) {
            for i in ix {
                rows[i].state = State::Eligible;
            }
        }
    }
    Ok(())
}

pub fn scan(root: &P) -> Res<Scan> {
    let d: Data = Data::load()?;
    let mut rf: Romfs = Romfs::open(root)?;
    let mut cache: HM<String, Byml> = HM::new();
    let mut rows: Vec<Row> = Vec::new();
    let mut files: BS<String> = BS::new();
    for l in d.cat.locs.iter().filter(|l| l.kind == "chest" && l.via == Via::Drop && !pu::is_held(&l.name)) {
        let mut found: Vec<Found> = Vec::new();
        let mut missing: bool = false;
        for r in &l.refs {
            for f in &r.files {
                if !cache.contains_key(f) {
                    cache.insert(f.clone(), rf.read_byml(f)?);
                }
                let y: &Byml = &cache[f];
                let hits: Vec<Found> = find(y, f, r.hash);
                missing |= hits.is_empty();
                found.extend(hits);
            }
        }
        let state: State = judge(&d, l, &found, missing);
        rows.push(Row { name: l.name.clone(), state });
    }
    promote_pools(&d, &mut rows)?;
    for row in rows.iter().filter(|r| r.state == State::Eligible) {
        if let Some(l) = d.cat.by_name(&row.name) {
            for r in &l.refs {
                files.extend(r.files.iter().cloned());
            }
        }
    }
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    let list: Vec<String> = files.into_iter().collect();
    let game: String = fingerprint(&rf, &list)?;
    Ok(Scan { game, files: list, rows })
}

pub fn to_json(s: &Scan) -> Json {
    let rows: Vec<Json> = s
        .rows
        .iter()
        .map(|r| Json::obj().with("name", r.name.as_str().into()).with("state", r.state.name().into()))
        .collect();
    Json::obj()
        .with("format", 1i64.into())
        .with("game", s.game.as_str().into())
        .with("files", Json::strs(s.files.iter().map(|f| f.as_str())))
        .with("chests", Json::Arr(rows))
}

pub fn summary(s: &Scan) -> String {
    let mut out: String = format!("game {}\nfiles {}\n", s.game, s.files.len());
    for st in [
        State::Eligible,
        State::Pooled,
        State::Pool,
        State::Event,
        State::Check,
        State::Special,
        State::Unmapped,
        State::Mismatch,
    ] {
        let n: usize = s.rows.iter().filter(|r| r.state == st).count();
        out.push_str(&format!("{}: {} ({})\n", st.name(), n, st.why()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(gyaml: &str, check: bool, sub: bool) -> Found {
        Found { file: "f".to_string(), gyaml: gyaml.to_string(), pos: Vec::new(), drop: Some("TBox_Shield_Deku".to_string()), event: false, check, sub }
    }

    #[test]
    fn only_a_06_chests_with_both_branches_are_rewritable() {
        assert!(rewritable(&found("Obj_TBox_A_06", true, true)));
        assert!(rewritable(&found("Obj_TBox_A_06_NoBake", true, true)));
        assert!(!rewritable(&found("Obj_TBox_A_06", true, false)));
        assert!(!rewritable(&found("Obj_TBox_A_01", true, true)));
        let mut f: Found = found("Obj_TBox_A_06", true, true);
        f.drop = None;
        assert!(!rewritable(&f));
    }
}
