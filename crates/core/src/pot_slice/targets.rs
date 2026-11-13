use std::collections::BTreeMap as BM;

use crate::error::{Error as E, Res};
use crate::pot_slice::data::Kind;
use crate::pot_slice::pickup;
use crate::settings::seed::SeedFile;
use crate::slice::data::{Data, State};

#[derive(Debug, Clone, PartialEq)]
pub struct Give {
    pub name: String,
    pub kind: Kind,
    pub hash: u64,
    pub files: Vec<String>,
    pub was: String,
    pub item: String,
    pub actor: String,
}

pub fn is_loc(d: &Data, name: &str) -> bool {
    d.pots.has(name)
}

pub fn any(d: &Data, seed: &SeedFile) -> bool {
    seed.placements.keys().any(|k| is_loc(d, k))
}

pub fn targets(d: &Data, seed: &SeedFile) -> Res<Vec<Give>> {
    targets_with(d, seed, &BM::new())
}

pub fn actor_of(d: &Data, loc: &str, item: &str, fixed: &BM<String, String>) -> Res<String> {
    if let Some(a) = pickup::of(item) {
        return Ok(a.to_string());
    }
    crate::slice::key_spots::resolve(d, item, fixed.get(loc).map(|s| s.as_str()))
        .map(|p| p.pack)
        .map_err(|e| E::Msg(format!("{item} has no world pickup and cannot be placed at {loc}: {e}")))
}

pub fn targets_with(d: &Data, seed: &SeedFile, fixed: &BM<String, String>) -> Res<Vec<Give>> {
    let mut out: Vec<Give> = Vec::new();
    for (loc, item) in &seed.placements {
        let Some(l) = d.pots.get(loc) else {
            continue;
        };
        let k: Kind = Kind::of(&l.kind).ok_or_else(|| E::Msg(format!("{loc} is not a breakable this build knows")))?;
        if d.pots.state_of(loc) != Some(State::Eligible) {
            return Err(E::Msg(format!("{loc} cannot be placed, its match or its saved row is not safe")));
        }
        let actor: String = actor_of(d, loc, item, fixed)?;
        for r in &l.refs {
            out.push(Give { name: loc.clone(), kind: k, hash: r.hash, files: r.files.clone(), was: l.item.clone(), item: item.clone(), actor: actor.clone() });
        }
    }
    Ok(out)
}

pub fn balance(d: &Data, seed: &SeedFile, item: &str, acts: &mut Vec<String>) {
    let mut here: isize = 0;
    for (loc, got) in &seed.placements {
        if !is_loc(d, loc) {
            continue;
        }
        if d.vanilla_of(loc) == Some(item) {
            here += 1;
        }
        if got == item {
            here -= 1;
        }
    }
    if here > 0 {
        let canon: Option<String> = d.item(item).and_then(|i| i.actor.clone());
        for _ in 0..here {
            acts.extend(canon.clone());
        }
    } else if here < 0 {
        let cut: usize = acts.len().saturating_sub((-here) as usize);
        acts.truncate(cut);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use std::collections::BTreeMap as BM;

    fn seed(pairs: &[(&str, &str)]) -> SeedFile {
        let m: BM<String, String> = pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        SeedFile::new("s", &Settings::default(), "g", m).unwrap()
    }

    fn eligible(d: &Data) -> Vec<String> {
        let mut v: Vec<String> = d.pots.state.iter().filter(|(_, s)| **s == State::Eligible).map(|(n, _)| n.clone()).collect();
        v.sort();
        v
    }

    #[test]
    fn a_placed_pot_becomes_one_give_per_hash_with_the_pickup_actor() {
        let d: Data = Data::load().unwrap();
        let n: String = eligible(&d)[0].clone();
        let t: Vec<Give> = targets(&d, &seed(&[(&n, "Rupees (20)"), ("KF Midos Top Left Chest", "Boomerang")])).unwrap();
        assert_eq!(t.len(), d.pots.get(&n).unwrap().refs.len());
        assert!(t.iter().all(|g| g.actor == "Obj_Rupee_Red" && g.name == n));
    }

    #[test]
    fn unsafe_placements_are_refused() {
        let d: Data = Data::load().unwrap();
        let n: String = eligible(&d)[0].clone();
        assert!(targets(&d, &seed(&[(&n, "Hookshot")])).is_err());
        let bad: Option<String> = d.pots.state.iter().find(|(_, s)| **s != State::Eligible).map(|(n, _)| n.clone());
        assert!(targets(&d, &seed(&[(&bad.unwrap(), "Rupees (20)")])).is_err());
    }

    #[test]
    fn chest_actor_pools_follow_the_items_that_moved_through_pots() {
        let d: Data = Data::load().unwrap();
        let n: String = eligible(&d).into_iter().find(|n| d.vanilla_of(n) == Some("Recovery Heart")).unwrap();
        let moved: SeedFile = seed(&[(&n, "Rupees (5)")]);
        let mut hearts: Vec<String> = vec!["a".to_string(), "b".to_string()];
        balance(&d, &moved, "Recovery Heart", &mut hearts);
        assert_eq!(hearts.len(), 3);
        let mut blues: Vec<String> = vec!["a".to_string(), "b".to_string()];
        balance(&d, &moved, "Rupees (5)", &mut blues);
        assert_eq!(blues.len(), 1);
        let none: SeedFile = seed(&[("KF Midos Top Left Chest", "Rupees (5)")]);
        let mut same: Vec<String> = vec!["a".to_string()];
        balance(&d, &none, "Rupees (5)", &mut same);
        assert_eq!(same.len(), 1);
        assert!(any(&d, &moved) && !any(&d, &none));
    }
}
