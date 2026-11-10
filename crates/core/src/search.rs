use std::cell::RefCell;

use crate::logic_items::ItemId;
use crate::rules::{eval, Age, Ctx, Reach, Rule};
use crate::world::{Exit, ExitId, Loc, LocId, Placement, RegionId, World};

#[derive(Clone)]
pub struct Search<'a> {
    w: &'a World,
    pub cnt: Vec<u32>,
    pub visited: Vec<bool>,
    reached: [Vec<bool>; 2],
    tods: RefCell<[Vec<u8>; 2]>,
    open: [Vec<bool>; 2],
    queued: [Vec<bool>; 2],
    queue: [Vec<u32>; 2],
    dirty: bool,
}

impl<'a> Search<'a> {
    pub fn new(w: &'a World) -> Search<'a> {
        let n: usize = w.nspots();
        let r: usize = w.regions.len();
        let mut s: Search<'a> = Search {
            w,
            cnt: vec![0; w.items.len()],
            visited: vec![false; w.locs.len()],
            reached: [vec![false; r], vec![false; r]],
            tods: RefCell::new([vec![0; r], vec![0; r]]),
            open: [vec![false; n], vec![false; n]],
            queued: [vec![false; n], vec![false; n]],
            queue: [Vec::new(), Vec::new()],
            dirty: false,
        };
        for (i, c) in &w.start {
            s.cnt[*i] += *c;
        }
        for a in 0..2 {
            s.reach(a, w.root, None);
        }
        s
    }

