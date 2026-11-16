use std::collections::BTreeMap as BM;

use crate::error::{Error as E, Res};
use crate::pot_slice::data::Kind;
use crate::pickup_pack::fall::dropped;
use crate::pickup_pack::kit::name_of;
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
    pub kit: bool,
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

pub fn actor_of(d: &Data, loc: &str, item: &str, fixed: &BM<String, String>) -> Res<(String, bool)> {
    if let Some(a) = pickup::shipped_kit(item) {
        return Ok((name_of(a), true));
    }
    if let Some(a) = pickup::of(item) {
        return Ok((dropped(a), pickup::holds(item)));
    }
    crate::slice::key_spots::resolve(d, item, fixed.get(loc).map(|s| s.as_str()))
        .map(|p| (p.kit(), true))
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
        let (actor, kit): (String, bool) = actor_of(d, loc, item, fixed)?;
        if kit && !k.kit() {
            return Err(E::Msg(format!("{loc} is a {}, which has no variant that keeps {item} until it is picked up", l.kind.replace('_', " "))));
        }
        for r in &l.refs {
            out.push(Give { name: loc.clone(), kind: k, hash: r.hash, files: r.files.clone(), was: l.item.clone(), item: item.clone(), actor: actor.clone(), kit });
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
    use crate::pot_slice::data::Kind;
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
        let once: String = eligible(&d).into_iter().find(|n| crate::slice::copies::copies(&d, n) < 2 && d.pots.kind_of(n) == Some(Kind::Pot)).unwrap();
        let t: Vec<Give> = targets(&d, &seed(&[(&once, "Megaton Hammer")])).unwrap();
        assert!(t.iter().all(|g| g.kit && g.actor == "Obj_Hammer_GS"));
        assert!(targets(&d, &seed(&[(&once, "Fire Arrows")])).unwrap().iter().all(|g| g.kit && g.actor == "Obj_Arrow_Fire_GS"));
        assert!(targets(&d, &seed(&[(&once, "Gold Skulltula Token")])).unwrap().iter().all(|g| g.kit && g.actor == "Obj_Token_Gold_Fall"));
        assert!(targets(&d, &seed(&[(&once, "Rupees (20)")])).unwrap().iter().all(|g| !g.kit));
        let bad: Option<String> = d.pots.state.iter().find(|(_, s)| **s != State::Eligible).map(|(n, _)| n.clone());
        assert!(targets(&d, &seed(&[(&bad.unwrap(), "Rupees (20)")])).is_err());
    }

    #[test]
    fn a_token_from_any_container_is_the_falling_twin_and_every_container_takes_it() {
        let d: Data = Data::load().unwrap();
        let twins: &str = "KF House of Twins Pot 1";
        let t: Vec<Give> = targets(&d, &seed(&[(twins, "Gold Skulltula Token")])).unwrap();
        assert_eq!(t.len(), 2);
        let files: std::collections::BTreeSet<&str> = t.iter().flat_map(|g| g.files.iter().map(|f| f.as_str())).collect();
        assert!(files.iter().any(|f| f.contains("_Child_")) && files.iter().any(|f| f.contains("_Adult_")), "{files:?}");
        assert!(t.iter().all(|g| g.kit && g.actor == "Obj_Token_Gold_Fall" && g.kind == Kind::Pot));
        let boxes: Option<String> = eligible(&d).into_iter().find(|n| d.pots.kind_of(n).is_some_and(|k| k != Kind::Pot) && !d.pots.is_remake(n));
        let other: String = boxes.unwrap();
        let o: Vec<Give> = targets(&d, &seed(&[(&other, "Gold Skulltula Token")])).unwrap();
        assert!(!o.is_empty() && o.iter().all(|g| g.kit && g.actor == "Obj_Token_Gold_Fall" && g.kind != Kind::Pot), "{o:?}");
        assert_eq!(actor_of(&d, &other, "Gold Skulltula Token", &BM::new()).unwrap(), ("Obj_Token_Gold_Fall".to_string(), true));
        assert_eq!(actor_of(&d, &other, "Rupees (20)", &BM::new()).unwrap(), ("Obj_Rupee_Red".to_string(), false));
        assert_eq!(actor_of(&d, &other, "Fire Arrows", &BM::new()).unwrap(), ("Obj_Arrow_Fire_GS".to_string(), true));
    }

    #[test]
    fn the_falling_token_serves_every_container_kind() {
        let d: Data = Data::load().unwrap();
        let mut seen: Vec<Kind> = Vec::new();
        for k in [Kind::Pot, Kind::Crate, Kind::Flying, Kind::Hive, Kind::Rock, Kind::Grass] {
            let Some(n) = eligible(&d).into_iter().find(|n| d.pots.kind_of(n) == Some(k)) else {
                continue;
            };
            assert_eq!(actor_of(&d, &n, "Gold Skulltula Token", &BM::new()).unwrap(), ("Obj_Token_Gold_Fall".to_string(), true), "{n}");
            let t: Vec<Give> = targets(&d, &seed(&[(&n, "Gold Skulltula Token")])).unwrap();
            assert!(!t.is_empty() && t.iter().all(|g| g.kit && g.kind == k && g.actor == "Obj_Token_Gold_Fall"), "{n} {t:?}");
            seen.push(k);
        }
        assert!(seen.len() >= 5, "{seen:?}");
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

    #[test]
    fn a_placed_beehive_becomes_three_gives_and_takes_a_kit() {
        let d: Data = Data::load().unwrap();
        let t: Vec<Give> = targets(&d, &seed(&[("Kak Hole Beehive 1", "Rupees (20)")])).unwrap();
        assert_eq!(t.len(), 3);
        assert!(t.iter().all(|g| g.kind == Kind::Hive && g.actor == "Obj_Rupee_Red" && !g.kit && g.was.is_empty()));
        assert_eq!(t.iter().map(|g| g.hash).collect::<std::collections::BTreeSet<u64>>().len(), 3);
        let blue: Vec<Give> = targets(&d, &seed(&[("Kak Hole Beehive 3", "Recovery Heart")])).unwrap();
        assert!(blue.iter().all(|g| g.was == "Obj_Rupee_Blue" && g.actor == "Obj_Heart"));
        let kit: Vec<Give> = targets(&d, &seed(&[("Kak Hole Beehive 1", "Megaton Hammer")])).unwrap();
        assert!(kit.len() == 3 && kit.iter().all(|g| g.kit && g.actor == "Obj_Hammer_GS"));
        let rock: Vec<Give> = targets(&d, &seed(&[("KF Child Rock 1", "Piece of Heart")])).unwrap();
        assert!(rock.len() == 1 && rock[0].kit && rock[0].kind == Kind::Rock && rock[0].actor == "Obj_Heart_Piece_GS");
    }
}
