use std::collections::{HashMap as HM, HashSet as HS};

use crate::error::{Error as E, Res};
use crate::logic_items::{ItemId, Items};
use crate::logic_json::{parse, Json};
use crate::logic_val::Settings;
use crate::rules::{Aliases, Compiler, Rule, Spot, Sub, ALL_TIME, DAMPE, DAY};

pub type RegionId = usize;
pub type ExitId = usize;
pub type LocId = usize;

#[derive(Clone, Debug)]
pub struct Region {
    pub name: String,
    pub provides_time: u8,
    pub locations: Vec<LocId>,
    pub exits: Vec<ExitId>,
    pub scene: Option<String>,
    pub dungeon: Option<String>,
    pub hint: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Exit {
    pub name: String,
    pub from: RegionId,
    pub to: RegionId,
    pub rule: Rule,
}

#[derive(Clone, Debug)]
pub struct Loc {
    pub name: String,
    pub region: RegionId,
    pub rule: Rule,
    pub event: Option<ItemId>,
}

#[derive(Clone, Debug)]
pub struct ItemDef {
    pub name: String,
    pub adv: bool,
    pub count: u32,
}

#[derive(Clone, Debug)]
pub struct Spec {
    pub settings: Settings,
    pub items: Vec<ItemDef>,
    pub groups: Vec<(String, Vec<String>)>,
    pub start: Vec<(String, u32)>,
    pub helpers: Vec<Json>,
    pub regions: Vec<Json>,
    pub goal: String,
    pub root: String,
    pub lenient: bool,
}

impl Default for Spec {
    fn default() -> Spec {
        Spec::new()
    }
}

fn str_list(j: &Json, what: &str) -> Res<Vec<String>> {
    let a: &[Json] = j.as_arr().ok_or_else(|| E::Msg(format!("{what} must be a list")))?;
    let mut out: Vec<String> = Vec::with_capacity(a.len());
    for x in a {
        out.push(x.as_str().ok_or_else(|| E::Msg(format!("{what} must hold strings")))?.to_string());
    }
    Ok(out)
}

impl Spec {
    pub fn new() -> Spec {
        Spec {
            settings: Settings::new(),
            items: Vec::new(),
            groups: Vec::new(),
            start: Vec::new(),
            helpers: Vec::new(),
            regions: Vec::new(),
            goal: "True".to_string(),
            root: "Root".to_string(),
            lenient: false,
        }
    }

    pub fn add_item(&mut self, name: &str, adv: bool, count: u32) {
        self.items.push(ItemDef { name: name.to_string(), adv, count });
    }

    pub fn add_regions(&mut self, text: &str) -> Res<()> {
        self.regions.push(parse(text)?);
        Ok(())
    }

    pub fn add_helpers(&mut self, text: &str) -> Res<()> {
        self.helpers.push(parse(text)?);
        Ok(())
    }

    pub fn from_bundle(text: &str) -> Res<Spec> {
        Spec::from_json(&parse(text)?)
    }

