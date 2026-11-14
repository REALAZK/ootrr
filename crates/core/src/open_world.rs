pub mod ainb_edit;
pub mod guard;
pub mod skips;

use std::fs;
use std::path::PathBuf as PB;

use crate::bfevfl::Bf;
use crate::error::{Error, Res};
use crate::qol::{edit_plain, edit_zs, Have};
use crate::romfs::Romfs;
use crate::settings::Settings;
use crate::staging::Out;
use guard::Run;

pub const ZORA: &str = "zora_fountain";
pub const JABU: &str = "jabu_open";
pub const DOOR: &str = "open_door_of_time";
pub const TIME: &str = "time_always_passes";
pub const BEDS: &str = "early_beds";
pub const EPONA: &str = "skip_epona_race";
pub const MASKS: &str = "skip_mask_quest";
pub const BEANS: &str = "plant_beans";
pub const BRIDGE: &str = "gerudo_bridge_open";

pub const FOUNTAIN: &str = "Logic/ZoraFountain_Child_Prefab_41c4.logic.module.ainb";
pub const WATER_FILE: &str = "Event/EventFlow/Get_StoneOfWater.bfevfl.zs";
pub const WATER_FLOW: &str = "Get_StoneOfWater";
pub const ARRIVE_FILE: &str = "Event/EventFlow/Dm_SY_BancChangeArrive.bfevfl.zs";
pub const ARRIVE_FLOW: &str = "Dm_SY_BancChangeArrive";
pub const BED_FILE: &str = "Event/EventFlow/LinkSleepInBed.bfevfl.zs";
pub const BED_FLOW: &str = "LinkSleepInBed";

pub const WATER: &str = "Get_StoneOfWater";
pub const MOVED_EXP: &str = "Get_StoneOfWater_IsAfter_Step06_Exp";
pub const BACK_STEPS: [&str; 9] = ["Ready", "Step01", "Step02", "Step03", "Step03_Impa", "Step04", "Step05", "Step05_01", "Step06"];
pub const MEET_ZELDA: &str = "HavePlayedEvent.HyruleCastle_Goto_HyruleCastle_MeetZelda";
pub const BED_SWITCHES: [&str; 3] = ["Event92", "Event187", "Event168"];

pub const ZORA_PREFIX: &str = "OOTRROpenZora";

