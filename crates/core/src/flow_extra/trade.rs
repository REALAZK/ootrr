use crate::error::{Error as E, Res};
use crate::flow_extra::data::{Extra, Spec};
use crate::flow_extra::op::How;
use crate::flow_locations::{Change, Loc};
use crate::slice::data::{Data, Item};

pub const SETTING: &str = "shuffle_trade_checks";
pub const ROLE: &str = "clone";

pub const LOCS: [&str; 10] = [
    "Kak Anju as Adult",
    "Kak Anju Trade Pocket Cucco",
    "LW Trade Cojiro",
    "Kak Granny Trade Odd Mushroom",
    "LW Trade Odd Potion",
    "GV Trade Poachers Saw",
    "DMT Trade Broken Sword",
    "ZD Trade Prescription",
    "LH Trade Eyeball Frog",
    "DMT Trade Eyedrops",
];

pub const ACTORS: [&str; 14] = [
    "Item_ClaimCheck",
    "Item_EyeDrop",
    "Item_EyeballFrog",
    "Item_Prescription",
    "Item_BrokenSword",
    "Item_PoacherSaw",
    "Item_OddPowder",
    "Item_OddMushroom",
    "Item_CojiroCucco",
    "Item_PocketCucco",
    "Item_PocketEgg",
    "Item_ZeldaLetter",
    "Item_AlarmCucco",
    "Item_WeirdEgg",
];

pub fn is(name: &str) -> bool {
    LOCS.contains(&name)
}

pub fn chain(actor: &str) -> bool {
    ACTORS.contains(&actor)
}

pub fn keeps(d: &Data, name: &str, i: &Item) -> bool {
    !i.actor.as_deref().is_some_and(chain) || d.vanilla_of(name) == Some(i.name.as_str())
}

pub fn changes(l: &Loc, item: &str) -> Res<Vec<Change>> {
    if chain(item) {
        return Err(E::Msg(format!(
            "{} cannot hand out {item}, a second trade quest item would hide the chain item in the shared menu slot",
            l.name
        )));
    }
    let x: Extra = Extra::load()?;
    let s: &Spec = x.by_name(&l.name).ok_or_else(|| E::Msg(format!("{} has no clone edit in data/flow_extra.json", l.name)))?;
    let mut out: Vec<Change> = Vec::new();
    for o in &s.ops {
        if let How::Clone { param, old, .. } = &o.how {
            out.push(Change {
                location: l.name.clone(),
                file: o.file.clone(),
                index: o.index,
                event: o.event.clone(),
                param: param.clone(),
                role: ROLE.to_string(),
                old: old.clone(),
                new: item.to_string(),
            });
        }
    }
    if out.is_empty() {
        return Err(E::Msg(format!("{} has no clone edit in data/flow_extra.json", l.name)));
    }
    Ok(out)
}