    pub fn from_json(j: &Json) -> Res<Spec> {
        let mut s: Spec = Spec::new();
        if let Some(g) = j.get("goal") {
            s.goal = g.as_str().ok_or(E::Format("goal must be a string"))?.to_string();
        }
        if let Some(r) = j.get("root") {
            s.root = r.as_str().ok_or(E::Format("root must be a string"))?.to_string();
        }
        if let Some(x) = j.get("settings") {
            s.settings = Settings::from_json(x)?;
        }
        if let Some(x) = j.get("items") {
            let a: &[Json] = x.as_arr().ok_or(E::Format("items must be a list"))?;
            for e in a {
                match e {
                    Json::Str(n) => s.add_item(n, false, 1),
                    Json::Obj(_) => {
                        let n: &str = e.get("name").and_then(|v| v.as_str()).ok_or(E::Format("item needs a name"))?;
                        let adv: bool = e.get("advancement").and_then(|v| v.as_bool()).unwrap_or(false);
                        let c: f64 = e.get("count").and_then(|v| v.as_f64()).unwrap_or(1.0).max(0.0);
                        if c > 1.0e6 {
                            return Err(E::Format("item count is too large"));
                        }
                        let c: u32 = c as u32;
                        s.add_item(n, adv, c);
                    }
                    _ => return Err(E::Format("bad item entry")),
                }
            }
        }
        if let Some(x) = j.get("groups") {
            let o: &[(String, Json)] = x.as_obj().ok_or(E::Format("groups must be an object"))?;
            for (k, v) in o {
                s.groups.push((k.clone(), str_list(v, "group")?));
            }
        }
        if let Some(x) = j.get("start") {
            match x {
                Json::Obj(o) => {
                    for (k, v) in o {
                        let n: f64 = v.as_f64().ok_or(E::Format("start counts must be numbers"))?;
                        s.start.push((k.clone(), n.max(0.0) as u32));
                    }
                }
                Json::Arr(_) => {
                    for n in str_list(x, "start")? {
                        s.start.push((n, 1));
                    }
                }
                _ => return Err(E::Format("bad start entry")),
            }
        }
        if let Some(x) = j.get("helpers") {
            match x {
                Json::Obj(_) => s.helpers.push(x.clone()),
                Json::Arr(a) => s.helpers.extend(a.iter().cloned()),
                _ => return Err(E::Format("bad helpers entry")),
            }
        }
        if let Some(x) = j.get("regions") {
            x.as_arr().ok_or(E::Format("regions must be a list"))?;
            s.regions.push(x.clone());
        }
        Ok(s)
    }
}

#[derive(Clone, Debug)]
pub struct Placement {
    pub item: Vec<Option<ItemId>>,
}

impl Placement {
    pub fn new(n: usize) -> Placement {
        Placement { item: vec![None; n] }
    }

    pub fn get(&self, l: LocId) -> Option<ItemId> {
        self.item.get(l).copied().flatten()
    }