    pub fn world(&self) -> &'a World {
        self.w
    }

    pub fn add(&mut self, i: ItemId, n: u32) {
        self.cnt[i] += n;
        self.dirty = true;
        let w: &World = self.w;
        for s in &w.users[i] {
            for a in 0..2 {
                self.push(a, *s as usize, false);
            }
        }
    }

    pub fn remove(&mut self, i: ItemId) {
        if self.cnt[i] > 0 {
            self.cnt[i] -= 1;
        }
    }

    pub fn add_all(&mut self, v: &[ItemId]) {
        for i in v {
            self.add(*i, 1);
        }
    }

    fn push(&mut self, a: usize, s: usize, open: bool) {
        if open {
            self.open[a][s] = true;
        }
        if self.open[a][s] && !self.queued[a][s] {
            self.queued[a][s] = true;
            self.queue[a].push(s as u32);
        }
    }

    fn wanted(&self, l: LocId, place: Option<&Placement>) -> bool {
        if self.visited[l] {
            return false;
        }
        if self.w.locs[l].event.is_some() {
            return true;
        }
        match place {
            Some(p) => p.get(l).is_some_and(|i| self.w.items.adv[i]),
            None => false,
        }
    }

    fn reach(&mut self, a: usize, r: RegionId, place: Option<&Placement>) {
        if self.reached[a][r] {
            return;
        }
        self.reached[a][r] = true;
        self.dirty = true;
        let w: &World = self.w;
        self.tods.get_mut()[a][r] = w.regions[r].provides_time;
        for e in &w.regions[r].exits {
            if !self.reached[a][w.exits[*e].to] {
                self.push(a, *e, true);
            }
        }
        let ne: usize = w.exits.len();
        for l in &w.regions[r].locations {
            if self.wanted(*l, place) {
                self.push(a, ne + *l, true);
            }
        }
    }

    fn rescan(&mut self, place: &Placement) {
        let w: &World = self.w;
        let ne: usize = w.exits.len();
        for l in &w.real {
            if !self.wanted(*l, Some(place)) {
                continue;
            }
            let r: RegionId = w.locs[*l].region;
            for a in 0..2 {
                if self.reached[a][r] && !self.open[a][ne + *l] {
                    self.push(a, ne + *l, true);
                }
            }
        }
    }

    fn pop(&mut self) -> Option<(usize, usize)> {
        for a in 0..2 {
            if let Some(s) = self.queue[a].pop() {
                self.queued[a][s as usize] = false;
                return Some((a, s as usize));
            }
        }
        None
    }

    fn test(&self, r: &Rule, a: usize, region: RegionId) -> bool {
        if r.always {
            return true;
        }
        if r.never {
            return false;
        }
        let c: Ctx = Ctx {
            cnt: &self.cnt,
            age: Some(Age::BOTH[a]),
            tod: 0,
            region,
            q: Some(self),
        };
        eval(&r.node, &c)
    }

    fn step(&mut self, a: usize, s: usize, place: &Placement, immediate: bool, found: &mut Vec<LocId>) {
        if !self.open[a][s] {
            return;
        }
        let w: &World = self.w;
        let ne: usize = w.exits.len();
        if s < ne {
            let x: &Exit = &w.exits[s];
            if self.reached[a][x.to] {
                self.open[a][s] = false;
                return;
            }
            if self.test(&x.rule, a, x.from) {
                self.open[a][s] = false;
                self.reach(a, x.to, Some(place));
            }
            return;
        }
        let l: LocId = s - ne;
        if self.visited[l] {
            self.open[a][s] = false;
            return;
        }
        let lc: &Loc = &w.locs[l];
        if !self.test(&lc.rule, a, lc.region) {
            return;
        }
        self.visited[l] = true;
        self.open[0][s] = false;
        self.open[1][s] = false;
        self.dirty = true;
        match lc.event {
            Some(i) => self.add(i, 1),
            None => {
                found.push(l);
                if immediate && let Some(i) = place.get(l) {
                    self.add(i, 1);
                }
            }
        }
    }

    fn requeue_tod(&mut self) -> bool {
        let w: &World = self.w;
        let mut any: bool = false;
        for s in &w.tod_users {
            for a in 0..2 {
                if self.open[a][*s as usize] && !self.queued[a][*s as usize] {
                    self.push(a, *s as usize, false);
                    any = true;
                }
            }
        }
        any
    }

    pub fn settle(&mut self, place: &Placement, immediate: bool) -> Vec<LocId> {
        self.rescan(place);
        let mut found: Vec<LocId> = Vec::new();
        loop {
            while let Some((a, s)) = self.pop() {
                self.step(a, s, place, immediate, &mut found);
            }
            if self.dirty && !self.w.tod_users.is_empty() {
                self.dirty = false;
                if self.requeue_tod() {
                    continue;
                }
            }
            break;
        }
        found
    }

    pub fn won(&self) -> bool {
        let c: Ctx = Ctx {
            cnt: &self.cnt,
            age: None,
            tod: 0,
            region: self.w.root,
            q: None,
        };
        eval(&self.w.goal.node, &c)
    }

    pub fn region_reached(&self, r: RegionId, age: Option<Age>) -> bool {
        match age {
            Some(a) => self.reached[a.idx()][r],
            None => self.reached[0][r] || self.reached[1][r],
        }
    }

    pub fn loc_access(&self, l: LocId) -> bool {
        let lc: &Loc = &self.w.locs[l];
        (0..2).any(|a| self.reached[a][lc.region] && self.test(&lc.rule, a, lc.region))
    }

    pub fn expand_tod(&self, goal: RegionId, a: usize, flag: u8) -> bool {
        let w: &World = self.w;
        let mut t: Vec<u8> = self.tods.borrow()[a].clone();
        let mut q: Vec<ExitId> = Vec::new();
        for (r, f) in t.iter().enumerate() {
            if f & flag != 0 {
                q.extend(w.regions[r].exits.iter().copied());
            }
        }
        let mut k: usize = 0;
        let mut hit: bool = false;
        while k < q.len() {
            let x: &Exit = &w.exits[q[k]];
            k += 1;
            if !self.reached[a][x.to] || flag & !t[x.to] == 0 {
                continue;
            }
            let c: Ctx = Ctx {
                cnt: &self.cnt,
                age: Some(Age::BOTH[a]),
                tod: flag,
                region: x.from,
                q: None,
            };
            if x.rule.always || (!x.rule.never && eval(&x.rule.node, &c)) {
                t[x.to] |= flag;
                if x.to == goal {
                    hit = true;
                    break;
                }
                q.extend(w.regions[x.to].exits.iter().copied());
            }
        }
        self.tods.borrow_mut()[a] = t;
        hit
    }
}

