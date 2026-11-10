use std::collections::BTreeMap as BM;

use crate::slice::data::{Data, State};
use crate::slice::support::{Keep, Mode, Plan};
use crate::world::{LocId, World};

#[derive(Debug, Clone)]
pub struct Pick {
    pub free: Vec<LocId>,
    pub left: BM<String, usize>,
    pub flow: usize,
}

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
    if p.excluded.iter().any(|e| e == name) {
        return Some("excluded by the player".to_string());
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
    None
}

pub fn pick(d: &Data, w: &World, p: &Plan) -> Pick {
    let mut free: Vec<LocId> = Vec::new();
    let mut left: BM<String, usize> = BM::new();
    for r in &d.scan.rows {
        let open: bool = r.state == State::Eligible || (r.state == State::Event && d.event_ok(&r.name));
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
            if !d.flow_ok(&l.name) {
                *left.entry("reward event is not supported yet".to_string()).or_insert(0) += 1;
                continue;
            }
            flow += 1;
            if p.excluded.iter().any(|e| *e == l.name) {
                *left.entry("excluded by the player".to_string()).or_insert(0) += 1;
                continue;
            }
            match w.loc(&l.name) {
                Some(x) => free.push(x),
                None => *left.entry("location is not in the logic world".to_string()).or_insert(0) += 1,
            }
        }
    }
    free.sort_unstable();
    Pick { free, left, flow }
}