    pub fn put(&mut self, l: LocId, i: ItemId) {
        self.item[l] = Some(i);
    }
}

#[derive(Clone, Debug)]
pub struct World {
    pub regions: Vec<Region>,
    pub exits: Vec<Exit>,
    pub locs: Vec<Loc>,
    pub items: Items,
    pub pool_counts: Vec<u32>,
    pub settings: Settings,
    pub goal: Rule,
    pub start: Vec<(ItemId, u32)>,
    pub groups: HM<String, Vec<ItemId>>,
    pub root: RegionId,
    pub real: Vec<LocId>,
    pub users: Vec<Vec<u32>>,
    pub tod_users: Vec<u32>,
    region_ix: HM<String, RegionId>,
    loc_ix: HM<String, LocId>,
}

struct Raw {
    name: String,
    scene: Option<String>,
    dungeon: Option<String>,
    hint: Option<String>,
    provides: u8,
    locations: Vec<(String, String)>,
    events: Vec<(String, String)>,
    exits: Vec<(String, String)>,
}

fn pairs(j: &Json, key: &str, region: &str) -> Res<Vec<(String, String)>> {
    let mut out: Vec<(String, String)> = Vec::new();
    if let Some(x) = j.get(key) {
        let o: &[(String, Json)] = x
            .as_obj()
            .ok_or_else(|| E::Msg(format!("region {region}: {key} must be an object")))?;
        for (k, v) in o {
            let r: &str = v
                .as_str()
                .ok_or_else(|| E::Msg(format!("region {region}: rule for {k} must be a string")))?;
            out.push((k.clone(), r.to_string()));
        }
    }
    Ok(out)
}

fn raw_region(j: &Json) -> Res<Raw> {
    let name: String = j
        .get("region_name")
        .and_then(|v| v.as_str())
        .ok_or(E::Format("region needs a region_name"))?
        .to_string();
    let text = |k: &str| -> Option<String> { j.get(k).and_then(|v| v.as_str()).map(|s| s.to_string()) };
    let mut provides: u8 = 0;
    if j.get("time_passes").and_then(|v| v.as_bool()).unwrap_or(false) {
        provides = ALL_TIME;
    }
    if let Some(p) = j.get("provides_time").and_then(|v| v.as_str()) {
        provides = match p {
            "DAY" => DAY,
            "DAMPE" => DAMPE,
            "ALL" => ALL_TIME,
            "NONE" => 0,
            _ => return Err(E::Msg(format!("region {name}: bad provides_time {p}"))),
        };
    }
    Ok(Raw {
        scene: text("scene"),
        dungeon: text("dungeon"),
        hint: text("hint"),
        provides,
        locations: pairs(j, "locations", &name)?,
        events: pairs(j, "events", &name)?,
        exits: pairs(j, "exits", &name)?,
        name,
    })
}

impl World {
    pub fn build(spec: &Spec) -> Res<World> {
        let mut items: Items = Items::new();
        let mut pool_counts: Vec<u32> = Vec::new();
        for d in &spec.items {
            items.add(&d.name, d.adv, false)?;
            pool_counts.push(d.count);
        }
        let mut groups: HM<String, Vec<ItemId>> = HM::new();
        for (g, names) in &spec.groups {
            let mut ids: Vec<ItemId> = Vec::with_capacity(names.len());
            for n in names {
                ids.push(items.find(n).ok_or_else(|| E::Msg(format!("group {g}: unknown item {n}")))?);
            }
            groups.insert(g.clone(), ids);
        }
        let mut start: Vec<(ItemId, u32)> = Vec::new();
        for (n, c) in &spec.start {
            let i: ItemId = items.find(n).ok_or_else(|| E::Msg(format!("start: unknown item {n}")))?;
            start.push((i, *c));
        }
        let mut aliases: Aliases = Aliases::new();
        for h in &spec.helpers {
            aliases.load(h)?;
        }
        let mut raws: Vec<Raw> = Vec::new();
        for list in &spec.regions {
            let a: &[Json] = list.as_arr().ok_or(E::Format("region file must be a list"))?;
            for r in a {
                raws.push(raw_region(r)?);
            }
        }
        let mut region_ix: HM<String, RegionId> = HM::new();
        let mut regions: Vec<Region> = Vec::with_capacity(raws.len());
        for r in &raws {
            if region_ix.insert(r.name.clone(), regions.len()).is_some() {
                return Err(E::Msg(format!("duplicate region {}", r.name)));
            }
            regions.push(Region {
                name: r.name.clone(),
                provides_time: r.provides,
                locations: Vec::new(),
                exits: Vec::new(),
                scene: r.scene.clone(),
                dungeon: r.dungeon.clone(),
                hint: r.hint.clone(),
            });
        }
        let root: RegionId = *region_ix
            .get(&spec.root)
            .ok_or_else(|| E::Msg(format!("missing root region {}", spec.root)))?;
        let mut locs: Vec<Loc> = Vec::new();
        let mut loc_ix: HM<String, LocId> = HM::new();
        let mut exits: Vec<Exit> = Vec::new();
        let mut defined: HS<ItemId> = HS::new();
        for (i, c) in &start {
            if *c > 0 {
                defined.insert(*i);
            }
        }
        let mut cmp: Compiler = Compiler::new(&mut items, &spec.settings, &aliases, &groups, spec.lenient);
        for (rid, r) in raws.iter().enumerate() {
            for (n, text) in &r.locations {
                let sp: Spot = Spot { region: r.name.clone(), name: n.clone() };
                let rule: Rule = cmp.rule(text, &sp)?;
                let full: String = if spec.lenient && loc_ix.contains_key(n) {
                    format!("{n} @ {}", r.name)
                } else {
                    n.clone()
                };
                if loc_ix.insert(full.clone(), locs.len()).is_some() {
                    return Err(E::Msg(format!("duplicate location {full}")));
                }
                regions[rid].locations.push(locs.len());
                locs.push(Loc { name: full, region: rid, rule, event: None });
            }
            for (ev, text) in &r.events {
                let n: String = format!("{ev} from {}", r.name);
                let sp: Spot = Spot { region: r.name.clone(), name: n.clone() };
                let rule: Rule = cmp.rule(text, &sp)?;
                let item: ItemId = cmp.items.event_named(ev);
                defined.insert(item);
                if rule.never {
                    continue;
                }
                if loc_ix.insert(n.clone(), locs.len()).is_some() {
                    return Err(E::Msg(format!("duplicate event location {n}")));
                }
                regions[rid].locations.push(locs.len());
                locs.push(Loc { name: n, region: rid, rule, event: Some(item) });
            }
            for (to, text) in &r.exits {
                let n: String = format!("{} -> {to}", r.name);
                let sp: Spot = Spot { region: r.name.clone(), name: n.clone() };
                let rule: Rule = cmp.rule(text, &sp)?;
                let tid: RegionId = *region_ix
                    .get(to)
                    .ok_or_else(|| E::Msg(format!("exit {n}: unknown region {to}")))?;
                regions[rid].exits.push(exits.len());
                exits.push(Exit { name: n, from: rid, to: tid, rule });
            }
        }
        let gsp: Spot = Spot { region: spec.root.clone(), name: "goal".to_string() };
        let goal: Rule = cmp.rule(&spec.goal, &gsp)?;
        let mut k: usize = 0;
        while k < cmp.subs.len() {
            let s: Sub = cmp.subs[k].clone();
            k += 1;
            let rid: RegionId = *region_ix
                .get(&s.region)
                .ok_or_else(|| E::Msg(format!("at(): unknown region {}", s.region)))?;
            let sp: Spot = Spot { region: s.region.clone(), name: s.name.clone() };
            let rule: Rule = cmp.rule_ast(&s.ast, &sp)?;
            defined.insert(s.item);
            if rule.never {
                continue;
            }
            if loc_ix.insert(s.name.clone(), locs.len()).is_some() {
                return Err(E::Msg(format!("duplicate location {}", s.name)));
            }
            regions[rid].locations.push(locs.len());
            locs.push(Loc { name: s.name.clone(), region: rid, rule, event: Some(s.item) });
        }
        drop(cmp);
        if !spec.lenient {
            for i in 0..items.len() {
                if items.event[i] && !defined.contains(&i) {
                    return Err(E::Msg(format!("event {} is used but never defined", items.names[i])));
                }
            }
        }
        pool_counts.resize(items.len(), 0);
        let mut users: Vec<Vec<u32>> = vec![Vec::new(); items.len()];
        let mut tod_users: Vec<u32> = Vec::new();
        let ne: usize = exits.len();
        for (e, x) in exits.iter().enumerate() {
            index_rule(&x.rule, e as u32, &mut users, &mut tod_users);
        }
        for (l, x) in locs.iter().enumerate() {
            index_rule(&x.rule, (ne + l) as u32, &mut users, &mut tod_users);
        }
        let real: Vec<LocId> = (0..locs.len()).filter(|l| locs[*l].event.is_none()).collect();
        Ok(World {
            regions,
            exits,
            locs,
            items,
            pool_counts,
            settings: spec.settings.clone(),
            goal,
            start,
            groups,
            root,
            real,
            users,
            tod_users,
            region_ix,
            loc_ix,
        })
    }