impl Reach for Search<'_> {
    fn reach_tod(&self, region: usize, age: Age, flag: u8) -> bool {
        let a: usize = age.idx();
        if !self.reached[a][region] {
            return false;
        }
        if self.tods.borrow()[a][region] & flag != 0 {
            return true;
        }
        self.expand_tod(region, a, flag)
    }
}

pub fn explore<'a>(w: &'a World, place: &Placement, extra: &[ItemId]) -> Search<'a> {
    let mut s: Search<'a> = Search::new(w);
    s.add_all(extra);
    s.settle(place, true);
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Spec;

    fn world(settings: &[(&str, bool)], start: &[&str], regions: &str, helpers: &str) -> World {
        let goal: &str = if regions.contains("\"Done\"") { "Done" } else { "True" };
        let mut s: Spec = Spec::new();
        for n in ["Gem", "Ocarina", "Time_Travel", "Hammer"] {
            s.add_item(n, true, 1);
        }
        for (k, v) in settings {
            s.settings.set_bool(k, *v);
        }
        for n in start {
            s.start.push((n.to_string(), 1));
        }
        s.settings.set_str("starting_age", "child");
        s.add_helpers(helpers).unwrap();
        s.add_regions(regions).unwrap();
        s.goal = goal.to_string();
        World::build(&s).unwrap()
    }

    fn fill_gem(w: &World) -> Placement {
        let g: ItemId = w.item("Gem").unwrap();
        let mut p: Placement = Placement::new(w.locs.len());
        for l in &w.real {
            p.put(*l, g);
        }
        p
    }

    const AGES: &str = r#"[
        {"region_name": "Root", "exits": {"Root Exits": "is_starting_age or Time_Travel"},
         "events": {"Done": "False or Hammer"}},
        {"region_name": "Root Exits", "exits": {"Child Spawn": "is_child", "Adult Spawn": "is_adult"}},
        {"region_name": "Child Spawn", "exits": {"Meadow": "True"}},
        {"region_name": "Adult Spawn", "exits": {"Meadow": "True"}},
        {"region_name": "Meadow", "locations": {"Child Chest": "is_child", "Adult Chest": "is_adult",
                                                  "Any Chest": "True"}}
    ]"#;

    const AGE_HELPERS: &str = r#"{
        "is_child": "age == 'child'",
        "is_adult": "age == 'adult'",
        "is_starting_age": "age == starting_age"
    }"#;

    #[test]
    fn only_the_starting_age_is_reachable_without_time_travel() {
        let w: World = world(&[], &[], AGES, AGE_HELPERS);
        let p: Placement = fill_gem(&w);
        let s: Search = explore(&w, &p, &[]);
        let g: ItemId = w.item("Gem").unwrap();
        assert!(s.visited[w.loc("Child Chest").unwrap()]);
        assert!(!s.visited[w.loc("Adult Chest").unwrap()]);
        assert!(s.visited[w.loc("Any Chest").unwrap()]);
        assert_eq!(s.cnt[g], 2);
        let m: RegionId = w.region("Meadow").unwrap();
        assert!(s.region_reached(m, Some(Age::Child)));
        assert!(!s.region_reached(m, Some(Age::Adult)));
    }

    #[test]
    fn time_travel_opens_the_other_age() {
        let w: World = world(&[], &["Time_Travel"], AGES, AGE_HELPERS);
        let p: Placement = fill_gem(&w);
        let s: Search = explore(&w, &p, &[]);
        let g: ItemId = w.item("Gem").unwrap();
        assert!(s.visited[w.loc("Adult Chest").unwrap()]);
        assert_eq!(s.cnt[g], 3);
    }

    #[test]
    fn items_found_later_unlock_parked_rules() {
        let w: World = world(&[], &[], AGES, AGE_HELPERS);
        let mut s: Search = Search::new(&w);
        let p: Placement = fill_gem(&w);
        s.settle(&p, true);
        assert!(!s.visited[w.loc("Adult Chest").unwrap()]);
        s.add(w.item("Time_Travel").unwrap(), 1);
        s.settle(&p, true);
        assert!(s.visited[w.loc("Adult Chest").unwrap()]);
    }

    #[test]
    fn events_gate_the_goal() {
        let w: World = world(&[], &[], AGES, AGE_HELPERS);
        let p: Placement = fill_gem(&w);
        assert!(!explore(&w, &p, &[]).won());
        assert!(explore(&w, &p, &[w.item("Hammer").unwrap()]).won());
    }

    const NIGHT: &str = r#"[
        {"region_name": "Root", "exits": {"Field": "True"}},
        {"region_name": "Field", "locations": {"Night Chest": "at_night", "Day Chest": "at_day"},
         "exits": {"Inn": "True", "Cellar": "True"}},
        {"region_name": "Inn", "time_passes": true, "exits": {"Field": "True"}},
        {"region_name": "Cellar", "exits": {}}
    ]"#;

    const NIGHT_BLIND: &str = r#"[
        {"region_name": "Root", "exits": {"Field": "True"}},
        {"region_name": "Field", "locations": {"Night Chest": "at_night"}, "exits": {"Cellar": "True"}},
        {"region_name": "Cellar", "exits": {"Field": "True"}}
    ]"#;

    #[test]
    fn a_time_passing_region_makes_night_reachable() {
        let w: World = world(&[("ensure_tod_access", true)], &[], NIGHT, "{}");
        let p: Placement = fill_gem(&w);
        let s: Search = explore(&w, &p, &[]);
        assert!(s.visited[w.loc("Night Chest").unwrap()]);
        assert!(s.visited[w.loc("Day Chest").unwrap()]);
    }

    #[test]
    fn night_needs_a_skip_when_no_region_passes_time() {
        let helpers: &str = r#"{"skip_to_night": "Ocarina"}"#;
        let w: World = world(&[("ensure_tod_access", true)], &[], NIGHT_BLIND, helpers);
        let p: Placement = fill_gem(&w);
        let s: Search = explore(&w, &p, &[]);
        assert!(!s.visited[w.loc("Night Chest").unwrap()]);
        let t: Search = explore(&w, &p, &[w.item("Ocarina").unwrap()]);
        assert!(t.visited[w.loc("Night Chest").unwrap()]);
        let off: World = world(&[("ensure_tod_access", false)], &[], NIGHT_BLIND, helpers);
        let p2: Placement = fill_gem(&off);
        assert!(explore(&off, &p2, &[]).visited[off.loc("Night Chest").unwrap()]);
    }

    #[test]
    fn spheres_do_not_collect_mid_sphere() {
        let regions: &str = r#"[
            {"region_name": "Root", "locations": {"First": "True", "Second": "Gem"}}
        ]"#;
        let w: World = world(&[], &[], regions, "{}");
        let mut p: Placement = Placement::new(w.locs.len());
        p.put(w.loc("First").unwrap(), w.item("Gem").unwrap());
        let mut s: Search = Search::new(&w);
        let one: Vec<LocId> = s.settle(&p, false);
        assert_eq!(one, vec![w.loc("First").unwrap()]);
        s.add(w.item("Gem").unwrap(), 1);
        let two: Vec<LocId> = s.settle(&p, false);
        assert!(two.is_empty());
        let q: Placement = {
            let mut q: Placement = p.clone();
            q.put(w.loc("Second").unwrap(), w.item("Gem").unwrap());
            q
        };
        let three: Vec<LocId> = s.settle(&q, false);
        assert_eq!(three, vec![w.loc("Second").unwrap()]);
    }

    fn naive(w: &World, place: &Placement, extra: &[ItemId]) -> Vec<u32> {
        let mut cnt: Vec<u32> = vec![0; w.items.len()];
        for (i, c) in &w.start {
            cnt[*i] += *c;
        }
        for i in extra {
            cnt[*i] += 1;
        }
        let mut done: Vec<bool> = vec![false; w.locs.len()];
        loop {
            let mut changed: bool = false;
            for a in 0..2 {
                let age: Age = Age::BOTH[a];
                let mut reach: Vec<bool> = vec![false; w.regions.len()];
                reach[w.root] = true;
                let mut moved: bool = true;
                while moved {
                    moved = false;
                    for x in &w.exits {
                        let c: Ctx = Ctx { cnt: &cnt, age: Some(age), tod: 0, region: x.from, q: None };
                        if reach[x.from] && !reach[x.to] && eval(&x.rule.node, &c) {
                            reach[x.to] = true;
                            moved = true;
                        }
                    }
                }
                for (l, lc) in w.locs.iter().enumerate() {
                    if done[l] || !reach[lc.region] {
                        continue;
                    }
                    let item: Option<ItemId> = match lc.event {
                        Some(i) => Some(i),
                        None => place.get(l).filter(|i| w.items.adv[*i]),
                    };
                    let Some(i) = item else { continue };
                    let c: Ctx = Ctx { cnt: &cnt, age: Some(age), tod: 0, region: lc.region, q: None };
                    if eval(&lc.rule.node, &c) {
                        done[l] = true;
                        cnt[i] += 1;
                        changed = true;
                    }
                }
            }
            if !changed {
                return cnt;
            }
        }
    }

    #[test]
    fn incremental_search_matches_a_naive_fixpoint() {
        use crate::fill::{fill, FillCfg};
        use crate::logic_synth::synth_world;
        use crate::rng::Rng;
        let mut r: Rng = Rng::from_seed(77);
        let mut checked: usize = 0;
        let mut partial: usize = 0;
        let mut deep: usize = 0;
        for k in 0..24u64 {
            let n: usize = 40 + r.below(300) as usize;
            let w: World = synth_world(n, k).unwrap();
            let pool: Vec<ItemId> = w.pool();
            let base: Placement = fill(&w, &pool, &mut Rng::from_seed(k), &FillCfg::new()).unwrap().place;
            for drop in [0u64, 10, 40] {
                let mut p: Placement = base.clone();
                for l in &w.real {
                    if r.below(100) < drop {
                        p.item[*l] = None;
                    }
                }
                let fast: Search = explore(&w, &p, &[]);
                let slow: Vec<u32> = naive(&w, &p, &[]);
                let placed: usize = p.item.iter().flatten().filter(|i| w.items.adv[**i]).count();
                let got: u32 = (0..w.items.len()).filter(|i| !w.items.event[*i]).map(|i| fast.cnt[i]).sum();
                if (got as usize) < placed {
                    partial += 1;
                }
                if got >= 5 {
                    deep += 1;
                }
                assert_eq!(fast.cnt, slow, "world {k} with {n} locations");
                let mut extra: Vec<ItemId> = pool.clone();
                r.shuffle(&mut extra);
                extra.truncate(5);
                let fast2: Search = explore(&w, &p, &extra);
                assert_eq!(fast2.cnt, naive(&w, &p, &extra));
                checked += 1;
            }
        }
        assert_eq!(checked, 72);
        assert!(partial > 10 && deep > 10, "partial {partial}, deep {deep}");
    }

    #[test]
    fn clones_are_independent() {
        let w: World = world(&[], &[], AGES, AGE_HELPERS);
        let p: Placement = fill_gem(&w);
        let base: Search = Search::new(&w);
        let mut a: Search = base.clone();
        a.add(w.item("Time_Travel").unwrap(), 1);
        a.settle(&p, true);
        let mut b: Search = base.clone();
        b.settle(&p, true);
        assert!(a.visited[w.loc("Adult Chest").unwrap()]);
        assert!(!b.visited[w.loc("Adult Chest").unwrap()]);
    }
}