pub fn plain(c: &Change) -> bool {
    c.role != ROLE
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap as BM, BTreeSet as BS, HashMap as HM};
    use std::path::PathBuf as PB;

    use super::*;
    use crate::bfevfl::{Bf, Flow};
    use crate::flow_extra::op::Op;
    use crate::flow_extra::sim::{edited, granted, run, run_from, sim, stem, Scene};
    use crate::flow_locations::{path_of, plan, Choice, Data as FlowData, Opts};
    use crate::flow_sim::run::{End, Fin, Sim};
    use crate::flow_sim::state::{State, V};
    use crate::romfs::Romfs;
    use crate::settings::{Settings, Value};
    use crate::slice::generate::{generate_data, GenOpts, Generated};
    use crate::slice::support::plan as plan_of;

    const DUMP: &str = "C:/o/nup dump/updates/131072/romfs";
    const NEW: &str = "Item_Longshot";
    const FILLER: &str = "Obj_Rupee_Blue";
    const STEP: &str = "Step_SubEvent_GetSwordBigGoron";

    struct Hand {
        loc: &'static str,
        flow: &'static str,
        entry: &'static str,
        step: &'static str,
        shown: Option<&'static str>,
        gives: &'static str,
    }

    const HANDS: [Hand; 10] = [
        Hand { loc: "Kak Anju as Adult", flow: "Npc_Hylia_049_Adult", entry: "SubEvent_GetSwordBigGoron_Start", step: "Ready", shown: None, gives: "Item_PocketEgg" },
        Hand { loc: "Kak Anju Trade Pocket Cucco", flow: "Npc_Hylia_049_Adult", entry: "SubEvent_GetSwordBigGoron_UntilStep03_ShowItemTalk", step: "Step02", shown: Some("Item_PocketCucco"), gives: "Item_CojiroCucco" },
        Hand { loc: "LW Trade Cojiro", flow: "Npc_Hylia_017_Adult", entry: "SubEvent_GetSwordBigGoron_Step03", step: "Step03", shown: Some("Item_CojiroCucco"), gives: "Item_OddMushroom" },
        Hand { loc: "Kak Granny Trade Odd Mushroom", flow: "Npc_Hylia_053_Adult", entry: "SubEvent_GetSwordBigGoron_Step04", step: "Step04", shown: Some("Item_OddMushroom"), gives: "Item_OddPowder" },
        Hand { loc: "LW Trade Odd Potion", flow: "Npc_Kokiri_Fado_Adult", entry: "SubEvent_GetSwordBigGoron_Step05", step: "Step05", shown: Some("Item_OddPowder"), gives: "Item_PoacherSaw" },
        Hand { loc: "GV Trade Poachers Saw", flow: "Npc_Hylia_012_Adult", entry: "SubEvent_GetSwordBigGoron_Step06", step: "Step06", shown: Some("Item_PoacherSaw"), gives: "Item_BrokenSword" },
        Hand { loc: "DMT Trade Broken Sword", flow: "Npc_Goron_Biggoron_Adult", entry: "SubEvent_GetSwordBigGoron_Step07", step: "Step07", shown: Some("Item_BrokenSword"), gives: "Item_Prescription" },
        Hand { loc: "ZD Trade Prescription", flow: "Npc_Zora_KingZora_adult", entry: "SubEvent_GetSwordBigGoron_Step08", step: "Step08", shown: Some("Item_Prescription"), gives: "Item_EyeballFrog" },
        Hand { loc: "LH Trade Eyeball Frog", flow: "Npc_Hylia_045_Adult", entry: "SubEvent_GetSwordBigGoron_Step09", step: "Step09", shown: Some("Item_EyeballFrog"), gives: "Item_EyeDrop" },
        Hand { loc: "DMT Trade Eyedrops", flow: "Dm_DeathMtTrail_0050", entry: "C08", step: "Step10", shown: Some("Item_EyeDrop"), gives: "Item_ClaimCheck" },
    ];

    const RETRIES: [(&str, &str, &str, &str); 5] = [
        ("Npc_Hylia_017_Adult", "SubEvent_GetSwordBigGoron_Step04Retry", "Step03_03", "Item_OddMushroom"),
        ("Npc_Zora_KingZora_adult", "SubEvent_GetSwordBigGoron_Step09Retry", "Step08_01", "Item_EyeballFrog"),
        ("Npc_Zora_KingZora_adult", "EyeballFrogFailed", "Step08_01", "Item_EyeballFrog"),
        ("Npc_Hylia_045_Adult", "SubEvent_GetSwordBigGoron_Step09Retry", "Step09_01", "Item_EyeDrop"),
        ("Npc_Hylia_045_Adult", "CarryMedicine", "Step09_01", "Item_EyeDrop"),
    ];

    fn root() -> Option<PB> {
        let p: PB = std::env::var_os("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|| PB::from(DUMP));
        if p.join("Event/EventFlow").is_dir() {
            Some(p)
        } else {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            None
        }
    }

    fn scene(h: &Hand) -> Scene {
        Scene {
            flow: h.flow.to_string(),
            entry: h.entry.to_string(),
            set: vec![(STEP.to_string(), h.step.to_string())],
            own: h.shown.map(|s| vec![(s.to_string(), 1)]).unwrap_or_default(),
        }
    }

    fn all(rf: &mut Romfs, item: &str) -> HM<String, Flow> {
        let d: FlowData = FlowData::load().unwrap();
        let cs: Vec<Choice> = LOCS.iter().map(|l| Choice { location: l.to_string(), item: item.to_string() }).collect();
        let (ch, _, _): (Vec<Change>, usize, Vec<String>) = plan(&d, &cs, &Opts::default()).unwrap();
        assert_eq!(ch.len(), 10);
        let os: BM<String, Vec<Op>> = crate::flow_extra::ops_for(&ch).unwrap();
        assert_eq!(os.values().map(|v| v.len()).sum::<usize>(), 10);
        let mut out: HM<String, Flow> = HM::new();
        for (file, o) in &os {
            let raw: Vec<u8> = rf.read_zs(&path_of(file)).unwrap();
            let e: Bf = crate::flow_extra::edit_flow(&raw, &[], o).unwrap();
            let mut back: Bf = Bf::parse(&e.write().unwrap()).unwrap();
            assert_eq!(back, e, "{file}");
            out.insert(stem(file), back.flows.remove(0));
        }
        out
    }

    fn shape(f: &Fin, drop: &str) -> String {
        let gr: Vec<(&String, &i64)> = f.m.gr.iter().filter(|(k, _)| k.as_str() != drop).collect();
        format!("{:?} | {gr:?} | {:?} | {:?}", f.end, f.m.reqs, f.m.writes)
    }

    fn shapes(fins: &[Fin], drop: &str) -> BS<String> {
        fins.iter().map(|f| shape(f, drop)).collect()
    }

    fn set(s: &mut Sim, st: &mut State, k: &str, v: &str) {
        let h: u32 = s.db.reg(k);
        let x: V = s.db.parse(h, v).unwrap();
        st.v.insert(h, x);
        s.settle(st);
    }

    #[test]
    fn plan_clones_the_first_grant_and_never_touches_the_vanilla_parameter() {
        let d: FlowData = FlowData::load().unwrap();
        for h in &HANDS {
            let (ch, same, _): (Vec<Change>, usize, Vec<String>) = plan(&d, &[Choice { location: h.loc.to_string(), item: NEW.to_string() }], &Opts::default()).unwrap();
            assert_eq!(same, 0, "{}", h.loc);
            assert_eq!(ch.len(), 1, "{}", h.loc);
            assert!(ch.iter().all(|c| c.role == ROLE && c.old == h.gives && c.new == NEW), "{}", h.loc);
            let (back, same, _): (Vec<Change>, usize, Vec<String>) = plan(&d, &[Choice { location: h.loc.to_string(), item: h.gives.to_string() }], &Opts::default()).unwrap();
            assert!(back.is_empty() && same == 1, "{}", h.loc);
            for a in ACTORS.iter().filter(|a| **a != h.gives) {
                let o: Opts = Opts { untested: true, songs: false };
                assert!(plan(&d, &[Choice { location: h.loc.to_string(), item: a.to_string() }], &o).is_err(), "{} {a}", h.loc);
            }
        }
        assert!(plan(&d, &[Choice { location: "HC Malon Egg".to_string(), item: "Item_Prescription".to_string() }], &Opts::default()).is_err());
        let o: Opts = Opts { untested: true, songs: false };
        assert!(plan(&d, &[Choice { location: "HC Malon Egg".to_string(), item: "Item_Prescription".to_string() }], &o).is_ok());
        let (ch, _, _): (Vec<Change>, usize, Vec<String>) = plan(&d, &[Choice { location: "HC Malon Egg".to_string(), item: "Item_WeirdEgg".to_string() }], &Opts::default()).unwrap();
        assert!(ch.is_empty());
    }

    #[test]
    fn every_trade_location_is_gated_with_one_clone_and_a_filler() {
        let fl: FlowData = FlowData::load().unwrap();
        let d: Data = Data::load().unwrap();
        let x: Extra = Extra::load().unwrap();
        for h in &HANDS {
            let l: &Loc = fl.by_name(h.loc).unwrap();
            assert!(l.gated && !l.supported, "{}", h.loc);
            assert_eq!(l.item.as_deref(), Some(h.gives), "{}", h.loc);
            let s: &Spec = x.by_name(h.loc).unwrap();
            assert_eq!(s.ops.len(), 1, "{}", h.loc);
            assert!(matches!(&s.ops[0].how, How::Clone { old, new, .. } if old == h.gives && new == crate::flow_extra::ITEM), "{}", h.loc);
            assert_eq!(d.vanilla_of(h.loc), Some("Rupees (5)"), "{}", h.loc);
            assert!(d.flow_ok(h.loc), "{}", h.loc);
            assert!(!crate::flow_extra::bound(&d, "Rupees (5)"));
        }
        for a in ACTORS {
            assert!(d.items.iter().any(|i| i.actor.as_deref() == Some(a)), "{a}");
        }
    }

    #[test]
    fn trade_items_stay_at_their_own_vanilla_spot_only() {
        let d: Data = Data::load().unwrap();
        let egg: &Item = d.item("Weird Egg").unwrap();
        let pres: &Item = d.item("Prescription").unwrap();
        assert!(keeps(&d, "HC Malon Egg", egg));
        assert!(!keeps(&d, "HC Malon Egg", pres));
        assert!(!keeps(&d, "HC Zeldas Letter", egg));
        assert!(!keeps(&d, "Kak Anju as Adult", d.item("Pocket Egg").unwrap()));
        assert!(!keeps(&d, "Deku Tree Map Chest", egg));
        assert!(keeps(&d, "Deku Tree Map Chest", d.item("Progressive Hookshot").unwrap()));
        assert!(keeps(&d, "LW Trade Cojiro", d.item("Rupees (5)").unwrap()));
    }

    fn with(trade: bool) -> Settings {
        let mut s: Settings = Settings::default();
        s.set("shuffle_rewards", Value::s("supported")).unwrap();
        s.set(SETTING, Value::Bool(trade)).unwrap();
        s
    }

    #[test]
    fn the_toggle_is_off_by_default_needs_supported_rewards_and_keeps_the_string() {
        assert!(!plan_of(&Settings::default()).unwrap().trade);
        assert!(!plan_of(&with(false)).unwrap().trade);
        assert!(plan_of(&with(true)).unwrap().trade);
        let mut s: Settings = Settings::default();
        s.set("shuffle_rewards", Value::s("off")).unwrap();
        s.set(SETTING, Value::Bool(true)).unwrap();
        assert!(!plan_of(&s).unwrap().trade);
        let mut d: Settings = Settings::default();
        d.set(SETTING, Value::Bool(false)).unwrap();
        assert_eq!(Settings::default().encode().unwrap(), d.encode().unwrap());
    }

    #[test]
    fn toggle_off_places_nothing_at_the_trade_locations_and_the_plando_refuses_them() {
        let d: Data = Data::load().unwrap();
        let g: Generated = generate_data(&d, &with(false), "trade off", &GenOpts::default()).unwrap();
        for l in LOCS {
            assert!(!g.seed_file.placements.contains_key(l), "{l}");
        }
        let o: GenOpts = GenOpts { plando: vec![crate::slice::plando::parse("HC Malon Egg=Prescription").unwrap()], ..GenOpts::default() };
        assert!(generate_data(&d, &with(false), "trade plando", &o).is_err());
        let o: GenOpts = GenOpts { plando: vec![crate::slice::plando::parse("LW Trade Cojiro=Hover Boots").unwrap()], ..GenOpts::default() };
        assert!(generate_data(&d, &with(false), "trade plando", &o).is_err());
    }

    #[test]
    fn toggle_on_adds_the_ten_checks_and_no_trade_item_leaves_its_spot() {
        let d: Data = Data::load().unwrap();
        let off: Generated = generate_data(&d, &with(false), "trade", &GenOpts::default()).unwrap();
        for n in 0..3 {
            let g: Generated = generate_data(&d, &with(true), &format!("trade {n}"), &GenOpts::default()).unwrap();
            for l in LOCS {
                assert!(g.seed_file.placements.contains_key(l), "{l}");
            }
            assert_eq!(g.seed_file.placements.len(), off.seed_file.placements.len() + LOCS.len());
            for (loc, it) in &g.seed_file.placements {
                let i: &Item = d.item(it).unwrap();
                assert!(keeps(&d, loc, i), "{loc} holds {it}");
                if is(loc) {
                    assert!(d.flow_accepts(i) || it == "Rupees (5)", "{loc} holds {it}");
                }
            }
        }
        let o: GenOpts = GenOpts { plando: vec![crate::slice::plando::parse("LW Trade Cojiro=Hover Boots").unwrap()], ..GenOpts::default() };
        let g: Generated = generate_data(&d, &with(true), "trade plando", &o).unwrap();
        assert_eq!(g.seed_file.placements.get("LW Trade Cojiro").map(|s| s.as_str()), Some("Hover Boots"));
        let o: GenOpts = GenOpts { plando: vec![crate::slice::plando::parse("LW Trade Cojiro=Odd Mushroom").unwrap()], ..GenOpts::default() };
        assert!(generate_data(&d, &with(true), "trade plando", &o).is_err());
    }

    #[test]
    fn the_chain_actor_placed_back_leaves_the_flows_untouched() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        for h in &HANDS {
            assert!(edited(&mut rf, h.loc, h.gives).unwrap().is_empty(), "{}", h.loc);
        }
    }

    #[test]
    fn each_hand_in_gives_the_shuffled_item_once_and_the_chain_runs_as_vanilla() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        for item in [NEW, FILLER] {
            let fl: HM<String, Flow> = all(&mut rf, item);
            let mut a: Sim = sim(&r, HM::new()).unwrap();
            let mut b: Sim = sim(&r, fl).unwrap();
            for h in &HANDS {
                let sc: Scene = scene(h);
                let va: Vec<Fin> = run(&mut a, &sc).unwrap();
                let vb: Vec<Fin> = run(&mut b, &sc).unwrap();
                assert!(va.iter().any(|f| granted(f, h.gives) == 1), "{} vanilla", h.loc);
                assert!(va.iter().all(|f| granted(f, item) == 0), "{}", h.loc);
                assert_eq!(shapes(&va, item), shapes(&vb, item), "{} {item}", h.loc);
                for f in &vb {
                    assert_eq!(granted(f, item), granted(f, h.gives), "{} {item} {:?}", h.loc, f.m.gr);
                    assert!(granted(f, item) <= 1, "{}", h.loc);
                }
                if let Some(s) = h.shown {
                    assert!(vb.iter().filter(|f| granted(f, h.gives) == 1).all(|f| granted(f, s) == -1), "{} removes {s}", h.loc);
                }
                assert!(vb.iter().filter(|f| f.end == End::Done).any(|f| granted(f, item) == 1), "{}", h.loc);
            }
        }
    }

    #[test]
    fn the_spoil_retries_hand_back_only_the_chain_item() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let fl: HM<String, Flow> = all(&mut rf, NEW);
        let mut a: Sim = sim(&r, HM::new()).unwrap();
        let mut b: Sim = sim(&r, fl).unwrap();
        for (flow, entry, step, gives) in RETRIES {
            let sc: Scene = Scene { flow: flow.to_string(), entry: entry.to_string(), set: vec![(STEP.to_string(), step.to_string())], own: Vec::new() };
            let va: Vec<Fin> = run(&mut a, &sc).unwrap();
            let vb: Vec<Fin> = run(&mut b, &sc).unwrap();
            assert!(vb.iter().all(|f| granted(f, NEW) == 0), "{flow} {entry}");
            assert_eq!(shapes(&va, NEW), shapes(&vb, NEW), "{flow} {entry}");
            assert!(va.iter().any(|f| granted(f, gives) == 1), "{flow} {entry} gives {gives}");
        }
    }

    fn after(s: &mut Sim, st: State, flow: &str, entry: &str, want: &str) -> (State, Vec<Fin>) {
        let sc: Scene = Scene { flow: flow.to_string(), entry: entry.to_string(), set: Vec::new(), own: Vec::new() };
        let fins: Vec<Fin> = run_from(s, &sc, st).unwrap();
        let st: State = fins.iter().find(|f| granted(f, want) == 1).map(|f| f.m.st.clone()).unwrap_or_else(|| panic!("{flow} {entry} never gives {want}"));
        (st, fins)
    }

    #[test]
    fn the_egg_to_cojiro_sequence_keeps_its_order_with_shuffled_items() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let fl: HM<String, Flow> = all(&mut rf, NEW);
        let mut s: Sim = sim(&r, fl).unwrap();
        let mut st: State = State::new();
        set(&mut s, &mut st, STEP, "Ready");
        let (mut st, f1): (State, Vec<Fin>) = after(&mut s, st, "Npc_Hylia_049_Adult", "SubEvent_GetSwordBigGoron_Start", "Item_PocketEgg");
        assert!(f1.iter().filter(|f| granted(f, "Item_PocketEgg") == 1).all(|f| granted(f, NEW) == 1));
        set(&mut s, &mut st, STEP, "WaitEggHatch");
        let (mut st, f2): (State, Vec<Fin>) = after(&mut s, st, "SubEvent_GetSwordBigGoron", "WaitEggHatch_Npc_Navi_StepStart", "Item_PocketCucco");
        assert!(f2.iter().all(|f| granted(f, NEW) == 0));
        assert!(f2.iter().filter(|f| granted(f, "Item_PocketCucco") == 1).all(|f| granted(f, "Item_PocketEgg") == -1));
        set(&mut s, &mut st, STEP, "Step02");
        let (st, f3): (State, Vec<Fin>) = after(&mut s, st, "Npc_Hylia_049_Adult", "SubEvent_GetSwordBigGoron_UntilStep03_ShowItemTalk", "Item_CojiroCucco");
        assert!(f3.iter().filter(|f| granted(f, "Item_CojiroCucco") == 1).all(|f| granted(f, NEW) == 1 && granted(f, "Item_PocketCucco") == -1));
        assert!(f3.iter().any(|f| f.m.reqs.iter().any(|q| q.starts_with("SubEvent_GetSwordBigGoron -> Step03"))));
        for e in ["IsAfter_StartSubEvent_Talk", "Talk"] {
            let sc: Scene = Scene { flow: "Npc_Hylia_049_Adult".to_string(), entry: e.to_string(), set: Vec::new(), own: Vec::new() };
            let again: Vec<Fin> = run_from(&mut s, &sc, st.clone()).unwrap();
            assert!(again.iter().all(|f| f.m.gr.values().all(|n| *n == 0)), "{e} gives nothing after the hand in");
        }
        for e in ["SubEvent_GetSwordBigGoron_UntilStep03_ShowItemTalk", "SubEvent_GetSwordBigGoron_UntilStep03_Talk", "Talk_WithNPC", "Before_StartSubEvent_Talk", "SubEvent_GetSwordBigGoron_Start"] {
            let sc: Scene = Scene { flow: "Npc_Hylia_049_Adult".to_string(), entry: e.to_string(), set: Vec::new(), own: Vec::new() };
            let again: Vec<Fin> = run_from(&mut s, &sc, st.clone()).unwrap();
            for f in &again {
                assert_eq!(granted(f, NEW), granted(f, "Item_CojiroCucco") + granted(f, "Item_PocketEgg"), "{e} {:?}", f.m.gr);
            }
        }
    }

    #[test]
    fn the_child_egg_hatch_is_untouched() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let fl: HM<String, Flow> = all(&mut rf, NEW);
        assert!(!fl.contains_key("Goto_HyruleCastle"));
        let mut s: Sim = sim(&r, fl).unwrap();
        let mut st: State = State::new();
        set(&mut s, &mut st, "Step_Goto_HyruleCastle", "Step02_01");
        let (mut st, _): (State, Vec<Fin>) = after(&mut s, st, "Goto_HyruleCastle", "Goto_HyruleCastle_AfterMalonAppear", "Item_WeirdEgg");
        set(&mut s, &mut st, "Step_Goto_HyruleCastle", "Step03_01");
        let (_, f): (State, Vec<Fin>) = after(&mut s, st, "Goto_HyruleCastle", "Step03_01_Npc_Navi_StepStart", "Item_AlarmCucco");
        assert!(f.iter().filter(|x| granted(x, "Item_AlarmCucco") == 1).all(|x| granted(x, "Item_WeirdEgg") == -1 && granted(x, NEW) == 0));
    }
}
