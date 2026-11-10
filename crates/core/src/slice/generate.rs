use std::collections::BTreeMap as BM;
use std::fmt::Write as W;

use crate::error::{Error as E, Res};
use crate::logic_items::ItemId;
use crate::rng::Rng;
use crate::settings::Settings;
use crate::settings::seed::SeedFile;
use crate::slice::data::Data;
use crate::slice::fill::{changed, fill, vanilla_place};
use crate::slice::logic::world;
use crate::slice::pick::{pick, scoped, Pick};
use crate::slice::support::{plan, Plan};
use crate::spoiler::{playthrough, Spoiler};
use crate::world::{LocId, Placement, World};

pub const TRIES: u32 = 600;

#[derive(Debug, Clone)]
pub struct GenOpts {
    pub spoiler: bool,
    pub tries: u32,
}

impl Default for GenOpts {
    fn default() -> GenOpts {
        GenOpts { spoiler: false, tries: TRIES }
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

pub fn allowed(d: &Data, w: &World, p: &Plan, it: ItemId, l: LocId) -> bool {
    match d.item(&w.items.names[it]) {
        Some(i) if scoped(p, &i.kind) => {
            i.dungeon.is_some() && i.dungeon.as_deref() == w.regions[w.locs[l].region].dungeon.as_deref()
        }
        Some(i) => {
            i.actor.is_some() && (d.flow.by_name(&w.locs[l].name).is_none() || d.flow_accepts(i))
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
        "Logic: OoTR glitchless rules for the vanilla world, dungeon keys assumed available inside their own dungeon, every other location holds its vanilla item."
    );
    out
}

pub fn generate_data(d: &Data, s: &Settings, seed: &str, o: &GenOpts) -> Res<Generated> {
    if d.scan.rows.is_empty() {
        return Err(E::Format("the chest list in data/slice.json is empty, run slice-scan first"));
    }
    let p: Plan = plan(s)?;
    let w: World = world(d)?;
    let vanilla: Placement = vanilla_place(&w, d)?;
    let pk: Pick = pick(d, &w, &p);
    let ok = |it: ItemId, l: LocId| allowed(d, &w, &p, it, l);
    for l in &pk.free {
        if let Some(i) = vanilla.get(*l)
            && !ok(i, *l)
        {
            return Err(E::Msg(format!(
                "the vanilla item {} of {} is not allowed there under these settings",
                w.items.names[i], w.locs[*l].name
            )));
        }
    }
    let mut rng: Rng = Rng::from_text(seed);
    let got: Placement = if pk.free.is_empty() {
        vanilla.clone()
    } else {
        fill(&w, &vanilla, &pk.free, &ok, &mut rng, o.tries)?
    };
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
        let (t, st): (String, Steps) = spoiler_text(&w, &vanilla, &got, &pk)?;
        (Some(t), st)
    } else {
        (None, Vec::new())
    };
    let rep: String = report(seed, &p, &pk, &counts, &moved);
    Ok(Generated { seed_file, counts, report: rep, spoiler: sp, spheres })
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
        let o: GenOpts = GenOpts { spoiler: true, tries: TRIES };
        let b: Generated = generate_data(&d, &all_chests(), "s", &o).unwrap();
        let t: String = b.spoiler.unwrap();
        assert!(t.contains("Randomized chests") && t.contains("Sphere 1"));
    }

    #[test]
    fn an_empty_scan_is_refused() {
        let scan: &str = "{\"format\":1,\"game\":\"g\",\"files\":[],\"chests\":[]}";
        let d: Data = Data::new(ITEMS_JSON, VANILLA_JSON, scan, CATALOG_JSON).unwrap();
        assert!(generate_data(&d, &all_chests(), "x", &GenOpts::default()).is_err());
    }
}