pub type Job = ((&'static str, &'static str), (&'static str, &'static str), Option<&'static str>);

/// The Door of Time is not here: open_door_of_time is the story-state door of
/// crate::door_open (Plan::door), which also handles the pedestal and Sheik.
pub const LOGIC_JOBS: [(&str, Job); 1] = [
    (FOUNTAIN, ((WATER, "Step08"), (WATER, "Step06"), Some("Logic_Create"))),
];

pub const ZORA_NOTE: &str = "King Zora starts moved aside, so Zora's Fountain needs no Ruto's Letter. Ruto's Letter stays in the item pool. Any event that would move the Zora quest back to before King Zora moved (the diving game, getting or reading the letter, the arrival scenes) now leaves the quest where it is. Experimental and untested in the game.";
pub const JABU_NOTE: &str = "Jabu Jabu opens his mouth without a fish once King Zora has moved. Experimental and untested in the game.";
pub const BEDS_NOTE: &str = "Beds offer Sleep until morning and Sleep until night before the Zelda meeting. Experimental and untested in the game.";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Opts {
    pub zora: bool,
    pub jabu: bool,
    pub time: bool,
    pub beds: bool,
    pub epona: bool,
    pub masks: bool,
    pub beans: bool,
    pub bridge: bool,
}

impl Opts {
    pub fn any(&self) -> bool {
        self.zora || self.jabu || self.time || self.beds || self.epona || self.masks || self.beans || self.bridge
    }

    pub fn edits_files(&self) -> bool {
        self.zora || self.jabu || self.beds || self.beans || self.bridge
    }

    pub fn logic(&self) -> Vec<(&'static str, &'static str)> {
        let mut out: Vec<(&'static str, &'static str)> = Vec::new();
        if self.zora {
            out.push((ZORA, "open"));
        }
        if self.jabu {
            out.push((JABU, "1"));
        }
        if !self.time {
            out.push((TIME, "0"));
        }
        if self.masks {
            out.push(("complete_mask_quest", "1"));
        }
        if self.beans {
            out.push((BEANS, "1"));
        }
        if self.bridge {
            out.push((BRIDGE, "1"));
        }
        out
    }

    pub fn defaults(&self) -> Vec<(&'static str, &'static str)> {
        let mut out: Vec<(&'static str, &'static str)> = Vec::new();
        if self.zora {
            out.push((ZORA, "open"));
        }
        if self.jabu {
            out.push((JABU, "true"));
        }
        if self.time {
            out.push((TIME, "true"));
        }
        if self.epona {
            out.push((EPONA, "true"));
        }
        if self.masks {
            out.push((MASKS, "true"));
        }
        if self.beans {
            out.push((BEANS, "true"));
        }
        out
    }

    pub fn notes(&self) -> Res<Vec<String>> {
        let mut out: Vec<String> = Vec::new();
        if self.zora {
            out.push(ZORA_NOTE.to_string());
        }
        if self.jabu {
            out.push(JABU_NOTE.to_string());
        }
        for (s, v) in self.defaults() {
            out.push(crate::start_defaults::presets::note(s, v)?);
        }
        if self.beds {
            out.push(BEDS_NOTE.to_string());
        }
        if self.bridge {
            out.push(skips::BRIDGE_NOTE.to_string());
        }
        Ok(out)
    }
}

pub fn read(s: &Settings, bad: &mut Vec<String>) -> Opts {
    let zora: bool = match s.text(ZORA) {
        Ok("closed") => false,
        Ok("open") => true,
        Ok(v) => {
            bad.push(format!("{ZORA} is {v}, which this build cannot apply yet (only closed and open work)"));
            false
        }
        Err(e) => {
            bad.push(e.to_string());
            false
        }
    };
    let mut flag = |id: &str| -> bool {
        s.flag(id).unwrap_or_else(|e| {
            bad.push(e.to_string());
            false
        })
    };
    let jabu: bool = flag(JABU);
    let time: bool = flag(TIME);
    let beds: bool = time && flag(BEDS);
    let epona: bool = flag(EPONA);
    let masks: bool = flag(MASKS);
    let beans: bool = flag(BEANS);
    let bridge: bool = flag(BRIDGE);
    Opts { zora, jabu, time, beds, epona, masks, beans, bridge }
}

fn flow_edit(raw: &[u8], f: impl FnOnce(&mut Bf) -> Res<()>) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    f(&mut b)?;
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(Error::Format("open world: the edited flow does not parse back"));
    }
    Ok(out)
}

fn logic_out(rf: &Romfs, have: Have, file: &str) -> Res<Out> {
    let job: Job = LOGIC_JOBS.iter().find(|(f, _)| *f == file).map(|(_, j)| *j).ok_or(Error::Format("open world: unknown logic module"))?;
    edit_plain(rf, have, file, |raw| ainb_edit::edit(raw, &[job]))
}

pub fn stage(rf: &mut Romfs, o: &Opts, have: Have) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    if o.zora {
        outs.push(edit_zs(rf, have, WATER_FILE, |raw| flow_edit(raw, |b| guard::guard_steps(b, WATER_FLOW, WATER, &BACK_STEPS, MOVED_EXP, Run::WhenFalse, ZORA_PREFIX).map(|_| ())))?);
        outs.push(edit_zs(rf, have, ARRIVE_FILE, |raw| flow_edit(raw, |b| guard::guard_steps(b, ARRIVE_FLOW, WATER, &BACK_STEPS, MOVED_EXP, Run::WhenFalse, ZORA_PREFIX).map(|_| ())))?);
    }
    if o.jabu {
        outs.push(logic_out(rf, have, FOUNTAIN)?);
    }
    if o.beds {
        outs.push(edit_zs(rf, have, BED_FILE, |raw| flow_edit(raw, |b| guard::pin(b, BED_FLOW, &BED_SWITCHES, MEET_ZELDA, 0).map(|_| ())))?);
    }
    if o.beans {
        outs.push(edit_zs(rf, have, skips::BEAN_FILE, skips::bean_bytes)?);
    }
    if o.bridge {
        outs.push(edit_plain(rf, have, skips::BRIDGE_MODULE, skips::bridge_bytes)?);
    }
    Ok(outs)
}