    pub fn region(&self, name: &str) -> Option<RegionId> {
        self.region_ix.get(name).copied()
    }

    pub fn loc(&self, name: &str) -> Option<LocId> {
        self.loc_ix.get(name).copied()
    }

    pub fn item(&self, name: &str) -> Option<ItemId> {
        self.items.find(name)
    }

    pub fn pool(&self) -> Vec<ItemId> {
        let mut out: Vec<ItemId> = Vec::new();
        for (i, c) in self.pool_counts.iter().enumerate() {
            for _ in 0..*c {
                out.push(i);
            }
        }
        out
    }

    pub fn pool_of(&self, names: &[(&str, u32)]) -> Res<Vec<ItemId>> {
        let mut out: Vec<ItemId> = Vec::new();
        for (n, c) in names {
            let i: ItemId = self.item(n).ok_or_else(|| E::Msg(format!("unknown item {n}")))?;
            for _ in 0..*c {
                out.push(i);
            }
        }
        Ok(out)
    }

    pub fn nspots(&self) -> usize {
        self.exits.len() + self.locs.len()
    }
}

fn index_rule(r: &Rule, spot: u32, users: &mut [Vec<u32>], tod_users: &mut Vec<u32>) {
    if r.always || r.never {
        return;
    }
    for d in &r.deps {
        users[*d].push(spot);
    }
    if r.tod {
        tod_users.push(spot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(regions: &str) -> Spec {
        let mut s: Spec = Spec::new();
        s.add_item("Key", true, 1);
        s.add_item("Rupee", false, 3);
        s.add_regions(regions).unwrap();
        s
    }

    #[test]
    fn builds_regions_exits_and_locations() {
        let s: Spec = spec(
            r#"[
            {"region_name": "Root", "exits": {"Hall": "True"}},
            {"region_name": "Hall", "locations": {"Chest": "Key"}, "events": {"Door Open": "Key"},
             "exits": {"Root": "True"}}
        ]"#,
        );
        let w: World = World::build(&s).unwrap();
        assert_eq!(w.regions.len(), 2);
        assert_eq!(w.exits.len(), 2);
        assert_eq!(w.locs.len(), 2);
        assert_eq!(w.real.len(), 1);
        assert_eq!(w.loc("Chest"), Some(0));
        assert_eq!(w.loc("Door Open from Hall"), Some(1));
        assert!(w.locs[1].event.is_some());
        assert_eq!(w.region("Hall"), Some(1));
        assert_eq!(w.pool().len(), 4);
    }

    #[test]
    fn rejects_broken_worlds() {
        assert!(World::build(&spec(r#"[{"region_name": "Hall"}]"#)).is_err());
        assert!(World::build(&spec(r#"[{"region_name": "Root", "exits": {"Nowhere": "True"}}]"#)).is_err());
        assert!(World::build(&spec(r#"[{"region_name": "Root"}, {"region_name": "Root"}]"#)).is_err());
        let dup: &str = r#"[{"region_name": "Root", "locations": {"A": "True"}},
            {"region_name": "B", "locations": {"A": "True"}}]"#;
        assert!(World::build(&spec(dup)).is_err());
        let ghost: &str = r#"[{"region_name": "Root", "locations": {"A": "Lonely_Event"}}]"#;
        assert!(World::build(&spec(ghost)).is_err());
        let bad: &str = r#"[{"region_name": "Root", "locations": {"A": "Key and"}}]"#;
        assert!(World::build(&spec(bad)).is_err());
    }

    #[test]
    fn unreachable_events_are_dropped_but_count_as_defined() {
        let s: Spec = spec(
            r#"[{"region_name": "Root", "events": {"Never Seen": "False"},
                 "locations": {"A": "Never_Seen"}}]"#,
        );
        let w: World = World::build(&s).unwrap();
        assert_eq!(w.locs.len(), 1);
        assert!(!w.locs[0].rule.never);
    }

    #[test]
    fn at_creates_subrule_locations_in_the_target_region() {
        let s: Spec = spec(
            r#"[
            {"region_name": "Root", "locations": {"A": "at('Far', Key)"}, "exits": {"Far": "True"}},
            {"region_name": "Far"}
        ]"#,
        );
        let w: World = World::build(&s).unwrap();
        let far: RegionId = w.region("Far").unwrap();
        assert_eq!(w.regions[far].locations.len(), 1);
        let l: LocId = w.regions[far].locations[0];
        assert_eq!(w.locs[l].name, "Far Subrule 1");
        assert!(w.locs[l].event.is_some());
    }

    #[test]
    fn settings_change_the_compiled_rules() {
        let text: &str = r#"[{"region_name": "Root", "locations": {"A": "open_gate or Key"}}]"#;
        let mut a: Spec = spec(text);
        a.settings.set_bool("open_gate", true);
        let mut b: Spec = spec(text);
        b.settings.set_bool("open_gate", false);
        assert!(World::build(&a).unwrap().locs[0].rule.always);
        assert!(!World::build(&b).unwrap().locs[0].rule.always);
    }

    #[test]
    fn bundle_loads_everything() {
        let t: &str = r#"{
            "goal": "Done",
            "settings": {"easy": true},
            "items": ["Junk", {"name": "Key", "advancement": true, "count": 2}],
            "groups": {"keys": ["Key"]},
            "start": {"Key": 1},
            "helpers": {"has_key": "Key"},
            "regions": [{"region_name": "Root", "events": {"Done": "has_key"}}]
        }"#;
        let s: Spec = Spec::from_bundle(t).unwrap();
        let w: World = World::build(&s).unwrap();
        assert_eq!(w.pool(), vec![0, 1, 1]);
        assert_eq!(w.start.len(), 1);
        assert_eq!(w.groups["keys"], vec![1]);
        assert!(w.settings.flag("easy"));
    }
}
