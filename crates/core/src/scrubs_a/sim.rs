#[cfg(test)]
mod tests {
    use std::collections::HashMap as HM;
    use std::path::{Path as P, PathBuf as PB};

    use crate::bfevfl::{Bf, Flow};
    use crate::flow_extra::sim::{broken, edited, granted, run, run_from, sim, Scene};
    use crate::flow_locations::{path_of, Data, Loc};
    use crate::flow_sim::run::{End, Fin, Sim};
    use crate::flow_sim::state::State;
    use crate::romfs::Romfs;

    const DUMP: &str = "C:/o/nup dump/updates/131072/romfs";
    const FLOW: &str = "Enemy_Scrub_Business_Common";
    const FILE: &str = "Enemy_Scrub_Business_Common.bfevfl";
    const NEW: &str = "Item_Longshot";

    struct Case {
        loc: &'static str,
        entry: &'static str,
        shown: &'static str,
        flag: &'static str,
        old: &'static [&'static str],
        price: i64,
    }

    const CASES: [Case; 4] = [
        Case { loc: "DMC Deku Scrub", entry: "Enemy_Scrub_Business_Heart_02", shown: "Obj_Heart_Piece", flag: "DeathMountain_Enemy_Scrub_Business_HeartPiece_01", old: &["Obj_Heart_Piece"], price: 100 },
        Case { loc: "LW Deku Scrub Near Bridge", entry: "Enemy_Scrub_Business_DekuStickPouch_01", shown: "Item_StickBag_Medium", flag: "LostWood_Enemy_Scrub_Business_DekuStickPouch_01", old: &["Item_StickBag_Medium", "Item_StickBag_Large"], price: 40 },
        Case { loc: "LW Deku Scrub Grotto Front", entry: "Enemy_Scrub_Business_DekuNutsPouch_01", shown: "Item_NutBag_Medium", flag: "Lostwood_Enemy_Scrub_Business_DekuNutsPouch_01", old: &["Item_NutBag_Medium", "Item_NutBag_Large"], price: 40 },
        Case { loc: "HF Deku Scrub Grotto", entry: "Enemy_Scrub_Business_Heart", shown: "Obj_Heart_Piece", flag: "HyruleField_Enemy_Scrub_Business_HeartPiece_01", old: &["Obj_Heart_Piece"], price: 100 },
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

    fn scene(c: &Case, rupees: i64, set: &[(&str, &str)], own: &[(&str, i64)]) -> Scene {
        let mut v: Vec<(String, String)> = vec![
            ("Scrub_Business_TempItemName_00".to_string(), c.shown.to_string()),
            ("PlayerStatus.CurrentRupee".to_string(), rupees.to_string()),
            ("Enemy_Scrub_Business_Secret_Success".to_string(), "true".to_string()),
            (c.flag.to_string(), "false".to_string()),
        ];
        v.extend(set.iter().map(|(k, x)| (k.to_string(), x.to_string())));
        Scene { flow: FLOW.to_string(), entry: c.entry.to_string(), set: v, own: own.iter().map(|(k, n)| (k.to_string(), *n)).collect() }
    }

    fn vanilla(r: &P) -> Flow {
        let mut rf: Romfs = Romfs::open(r).unwrap();
        let raw: Vec<u8> = rf.read_zs(&path_of(FILE)).unwrap();
        let mut b: Bf = Bf::parse(&raw).unwrap();
        b.flows.remove(0)
    }

    fn total(fins: &[Fin], items: &[&str]) -> i64 {
        fins.iter().map(|f| items.iter().map(|i| granted(f, i)).sum::<i64>()).max().unwrap_or(0)
    }

    fn wrote(f: &Fin, flag: &str) -> Option<String> {
        f.m.writes.get(flag).cloned()
    }

    fn sims(r: &P, loc: &str) -> (Sim, Sim) {
        let mut rf: Romfs = Romfs::open(r).unwrap();
        let fl: HM<String, Flow> = edited(&mut rf, loc, NEW).unwrap();
        assert_eq!(fl.len(), 1, "{loc}");
        (sim(r, HM::new()).unwrap(), sim(r, fl).unwrap())
    }

    #[test]
    fn the_vanilla_item_placed_back_leaves_the_flow_untouched() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        assert!(edited(&mut rf, "DMC Deku Scrub", "Obj_Heart_Piece").unwrap().is_empty());
        assert!(edited(&mut rf, "HF Deku Scrub Grotto", "Obj_Heart_Piece").unwrap().is_empty());
        let d: Data = Data::load().unwrap();
        for c in &CASES[1..3] {
            let l: &Loc = d.by_name(c.loc).unwrap();
            let a: HM<String, Flow> = edited(&mut rf, c.loc, l.item.as_deref().unwrap()).unwrap();
            let f: &Flow = &a[FLOW];
            let v: Flow = vanilla(&r);
            let diff: Vec<usize> = (0..v.events.len()).filter(|i| v.events[*i] != f.events[*i]).collect();
            assert!(diff.iter().all(|i| l.edits.iter().any(|e| e.index == *i && e.item != l.item.as_deref().unwrap_or(""))), "{} {diff:?}", c.loc);
        }
    }

    #[test]
    fn only_the_sale_events_change_and_the_file_round_trips() {
        let Some(r) = root() else { return };
        let v: Flow = vanilla(&r);
        let d: Data = Data::load().unwrap();
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        for c in &CASES {
            let l: &Loc = d.by_name(c.loc).unwrap();
            let fl: HM<String, Flow> = edited(&mut rf, c.loc, NEW).unwrap();
            let f: &Flow = &fl[FLOW];
            assert_eq!(f.events.len(), v.events.len(), "{}", c.loc);
            assert_eq!(f.entries, v.entries, "{}", c.loc);
            assert_eq!(f.actors, v.actors, "{}", c.loc);
            let mut diff: Vec<usize> = (0..v.events.len()).filter(|i| v.events[*i] != f.events[*i]).collect();
            let mut want: Vec<usize> = l.edits.iter().map(|e| e.index).collect();
            diff.sort_unstable();
            want.sort_unstable();
            assert_eq!(diff, want, "{}", c.loc);
            for i in want {
                let a: String = format!("{:?}", v.events[i]);
                let b: String = format!("{:?}", f.events[i]);
                let old: &str = &l.edits.iter().find(|e| e.index == i).unwrap().item;
                assert_eq!(a.replacen(&format!("\"{old}\""), &format!("\"{NEW}\""), 1), b, "{} {i}", c.loc);
            }
        }
    }

    #[test]
    fn each_scrub_sells_the_new_item_once_and_only_a_sale_writes_the_sold_flag() {
        let Some(r) = root() else { return };
        let stick: [(&str, &str); 1] = [("IsGet.Item_DekuStick", "true")];
        for c in &CASES {
            let (mut a, mut b): (Sim, Sim) = sims(&r, c.loc);
            let mut setups: Vec<(Vec<(&str, &str)>, Vec<(&str, i64)>)> = vec![(Vec::new(), Vec::new())];
            if c.loc.contains("Bridge") {
                setups.push((stick.to_vec(), Vec::new()));
                setups.push((stick.to_vec(), vec![("Item_StickBag_Medium", 1)]));
            }
            if c.loc.contains("Grotto Front") {
                setups.push((Vec::new(), vec![("Item_NutBag_Medium", 1)]));
            }
            for (set, own) in &setups {
                let sc: Scene = scene(c, 300, set, own);
                let va: Vec<Fin> = run(&mut a, &sc).unwrap();
                let vb: Vec<Fin> = run(&mut b, &sc).unwrap();
                let tag: String = format!("{} {set:?} {own:?}", c.loc);
                assert_eq!(total(&va, c.old), 1, "{tag}");
                assert_eq!(total(&vb, &[NEW]), 1, "{tag}");
                assert_eq!(total(&vb, c.old), 0, "{tag}");
                assert_eq!(vb.len(), va.len(), "{tag}");
                assert_eq!(broken(&vb), broken(&va), "{tag}");
                let sold: Vec<&Fin> = vb.iter().filter(|f| granted(f, NEW) == 1).collect();
                let kept: Vec<&Fin> = vb.iter().filter(|f| granted(f, NEW) == 0).collect();
                assert!(!sold.is_empty() && !kept.is_empty(), "{tag}");
                assert!(sold.iter().all(|f| wrote(f, c.flag).as_deref() == Some("true") && f.end == End::Done), "{tag}");
                assert!(sold.iter().all(|f| wrote(f, "PlayerStatus.CurrentRupee") == Some((300 - c.price).to_string())), "{tag}");
                assert!(kept.iter().all(|f| wrote(f, c.flag).is_none() && wrote(f, "PlayerStatus.CurrentRupee").is_none()), "{tag}");
                for (x, y) in va.iter().zip(vb.iter()) {
                    assert_eq!(wrote(x, c.flag), wrote(y, c.flag), "{tag}");
                }
            }
        }
    }

    #[test]
    fn too_few_rupees_sell_nothing_and_write_nothing() {
        let Some(r) = root() else { return };
        for c in &CASES {
            let (_, mut b): (Sim, Sim) = sims(&r, c.loc);
            let vb: Vec<Fin> = run(&mut b, &scene(c, c.price - 10, &[], &[])).unwrap();
            assert_eq!(total(&vb, &[NEW]), 0, "{}", c.loc);
            assert!(vb.iter().all(|f| wrote(f, c.flag).is_none()), "{}", c.loc);
            assert_eq!(broken(&vb), 0, "{}", c.loc);
        }
    }

    #[test]
    fn a_second_talk_after_the_sale_behaves_as_in_vanilla() {
        let Some(r) = root() else { return };
        for c in &CASES {
            let (mut a, mut b): (Sim, Sim) = sims(&r, c.loc);
            let sc: Scene = scene(c, 300, &[], &[]);
            let va: Vec<Fin> = run(&mut a, &sc).unwrap();
            let vb: Vec<Fin> = run(&mut b, &sc).unwrap();
            let sa: State = va.iter().find(|f| c.old.iter().map(|i| granted(f, i)).sum::<i64>() == 1).unwrap().m.st.clone();
            let sb: State = vb.iter().find(|f| granted(f, NEW) == 1).unwrap().m.st.clone();
            let again_a: Vec<Fin> = run_from(&mut a, &sc, sa).unwrap();
            let again_b: Vec<Fin> = run_from(&mut b, &sc, sb).unwrap();
            let twice_a: i64 = total(&again_a, c.old);
            let twice_b: i64 = total(&again_b, &[NEW]);
            eprintln!("{}: vanilla second talk grants {twice_a}, edited {twice_b}", c.loc);
            assert_eq!(twice_a, twice_b, "{}", c.loc);
        }
    }
}
