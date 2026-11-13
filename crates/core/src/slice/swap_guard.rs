use crate::error::{Error as E, Res};
use crate::pickup_persist::{check_rows, check_swaps_with, rows, Extra};
use crate::romfs::Romfs;
use crate::slice::copies::fits;
use crate::slice::data::Data;
use crate::slice::edit::Target;
use crate::slice::pickups::is_world;

pub fn check(rf: &mut Romfs, d: &Data, ts: &[Target], extra: &Extra) -> Res<()> {
    let world: Vec<&Target> = ts.iter().filter(|t| d.cat.by_name(&t.name).is_some_and(is_world)).collect();
    let actors: Vec<&str> = world.iter().map(|t| t.new_actor.as_str()).collect();
    check_swaps_with(rf, &actors, extra)?;
    let added: Vec<&Target> = world.into_iter().filter(|t| extra.has(&t.new_actor)).collect();
    if added.is_empty() {
        return Ok(());
    }
    if let Some(t) = added.iter().find(|t| !fits(d, &t.name, &t.new_item)) {
        return Err(E::Msg(format!("{} exists once per age, so {} placed there could be taken twice", t.name, t.new_item)));
    }
    let spots: Vec<(u64, &str)> = added.iter().map(|t| (t.hash, t.new_actor.as_str())).collect();
    check_rows(&rows(rf)?, &spots)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap as HM;
    use std::fs;
    use std::path::{Path as P, PathBuf as PB};

    use super::*;
    use crate::byml::Byml;
    use crate::catalog::Loc;
    use crate::modpack::Host;
    use crate::pickup_pack::guard;
    use crate::pickup_pack::place::current;
    use crate::pickup_pack::write::{run, Done, Opts};
    use crate::pickup_pack::{pack_of, Spec, Style};
    use crate::settings::seed::SeedFile;
    use crate::settings::{Settings, Value};
    use crate::slice::edit::{build_with, Built};
    use crate::slice::generate::{generate_data, GenOpts, Generated};
    use crate::slice::pickups::{targets_with, SETTING};

    const SPOT: &str = "LLR Freestanding PoH";
    const TWIN: &str = "KF Sarias House Recovery Heart 1";

    fn real() -> Option<PB> {
        let p: PB = std::env::var("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|_| PB::from("C:/o/nup dump/updates/131072/romfs"));
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Some(p)
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    fn spec(item: &str, name: &str) -> Spec {
        Spec { item: item.to_string(), name: name.to_string(), style: Style::Touch, heap: None }
    }

    fn opts(base: Option<PB>, dry: bool) -> Opts {
        Opts { host: Host::Eden, base, rst_from: None, places: Vec::new(), dry }
    }

    fn pool(root: &P, item: &str, name: &str) -> Extra {
        let d: Done = run(root, P::new("unused"), "unused", &spec(item, name), &opts(None, true)).unwrap();
        guard::extra(&d.files, &d.carried).unwrap()
    }

    fn seed_with(d: &Data, item: &str, at: &str, tag: &str) -> SeedFile {
        let mut s: Settings = Settings::default();
        s.set(SETTING, Value::s("all")).unwrap();
        s.set("shuffle_chests", Value::s("all")).unwrap();
        let g: Generated = generate_data(d, &s, tag, &GenOpts::default()).unwrap();
        let mut seed: SeedFile = g.seed_file;
        let from: String = seed.placements.iter().find(|(_, i)| i.as_str() == item).map(|(l, _)| l.clone()).unwrap();
        let old: String = seed.placements.get(at).cloned().unwrap();
        seed.placements.insert(at.to_string(), item.to_string());
        seed.placements.insert(from, old);
        seed
    }

    fn widen(root: &P, d: &Data, seed: &SeedFile, x: &Extra) -> Extra {
        let v: crate::slice::tier_order::Verdict = crate::slice::tier_order::fixed_actors(d, seed).unwrap();
        let mut w: std::collections::BTreeMap<String, crate::slice::seed_packs::Want> = crate::slice::seed_packs::need(d, seed, &v.assign).unwrap();
        w.retain(|k, _| !x.has(k));
        let mut out: Extra = crate::slice::seed_packs::pool(root, Host::Eden, &w).unwrap();
        out.packs.extend(x.packs.clone());
        out.flags.extend(x.flags.clone());
        out
    }

    #[test]
    fn a_seed_build_swaps_a_freestanding_spot_to_a_new_pack_and_the_guard_passes() {
        let Some(root) = real() else {
            return;
        };
        let d: Data = Data::load().unwrap();
        let x: Extra = pool(&root, "Item_Hammer", "Obj_Hammer");
        let seed: SeedFile = seed_with(&d, "Megaton Hammer", SPOT, "swap guard hammer");
        let load: PB = std::env::temp_dir().join(format!("ootrr_swap_guard_{}", std::process::id()));
        let _ = fs::remove_dir_all(&load);
        let none: String = build_with(&seed, &root, &load, "plain", Host::Eden, &Extra::default()).unwrap_err().to_string();
        assert!(none.contains("has no world pickup"), "{none}");
        let b: Built = build_with(&seed, &root, &load, "packed", Host::Eden, &widen(&root, &d, &seed, &x)).unwrap();
        let out: PB = b.root.join("romfs");
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let l: &Loc = d.cat.by_name(SPOT).unwrap();
        let mut n: usize = 0;
        for r in &l.refs {
            for f in &r.files {
                let y: Byml = Byml::parse(&rf.zs.decompress(&fs::read(out.join(f)).unwrap()).unwrap()).unwrap();
                assert_eq!(current(&y, r.hash).as_deref(), Some("Obj_Hammer"), "{f}");
                n += 1;
            }
        }
        assert!(n > 0);
        assert!(!out.join(pack_of("Obj_Hammer")).is_file());
        let done: Done = run(&root, &load, "packed", &spec("Item_Hammer", "Obj_Hammer"), &opts(Some(out.clone()), false)).unwrap();
        assert_eq!(done.root.as_deref(), Some(b.root.as_path()));
        assert!(done.report.contains("Obj_Hammer stays collected once taken"));
        assert!(out.join(pack_of("Obj_Hammer")).is_file());
        let seen: Extra = Extra::load(&mut rf, &out).unwrap();
        assert!(seen.has("Obj_Hammer"));
        assert_eq!(seen.flags.get("Obj_Hammer"), Some(&true));
        let ts: Vec<Target> = targets_with(&d, l, "Megaton Hammer", &seen, None).unwrap();
        assert_eq!(ts.len(), l.refs.len());
        assert!(check(&mut rf, &d, &ts, &seen).is_ok());
        let _ = fs::remove_dir_all(&load);
    }

    #[test]
    fn the_hookshot_pack_passes_on_a_single_spot_with_the_tier_the_order_names() {
        let Some(root) = real() else {
            return;
        };
        let d: Data = Data::load().unwrap();
        let x: Extra = pool(&root, "Item_Hookshot", "Obj_Hookshot");
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let l: &Loc = d.cat.by_name(SPOT).unwrap();
        assert!(targets_with(&d, l, "Progressive Hookshot", &x, None).is_err());
        assert!(targets_with(&d, l, "Progressive Hookshot", &x, Some("Item_Longshot")).is_err());
        let ts: Vec<Target> = targets_with(&d, l, "Progressive Hookshot", &x, Some("Item_Hookshot")).unwrap();
        assert!(ts.iter().all(|t| t.new_actor == "Obj_Hookshot"));
        assert!(check(&mut rf, &d, &ts, &x).is_ok());
        assert!(check(&mut rf, &d, &ts, &Extra::default()).is_err());
    }

    #[test]
    fn a_pack_that_breaks_the_rule_is_refused_before_anything_is_written() {
        let Some(root) = real() else {
            return;
        };
        let d: Data = Data::load().unwrap();
        let good: Extra = pool(&root, "Item_Hammer", "Obj_Hammer");
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let l: &Loc = d.cat.by_name(SPOT).unwrap();
        let ts: Vec<Target> = targets_with(&d, l, "Megaton Hammer", &good, None).unwrap();
        let mut off: Extra = good.clone();
        off.flags.insert("Obj_Hammer".to_string(), false);
        let e: String = check(&mut rf, &d, &ts, &off).unwrap_err().to_string();
        assert!(e.contains("IsRespawn"), "{e}");
        let mut key: Extra = Extra::default();
        key.packs.insert("Obj_Hammer".to_string(), crate::sarc::parse(&rf.read_zs(&pack_of("Obj_Dungeon_SmallKey")).unwrap()).unwrap());
        key.flags.insert("Obj_Hammer".to_string(), true);
        assert!(check(&mut rf, &d, &ts, &key).is_err());
        let twin: &Loc = d.cat.by_name(TWIN).unwrap();
        let two: Vec<Target> = targets_with(&d, twin, "Megaton Hammer", &good, None).unwrap();
        let e: String = check(&mut rf, &d, &two, &good).unwrap_err().to_string();
        assert!(e.contains("once per age"), "{e}");
        let mut moved: Vec<Target> = ts.clone();
        moved[0].hash = 1;
        let e: String = check(&mut rf, &d, &moved, &good).unwrap_err().to_string();
        assert!(e.contains("no permanent saved row"), "{e}");
        let unsaved: HM<u64, (i64, i64)> = HM::from([(ts[0].hash, (88, 0))]);
        assert!(check_rows(&unsaved, &[(ts[0].hash, "Obj_Hammer")]).is_err());
        let seed: SeedFile = seed_with(&d, "Megaton Hammer", SPOT, "swap guard refused");
        let load: PB = std::env::temp_dir().join(format!("ootrr_swap_guard_no_{}", std::process::id()));
        let _ = fs::remove_dir_all(&load);
        let e: String = build_with(&seed, &root, &load, "refused", Host::Eden, &widen(&root, &d, &seed, &off)).unwrap_err().to_string();
        assert!(e.contains("IsRespawn"), "{e}");
        assert!(!load.exists());
    }
}
