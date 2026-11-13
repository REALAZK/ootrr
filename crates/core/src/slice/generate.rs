use std::collections::{BTreeMap as BM, HashMap as HM};
use std::fmt::Write as W;
use std::sync::Arc;
use std::time::Duration;

use crate::error::{Error as E, Res};
use crate::logic_items::ItemId;
use crate::progressive::family_of;
use crate::rng::Rng;
use crate::settings::Settings;
use crate::settings::seed::SeedFile;
use crate::slice::data::{Data, State};
use crate::slice::fill::{changed, fill_ordered, vanilla_place, SWITCH};
use crate::slice::logic::world_with;
use crate::slice::pick::{pick, scoped, Pick};
use crate::slice::pickups as pu;
use crate::slice::plando::{loc_of, resolve, Entry};
use crate::slice::support::{plan, Keep, Mode, Plan, Songs};
use crate::slice::watch::{Live, Watch};
use crate::spoiler::{playthrough, Spoiler};
use crate::world::{LocId, Placement, World};

pub const TRIES: u32 = 600;
pub const TIER_TRIES: u32 = 40;
pub const LIMIT: Duration = Duration::from_secs(90);

#[derive(Debug, Clone)]
pub struct GenOpts {
    pub spoiler: bool,
    pub tries: u32,
    pub plando: Vec<Entry>,
    pub limit: Option<Duration>,
    pub live: Option<Arc<Live>>,
    pub switch: u32,
}

