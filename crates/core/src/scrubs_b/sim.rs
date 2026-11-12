#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap as BM, HashMap as HM};
    use std::path::{Path as P, PathBuf as PB};

    use crate::bfevfl::{Bf, Flow, Val};
    use crate::byml::{Byml, Node};
    use crate::flow_extra::sim::{broken, edited, granted, run, run_from, sim, Scene};
    use crate::flow_locations::{path_of, StagedFile};
    use crate::flow_sim::run::{lines, End, Fin, Sim};
    use crate::flow_sim::state::State;
    use crate::placement::hash_of;
    use crate::progressive::{ANALYZED_TABLE, SETTING_TABLE};
    use crate::romfs::Romfs;
    use crate::scrubs_b::{book, edit, jobs, point, stage_banc, stage_flow, stage_rows, Book, Job, Sale, Scrub, FILE, SHOWN};
    use crate::settings::seed::SeedFile;
    use crate::settings::{Settings, Value};
    use crate::slice::data::Data;
    use crate::slice::edit::Edited;
    use crate::staging::Out;

    const DUMP: &str = "C:/o/nup dump/updates/131072/romfs";
    const FLOW: &str = "Enemy_Scrub_Business_Common";
    const NEW: &str = "Item_Longshot";
    const SPARE: &str = "HavePlayedEvent.Enemy_Scrub_Business_Shield_Deku_01";
    const FLAGS: &str = include_str!("../../../../data/flags.json");

    fn root() -> Option<PB> {
        let p: PB = std::env::var_os("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|| PB::from(DUMP));
        if p.join("Event/EventFlow").is_dir() {
            Some(p)
        } else {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            None
        }
    }

    fn vanilla(r: &P) -> Flow {
        let mut rf: Romfs = Romfs::open(r).unwrap();
        let raw: Vec<u8> = rf.read_zs(&path_of(FILE)).unwrap();
        let mut b: Bf = Bf::parse(&raw).unwrap();
        b.flows.remove(0)
    }

    fn flag_of(s: &Scrub) -> String {
        if s.flag.is_empty() { SPARE.to_string() } else { s.flag.clone() }
    }

    fn sale(s: &Scrub, item: &str) -> Sale {
        Sale { entry: s.entry(), flag: flag_of(s), actor: item.to_string(), price: s.price }
    }

    fn with_sales(r: &P, base: Flow, sales: &[Sale]) -> Flow {
        let b: &Book = book().unwrap();
        let mut bf: Bf = Bf::parse(&Romfs::open(r).unwrap().read_zs(&path_of(FILE)).unwrap()).unwrap();
        bf.flows[0] = base;
        edit(&mut bf.flows[0], sales, &b.label).unwrap();
        let back: Bf = Bf::parse(&bf.write().unwrap()).unwrap();
        assert_eq!(back, bf);
        back.flows.into_iter().next().unwrap()
    }

    fn one(f: Flow) -> HM<String, Flow> {
        let mut m: HM<String, Flow> = HM::new();
        m.insert(FLOW.to_string(), f);
        m
    }

    fn scene(entry: &str, set: &[(&str, &str)], own: &[(&str, i64)]) -> Scene {
        Scene {
            flow: FLOW.to_string(),
            entry: entry.to_string(),
            set: set.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            own: own.iter().map(|(k, n)| (k.to_string(), *n)).collect(),
        }
    }

    fn summary(fins: &[Fin]) -> Vec<String> {
        let mut v: Vec<String> = fins
            .iter()
            .map(|f| {
                let g: Vec<String> = f.m.gr.iter().filter(|(_, n)| **n != 0).map(|(k, n)| format!("{k}x{n}")).collect();
                let w: Vec<String> = f.m.writes.iter().map(|(k, x)| format!("{k}={x}")).collect();
                format!("{:?}|{}|{}|{}", f.end, g.join(","), w.join(","), f.m.reqs.join(","))
            })
            .collect();
        v.sort();
        v.dedup();
        v
    }

    fn death(f: &Fin) -> bool {
        lines(&f.m.log).iter().any(|(_, t)| t.contains("EventTriggerRequestDeath"))
    }

    fn wrote(f: &Fin, k: &str) -> Option<String> {
        f.m.writes.get(k).cloned()
    }

    #[test]
    fn the_spare_flags_are_persistent_bools_that_no_row_and_no_vanilla_flow_names() {
        let Some(r) = root() else { return };
        let b: &Book = book().unwrap();
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let t: Byml = Byml::parse(&rf.read_zs(SETTING_TABLE).unwrap()).unwrap();
        let ids: Vec<String> = t.root.as_arr().unwrap().iter().filter_map(|n| n.get("__RowId").and_then(|v| v.as_str()).map(|s| s.to_string())).collect();
        let mut flags: Vec<String> = b.scrubs.iter().filter(|s| !s.flag.is_empty()).map(|s| s.flag.clone()).collect();
        flags.push(SPARE.to_string());
        let flows: Vec<String> = rf.list("Event/EventFlow").unwrap().into_iter().filter(|p| p.ends_with(".bfevfl.zs")).collect();
        assert!(flows.len() > 900, "{}", flows.len());
        let mut raws: Vec<Vec<u8>> = Vec::new();
        for p in &flows {
            raws.push(rf.read_zs(p).unwrap());
        }
        for f in &flags {
            let line: &str = FLAGS.lines().find(|l| l.contains(&format!("\"name\":\"{f}\""))).unwrap_or_else(|| panic!("{f} is not in flags.json"));
            assert!(line.contains("\"type\":\"Bool\"") && line.contains("\"reset\":8") && line.contains("\"save\":0"), "{line}");
            let row: &str = f.strip_prefix("HavePlayedEvent.").unwrap();
            assert!(!ids.iter().any(|i| i == row), "{row} has an EventSetting row");
            let needle: &[u8] = f.as_bytes();
            let hits: usize = raws.iter().filter(|x| x.windows(needle.len()).any(|w| w == needle)).count();
            assert_eq!(hits, 0, "{f} is named by {hits} vanilla flows");
        }
        for s in &b.scrubs {
            assert!(!ids.iter().any(|i| *i == s.entry()), "{}", s.entry());
            assert!(ids.contains(&b.template));
        }
    }

    #[test]
    fn the_base_alone_keeps_every_vanilla_entry_the_same() {
        let Some(r) = root() else { return };
        let v: Flow = vanilla(&r);
        let e: Flow = with_sales(&r, v.clone(), &[]);
        assert_eq!(e, v);
        let mut bf: Bf = Bf::parse(&Romfs::open(&r).unwrap().read_zs(&path_of(FILE)).unwrap()).unwrap();
        crate::scrubs_b::base(&mut bf.flows[0]).unwrap();
        let back: Bf = Bf::parse(&bf.write().unwrap()).unwrap();
        assert_eq!(back, bf);
        let based: Flow = back.flows[0].clone();
        let diff: Vec<usize> = (0..v.events.len()).filter(|i| v.events[*i] != based.events[*i]).collect();
        assert_eq!(diff, vec![155, 171, 172, 327]);
        assert_eq!((based.events.len(), based.entries.len()), (v.events.len(), v.entries.len()));
        let mut a: Sim = sim(&r, HM::new()).unwrap();
        let mut b: Sim = sim(&r, one(based)).unwrap();
        let own: [(&str, i64); 3] = [("Item_BulletBag_Small", 1), ("Item_BombBag_Small", 1), ("Item_Quiver_Small", 1)];
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let t: Byml = Byml::parse(&rf.read_zs(SETTING_TABLE).unwrap()).unwrap();
        let started: Vec<String> = t
            .root
            .as_arr()
            .unwrap()
            .iter()
            .filter(|n| n.get("FlowchartFileName").and_then(|v| v.as_str()).is_some_and(|x| x.ends_with("/Enemy_Scrub_Business_Common.evfc")))
            .filter_map(|n| n.get("EntryPointName").and_then(|v| v.as_str()).map(|x| x.to_string()))
            .collect();
        assert!(started.len() >= 10 && !started.iter().any(|x| x == "SimpleShopRoot_Enemy_Scrub_Business_Pouch"));
        for en in v.entries.iter().map(|x| x.name.clone()).filter(|x| x != "SimpleShopRoot_Enemy_Scrub_Business_Pouch") {
            for shown in ["Obj_Heart_Piece", "Item_StickBag_Medium", "Item_NutBag_Medium", "Shop_DekuSeedBundle_02"] {
                let st: [(&str, &str); 4] = [
                    ("PlayerStatus.CurrentRupee", "300"),
                    ("Scrub_Business_TempItemName_00", shown),
                    ("Enemy_Scrub_Business_Secret_Success", "true"),
                    ("IsGet.Item_DekuStick", "true"),
                ];
                let sc: Scene = scene(&en, &st, &own);
                let x: Vec<String> = summary(&run(&mut a, &sc).unwrap());
                let y: Vec<String> = summary(&run(&mut b, &sc).unwrap());
                assert_eq!(x, y, "{en} {shown}");
            }
        }
    }

    #[test]
    fn the_base_on_top_of_a_group_a_edit_keeps_the_crater_item() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let a: Flow = edited(&mut rf, "DMC Deku Scrub", NEW).unwrap().remove(FLOW).unwrap();
        let b: &Book = book().unwrap();
        let s: &Scrub = b.by_key("Jabu").unwrap();
        let e: Flow = with_sales(&r, a.clone(), &[sale(s, "Item_HoverBoot")]);
        let mut x: Sim = sim(&r, one(a)).unwrap();
        let mut y: Sim = sim(&r, one(e)).unwrap();
        let sc: Scene = scene("Enemy_Scrub_Business_Heart_02", &[("PlayerStatus.CurrentRupee", "300"), ("Scrub_Business_TempItemName_00", SHOWN)], &[]);
        let va: Vec<Fin> = run(&mut x, &sc).unwrap();
        let vb: Vec<Fin> = run(&mut y, &sc).unwrap();
        assert_eq!(summary(&va), summary(&vb));
        assert!(vb.iter().any(|f| granted(f, NEW) == 1));
        assert!(vb.iter().all(|f| granted(f, "Item_HoverBoot") == 0));
    }

    #[test]
    fn every_scrub_sells_once_sets_its_flag_only_on_a_sale_and_leaves_once_sold() {
        let Some(r) = root() else { return };
        let b: &Book = book().unwrap();
        let v: Flow = vanilla(&r);
        for s in &b.scrubs {
            let f: String = flag_of(s);
            let e: Flow = with_sales(&r, v.clone(), &[sale(s, NEW)]);
            assert_eq!(e.events.len(), v.events.len() + 4, "{}", s.key);
            assert_eq!(e.entries.len(), v.entries.len() + 1, "{}", s.key);
            for (i, x) in v.events.iter().enumerate().filter(|(i, _)| ![155, 171, 172, 327].contains(i)) {
                assert_eq!(&e.events[i], x, "{} {i}", s.key);
            }
            let mut m: Sim = sim(&r, one(e)).unwrap();
            let tag: &str = &s.key;
            let fresh: [(&str, &str); 3] = [("PlayerStatus.CurrentRupee", "300"), ("Scrub_Business_TempItemName_00", SHOWN), (f.as_str(), "false")];
            let sc: Scene = scene(&s.entry(), &fresh, &[]);
            let fins: Vec<Fin> = run(&mut m, &sc).unwrap();
            assert_eq!(broken(&fins), 0, "{tag}");
            let sold: Vec<&Fin> = fins.iter().filter(|x| granted(x, NEW) > 0).collect();
            let kept: Vec<&Fin> = fins.iter().filter(|x| granted(x, NEW) == 0).collect();
            assert!(!sold.is_empty() && !kept.is_empty(), "{tag}");
            let left: String = (300 - s.price).to_string();
            for x in &sold {
                assert_eq!(granted(x, NEW), 1, "{tag}");
                assert_eq!(x.end, End::Done, "{tag}");
                assert_eq!(wrote(x, &f).as_deref(), Some("true"), "{tag}");
                assert_eq!(wrote(x, "PlayerStatus.CurrentRupee"), Some(left.clone()), "{tag}");
                assert!(death(x), "{tag}");
                assert_eq!(x.m.gr.values().filter(|n| **n != 0).count(), 1, "{tag}");
            }
            for x in &kept {
                assert!(wrote(x, &f).is_none() && wrote(x, "PlayerStatus.CurrentRupee").is_none(), "{tag}");
                assert!(x.m.gr.values().all(|n| *n == 0), "{tag}");
            }
            let poor: String = (s.price - 10).to_string();
            let short: Vec<Fin> = run(&mut m, &scene(&s.entry(), &[("PlayerStatus.CurrentRupee", poor.as_str()), ("Scrub_Business_TempItemName_00", SHOWN), (f.as_str(), "false")], &[])).unwrap();
            assert!(short.iter().all(|x| granted(x, NEW) == 0 && wrote(x, &f).is_none()), "{tag}");
            let done: Vec<Fin> = run(&mut m, &scene(&s.entry(), &[("PlayerStatus.CurrentRupee", "300"), ("Scrub_Business_TempItemName_00", SHOWN), (f.as_str(), "true")], &[])).unwrap();
            assert!(!done.is_empty(), "{tag}");
            assert!(done.iter().all(|x| granted(x, NEW) == 0 && wrote(x, &f).is_none() && wrote(x, "PlayerStatus.CurrentRupee").is_none() && death(x)), "{tag}");
            let after: State = sold[0].m.st.clone();
            let again: Vec<Fin> = run_from(&mut m, &sc, after).unwrap();
            assert!(!again.is_empty() && again.iter().all(|x| granted(x, NEW) == 0 && death(x)), "{tag} second talk");
            let raw: Vec<Fin> = run(&mut m, &scene(&s.entry(), &[("PlayerStatus.CurrentRupee", "300"), ("Scrub_Business_TempItemName_00", "Shop_DekuSeedBundle_02"), (f.as_str(), "false")], &[])).unwrap();
            assert!(raw.iter().all(|x| granted(x, NEW) == 0 && wrote(x, &f).is_none()), "{tag} without the Banc string");
        }
    }

    #[test]
    fn all_ten_entries_fit_in_one_flow_and_each_gives_only_its_own_item() {
        let Some(r) = root() else { return };
        let b: &Book = book().unwrap();
        let v: Flow = vanilla(&r);
        let items: [&str; 10] = ["Item_Longshot", "Item_HoverBoot", "Item_GlassTruth", "Item_Hookshot", "Item_Boomerang", "Item_Hammer", "Item_Bottle", "Item_DinFire", "Item_MirrorShield", "Item_IronBoot"];
        let sales: Vec<Sale> = b.scrubs.iter().zip(items).enumerate().map(|(i, (s, it))| Sale { entry: s.entry(), flag: if i % 2 == 0 { SPARE.to_string() } else { "HavePlayedEvent.Enemy_Scrub_Business_DekuStick_01".to_string() }, actor: it.to_string(), price: s.price }).collect();
        let e: Flow = with_sales(&r, v.clone(), &sales);
        assert_eq!(e.events.len(), v.events.len() + 40);
        assert_eq!(e.entries.len(), v.entries.len() + 10);
        let mut m: Sim = sim(&r, one(e)).unwrap();
        for (s, it) in sales.iter().zip(items) {
            let fins: Vec<Fin> = run(&mut m, &scene(&s.entry, &[("PlayerStatus.CurrentRupee", "300"), ("Scrub_Business_TempItemName_00", SHOWN)], &[])).unwrap();
            for x in &fins {
                for o in items.iter().filter(|o| **o != it) {
                    assert_eq!(granted(x, o), 0, "{} {o}", s.entry);
                }
            }
            assert!(fins.iter().any(|x| granted(x, it) == 1 && wrote(x, &s.flag).as_deref() == Some("true")), "{}", s.entry);
        }
    }

    #[test]
    fn every_placement_is_found_in_the_dump_and_points_at_its_new_row() {
        let Some(r) = root() else { return };
        let b: &Book = book().unwrap();
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        for s in &b.scrubs {
            for sp in &s.spots {
                let mut y: Byml = Byml::parse(&rf.read_zs(&sp.file).unwrap()).unwrap();
                let before: Byml = y.clone();
                assert!(point(&mut y, sp, &s.entry()).unwrap(), "{} {}", s.key, sp.hash);
                let back: Byml = Byml::parse(&y.write().unwrap()).unwrap();
                assert_eq!(back.root, y.root);
                let all = |z: &Byml| z.root.get("Actors").unwrap().as_arr().unwrap().to_vec();
                let (x, z): (Vec<Node>, Vec<Node>) = (all(&before), all(&back));
                assert_eq!(x.len(), z.len());
                for (p, q) in x.iter().zip(z.iter()) {
                    if hash_of(p) == Some(sp.hash) {
                        let d: &Node = q.get("Dynamic").unwrap();
                        assert_eq!(d.get("TalkEventName").and_then(|v| v.as_str()), Some(s.entry().as_str()));
                        assert_eq!(d.get("ScrubBusiness_ShopItemActorName0").and_then(|v| v.as_str()), Some(SHOWN));
                        let hit: Option<&str> = d.get("AttackedEventName").and_then(|v| v.as_str());
                        let en: String = s.entry();
                        assert_eq!(hit, if sp.attacked.is_empty() { None } else { Some(en.as_str()) });
                    } else {
                        assert_eq!(p, q);
                    }
                }
            }
        }
    }

    fn seed(m: &[(&str, &str)]) -> SeedFile {
        let d: Data = Data::load().unwrap();
        let mut s: Settings = Settings::default();
        s.set("shuffle_rewards", Value::s("supported")).unwrap();
        s.set(crate::scrubs_b::SETTING, Value::Bool(true)).unwrap();
        let p: BM<String, String> = m.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        SeedFile::new("t", &s, &d.scan.game, p).unwrap()
    }

    #[test]
    fn the_vanilla_item_placed_back_makes_no_job_and_other_items_do() {
        let d: Data = Data::load().unwrap();
        assert!(jobs(&d, &seed(&[("LW Deku Scrub Grotto Rear", "Deku Seeds (30)"), ("Jabu Jabus Belly Deku Scrub", "Deku Nuts (5)")])).unwrap().is_empty());
        let j: Vec<Job> = jobs(&d, &seed(&[("LW Deku Scrub Grotto Rear", "Hover Boots"), ("Jabu Jabus Belly Deku Scrub", "Deku Nuts (5)"), ("HF Deku Scrub Grotto", "Lens of Truth")])).unwrap();
        assert_eq!(j.len(), 1);
        assert_eq!((j[0].sale.entry.as_str(), j[0].sale.actor.as_str(), j[0].sale.price), ("OOTRR_Scrub_LwGrottoRear", "Item_HoverBoot", 30));
        assert_eq!(j[0].sale.flag, "HavePlayedEvent.Enemy_Scrub_Business_DekuStick_01");
        assert!(jobs(&d, &seed(&[("Jabu Jabus Belly Deku Scrub", "Rupees (50)")])).is_err());
        assert!(jobs(&d, &seed(&[("Jabu Jabus Belly Deku Scrub", "Progressive Hookshot")])).is_err());
    }

    #[test]
    fn staging_writes_the_flow_the_banc_files_and_both_tables_and_merges_into_earlier_output() {
        let Some(r) = root() else { return };
        let d: Data = Data::load().unwrap();
        let b: &Book = book().unwrap();
        let j: Vec<Job> = jobs(&d, &seed(&[("LW Deku Scrub Grotto Rear", "Hover Boots"), ("Jabu Jabus Belly Deku Scrub", "Lens of Truth")])).unwrap();
        assert_eq!(j.len(), 2);
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let mut none: Vec<StagedFile> = Vec::new();
        stage_flow(&mut rf, &[], &mut none).unwrap();
        assert!(none.is_empty());
        let mut files: Vec<StagedFile> = Vec::new();
        stage_flow(&mut rf, &j, &mut files).unwrap();
        assert_eq!(files.len(), 1);
        let f: Flow = Bf::parse(&rf.zs.decompress(&files[0].z).unwrap()).unwrap().flows.remove(0);
        for x in &j {
            let en = f.entries.iter().find(|e| e.name == x.sale.entry).unwrap();
            let p = f.events[usize::from(en.subs[0])].kind.params().unwrap();
            assert_eq!(p.get("SaleActor"), Some(&Val::Str(x.sale.actor.clone())));
        }
        let mut edits: Vec<Edited> = Vec::new();
        assert_eq!(stage_banc(&mut rf, &j, &mut edits).unwrap(), 3);
        assert_eq!(edits.len(), 3);
        let mut outs: Vec<Out> = Vec::new();
        assert_eq!(stage_rows(&mut rf, &j, &mut outs, false).unwrap(), 0);
        assert_eq!(stage_rows(&mut rf, &j, &mut outs, true).unwrap(), 2);
        for o in &outs {
            let y: Byml = Byml::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap();
            let rows: &[Node] = y.root.as_arr().unwrap();
            let t: &Node = rows.iter().find(|n| n.get("__RowId").and_then(|v| v.as_str()) == Some(b.template.as_str())).unwrap();
            for x in &j {
                let n: &Node = rows.iter().find(|n| n.get("__RowId").and_then(|v| v.as_str()) == Some(x.sale.entry.as_str())).unwrap();
                let mut want: Node = t.clone();
                want.set("__RowId", Node::Str(x.sale.entry.clone()));
                if o.path == SETTING_TABLE {
                    want.set("EntryPointName", Node::Str(x.sale.entry.clone()));
                }
                assert_eq!(n, &want, "{} {}", o.path, x.sale.entry);
            }
        }
        let used: std::collections::BTreeSet<(String, crate::progressive::Size)> = [("Hookshot".to_string(), crate::progressive::Size::Small)].into_iter().collect();
        let mut up: Vec<Out> = crate::progressive::stage(&mut rf, &used, None, &[], &[]).unwrap();
        assert_eq!(stage_rows(&mut rf, &j, &mut up, false).unwrap(), 2);
        let y: Byml = Byml::parse(&rf.zs.decompress(&up.iter().find(|o| o.path == SETTING_TABLE).unwrap().z).unwrap()).unwrap();
        let ids: Vec<&str> = y.root.as_arr().unwrap().iter().filter_map(|n| n.get("__RowId").and_then(|v| v.as_str())).collect();
        assert!(ids.contains(&"OOTRR_Prog_Hookshot_Small") && ids.contains(&"OOTRR_Scrub_Jabu") && ids.contains(&"OOTRR_Scrub_LwGrottoRear"));
        assert!(up.iter().any(|o| o.path == ANALYZED_TABLE));
    }
}
