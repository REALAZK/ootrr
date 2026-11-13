use std::collections::BTreeSet as BS;
use std::sync::OnceLock;

use crate::error::{Error as E, Res};
use crate::json::{self, Json};
use crate::slice::data::Data;
use crate::slice::skulls::{Mode, KIND};

pub const NAMES_JSON: &str = include_str!("../../../../data/skull_names.json");
pub const HELD: &str = "the hash research has not settled this match yet";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    Release,
    Lean,
    Tour,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spot {
    pub hash: u64,
    pub name: String,
    pub grade: Grade,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub name: String,
    pub rule: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Names {
    pub spots: Vec<Spot>,
    pub rules: Vec<Rule>,
}

impl Names {
    pub fn spot(&self, name: &str) -> Option<&Spot> {
        self.spots.iter().find(|s| s.name == name)
    }
}

fn grade(t: &str) -> Res<Grade> {
    match t {
        "release" => Ok(Grade::Release),
        "lean" => Ok(Grade::Lean),
        "tour" => Ok(Grade::Tour),
        _ => Err(E::Msg(format!("skulltula names: {t} is not a known grade"))),
    }
}

fn text<'a>(v: &'a Json, k: &'static str) -> Res<&'a str> {
    v.get(k).and_then(|x| x.as_str()).ok_or_else(|| E::Msg(format!("skulltula names: a row has no {k}")))
}

pub fn parse_names(t: &str) -> Res<Names> {
    let j: Json = json::parse(t)?;
    if j.get("format").and_then(|v| v.as_i64()) != Some(1) {
        return Err(E::Format("skulltula names: unsupported format"));
    }
    let mut spots: Vec<Spot> = Vec::new();
    let mut seen: BS<u64> = BS::new();
    let mut named: BS<String> = BS::new();
    for v in j.get("spots").and_then(|v| v.as_arr()).ok_or(E::Format("skulltula names: no spots list"))? {
        let h: &str = text(v, "hash")?;
        let hash: u64 = h.parse().map_err(|_| E::Msg(format!("skulltula names: {h} is not a hash")))?;
        let name: String = text(v, "name")?.to_string();
        if !seen.insert(hash) || !named.insert(name.clone()) {
            return Err(E::Msg(format!("skulltula names: {name} or its hash is listed twice")));
        }
        spots.push(Spot { hash, name, grade: grade(text(v, "grade")?)? });
    }
    let mut rules: Vec<Rule> = Vec::new();
    for v in j.get("rules").and_then(|v| v.as_arr()).ok_or(E::Format("skulltula names: no rules list"))? {
        let name: String = text(v, "name")?.to_string();
        if rules.iter().any(|r: &Rule| r.name == name) {
            return Err(E::Msg(format!("skulltula names: {name} has two rules")));
        }
        rules.push(Rule { name, rule: text(v, "rule")?.to_string() });
    }
    Ok(Names { spots, rules })
}

pub fn names() -> Res<Names> {
    parse_names(NAMES_JSON)
}

pub fn shared() -> Option<&'static Names> {
    static T: OnceLock<Option<Names>> = OnceLock::new();
    T.get_or_init(|| names().ok()).as_ref()
}

pub fn listed(name: &str) -> bool {
    shared().is_some_and(|t| t.spot(name).is_some())
}

pub fn hashes(d: &Data, name: &str) -> Vec<u64> {
    if let Some(s) = shared().and_then(|t| t.spot(name)) {
        return vec![s.hash];
    }
    d.cat.by_name(name).map(|l| l.refs.iter().map(|r| r.hash).collect()).unwrap_or_default()
}

pub fn verdict(m: Mode, name: &str) -> Option<Option<String>> {
    if m != Mode::Matched {
        return None;
    }
    let s: &Spot = shared()?.spot(name)?;
    Some(if s.grade == Grade::Release { None } else { Some(HELD.to_string()) })
}