pub fn worked(rf: &mut Romfs, pk: &crate::modpack::Pack, o: &Opts) -> Res<Vec<Out>> {
    if !o.edits_files() {
        return Ok(Vec::new());
    }
    let root: PB = pk.work.join("romfs");
    let have = |p: &str| -> Option<Vec<u8>> { fs::read(p.split('/').fold(root.clone(), |a, s| a.join(s))).ok() };
    stage(rf, o, &have)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::{Flow, Kind};
    use crate::settings::Value;

    fn opts(pairs: &[(&str, Value)]) -> (Opts, Vec<String>) {
        let mut s: Settings = Settings::default();
        for (k, v) in pairs {
            s.set(k, v.clone()).unwrap();
        }
        let mut bad: Vec<String> = Vec::new();
        (read(&s, &mut bad), bad)
    }

    #[test]
    fn only_the_clock_is_on_by_default() {
        let (o, bad) = opts(&[]);
        assert!(bad.is_empty());
        assert_eq!(o, Opts { time: true, ..Opts::default() });
        assert!(!o.edits_files());
        assert!(o.logic().is_empty());
        assert_eq!(o.defaults(), vec![(TIME, "true")]);
        let (off, _) = opts(&[(TIME, Value::Bool(false))]);
        assert_eq!(off, Opts::default());
        assert!(!off.any() && off.defaults().is_empty() && off.notes().unwrap().is_empty());
        assert_eq!(off.logic(), vec![(TIME, "0")]);
    }

    #[test]
    fn each_setting_becomes_its_logic_value_its_new_game_defaults_and_a_note() {
        let (o, bad) = opts(&[(ZORA, Value::s("open")), (JABU, Value::Bool(true)), (DOOR, Value::Bool(true)), (BEDS, Value::Bool(true))]);
        assert!(bad.is_empty());
        assert_eq!(o, Opts { zora: true, jabu: true, time: true, beds: true, ..Opts::default() });
        assert_eq!(o.logic(), vec![(ZORA, "open"), (JABU, "1")]);
        assert_eq!(o.defaults(), vec![(ZORA, "open"), (JABU, "true"), (TIME, "true")]);
        let n: Vec<String> = o.notes().unwrap();
        assert_eq!(n.len(), 6);
        for t in ["King Zora starts moved", "Jabu Jabu opens", "Beds offer"] {
            assert!(n.iter().any(|x| x.contains(t)), "{t}");
        }
        let (b, _) = opts(&[(BEDS, Value::Bool(true)), (TIME, Value::Bool(false))]);
        assert!(!b.beds);
        let (_, bad) = opts(&[(ZORA, Value::s("adult"))]);
        assert!(bad.iter().any(|x| x.contains("zora_fountain is adult")));
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

    fn flow_of<'a>(b: &'a Bf, n: &str) -> &'a Flow {
        b.flows.iter().find(|f| f.name == n).unwrap()
    }

    fn steps(f: &Flow, challenge: &str) -> Vec<String> {
        f.events.iter().filter_map(|e| guard::step_of(f, e, challenge).map(|s| s.to_string())).collect()
    }

    fn guarded(f: &Flow, prefix: &str) -> Vec<(String, Vec<(u32, String)>)> {
        f.events
            .iter()
            .filter(|e| e.name.starts_with(prefix))
            .map(|e| {
                let Kind::Switch { cases, .. } = &e.kind else { panic!() };
                (guard::text(&e.kind, "GameDataName").unwrap().to_string(), cases.iter().map(|c| (c.val, f.events[c.ev as usize].name.clone())).collect())
            })
            .collect()
    }

    #[test]
    fn the_staged_files_parse_and_hold_exactly_the_planned_edits() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let all: Opts = Opts { zora: true, jabu: true, time: true, beds: true, ..Opts::default() };
        let outs: Vec<Out> = stage(&mut rf, &all, &|_: &str| None).unwrap();
        let paths: Vec<&str> = outs.iter().map(|o| o.path.as_str()).collect();
        assert_eq!(paths, vec![WATER_FILE, ARRIVE_FILE, FOUNTAIN, BED_FILE]);
        let mut flows: Vec<(String, Bf)> = Vec::new();
        for o in outs.iter().filter(|o| o.path.ends_with(".zs")) {
            flows.push((o.path.clone(), Bf::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap()));
        }
        let bf = |p: &str| -> Bf { flows.iter().find(|(k, _)| k == p).unwrap().1.clone() };
        let water: Bf = bf(WATER_FILE);
        let w: &Flow = flow_of(&water, WATER_FLOW);
        let g: Vec<(String, Vec<(u32, String)>)> = guarded(w, ZORA_PREFIX);
        assert_eq!(g.len(), 9);
        assert!(g.iter().all(|(exp, cs)| exp == MOVED_EXP && cs.iter().any(|(v, _)| *v == 1)));
        let orig: Bf = Bf::parse(&rf.read_zs(WATER_FILE).unwrap()).unwrap();
        assert_eq!(steps(flow_of(&orig, WATER_FLOW), WATER), steps(w, WATER));
        let arrive: Bf = bf(ARRIVE_FILE);
        assert_eq!(guarded(flow_of(&arrive, ARRIVE_FLOW), ZORA_PREFIX).len(), 2);
        let bed: Bf = bf(BED_FILE);
        let f: &Flow = flow_of(&bed, BED_FLOW);
        for n in BED_SWITCHES {
            let Kind::Switch { cases, .. } = &f.events.iter().find(|e| e.name == n).unwrap().kind else { panic!() };
            assert!(cases.iter().all(|c| c.ev == cases[0].ev), "{n}");
        }
        for o in &outs {
            if o.path.ends_with(".ainb") {
                assert_eq!(o.z.len(), o.new);
                assert_eq!(rf.read(&o.path).unwrap().len(), o.old);
            }
        }
    }

    fn plan_of(pairs: &[(&str, Value)]) -> crate::slice::support::Plan {
        let mut s: Settings = Settings::default();
        s.set(TIME, Value::Bool(false)).unwrap();
        for (k, v) in pairs {
            s.set(k, v.clone()).unwrap();
        }
        crate::slice::support::plan(&s).unwrap()
    }

    #[derive(Clone, Copy)]
    enum At<'a> {
        Region(&'a str),
        Loc(&'a str),
    }

    fn reach(pairs: &[(&str, Value)], gone: &dyn Fn(&str) -> bool, at: At) -> bool {
        let d: crate::slice::data::Data = crate::slice::data::Data::load().unwrap();
        let w: crate::world::World = crate::slice::logic::world_with(&d, &plan_of(pairs).logic()).unwrap();
        let mut p: crate::world::Placement = crate::slice::fill::vanilla_place(&w, &d).unwrap();
        let junk: crate::logic_items::ItemId = w.item("Rupees (5)").unwrap();
        for l in w.real.clone() {
            if p.get(l).is_some_and(|i| gone(&w.items.names[i])) {
                p.put(l, junk);
            }
        }
        let s: crate::search::Search = crate::search::explore(&w, &p, &[]);
        match at {
            At::Region(n) => s.region_reached(w.region(n).unwrap(), None),
            At::Loc(n) => s.loc_access(w.loc(n).unwrap()),
        }
    }

    #[test]
    fn an_open_fountain_needs_no_letter_and_the_letter_still_counts_as_a_second_bottle_only() {
        let no_letter = |n: &str| n == "Rutos Letter";
        assert!(!reach(&[], &no_letter, At::Region("ZD Behind King Zora")));
        assert!(reach(&[(ZORA, Value::s("open"))], &no_letter, At::Region("ZD Behind King Zora")));
        assert!(reach(&[], &|_| false, At::Region("ZD Behind King Zora")));
        assert!(!reach(&[(ZORA, Value::s("open"))], &|_| false, At::Region("Deliver Rutos Letter Region")));
        assert!(reach(&[], &|_| false, At::Region("Deliver Rutos Letter Region")));
    }

    #[test]
    fn an_open_jabu_needs_no_fish_but_still_needs_the_way_past_king_zora() {
        let dry = |n: &str| n.starts_with("Bottle") || n == "Rutos Letter" || n == "Fish" || n == "Buy Fish";
        let jabu: &str = "Jabu Jabus Belly Beginning";
        assert!(!reach(&[(ZORA, Value::s("open"))], &dry, At::Region(jabu)));
        assert!(reach(&[(ZORA, Value::s("open")), (JABU, Value::Bool(true))], &dry, At::Region(jabu)));
        assert!(!reach(&[(JABU, Value::Bool(true))], &dry, At::Region(jabu)));
        assert!(reach(&[], &|_| false, At::Region(jabu)));
    }

    #[test]
    fn an_open_door_needs_no_ocarina_and_the_ocarina_checks_keep_their_story_rule() {
        let quiet = |n: &str| n == "Ocarina" || n == "Song of Time";
        assert!(!reach(&[], &quiet, At::Loc("Master Sword Pedestal")));
        assert!(reach(&[(DOOR, Value::Bool(true))], &quiet, At::Loc("Master Sword Pedestal")));
        let story = |n: &str| n == "Zeldas Letter" || n == "Weird Egg";
        assert!(!reach(&[(DOOR, Value::Bool(true))], &story, At::Loc("HF Ocarina of Time Item")));
    }

    #[test]
    fn a_running_clock_gives_night_before_zelda_and_the_old_rule_stays_without_it() {
        let no_egg = |n: &str| n == "Weird Egg";
        let tour: &str = "Graveyard Dampe Gravedigging Tour";
        assert!(!reach(&[], &no_egg, At::Loc(tour)));
        assert!(reach(&[(TIME, Value::Bool(true))], &no_egg, At::Loc(tour)));
        assert!(reach(&[], &|_| false, At::Loc(tour)));
    }

    #[test]
    fn the_quest_skips_read_their_settings_and_become_logic_defaults_and_notes() {
        let (o, bad) = opts(&[(EPONA, Value::Bool(true)), (MASKS, Value::Bool(true)), (BEANS, Value::Bool(true)), (BRIDGE, Value::Bool(true)), (TIME, Value::Bool(false))]);
        assert!(bad.is_empty());
        assert_eq!(o, Opts { epona: true, masks: true, beans: true, bridge: true, ..Opts::default() });
        assert!(o.edits_files());
        assert_eq!(o.logic(), vec![(TIME, "0"), ("complete_mask_quest", "1"), (BEANS, "1"), (BRIDGE, "1")]);
        assert_eq!(o.defaults(), vec![(EPONA, "true"), (MASKS, "true"), (BEANS, "true")]);
        let n: Vec<String> = o.notes().unwrap();
        for t in ["Skip Epona's race", "Happy Mask Shop quest starts complete", "bean patch starts planted", "Gerudo Valley bridge is repaired"] {
            assert!(n.iter().any(|x| x.contains(t)), "{t}");
        }
        let (state_only, _) = opts(&[(MASKS, Value::Bool(true)), (EPONA, Value::Bool(true))]);
        assert!(!state_only.edits_files());
    }

    #[test]
    fn a_repaired_bridge_lets_adult_cross_without_epona_or_a_hookshot() {
        let walk = |n: &str| n == "Eponas Song" || n == "Progressive Hookshot";
        assert!(!reach(&[], &walk, At::Region("GV Fortress Side")));
        assert!(reach(&[(BRIDGE, Value::Bool(true))], &walk, At::Region("GV Fortress Side")));
        assert!(!reach(&[(BRIDGE, Value::Bool(true))], &walk, At::Loc("Hideout Gerudo Membership Card")));
    }

    #[test]
    fn planted_beans_need_no_bean_and_the_mask_skip_needs_no_mask_sales() {
        let no_bean = |n: &str| n == "Magic Bean" || n == "Buy Magic Bean";
        assert!(!reach(&[], &no_bean, At::Loc("Colossus Freestanding PoH")));
        assert!(reach(&[(BEANS, Value::Bool(true))], &no_bean, At::Loc("Colossus Freestanding PoH")));
        let no_mask = |n: &str| n == "Keaton Mask";
        assert!(!reach(&[], &no_mask, At::Loc("Market Mask Shop Item 1")));
        assert!(reach(&[(MASKS, Value::Bool(true))], &no_mask, At::Loc("Market Mask Shop Item 1")));
    }

    #[test]
    fn the_bean_salesman_holds_a_shuffled_pouch_item_and_no_bean_is_placed() {
        let d: crate::slice::data::Data = crate::slice::data::Data::load().unwrap();
        let mut off: Settings = Settings::default();
        off.set(TIME, Value::Bool(false)).unwrap();
        let g = crate::slice::generate::generate_data(&d, &off, "beans off", &crate::slice::GenOpts::default()).unwrap();
        assert!(!g.seed_file.placements.contains_key(skips::BEAN_SALESMAN));
        let mut s: Settings = off.clone();
        s.set(BEANS, Value::Bool(true)).unwrap();
        for k in 0..3 {
            let g = crate::slice::generate::generate_data(&d, &s, &format!("beans {k}"), &crate::slice::GenOpts::default()).unwrap();
            let it: &str = &g.seed_file.placements[skips::BEAN_SALESMAN];
            assert!(crate::flow_extra::pouch(d.item(it).unwrap()), "{it}");
            assert!(g.seed_file.placements.values().all(|v| !skips::BEAN_ITEMS.contains(&v.as_str())), "seed {k}");
        }
    }

    #[test]
    fn every_combination_generates_beatable_seeds_and_the_plan_says_what_it_does() {
        let d: crate::slice::data::Data = crate::slice::data::Data::load().unwrap();
        let combos: [&[(&str, Value)]; 6] = [
            &[(EPONA, Value::Bool(true)), (MASKS, Value::Bool(true)), (BEANS, Value::Bool(true)), (BRIDGE, Value::Bool(true)), ("open_kakariko", Value::s("open"))],
            &[(DOOR, Value::Bool(true))],
            &[(ZORA, Value::s("open")), (JABU, Value::Bool(true))],
            &[(JABU, Value::Bool(true)), (TIME, Value::Bool(true)), (BEDS, Value::Bool(true))],
            &[(ZORA, Value::s("open")), (JABU, Value::Bool(true)), (DOOR, Value::Bool(true)), (TIME, Value::Bool(true)), ("open_forest", Value::s("open")), ("open_kakariko", Value::s("open"))],
            &[(DOOR, Value::Bool(true)), (TIME, Value::Bool(true)), ("shuffle_skulltulas", Value::s("matched")), ("shuffle_dungeon_rewards", Value::Bool(true))],
        ];
        for (n, c) in combos.iter().enumerate() {
            let mut s: Settings = Settings::default();
            s.set(TIME, Value::Bool(false)).unwrap();
            for (k, v) in c.iter() {
                s.set(k, v.clone()).unwrap();
            }
            let p: crate::slice::support::Plan = crate::slice::support::plan(&s).unwrap();
            let w: crate::world::World = crate::slice::logic::world_with(&d, &p.logic()).unwrap();
            for k in 0..2 {
                let g = crate::slice::generate::generate_data(&d, &s, &format!("open {n} {k}"), &crate::slice::GenOpts::default()).unwrap();
                let back: Settings = g.seed_file.settings().unwrap().settings;
                assert_eq!(crate::slice::support::plan(&back).unwrap().open, p.open, "{n}");
                let mut pl: crate::world::Placement = crate::slice::fill::vanilla_place(&w, &d).unwrap();
                for (l, i) in &g.seed_file.placements {
                    pl.put(w.loc(l).unwrap(), w.item(i).unwrap());
                }
                assert!(crate::search::explore(&w, &pl, &[]).won(), "combo {n} seed {k}");
                for t in p.open.notes().unwrap() {
                    assert!(g.report.contains(&t), "{n}");
                }
            }
        }
    }

    #[test]
    fn a_built_seed_carries_every_edit_and_each_grown_file_grows_its_own_size_table_entry() {
        let Some(root) = real() else {
            return;
        };
        let d: crate::slice::data::Data = crate::slice::data::Data::load().unwrap();
        let mut s: Settings = Settings::default();
        for (k, v) in [(ZORA, Value::s("open")), (JABU, Value::Bool(true)), (DOOR, Value::Bool(true)), (BEDS, Value::Bool(true)), ("open_forest", Value::s("open")), ("open_kakariko", Value::s("open")), ("shuffle_pots", Value::s("all"))] {
            s.set(k, v).unwrap();
        }
        let g = crate::slice::generate::generate_data(&d, &s, "open built", &crate::slice::GenOpts::default()).unwrap();
        let load: PB = PB::from(env!("CARGO_MANIFEST_DIR")).join("../../target/open_world_mods");
        let b: crate::slice::edit::Built = crate::slice::edit::build(&g.seed_file, &root, &load, "open built").unwrap();
        let out: PB = b.root.join("romfs");
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let van: crate::rsizetable::Rst = crate::rsizetable::load(&mut rf).unwrap();
        let modt: crate::rsizetable::Rst = crate::rsizetable::Rst::parse(&rf.zs.decompress(&fs::read(out.join(crate::rsizetable::TABLE_PATH)).unwrap()).unwrap()).unwrap();
        let files: [&str; 9] = [crate::door_open::MODULE, crate::door_open::SWORD_MODULE, FOUNTAIN, crate::door_fix::FLOW_FILE, WATER_FILE, ARRIVE_FILE, BED_FILE, crate::chest_slot::LIST, crate::intro_skip::FILE];
        let mut grown: usize = 0;
        for f in files {
            let disk: Vec<u8> = fs::read(f.split('/').fold(out.clone(), |a, x| a.join(x))).unwrap_or_else(|_| panic!("{f} missing"));
            let (old, new): (usize, usize) = if f.ends_with(".zs") { (rf.read_zs(f).unwrap().len(), rf.zs.decompress(&disk).unwrap().len()) } else { (rf.read(f).unwrap().len(), disk.len()) };
            let (a, z): (u32, u32) = (van.get(f).unwrap_or_else(|| panic!("{f} has no size entry")), modt.get(f).unwrap());
            if new > old {
                grown += 1;
                assert!(z as u64 >= a as u64 + crate::rsizetable::align(new as u64) - crate::rsizetable::align(old as u64), "{f} {a} {z} {old} {new}");
            } else {
                assert!(z >= a, "{f}");
            }
        }
        assert!(grown >= 4, "{grown}");
        let list: crate::byml::Byml = crate::byml::Byml::parse(&rf.zs.decompress(&fs::read(out.join(crate::chest_slot::LIST)).unwrap()).unwrap()).unwrap();
        let rows: &crate::byml::Node = list.root.get("Data").unwrap();
        let bool_of = |n: &str| rows.get("Bool").unwrap().as_arr().unwrap().iter().find(|r| r.get("Hash") == Some(&crate::byml::Node::U32(crate::start_defaults::key(n)))).and_then(|r| r.get("DefaultValue")).cloned();
        for n in ["GaveFishToJabJab", "JabJabOpenedMouth", "HyruleField_ArriveEntrance", "HavePlayedEvent.Dm_HrlField_0020", "Npc_Hylia_Soldier_003_GateOpen"] {
            assert_eq!(bool_of(n), Some(crate::byml::Node::Bool(true)), "{n}");
        }
        let step: Option<crate::byml::Node> = rows.get("Enum").unwrap().as_arr().unwrap().iter().find(|r| r.get("Hash") == Some(&crate::byml::Node::U32(crate::start_defaults::key("Step_Get_StoneOfWater")))).and_then(|r| r.get("DefaultValue")).cloned();
        assert_eq!(step, Some(crate::byml::Node::U32(crate::start_defaults::key("Step07"))));
        assert!(rows.get(crate::pots::TABLE).is_some());
    }

    #[test]
    fn a_built_seed_with_the_quest_skips_carries_their_files_flags_and_the_salesman_item() {
        let Some(root) = real() else {
            return;
        };
        let d: crate::slice::data::Data = crate::slice::data::Data::load().unwrap();
        let mut s: Settings = Settings::default();
        for k in [EPONA, MASKS, BEANS, BRIDGE] {
            s.set(k, Value::Bool(true)).unwrap();
        }
        let g = crate::slice::generate::generate_data(&d, &s, "skips built", &crate::slice::GenOpts::default()).unwrap();
        let load: PB = PB::from(env!("CARGO_MANIFEST_DIR")).join("../../target/quest_skip_mods");
        let b: crate::slice::edit::Built = crate::slice::edit::build(&g.seed_file, &root, &load, "skips built").unwrap();
        let out: PB = b.root.join("romfs");
        let at = |f: &str| f.split('/').fold(out.clone(), |a, x| a.join(x));
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        assert!(!at(skips::EPONA_FILE).is_file());
        let beans: Bf = Bf::parse(&rf.zs.decompress(&fs::read(at(skips::BEAN_FILE)).unwrap()).unwrap()).unwrap();
        assert!(skips::bean_bytes(&beans.write().unwrap()).is_err());
        let bridge: crate::ainb::Ainb = crate::ainb::Ainb::parse(&fs::read(at(skips::BRIDGE_MODULE)).unwrap()).unwrap();
        assert!(skips::bridge_gate(&bridge).is_err());
        let shop: Bf = Bf::parse(&rf.zs.decompress(&fs::read(at("Event/EventFlow/SimpleShop.bfevfl.zs")).unwrap()).unwrap()).unwrap();
        let give: &crate::bfevfl::Event = &shop.flows[0].events[63];
        assert_eq!(give.name, "Event182");
        let placed: &str = &g.seed_file.placements[skips::BEAN_SALESMAN];
        let got: Option<&crate::bfevfl::Val> = give.kind.params().and_then(|c| c.get("ReveiveItem"));
        assert!(!matches!(got, Some(crate::bfevfl::Val::Str(x)) if x == "Item_MagicBean"), "{placed} {got:?}");
        let list: crate::byml::Byml = crate::byml::Byml::parse(&rf.zs.decompress(&fs::read(at(crate::chest_slot::LIST)).unwrap()).unwrap()).unwrap();
        let rows: &crate::byml::Node = list.root.get("Data").unwrap();
        let def = |t: &str, n: &str| rows.get(t).unwrap().as_arr().unwrap().iter().find(|r| r.get("Hash") == Some(&crate::byml::Node::U32(crate::start_defaults::key(n)))).and_then(|r| r.get("DefaultValue")).cloned();
        assert_eq!(def("Enum", "Step_SubEvent_MaskSalesman"), Some(crate::byml::Node::U32(crate::start_defaults::key("Complete"))));
        assert_eq!(def("Bool", "IsPlantedMagicBean_SmallFarm.14429198438059673734"), Some(crate::byml::Node::Bool(true)));
        assert_eq!(def("Int", "Get_MagicBeans"), Some(crate::byml::Node::I32(9)));
        let exp: Option<crate::byml::Node> = rows.get("BoolExp").unwrap().as_arr().unwrap().iter().find(|r| r.get("Hash") == Some(&crate::byml::Node::U32(crate::start_defaults::key("Get_Epona_IsCompleted_Exp")))).and_then(|r| r.get("Values")).cloned();
        let terms: Vec<crate::byml::Node> = exp.unwrap().as_arr().unwrap().to_vec();
        assert_eq!(terms.len(), 1);
        let n = |x: &crate::byml::Node| -> i64 {
            match x {
                crate::byml::Node::I32(v) => *v as i64,
                crate::byml::Node::U32(v) => *v as i64,
                crate::byml::Node::I64(v) => *v,
                crate::byml::Node::U64(v) => *v as i64,
                _ => -1,
            }
        };
        for t in &terms {
            let t: &[crate::byml::Node] = t.as_arr().unwrap();
            assert_eq!((n(&t[0]), n(&t[1])), (0, crate::start_defaults::key("IsGet.Item_Melody_EponaSong") as i64));
        }
    }

}
