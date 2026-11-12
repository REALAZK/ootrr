use std::collections::BTreeMap as BM;

use crate::pot_slice::data::Kind;
use crate::slice::data::{Data, State};
use crate::slice::support::{Mode, Plan};
use crate::world::{LocId, World};

pub fn why(s: State) -> &'static str {
    match s {
        State::Eligible => "shuffled",
        State::Pooled | State::Pool => "the match is only a guess inside a pool",
        State::Special => "not a plain breakable with a safe saved row",
        State::Unmapped => "its vanilla item has no world pickup in the remake",
        State::Mismatch => "the actor was not found or holds something else",
        State::Event | State::Check => "not a plain breakable",
    }
}

fn mode_of(p: &Plan, k: Kind) -> Mode {
    match k {
        Kind::Crate => p.drops.crates,
        _ => p.drops.pots,
    }
}

fn cut(m: Mode, dungeon: bool) -> Option<&'static str> {
    match (m, dungeon) {
        (Mode::Overworld, true) => Some("its shuffle setting is overworld only"),
        (Mode::Dungeons, false) => Some("its shuffle setting is dungeons only"),
        _ => None,
    }
}

pub fn add(d: &Data, w: &World, p: &Plan, free: &mut Vec<LocId>, left: &mut BM<String, usize>) -> usize {
    let mut seen: usize = 0;
    for l in &d.pots.locs {
        let Some(k) = Kind::of(&l.kind) else {
            continue;
        };
        let m: Mode = mode_of(p, k);
        if m == Mode::Off {
            continue;
        }
        seen += 1;
        let st: State = d.pots.state_of(&l.name).unwrap_or(State::Mismatch);
        let id: Option<LocId> = w.loc(&l.name);
        let dungeon: bool = id.is_some_and(|i| w.regions[w.locs[i].region].dungeon.is_some());
        let why: Option<&str> = if st != State::Eligible {
            Some(why(st))
        } else if p.excluded.iter().any(|e| *e == l.name) {
            Some("excluded by the player")
        } else if id.is_none() {
            Some("location is not in the logic world")
        } else {
            cut(m, dungeon)
        };
        match (why, id) {
            (None, Some(i)) => free.push(i),
            (Some(r), _) => *left.entry(r.to_string()).or_insert(0) += 1,
            (None, None) => {}
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Settings, Value};
    use crate::slice::logic::world;
    use crate::slice::support::plan;

    fn run(pots: &str, crates: &str) -> (usize, usize, Vec<String>) {
        let d: Data = Data::load().unwrap();
        let mut s: Settings = Settings::default();
        s.set("shuffle_pots", Value::s(pots)).unwrap();
        s.set("shuffle_crates", Value::s(crates)).unwrap();
        let p: Plan = plan(&s).unwrap();
        let w: World = world(&d).unwrap();
        let mut free: Vec<LocId> = Vec::new();
        let mut left: BM<String, usize> = BM::new();
        let seen: usize = add(&d, &w, &p, &mut free, &mut left);
        let names: Vec<String> = free.iter().map(|i| w.locs[*i].name.clone()).collect();
        (seen, free.len(), names)
    }

    #[test]
    fn nothing_is_added_while_the_settings_are_off() {
        assert_eq!(run("off", "off").0, 0);
        assert!(run("off", "off").2.is_empty());
    }

    #[test]
    fn modes_split_dungeon_pots_from_overworld_pots() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let all: usize = run("all", "off").1;
        let dun: Vec<String> = run("dungeons", "off").2;
        let ove: Vec<String> = run("overworld", "off").2;
        assert!(all > 40, "{all}");
        assert_eq!(dun.len() + ove.len(), all);
        assert!(dun.iter().all(|n| w.regions[w.locs[w.loc(n).unwrap()].region].dungeon.is_some()));
        assert!(ove.iter().all(|n| w.regions[w.locs[w.loc(n).unwrap()].region].dungeon.is_none()));
        assert!(!dun.is_empty());
    }

    #[test]
    fn crates_follow_their_own_setting() {
        let only: (usize, usize, Vec<String>) = run("off", "all");
        assert!(only.1 >= 1);
        let d: Data = Data::load().unwrap();
        assert!(only.2.iter().all(|n| d.pots.kind_of(n) == Some(Kind::Crate)));
    }
}