pub fn extra(d: &Data) -> Vec<&'static str> {
    let Some(t) = shared() else {
        return Vec::new();
    };
    t.spots.iter().filter(|s| d.cat.by_name(&s.name).is_none_or(|l| l.kind != KIND)).map(|s| s.name.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap as BM;
    use std::path::PathBuf as PB;

    use crate::byml::{Byml, Node};
    use crate::catalog::Conf;
    use crate::placement::{banc_files, hash_of};
    use crate::romfs::Romfs;
    use crate::search::{explore, Search};
    use crate::settings::{Settings, Value};
    use crate::slice::generate::{generate_data, GenOpts, Generated};
    use crate::slice::skulls::TOKEN;
    use crate::slice::support::{plan, Plan};
    use crate::world::{Placement, World};

    #[test]
    fn the_table_parses_and_rejects_bad_rows() {
        let t: Names = names().unwrap();
        assert_eq!(t.spots.len(), 48);
        assert_eq!(t.spots.iter().filter(|s| s.grade == Grade::Release).count(), 25);
        assert_eq!(t.spots.iter().filter(|s| s.grade == Grade::Lean).count(), 14);
        assert_eq!(t.spots.iter().filter(|s| s.grade == Grade::Tour).count(), 9);
        assert_eq!(t.rules.len(), 10);
        let one: &str = "{\"format\":1,\"spots\":[{\"hash\":\"1\",\"name\":\"A\",\"grade\":\"release\"},{\"hash\":\"1\",\"name\":\"B\",\"grade\":\"lean\"}],\"rules\":[]}";
        assert!(parse_names(one).is_err());
        assert!(parse_names("{\"format\":1,\"spots\":[{\"hash\":\"x\",\"name\":\"A\",\"grade\":\"release\"}],\"rules\":[]}").is_err());
        assert!(parse_names("{\"format\":1,\"spots\":[{\"hash\":\"1\",\"name\":\"A\",\"grade\":\"maybe\"}],\"rules\":[]}").is_err());
        assert!(parse_names("{\"format\":2}").is_err());
    }

    #[test]
    fn every_name_is_a_token_spot_and_every_hash_lands_once() {
        let d: Data = Data::load().unwrap();
        let t: Names = names().unwrap();
        let mut all: BS<&str> = d.cat.locs.iter().filter(|l| l.kind == KIND).map(|l| l.name.as_str()).collect();
        for s in &t.spots {
            assert_eq!(d.vanilla_of(&s.name), Some(TOKEN), "{}", s.name);
            all.insert(s.name.as_str());
        }
        assert_eq!(all.len(), 100);
        let mut by_hash: BM<u64, &str> = BM::new();
        for n in &all {
            for h in hashes(&d, n) {
                assert!(by_hash.insert(h, n).is_none(), "{h} is given to two names");
            }
        }
        let mut want: BS<u64> = d.cat.locs.iter().filter(|l| l.kind == KIND).flat_map(|l| l.refs.iter().map(|r| r.hash)).collect();
        want.extend(d.cat.gap_rem.iter().filter(|g| g.kind == KIND).flat_map(|g| g.refs.iter().map(|r| r.hash)));
        assert_eq!(by_hash.keys().copied().collect::<BS<u64>>(), want);
    }

    #[test]
    fn pinned_rows_keep_their_hash() {
        let d: Data = Data::load().unwrap();
        let c: Vec<String> = crate::slice::skulls::table().unwrap().contested;
        for l in d.cat.locs.iter().filter(|l| l.kind == KIND && l.conf != Conf::Pool && !c.contains(&l.name)) {
            let h: Vec<u64> = l.refs.iter().map(|r| r.hash).collect();
            assert_eq!(hashes(&d, &l.name), h, "{}", l.name);
        }
        assert_eq!(extra(&d), vec!["LW GS Above Theater", "ZR GS Tree", "Colossus GS Hill", "Colossus GS Tree"]);
    }

    #[test]
    fn only_matched_mode_reads_the_grades() {
        assert_eq!(verdict(Mode::Pinned, "LH GS Tree"), None);
        assert_eq!(verdict(Mode::Off, "LH GS Tree"), None);
        assert_eq!(verdict(Mode::Matched, "LH GS Tree"), Some(None));
        assert_eq!(verdict(Mode::Matched, "Colossus GS Tree"), Some(Some(HELD.to_string())));
        assert_eq!(verdict(Mode::Matched, "KF GS Bean Patch"), None);
    }

    #[test]
    fn matched_seeds_keep_the_house_of_skulltula_rewards_in_logic() {
        let d: Data = Data::load().unwrap();
        let mut s: Settings = Settings::default();
        s.set("shuffle_skulltulas", Value::s("matched")).unwrap();
        s.set("shuffle_rewards", Value::s("supported")).unwrap();
        let seed: &str = "skull names kak";
        let g: Generated = generate_data(&d, &s, seed, &GenOpts::default()).unwrap();
        let p: Plan = plan(&s).unwrap();
        let mut w: World = crate::slice::logic::world_with(&d, &p.logic()).unwrap();
        crate::slice::starts::add(&d, &p, &mut w, seed).unwrap();
        let mut pl: Placement = crate::slice::fill::vanilla_place(&w, &d).unwrap();
        for (loc, item) in &g.seed_file.placements {
            pl.put(w.loc(loc).unwrap(), w.item(item).unwrap());
        }
        let r: Search = explore(&w, &pl, &[]);
        assert!(r.won());
        for n in ["Kak 10 Gold Skulltula Reward", "Kak 20 Gold Skulltula Reward", "Kak 30 Gold Skulltula Reward", "Kak 40 Gold Skulltula Reward", "Kak 50 Gold Skulltula Reward"] {
            assert!(r.loc_access(w.loc(n).unwrap()), "{n}");
        }
        assert!(crate::slice::skulls::reachable_tokens(&w, &pl) >= 98);
        assert_eq!(g.seed_file.placements.keys().filter(|k| crate::slice::skulls::is_skull(&d, k)).count(), 77);
    }

    fn real() -> Option<PB> {
        let p: PB = std::env::var("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|_| PB::from("C:/o/nup dump/updates/131072/romfs"));
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Some(p)
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    #[test]
    fn every_named_hash_is_a_gold_skulltula_in_the_dump() {
        let Some(root) = real() else {
            return;
        };
        let d: Data = Data::load().unwrap();
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut found: BS<u64> = BS::new();
        for f in banc_files(&rf).unwrap() {
            let y: Byml = Byml::parse(&rf.read_zs(&f).unwrap()).unwrap();
            let Some(list) = y.root.get("Actors").and_then(|a| a.as_arr()) else {
                continue;
            };
            for a in list {
                let g: Option<&Node> = a.get("Gyaml");
                if g.and_then(|g| g.as_str()) == Some(crate::slice::skull_edit::SKULL) {
                    found.extend(hash_of(a));
                }
            }
        }
        assert_eq!(found.len(), 100);
        let t: Names = names().unwrap();
        for s in &t.spots {
            assert!(found.contains(&s.hash), "{}", s.name);
        }
        let mut all: BS<u64> = BS::new();
        for l in d.cat.locs.iter().filter(|l| l.kind == KIND) {
            all.extend(hashes(&d, &l.name));
        }
        for n in extra(&d) {
            all.extend(hashes(&d, n));
        }
        assert_eq!(all, found);
    }
}
