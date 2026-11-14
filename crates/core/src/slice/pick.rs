use std::collections::BTreeMap as BM;

use crate::slice::data::{Data, State};
use crate::slice::pickups as pu;
use crate::slice::support::{Keep, Mode, Plan, Songs};
use crate::world::{LocId, World};

#[derive(Debug, Clone)]
pub struct Pick {
    pub free: Vec<LocId>,
    pub left: BM<String, usize>,
    pub flow: usize,
}

const COUNTED_VANILLA: [&str; 3] = ["Small Key (Fire Temple)", "Small Key (Water Temple)", "Small Key (Spirit Temple)"];

fn class(kind: &str) -> u8 {
    match kind {
        "SmallKey" => 1,
        "BossKey" => 2,
        "GanonBossKey" => 3,
        "Map" | "Compass" => 4,
        _ => 0,
    }
}

pub fn scoped(p: &Plan, kind: &str) -> bool {
    match class(kind) {
        1 => p.small == Keep::Dungeon,
        2 => p.boss == Keep::Dungeon,
        3 => p.ganon == Keep::Dungeon,
        4 => p.map == Keep::Dungeon,
        _ => false,
    }
}

fn pinned_vanilla(p: &Plan, kind: &str) -> bool {
    match class(kind) {
        1 => p.small == Keep::Vanilla,
        2 => p.boss == Keep::Vanilla,
        3 => p.ganon == Keep::Vanilla,
        4 => p.map == Keep::Vanilla,
        _ => false,
    }
}

fn reason(d: &Data, w: &World, p: &Plan, name: &str) -> Option<String> {
    let Some(l) = w.loc(name) else {
        return Some("location is not in the logic world".to_string());
    };
    if !p.remake && crate::slice::remake_chests::gated(name) {
        return Some(crate::slice::remake_chests::OFF.to_string());
    }
    if !p.remake && crate::slice::held_chests::gated(name) {
        return Some(crate::slice::remake_chests::OFF.to_string());
    }
    let dungeon: bool = w.regions[w.locs[l].region].dungeon.is_some();
    match (p.chests, dungeon) {
        (Mode::Off, _) => return Some("chest shuffle is off".to_string()),
        (Mode::Overworld, true) => return Some("chest shuffle is overworld only".to_string()),
        (Mode::Dungeons, false) => return Some("chest shuffle is dungeons only".to_string()),
        _ => {}
    }
    let kind: &str = d.vanilla_of(name).and_then(|v| d.item(v)).map(|i| i.kind.as_str()).unwrap_or("");
    if pinned_vanilla(p, kind) {
        return Some("its dungeon item setting is vanilla".to_string());
    }
    if !p.keys_roam() && d.vanilla_of(name).is_some_and(|v| COUNTED_VANILLA.contains(&v)) {
        return Some("its dungeon's keys are counted for the vanilla layout".to_string());
    }
    if d.keeps_event(name) && !d.special(name) && d.vanilla_of(name).is_some_and(|v| d.family(v).is_some()) {
        return Some("it holds an upgrade item behind a story event".to_string());
    }
    None
}

pub fn pick(d: &Data, w: &World, p: &Plan) -> Pick {
    let mut free: Vec<LocId> = Vec::new();
    let mut left: BM<String, usize> = BM::new();
    for r in &d.scan.rows {
        let open: bool = r.state == State::Eligible || (r.state == State::Event && (d.event_ok(&r.name) || crate::slice::treasure::opens(p.treasure, &r.name)));
        let why: Option<String> = if !open {
            Some(r.state.why().to_string())
        } else {
            reason(d, w, p, &r.name)
        };
        match why {
            None => {
                if let Some(l) = w.loc(&r.name) {
                    free.push(l);
                }
            }
            Some(m) => *left.entry(m).or_insert(0) += 1,
        }
    }
    let mut flow: usize = 0;
    if p.rewards {
        for l in &d.flow.locs {
            if l.cat == "song" && p.songs != Songs::Off {
                continue;
            }
            if l.cat == crate::slice::support::DUNGEON_REWARD_CAT && !p.dungeon_rewards {
                continue;
            }
            if crate::scrubs_a::is_a(l) && !p.scrubs {
                continue;
            }
            if crate::scrubs_b::is_b(l) && !p.scrubs_b {
                continue;
            }
            if !d.flow_ok(&l.name) || !crate::flow_extra::open(l, p) {
                *left.entry("reward event is not supported yet".to_string()).or_insert(0) += 1;
                continue;
            }
            flow += 1;
            match w.loc(&l.name) {
                Some(x) => free.push(x),
                None => *left.entry("location is not in the logic world".to_string()).or_insert(0) += 1,
            }
        }
    }
    if p.songs != Songs::Off {
        for l in d.flow.locs.iter().filter(|l| l.cat == "song") {
            flow += 1;
            if !d.song_ok(&l.name) {
                *left.entry("this song lesson cannot hand out another item safely yet".to_string()).or_insert(0) += 1;
            } else {
                match w.loc(&l.name) {
                    Some(x) => free.push(x),
                    None => *left.entry("location is not in the logic world".to_string()).or_insert(0) += 1,
                }
            }
        }
    }
    if p.hearts {
        for l in d.cat.locs.iter().filter(|l| l.kind == "bossheart") {
            flow += 1;
            match w.loc(&l.name) {
                Some(x) => free.push(x),
                None => *left.entry("location is not in the logic world".to_string()).or_insert(0) += 1,
            }
        }
    }
    pu::select(d, w, p, &mut free, &mut left, &mut flow);
    flow += crate::pot_slice::pick::add(d, w, p, &mut free, &mut left);
    let (sk, n): (Vec<LocId>, usize) = crate::slice::skulls::pick(d, w, p, &mut left);
    free.extend(sk);
    flow += n;
    free.sort_unstable();
    free.dedup();
    Pick { free, left, flow }
}