impl Default for GenOpts {
    fn default() -> GenOpts {
        GenOpts { spoiler: false, tries: TRIES, plando: Vec::new(), limit: None, live: None, switch: SWITCH }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub candidates: usize,
    pub randomized: usize,
    pub left_vanilla: usize,
    pub changed: usize,
    pub fixed_elsewhere: usize,
}

#[derive(Debug, Clone)]
pub struct Generated {
    pub seed_file: SeedFile,
    pub counts: Counts,
    pub report: String,
    pub spoiler: Option<String>,
    pub spheres: Vec<Vec<(String, String)>>,
}

pub fn is_song(actor: &str) -> bool {
    actor.starts_with("Item_Melody_") && actor != "Item_Melody_ScarecrowSong"
}

fn by_place(d: &Data, w: &World, p: &Plan, it: ItemId, l: LocId) -> Option<bool> {
    let name: &str = &w.locs[l].name;
    let item: &str = &w.items.names[it];
    if let Some(ok) = crate::slice::treasure::at(p.treasure, name, item) {
        return Some(ok);
    }
    if p.remake && crate::slice::remake_overworld::keeps_out(d, name, item) {
        return Some(false);
    }
    if p.world != Mode::Off {
        if let Some(ok) = pu::at(d, name, item) {
            return Some(ok);
        }
        if pu::keeps_out(d, name, item) {
            return Some(false);
        }
    }
    if let Some(ok) = crate::pot_slice::rules::at(d, name, item) {
        return Some(ok);
    }
    if p.skulls != crate::slice::skulls::Mode::Off {
        if let Some(ok) = crate::slice::skulls::accepts(d, w, it, l) {
            return Some(ok);
        }
    }
    if pu::WORLD_ONLY.contains(&item) || d.pots.only.contains(item) {
        return Some(false);
    }
    None
}

pub fn allowed(d: &Data, w: &World, p: &Plan, it: ItemId, l: LocId) -> bool {
    if d.item(&w.items.names[it]).is_some_and(|i| !crate::flow_extra::trade::keeps(d, &w.locs[l].name, i)) {
        return false;
    }
    if let Some(ok) = by_place(d, w, p, it, l) {
        return ok;
    }
    match d.item(&w.items.names[it]) {
        Some(i) if scoped(p, &i.kind) => {
            i.dungeon.is_some()
                && i.dungeon.as_deref() == w.regions[w.locs[l].region].dungeon.as_deref()
                && (d.flow.by_name(&w.locs[l].name).is_none() || d.flow_accepts(i))
                && !crate::slice::dungeon_items::blocked(d, p, &i.kind, &w.locs[l].name)
        }
        Some(i) if crate::slice::dungeon_items::handles(p, &i.kind) => crate::slice::dungeon_items::allowed(d, w, p, i, l),
        Some(i) if p.keys_roam() && i.kind == "SmallKey" => {
            let name: &str = &w.locs[l].name;
            let dungeon: bool = w.regions[w.locs[l].region].dungeon.is_some();
            let place: bool = match p.small {
                Keep::AnyDungeon => dungeon,
                _ => true,
            };
            place
                && i.dungeon.is_some()
                && d.flow.by_name(name).is_none()
                && !d.special(name)
                && d.state_of(name) == Some(State::Eligible)
                && d.cat.by_name(name).is_some_and(|c| c.kind == "chest")
        }
        Some(i) => {
            let name: &str = &w.locs[l].name;
            let up: bool = family_of(&i.tiers).is_some();
            let song_item: bool = i.actor.as_deref().is_some_and(is_song);
            let song_loc: bool = d.song_ok(name);
            let heart_loc: bool = d.cat.by_name(name).is_some_and(|c| c.kind == "bossheart");
            if p.songs == Songs::Own && song_item != song_loc {
                return false;
            }
            i.actor.is_some()
                && crate::slice::flow_rules::allows(d, name, i)
                && (!p.extra || crate::flow_extra::fits(d, name, i))
                && (!p.scrubs || crate::scrubs_a::fits(d, name, i))
                && (!p.scrubs_b || crate::scrubs_b::fits(d, name, i))
                && !(up && ((d.keeps_event(name) && !d.special(name)) || heart_loc))
        }
        None => false,
    }
}

fn item_name(w: &World, v: &Placement, l: LocId) -> String {
    match v.get(l) {
        Some(i) => w.items.names[i].clone(),
        None => "(nothing)".to_string(),
    }
}

type Steps = Vec<Vec<(String, String)>>;

fn spoiler_text(w: &World, vanilla: &Placement, got: &Placement, pk: &Pick) -> Res<(String, Steps)> {
    let sp: Spoiler = playthrough(w, got)?;
    let steps: Steps = sp
        .spheres
        .iter()
        .map(|sphere| sphere.iter().map(|(l, i)| (w.locs[*l].name.clone(), w.items.names[*i].clone())).collect())
        .collect();
    let mut out: String = String::new();
    let _ = writeln!(out, "Randomized chests");
    for l in &pk.free {
        let _ = writeln!(
            out,
            "  {}: {} (vanilla {})",
            w.locs[*l].name,
            item_name(w, got, *l),
            item_name(w, vanilla, *l)
        );
    }
    let _ = writeln!(out, "Playthrough, {} required locations in {} spheres", sp.required.len(), sp.spheres.len());
    for (n, sphere) in sp.spheres.iter().enumerate() {
        let _ = writeln!(out, "Sphere {}", n + 1);
        for (l, i) in sphere {
            let _ = writeln!(out, "  {}: {}", w.locs[*l].name, w.items.names[*i]);
        }
    }
    Ok((out, steps))
}

fn report(s: &str, p: &Plan, pk: &Pick, c: &Counts, moved: &[String]) -> String {
    let mut out: String = String::new();
    let _ = writeln!(out, "Chest shuffle slice, seed {s:?}");
    let _ = writeln!(
        out,
        "Randomized {} chests and changed {} of them. {} of the {} candidate chests stay vanilla. {} other locations hold their vanilla items.",
        c.randomized, c.changed, c.left_vanilla, c.candidates, c.fixed_elsewhere
    );
    if !pk.left.is_empty() {
        let _ = writeln!(out, "Candidate chests left vanilla:");
        for (k, v) in &pk.left {
            let _ = writeln!(out, "  {v} because {k}");
        }
    }
    if !moved.is_empty() {
        let _ = writeln!(out, "Changed chests:");
        for m in moved {
            let _ = writeln!(out, "  {m}");
        }
    }
    for n in &p.notes {
        let _ = writeln!(out, "{n}");
    }
    let _ = writeln!(
        out,
        "Logic: OoTR glitchless rules for the vanilla world, small keys counted for real, with the Fire, Water and Spirit keys kept in their vanilla chests, every other location holds its vanilla item."
    );
    out
}

pub fn generate_data(d: &Data, s: &Settings, seed: &str, o: &GenOpts) -> Res<Generated> {
    if d.scan.rows.is_empty() {
        return Err(E::Format("the chest list in data/slice.json is empty, run slice-scan first"));
    }
    let watch: Watch = Watch::new(o.limit, o.live.clone());
    let mut p: Plan = plan(s)?;
    let mut w: World = world_with(d, &p.logic())?;
    let mut named: Vec<String> = Vec::with_capacity(p.excluded.len());
    for e in &p.excluded {
        let l: LocId = loc_of(&w, e).map_err(|x| E::Msg(x.to_string().replacen("plando:", "disabled locations:", 1)))?;
        named.push(w.locs[l].name.clone());
    }
    p.excluded = named;
    crate::slice::starts::add(d, &p, &mut w, seed)?;
    let vanilla: Placement = vanilla_place(&w, d)?;
    let pk: Pick = pick(d, &w, &p);
    if p.world != Mode::Off && !pk.free.iter().any(|l| pu::listed(d, &w.locs[*l].name)) {
        p.notes.retain(|n| n != pu::NOTE);
    }
    let ok = |it: ItemId, l: LocId| allowed(d, &w, &p, it, l);
    for l in &pk.free {
        if let Some(i) = vanilla.get(*l)
            && !ok(i, *l)
            && !d.item(&w.items.names[i]).is_some_and(|x| crate::slice::dungeon_items::exempt(&p, &x.kind))
        {
            return Err(E::Msg(format!(
                "the vanilla item {} of {} is not allowed there under these settings",
                w.items.names[i], w.locs[*l].name
            )));
        }
    }
    let plan: Vec<(LocId, ItemId)> = resolve(&w, &o.plando)?;
    for (l, i) in &plan {
        if !pk.free.contains(l) {
            return Err(E::Msg(format!(
                "plando: {} is not shuffled with these settings, so it cannot hold {}",
                w.locs[*l].name, w.items.names[*i]
            )));
        }
        if !ok(*i, *l) {
            return Err(E::Msg(format!("plando: {} cannot be placed at {} under these settings", w.items.names[*i], w.locs[*l].name)));
        }
    }
    let mut rng: Rng = Rng::from_text(seed);
    let got: Placement = if pk.free.is_empty() {
        vanilla.clone()
    } else {
        let fix: bool = crate::slice::tier_fix::active(d, &w, &pk.free);
        let narrow = |it: ItemId, l: LocId| ok(it, l) && !crate::slice::key_spots::widened(d, &w.locs[l].name, &w.items.names[it]);
        let order: Option<&dyn Fn(ItemId, LocId) -> bool> = if fix { Some(&narrow) } else { None };
        let caps: HM<LocId, usize> = crate::pot_slice::rules::caps(d, &w, &pk.free, &vanilla, order.unwrap_or(&ok));
        let rank = |l: LocId| caps.get(&l).copied().unwrap_or(0);
        let r: Option<&dyn Fn(LocId) -> usize> = if caps.is_empty() { None } else { Some(&rank) };
        let keep: Vec<LocId> = plan.iter().map(|x| x.0).collect();
        let mut n: u32 = 0;
        loop {
            let g: Placement = fill_ordered(&w, &vanilla, &pk.free, &ok, order, &mut rng, o.tries, &plan, !p.keys_roam(), r, o.switch, &watch)
                .map_err(|e| crate::slice::advice::explain(&p, e))?;
            if crate::slice::tier_order::fits(d, &w, &g, &pk.free)? {
                break g;
            }
            if fix {
                let mut fr: Rng = Rng::from_text(&format!("{seed} tier repair {n}"));
                let c: crate::slice::tier_fix::Ctx = crate::slice::tier_fix::Ctx { d, w: &w, free: &pk.free, keep: &keep, ok: &ok };
                if let Some(x) = crate::slice::tier_fix::repair(&c, &g, &mut fr)? {
                    break x;
                }
            }
            n += 1;
            if n >= TIER_TRIES {
                let hint: &str = if o.plando.is_empty() {
                    "these settings leave too little room, try another seed text or fewer shuffled pickup spots"
                } else {
                    "check the plando entries for upgrade items, or try another seed text"
                };
                let last: Vec<String> = crate::slice::tier_fix::verdict(d, &w, &g, &pk.free)?.bad.into_iter().take(2).collect();
                return Err(E::Msg(format!("no fill in {TIER_TRIES} tries kept every fixed upgrade tier in order, {hint}. The last fill failed because {}", last.join(", and "))));
            }
        }
    };
    if p.skulls != crate::slice::skulls::Mode::Off {
        crate::slice::skulls::check_tokens(&w, &vanilla, &got)?;
    }
    let mut map: BM<String, String> = BM::new();
    let mut moved: Vec<String> = Vec::new();
    for l in &pk.free {
        let new: String = item_name(&w, &got, *l);
        let old: String = item_name(&w, &vanilla, *l);
        if new != old {
            moved.push(format!("{}: {} instead of {}", w.locs[*l].name, new, old));
        }
        map.insert(w.locs[*l].name.clone(), new);
    }
    let counts: Counts = Counts {
        candidates: d.scan.rows.len() + pk.flow,
        randomized: pk.free.len(),
        left_vanilla: (d.scan.rows.len() + pk.flow).saturating_sub(pk.free.len()),
        changed: changed(&vanilla, &got, &pk.free),
        fixed_elsewhere: w.real.len() - pk.free.len(),
    };
    let seed_file: SeedFile = SeedFile::new(seed, s, &d.scan.game, map)?;
    let (sp, spheres): (Option<String>, Steps) = if o.spoiler {
        let (mut t, st): (String, Steps) = spoiler_text(&w, &vanilla, &got, &pk)?;
        let fixed: BM<String, String> = if crate::slice::tier_fix::active(d, &w, &pk.free) {
            crate::slice::tier_fix::verdict(d, &w, &got, &pk.free)?.assign
        } else {
            BM::new()
        };
        t.push_str(&crate::slice::skulls::spoiler_with(d, &w, &p, &pk.free, &got, &fixed));
        t.push_str(&crate::slice::key_spots::spoiler(d, &w, &pk.free, &got, &fixed));
        (Some(t), st)
    } else {
        (None, Vec::new())
    };
    let mut rep: String = report(seed, &p, &pk, &counts, &moved);
    if !plan.is_empty() {
        let _ = writeln!(rep, "Plandomizer: {} placements were fixed by hand.", plan.len());
    }
    Ok(Generated { seed_file, counts, report: rep, spoiler: sp, spheres })
}

pub fn plando_options(d: &Data, s: &Settings) -> Res<(Vec<String>, Vec<String>)> {
    let p: Plan = plan(s)?;
    let w: World = world_with(d, &p.logic())?;
    let vanilla: Placement = vanilla_place(&w, d)?;
    let pk: Pick = pick(d, &w, &p);
    let mut locs: Vec<String> = pk.free.iter().map(|l| w.locs[*l].name.clone()).collect();
    let mut items: Vec<String> = pk.free.iter().filter_map(|l| vanilla.get(*l)).map(|i| w.items.names[i].clone()).collect();
    locs.sort();
    items.sort();
    items.dedup();
    Ok((locs, items))
}

pub fn generate_with(s: &Settings, seed: &str, o: &GenOpts) -> Res<Generated> {
    generate_data(&Data::load()?, s, seed, o)
}

pub fn generate(s: &Settings, seed: &str) -> Res<Generated> {
    generate_with(s, seed, &GenOpts::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Value;
    use crate::slice::data::{CATALOG_JSON, ITEMS_JSON, VANILLA_JSON};
    use crate::slice::logic::world;

    const BIG: [&str; 6] = [
        "Forest Temple Bow Chest",
        "Dodongos Cavern Bomb Bag Chest",
        "Deku Tree Slingshot Chest",
        "Jabu Jabus Belly Boomerang Chest",
        "Ice Cavern Iron Boots Chest",
        "Bottom of the Well Lens of Truth Chest",
    ];

    fn synthetic() -> Data {
        let mut rows: Vec<String> = Vec::new();
        for n in BIG {
            rows.push(format!("{{\"name\":\"{n}\",\"state\":\"eligible\"}}"));
        }
        let scan: String = format!(
            "{{\"format\":1,\"game\":\"oot-remake:synthetic\",\"files\":[],\"chests\":[{}]}}",
            rows.join(",")
        );
        Data::new(ITEMS_JSON, VANILLA_JSON, &scan, CATALOG_JSON).unwrap()
    }

    fn all_chests() -> Settings {
        let mut s: Settings = Settings::default();
        s.set("shuffle_chests", Value::s("all")).unwrap();
        s.set(crate::slice::remake_chests::SETTING, Value::s("on")).unwrap();
        s
    }

    #[test]
    fn the_shipped_data_generates_a_valid_seed_for_many_seeds() {
        let d: Data = Data::load().unwrap();
        for n in 0..4 {
            let g: Generated = generate_data(&d, &Settings::default(), &format!("seed {n}"), &GenOpts::default()).unwrap();
            assert_eq!(g.counts.randomized, g.seed_file.placements.len());
            assert!(g.counts.changed <= g.counts.randomized);
            let back: SeedFile = SeedFile::parse(&g.seed_file.to_text()).unwrap();
            assert_eq!(back, g.seed_file);
        }
    }

    #[test]
    fn verified_event_chests_are_part_of_the_pool() {
        let d: Data = Data::load().unwrap();
        let g: Generated = generate_data(&d, &Settings::default(), "events", &GenOpts::default()).unwrap();
        for name in ["Deku Tree Slingshot Chest", "Jabu Jabus Belly Compass Chest", "Jabu Jabus Belly Boomerang Chest"] {
            assert!(g.seed_file.placements.contains_key(name), "{name}");
        }
        assert!(!g.seed_file.placements.contains_key("Ice Cavern Iron Boots Chest"));
        assert!(!g.seed_file.placements.keys().any(|k| k.starts_with("Market Treasure Chest Game")));
    }

    #[test]
    fn generation_is_deterministic() {
        let d: Data = synthetic();
        let a: Generated = generate_data(&d, &all_chests(), "same", &GenOpts::default()).unwrap();
        let b: Generated = generate_data(&d, &all_chests(), "same", &GenOpts::default()).unwrap();
        assert_eq!(a.seed_file.to_text(), b.seed_file.to_text());
        assert_eq!(a.report, b.report);
        let mut seen: Vec<BM<String, String>> = Vec::new();
        for n in 0..6 {
            let c: Generated = generate_data(&d, &all_chests(), &format!("other {n}"), &GenOpts::default()).unwrap();
            if !seen.contains(&c.seed_file.placements) {
                seen.push(c.seed_file.placements);
            }
        }
        assert!(seen.len() > 1);
    }

    #[test]
    fn progression_is_shuffled_and_always_beatable() {
        let d: Data = synthetic();
        let w: World = world(&d).unwrap();
        let mut moved: usize = 0;
        for n in 0..12 {
            let g: Generated = generate_data(&d, &all_chests(), &format!("p{n}"), &GenOpts::default()).unwrap();
            let mut place: Placement = vanilla_place(&w, &d).unwrap();
            let mut names: Vec<&String> = Vec::new();
            for (loc, item) in &g.seed_file.placements {
                let l: LocId = w.loc(loc).unwrap();
                place.put(l, w.item(item).unwrap());
                names.push(item);
            }
            assert!(crate::search::explore(&w, &place, &[]).won(), "seed p{n} is not beatable");
            let mut sorted: Vec<&String> = names.clone();
            sorted.sort();
            let mut want: Vec<String> = BIG.iter().map(|b| d.vanilla_of(b).unwrap().to_string()).collect();
            want.sort();
            assert_eq!(sorted.into_iter().cloned().collect::<Vec<String>>(), want);
            moved += g.counts.changed;
        }
        assert!(moved > 12 * 2);
    }

    #[test]
    fn dungeon_items_never_leave_their_dungeon() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let p: Plan = plan(&Settings::default()).unwrap();
        for n in 0..4 {
            let g: Generated = generate_data(&d, &Settings::default(), &format!("d{n}"), &GenOpts::default()).unwrap();
            for (loc, item) in &g.seed_file.placements {
                let l: LocId = w.loc(loc).unwrap();
                assert!(allowed(&d, &w, &p, w.item(item).unwrap(), l), "{item} at {loc}");
            }
        }
    }

    fn rebuilt(d: &Data, p: &Plan, placements: &BM<String, String>) -> bool {
        let w: World = world_with(d, &p.logic()).unwrap();
        let mut pl: Placement = vanilla_place(&w, d).unwrap();
        for (loc, item) in placements {
            pl.put(w.loc(loc).unwrap(), w.item(item).unwrap());
        }
        crate::search::explore(&w, &pl, &[]).won()
    }

    #[test]
    fn roaming_small_keys_move_between_dungeons_and_every_seed_is_beatable() {
        let d: Data = Data::load().unwrap();
        let mut left: usize = 0;
        for mode in ["any_dungeon", "keysanity"] {
            let s: Settings = with(&[("shuffle_chests", "all"), ("shuffle_smallkeys", mode)]);
            let p: Plan = plan(&s).unwrap();
            let w: World = world_with(&d, &p.logic()).unwrap();
            for n in 0..6 {
                let g: Generated = generate_data(&d, &s, &format!("roam {mode} {n}"), &GenOpts::default()).unwrap();
                assert!(rebuilt(&d, &p, &g.seed_file.placements), "{mode} {n} is not beatable");
                for (loc, item) in &g.seed_file.placements {
                    let Some(i) = d.item(item).filter(|i| i.kind == "SmallKey") else {
                        continue;
                    };
                    let here: Option<&str> = w.regions[w.locs[w.loc(loc).unwrap()].region].dungeon.as_deref();
                    if mode == "any_dungeon" {
                        assert!(here.is_some(), "{item} at {loc} is outside a dungeon");
                    }
                    if here != i.dungeon.as_deref() {
                        left += 1;
                    }
                }
            }
        }
        assert!(left > 10, "keys never left their own dungeon, {left}");
    }

    #[test]
    fn keys_stay_home_when_not_asked_to_roam() {
        let d: Data = Data::load().unwrap();
        let s: Settings = with(&[("shuffle_chests", "all")]);
        let p: Plan = plan(&s).unwrap();
        let w: World = world_with(&d, &p.logic()).unwrap();
        for n in 0..3 {
            let g: Generated = generate_data(&d, &s, &format!("home {n}"), &GenOpts::default()).unwrap();
            for (loc, item) in &g.seed_file.placements {
                if let Some(i) = d.item(item).filter(|i| i.kind == "SmallKey") {
                    let here: Option<&str> = w.regions[w.locs[w.loc(loc).unwrap()].region].dungeon.as_deref();
                    assert_eq!(here, i.dungeon.as_deref(), "{item} at {loc}");
                }
            }
        }
    }

    #[test]
    fn chest_modes_and_vanilla_keys_change_the_pool() {
        let d: Data = Data::load().unwrap();
        let mut s: Settings = Settings::default();
        s.set("shuffle_chests", Value::s("off")).unwrap();
        let off: Generated = generate_data(&d, &s, "x", &GenOpts::default()).unwrap();
        assert_eq!(off.counts.randomized, 0);
        assert!(off.seed_file.placements.is_empty());
        s.set("shuffle_chests", Value::s("overworld")).unwrap();
        let o: Generated = generate_data(&d, &s, "x", &GenOpts::default()).unwrap();
        assert!(o.counts.randomized >= 2);
        s.set("shuffle_chests", Value::s("dungeons")).unwrap();
        s.set("shuffle_smallkeys", Value::s("vanilla")).unwrap();
        s.set("shuffle_mapcompass", Value::s("vanilla")).unwrap();
        let k: Generated = generate_data(&d, &s, "x", &GenOpts::default()).unwrap();
        assert!(k.counts.randomized >= 1);
        assert!(!k.seed_file.placements.contains_key("Ice Cavern Map Chest"));
        assert!(!k.seed_file.placements.contains_key("Gerudo Training Ground Hidden Ceiling Chest"));
    }

    #[test]
    fn excluded_locations_stay_out_of_the_seed() {
        let d: Data = Data::load().unwrap();
        let mut s: Settings = Settings::default();
        s.set("disabled_locations", Value::set(&["ZD Chest"])).unwrap();
        let g: Generated = generate_data(&d, &s, "x", &GenOpts::default()).unwrap();
        assert!(!g.seed_file.placements.contains_key("ZD Chest"));
        assert!(g.report.contains("excluded by the player"));
    }

    #[test]
    fn unsupported_settings_stop_generation_with_a_message() {
        let d: Data = Data::load().unwrap();
        let mut s: Settings = Settings::default();
        s.set("shuffle_scrubs", Value::s("low")).unwrap();
        let e: String = generate_data(&d, &s, "x", &GenOpts::default()).err().unwrap().to_string();
        assert!(e.contains("shuffle_scrubs is low"));
    }

    #[test]
    fn spoiler_is_optional() {
        let d: Data = synthetic();
        let a: Generated = generate_data(&d, &all_chests(), "s", &GenOpts::default()).unwrap();
        assert!(a.spoiler.is_none());
        let o: GenOpts = GenOpts { spoiler: true, ..GenOpts::default() };
        let b: Generated = generate_data(&d, &all_chests(), "s", &o).unwrap();
        let t: String = b.spoiler.unwrap();
        assert!(t.contains("Randomized chests") && t.contains("Sphere 1"));
    }

    fn with(pairs: &[(&str, &str)]) -> Settings {
        let mut s: Settings = Settings::default();
        for (k, v) in pairs {
            s.set(k, Value::s(v)).unwrap();
        }
        s
    }

    #[test]
    fn boss_hearts_join_the_pool_only_when_asked() {
        let d: Data = Data::load().unwrap();
        let off: Generated = generate_data(&d, &Settings::default(), "hearts", &GenOpts::default()).unwrap();
        assert!(!off.seed_file.placements.contains_key("Deku Tree Queen Gohma Heart"));
        let mut s: Settings = Settings::default();
        s.set("shuffle_boss_hearts", Value::Bool(true)).unwrap();
        for n in 0..2 {
            let g: Generated = generate_data(&d, &s, &format!("hearts {n}"), &GenOpts::default()).unwrap();
            let hearts: Vec<&String> = g.seed_file.placements.keys().filter(|k| k.ends_with(" Heart")).collect();
            assert_eq!(hearts.len(), 8, "{hearts:?}");
            let held: usize = g.seed_file.placements.values().filter(|v| v.as_str() == "Heart Container").count();
            assert_eq!(held, 8);
        }
    }

    #[test]
    fn songs_stay_among_the_song_locations_in_own_mode() {
        let d: Data = Data::load().unwrap();
        let s: Settings = with(&[("shuffle_songs", "own")]);
        for n in 0..2 {
            let g: Generated = generate_data(&d, &s, &format!("songs {n}"), &GenOpts::default()).unwrap();
            for (loc, item) in &g.seed_file.placements {
                let song_loc: bool = d.song_ok(loc);
                let song_item: bool = d.item(item).and_then(|i| i.actor.as_deref()).is_some_and(is_song);
                assert_eq!(song_loc, song_item, "{loc} holds {item}");
            }
            let n_song: usize = g.seed_file.placements.keys().filter(|k| d.flow.by_name(k).is_some_and(|f| f.cat == "song")).count();
            assert_eq!(n_song, 10);
        }
    }

    #[test]
    fn songs_can_move_anywhere_in_anywhere_mode_and_every_seed_is_beatable() {
        let d: Data = Data::load().unwrap();
        let mut s: Settings = with(&[("shuffle_songs", "anywhere"), ("shuffle_rewards", "supported")]);
        s.set("shuffle_boss_hearts", Value::Bool(true)).unwrap();
        let mut out_of_place: usize = 0;
        for n in 0..2 {
            let g: Generated = generate_data(&d, &s, &format!("everything {n}"), &GenOpts::default()).unwrap();
            for (loc, item) in &g.seed_file.placements {
                let song_loc: bool = d.flow.by_name(loc).is_some_and(|f| f.cat == "song");
                let song_item: bool = d.item(item).and_then(|i| i.actor.as_deref()).is_some_and(is_song);
                if song_loc != song_item {
                    out_of_place += 1;
                }
            }
        }
        assert!(out_of_place > 0);
    }

    #[test]
    fn dungeon_items_never_land_on_event_locations_that_cannot_hand_them_out() {
        let d: Data = Data::load().unwrap();
        let s: Settings = with(&[("shuffle_songs", "own"), ("shuffle_rewards", "supported")]);
        for n in 0..3 {
            let g: Generated = generate_data(&d, &s, &format!("dungeon items {n}"), &GenOpts::default()).unwrap();
            for (loc, item) in &g.seed_file.placements {
                if d.flow.by_name(loc).is_some() {
                    assert!(d.item(item).is_some_and(|i| d.flow_accepts(i)) || d.vanilla_of(loc) == Some(item.as_str()), "{loc} holds {item}");
                }
            }
        }
    }

    #[test]
    fn a_start_reward_changes_the_logic_and_still_generates() {
        let d: Data = Data::load().unwrap();
        let s: Settings = with(&[("start_reward", "any")]);
        let g: Generated = generate_data(&d, &s, "start reward", &GenOpts::default()).unwrap();
        assert!(g.report.contains("start the game with one random"));
        assert_eq!(g.counts.randomized, g.seed_file.placements.len());
    }

    #[test]
    fn upgrade_items_never_land_on_a_boss_heart() {
        let d: Data = Data::load().unwrap();
        let mut s: Settings = Settings::default();
        s.set("shuffle_boss_hearts", Value::Bool(true)).unwrap();
        for n in 0..2 {
            let g: Generated = generate_data(&d, &s, &format!("up {n}"), &GenOpts::default()).unwrap();
            for (loc, item) in &g.seed_file.placements {
                if loc.ends_with(" Heart") {
                    assert!(d.family(item).is_none(), "{loc} holds {item}");
                }
            }
        }
    }

    fn plan_of(list: &[&str]) -> GenOpts {
        GenOpts { plando: list.iter().map(|x| crate::slice::plando::parse(x).unwrap()).collect(), ..GenOpts::default() }
    }

    #[test]
    fn a_plando_entry_is_honoured_and_every_seed_stays_beatable() {
        let d: Data = Data::load().unwrap();
        for n in 0..4 {
            let o: GenOpts = plan_of(&["KF Midos Top Left Chest=Boomerang", "KF Midos Top Right Chest=Bomb Bag"]);
            let r: Res<Generated> = generate_data(&d, &all_chests(), &format!("plando {n}"), &o);
            let g: Generated = match r {
                Ok(g) => g,
                Err(e) => panic!("{e}"),
            };
            let p = &g.seed_file.placements;
            assert_eq!(p.get("KF Midos Top Left Chest").map(|s| s.as_str()), Some("Boomerang"));
            assert_eq!(p.get("KF Midos Top Right Chest").map(|s| s.as_str()), Some("Bomb Bag"));
            assert!(g.report.contains("Plandomizer: 2"));
        }
    }

    #[test]
    fn bad_plando_entries_are_refused_with_a_reason() {
        let d: Data = Data::load().unwrap();
        let go = |s: &Settings, l: &[&str]| generate_data(&d, s, "p", &plan_of(l)).err().map(|e| e.to_string()).unwrap_or_default();
        assert!(go(&all_chests(), &["KF Midos Top Left Chest=Not An Item"]).contains("no item is named"));
        let m: String = go(&all_chests(), &["Midos Top Left=Boomerang"]);
        assert!(m.contains("no location is named") && m.contains("KF Midos Top Left Chest"), "{m}");
        assert!(go(&all_chests(), &["KF Midos Top Left Chest=Boomerang", "KF Midos Top Right Chest=Boomerang"]).contains("no copy left"));
        assert!(go(&all_chests(), &["KF Midos Top Left Chest=Boomerang", "KF Midos Top Left Chest=Deku Shield"]).contains("more than once"));
        assert!(go(&with(&[("shuffle_chests", "off")]), &["KF Midos Top Left Chest=Boomerang"]).contains("not shuffled"));
    }

    #[test]
    fn an_open_forest_generates_and_can_put_progression_past_mido() {
        let d: Data = Data::load().unwrap();
        for v in ["open", "closed_deku"] {
            let s: Settings = with(&[("open_forest", v)]);
            for n in 0..3 {
                let g: Generated = generate_data(&d, &s, &format!("forest {v} {n}"), &GenOpts::default()).unwrap();
                assert!(g.report.contains("Kokiri Forest starts open"));
            }
        }
    }

    #[test]
    fn disabled_locations_match_without_case_and_typos_are_refused() {
        let d: Data = Data::load().unwrap();
        let mut s: Settings = all_chests();
        s.set("disabled_locations", Value::set(&["kf midos top left chest"])).unwrap();
        let g: Generated = generate_data(&d, &s, "dis", &GenOpts::default()).unwrap();
        assert!(!g.seed_file.placements.contains_key("KF Midos Top Left Chest"));
        s.set("disabled_locations", Value::set(&["Midos Top Left"])).unwrap();
        let e: String = generate_data(&d, &s, "dis", &GenOpts::default()).err().unwrap().to_string();
        assert!(e.contains("disabled locations: no location is named") && e.contains("KF Midos Top Left Chest"), "{e}");
    }

    fn digest(p: &BM<String, String>) -> u64 {
        let mut h: u64 = 0xCBF2_9CE4_8422_2325;
        for (k, v) in p {
            for b in format!("{k}\t{v}\n").as_bytes() {
                h ^= *b as u64;
                h = h.wrapping_mul(0x0000_0100_0000_01B3);
            }
        }
        h
    }

    #[test]
    fn seeds_that_generated_before_keep_their_exact_placements() {
        let d: Data = Data::load().unwrap();
        let cases: [(&[(&str, &str)], &str, usize, u64); 4] = [
            (&[], "default 0", 77, 0x9b41_25fb_b7b3_954b),
            (&[], "default 1", 77, 0x74e9_a4ba_5cb4_40b9),
            (&[("shuffle_chests", "overworld")], "chests_ow 0", 10, 0x842f_cb4b_1197_ea2a),
            (&[("shuffle_chests", "dungeons")], "dungeons 0", 67, 0x169a_7c8a_d12b_3980),
        ];
        for (pairs, seed, n, want) in cases {
            let mut all: Vec<(&str, &str)> = vec![("shuffle_rewards", "off")];
            all.extend_from_slice(pairs);
            let g: Generated = generate_data(&d, &with(&all), seed, &GenOpts::default()).unwrap();
            assert_eq!(g.seed_file.placements.len(), n, "{seed}");
            assert_eq!(digest(&g.seed_file.placements), want, "{seed}");
        }
    }

    #[test]
    fn a_setting_that_cannot_fit_fails_at_once_with_a_plain_message() {
        let d: Data = Data::load().unwrap();
        let s: Settings = with(&[("shuffle_chests", "dungeons"), ("shuffle_bosskeys", "overworld")]);
        let t0: std::time::Instant = std::time::Instant::now();
        let e: String = generate_data(&d, &s, "x", &GenOpts::default()).err().unwrap().to_string();
        assert!(e.starts_with("No seed can be made with these settings, the shuffled items cannot all fit"), "{e}");
        assert!(e.contains("Boss Key") && e.contains("can only go to 0 free locations"), "{e}");
        assert!(t0.elapsed() < std::time::Duration::from_secs(20));
    }

    fn xcykrix() -> Settings {
        let mut s: Settings = with(&[
            ("open_forest", "open"),
            ("shuffle_rewards", "supported"),
            ("shuffle_songs", "anywhere"),
            ("shuffle_skulltulas", "matched"),
            (crate::slice::remake_chests::SETTING, "on"),
        ]);
        for k in ["shuffle_boss_hearts", "gossip_hints", "faster_chests"] {
            s.set(k, Value::Bool(true)).unwrap();
        }
        s
    }

    #[test]
    fn the_careful_fill_is_deterministic_and_its_seeds_are_valid() {
        let d: Data = Data::load().unwrap();
        let s: Settings = xcykrix();
        let p: Plan = plan(&s).unwrap();
        let w: World = world_with(&d, &p.logic()).unwrap();
        let o: GenOpts = GenOpts { switch: 0, spoiler: true, ..GenOpts::default() };
        let a: Generated = generate_data(&d, &s, "careful", &o).unwrap();
        let b: Generated = generate_data(&d, &s, "careful", &o).unwrap();
        assert_eq!(a.seed_file.to_text(), b.seed_file.to_text());
        assert_eq!(a.spoiler, b.spoiler);
        assert!(rebuilt(&d, &p, &a.seed_file.placements));
        for (loc, item) in &a.seed_file.placements {
            assert!(allowed(&d, &w, &p, w.item(item).unwrap(), w.loc(loc).unwrap()), "{item} at {loc}");
        }
        assert!(a.spoiler.unwrap().contains("Sphere 1"));
        let c: Generated = generate_data(&d, &s, "careful other", &o).unwrap();
        assert_ne!(a.seed_file.placements, c.seed_file.placements);
    }

    #[test]
    fn the_time_limit_and_a_cancel_end_the_run_with_a_message() {
        let d: Data = Data::load().unwrap();
        let o: GenOpts = GenOpts { limit: Some(Duration::ZERO), ..GenOpts::default() };
        let e: String = generate_data(&d, &xcykrix(), "slow", &o).err().unwrap().to_string();
        assert!(e.contains("time limit of 0 seconds") && e.contains("set Gold Skulltulas to Confident matches"), "{e}");
        let l: Arc<Live> = Arc::new(Live::default());
        l.cancel();
        let c: GenOpts = GenOpts { live: Some(Arc::clone(&l)), ..GenOpts::default() };
        let e: String = generate_data(&d, &Settings::default(), "stop", &c).err().unwrap().to_string();
        assert_eq!(e, crate::slice::watch::STOPPED);
        let r: Generated = generate_data(&d, &Settings::default(), "free", &GenOpts { limit: Some(LIMIT), ..GenOpts::default() }).unwrap();
        let n: Generated = generate_data(&d, &Settings::default(), "free", &GenOpts::default()).unwrap();
        assert_eq!(r.seed_file.to_text(), n.seed_file.to_text());
    }

    #[test]
    fn an_empty_scan_is_refused() {
        let scan: &str = "{\"format\":1,\"game\":\"g\",\"files\":[],\"chests\":[]}";
        let d: Data = Data::new(ITEMS_JSON, VANILLA_JSON, scan, CATALOG_JSON).unwrap();
        assert!(generate_data(&d, &all_chests(), "x", &GenOpts::default()).is_err());
    }
}
