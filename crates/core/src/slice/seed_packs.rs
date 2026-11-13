use std::collections::BTreeMap as BM;
use std::fmt::Write as W;
use std::fs;
use std::path::{Path as P, PathBuf as PB};

use crate::byml::Byml;
use crate::error::{Error as E, Res};
use crate::modpack::Host;
use crate::pickup_pack::rows::{self, ACTOR_INFO, GAME_ACTOR_INFO, POUCH_INFO};
use crate::pickup_pack::write::{run_with, Done, Opts};
use crate::pickup_pack::{guard, kit, pack_of, sizes, Layers, Spec};
use crate::pickup_persist::{check_swaps_with, Extra};
use crate::romfs::Romfs;
use crate::rsizetable::{Rst, TABLE_PATH};
use crate::settings::seed::SeedFile;
use crate::slice::data::Data;
use crate::slice::edit::{build_with, Built};
use crate::slice::key_spots::{resolve, shield, Pick};
use crate::slice::tier_order::{fixed_actors, Verdict};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Free,
    Pot,
    Skull,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Want {
    pub name: String,
    pub pick: Pick,
    pub kind: Kind,
    pub spots: Vec<String>,
}

impl Want {
    pub fn spec(&self) -> Res<(Spec, Layers)> {
        if self.kind == Kind::Skull {
            return Ok((self.pick.kit_spec()?, Layers::default()));
        }
        self.pick.spec()
    }
}

fn kind_of(d: &Data, loc: &str, item: &str) -> Option<Kind> {
    if crate::slice::skulls::is_skull(d, loc) {
        let t: crate::slice::skulls::Table = crate::slice::skulls::table().ok()?;
        let shipped: bool = item == crate::slice::skulls::TOKEN || crate::slice::skulls::pickup(&t, d, item).is_some();
        return (!shipped).then_some(Kind::Skull);
    }
    if d.pots.has(loc) {
        return crate::pot_slice::pickup::of(item).is_none().then_some(Kind::Pot);
    }
    if d.cat.by_name(loc).is_some_and(crate::slice::pickups::is_world) {
        let shipped: bool = crate::slice::pickups::world_actor(item).is_some() || shield(item).is_some();
        return (!shipped).then_some(Kind::Free);
    }
    None
}

pub fn need(d: &Data, seed: &SeedFile, fixed: &BM<String, String>) -> Res<BM<String, Want>> {
    let mut out: BM<String, Want> = BM::new();
    for (loc, item) in &seed.placements {
        if d.vanilla_of(loc) == Some(item.as_str()) {
            continue;
        }
        let Some(k) = kind_of(d, loc, item) else {
            continue;
        };
        let p: Pick = resolve(d, item, fixed.get(loc).map(|s| s.as_str())).map_err(|e| E::Msg(format!("{loc} holds {item}, which this build cannot place there yet: {e}")))?;
        let name: String = if k == Kind::Skull { p.kit() } else { p.pack.clone() };
        let w: &mut Want = out.entry(name.clone()).or_insert_with(|| Want { name, pick: p, kind: k, spots: Vec::new() });
        w.spots.push(loc.clone());
    }
    for w in out.values() {
        w.spec()?;
    }
    Ok(out)
}

fn opts(host: Host, base: Option<PB>, dry: bool) -> Opts {
    Opts { host, base, rst_from: None, places: Vec::new(), dry }
}

fn run_one(romfs: &P, load: &P, mod_name: &str, w: &Want, o: &Opts) -> Res<Done> {
    let (s, l): (Spec, Layers) = w.spec()?;
    if w.kind == Kind::Skull {
        return kit::run(romfs, load, mod_name, &s, o, &[]);
    }
    run_with(romfs, load, mod_name, &s, o, &l)
}

pub fn pool(romfs: &P, host: Host, wants: &BM<String, Want>) -> Res<Extra> {
    let mut out: Extra = Extra::default();
    for w in wants.values() {
        let d: Done = run_one(romfs, P::new("unused"), "unused", w, &opts(host, None, true)).map_err(|e| E::Msg(format!("the pickup pack {} cannot be built: {e}", w.name)))?;
        let x: Extra = guard::extra(&d.files, &d.carried)?;
        out.packs.extend(x.packs);
        out.flags.extend(x.flags);
    }
    Ok(out)
}

fn table(rf: &mut Romfs, base: &P, path: &str) -> Res<Byml> {
    Byml::parse(&rf.zs.decompress(&fs::read(base.join(path))?)?)
}

pub fn audit(rf: &mut Romfs, base: &P, wants: &BM<String, Want>) -> Res<Vec<String>> {
    let x: Extra = Extra::load(rf, base)?;
    let ys: Vec<Byml> = [ACTOR_INFO, GAME_ACTOR_INFO, POUCH_INFO].iter().map(|t| table(rf, base, t)).collect::<Res<Vec<Byml>>>()?;
    let rst: Rst = Rst::parse(&rf.zs.decompress(&fs::read(base.join(TABLE_PATH))?)?)?;
    let mut out: Vec<String> = Vec::new();
    for w in wants.values() {
        let raw: Vec<u8> = rf.zs.decompress(&fs::read(base.join(pack_of(&w.name)))?)?;
        if !x.has(&w.name) || x.flags.get(&w.name) != Some(&true) {
            return Err(E::Msg(format!("audit: {} is not in the mod or its game actor row does not keep it collected", w.name)));
        }
        if ys.iter().any(|y| rows::find(y, &w.name).is_none()) {
            return Err(E::Msg(format!("audit: {} lacks a row in one of the three actor tables", w.name)));
        }
        if rst.get(&pack_of(&w.name)) != Some(sizes::pack_size(raw.len())?) {
            return Err(E::Msg(format!("audit: {} has the wrong size table entry", w.name)));
        }
        out.push(format!("  {} | {} | {}", w.name, w.spots.join(", "), match w.kind {
            Kind::Free => "on the ground",
            Kind::Pot => "pot or crate drop",
            Kind::Skull => "skulltula kit drop",
        }));
    }
    for y in &ys {
        if !rows::sorted(y)? {
            return Err(E::Format("audit: an actor table is no longer sorted"));
        }
    }
    Ok(out)
}

pub fn build(seed: &SeedFile, romfs: &P, load: &P, mod_name: &str, host: Host) -> Res<Built> {
    let d: Data = Data::load()?;
    let order: Verdict = fixed_actors(&d, seed)?;
    let wants: BM<String, Want> = need(&d, seed, &order.assign)?;
    if wants.is_empty() {
        return build_with(seed, romfs, load, mod_name, host, &Extra::default());
    }
    if let Some(b) = order.bad.first() {
        return Err(E::Msg(format!("this seed places key items at pickup spots out of upgrade tier order, generate it again with this build: {b}")));
    }
    let x: Extra = pool(romfs, host, &wants)?;
    let mut rf: Romfs = Romfs::open(romfs)?;
    let pots: Vec<&str> = wants.values().filter(|w| w.kind == Kind::Pot).map(|w| w.name.as_str()).collect();
    check_swaps_with(&mut rf, &pots, &x)?;
    let mut b: Built = build_with(seed, romfs, load, mod_name, host, &x)?;
    let base: PB = b.root.join("romfs");
    for w in wants.values() {
        let done: Done = run_one(romfs, load, mod_name, w, &opts(host, Some(base.clone()), false))?;
        if done.root.as_deref() != Some(b.root.as_path()) {
            return Err(E::Msg(format!("the pickup pack {} went to another folder than the seed mod", w.name)));
        }
    }
    let lines: Vec<String> = audit(&mut rf, &base, &wants)?;
    b.counts.files += wants.len();
    let _ = writeln!(b.report, "{} pickup packs for key items at pickup spots were added to this mod with their actor rows and size table entries, all audited. None of them is tested in the game yet:", wants.len());
    for l in lines {
        let _ = writeln!(b.report, "{l}");
    }
    Ok(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    fn seed(pairs: &[(&str, &str)]) -> SeedFile {
        let m: BM<String, String> = pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        SeedFile::new("s", &Settings::default(), "g", m).unwrap()
    }

    #[test]
    fn a_seed_needs_one_pack_per_new_item_and_a_kit_per_skulltula_drop() {
        let d: Data = Data::load().unwrap();
        let pot: String = d.pots.state.iter().filter(|(n, s)| **s == crate::slice::data::State::Eligible && crate::slice::copies::copies(&d, n) < 2).map(|(n, _)| n.clone()).min().unwrap();
        let free: &str = "LLR Freestanding PoH";
        let skull: &str = "KF GS Bean Patch";
        let s: SeedFile = seed(&[(free, "Boomerang"), (pot.as_str(), "Megaton Hammer"), (skull, "Progressive Hookshot"), ("KF Midos Top Left Chest", "Bow")]);
        let fixed: BM<String, String> = [(skull.to_string(), "Item_Hookshot".to_string())].into_iter().collect();
        let w: BM<String, Want> = need(&d, &s, &fixed).unwrap();
        let names: Vec<&str> = w.keys().map(|k| k.as_str()).collect();
        assert_eq!(names, vec!["Obj_Boomerang", "Obj_Hammer", "Obj_Hookshot_GS"]);
        assert_eq!(w["Obj_Boomerang"].kind, Kind::Free);
        assert_eq!(w["Obj_Hammer"].kind, Kind::Pot);
        assert_eq!(w["Obj_Hookshot_GS"].spec().unwrap().0.name, "Obj_Hookshot_GS");
        assert!(need(&d, &s, &BM::new()).is_err());
        let plain: SeedFile = seed(&[(free, "Rupees (5)"), (pot.as_str(), "Rupees (20)"), ("KF Midos Top Left Chest", "Bow")]);
        assert!(need(&d, &plain, &BM::new()).unwrap().is_empty());
        let bow: SeedFile = seed(&[(skull, "Bow")]);
        assert!(need(&d, &bow, &[(skull.to_string(), "Item_Range_Bow".to_string())].into_iter().collect()).is_err());
        let two: SeedFile = seed(&[(free, "Megaton Hammer"), (pot.as_str(), "Megaton Hammer")]);
        assert_eq!(need(&d, &two, &BM::new()).unwrap()["Obj_Hammer"].spots.len(), 2);
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

    fn wide() -> Settings {
        let mut s: Settings = Settings::default();
        for (k, v) in [("shuffle_chests", "all"), ("shuffle_freestanding_items", "all"), ("shuffle_pots", "all"), ("shuffle_crates", "all"), ("shuffle_skulltulas", "matched")] {
            s.set(k, crate::settings::Value::s(v)).unwrap();
        }
        s
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum At {
        Free,
        Pot,
        Crate,
        Skull,
    }

    fn at_of(d: &Data, name: &str) -> Option<At> {
        if crate::slice::skulls::is_skull(d, name) {
            return Some(At::Skull);
        }
        if let Some(l) = d.pots.get(name) {
            return Some(if l.kind == "crate" { At::Crate } else { At::Pot });
        }
        d.cat.by_name(name).is_some_and(crate::slice::pickups::is_world).then_some(At::Free)
    }

    fn moved(w: &crate::world::World, pl: &crate::world::Placement, it: crate::logic_items::ItemId, l: crate::world::LocId) -> Option<crate::world::Placement> {
        let from: crate::world::LocId = w.real.iter().copied().find(|x| pl.get(*x) == Some(it))?;
        let mut next: crate::world::Placement = pl.clone();
        let was: crate::logic_items::ItemId = next.get(l)?;
        next.put(l, it);
        next.put(from, was);
        crate::search::explore(w, &next, &[]).won().then_some(next)
    }

    fn plando_for(d: &Data, s: &Settings, want: &[(&str, &str)]) -> Vec<String> {
        use crate::slice::pick::{pick, Pick as Chosen};
        use crate::slice::support::{plan, Plan};
        use crate::world::{LocId, World};
        let p: Plan = plan(s).unwrap();
        let w: World = crate::slice::logic::world_with(d, &p.logic()).unwrap();
        let pk: Chosen = pick(d, &w, &p);
        let (_, items): (Vec<String>, Vec<String>) = crate::slice::generate::plando_options(d, s).unwrap();
        let mut locs: Vec<LocId> = pk.free.clone();
        locs.sort_by(|a, b| w.locs[*a].name.cmp(&w.locs[*b].name));
        let mut used: Vec<LocId> = Vec::new();
        let mut out: Vec<String> = Vec::new();
        let mut pl: crate::world::Placement = crate::slice::fill::vanilla_place(&w, d).unwrap();
        for (item, class) in want {
            if !items.iter().any(|i| i == item) {
                continue;
            }
            let it = w.item(item).unwrap();
            let c: At = match *class {
                "free" => At::Free,
                "pot" => At::Pot,
                "crate" => At::Crate,
                _ => At::Skull,
            };
            let hit: Option<LocId> = locs.iter().copied().find(|l| {
                let n: &str = &w.locs[*l].name;
                !used.contains(l) && at_of(d, n) == Some(c) && d.vanilla_of(n) != Some(*item) && crate::slice::generate::allowed(d, &w, &p, it, *l) && moved(&w, &pl, it, *l).is_some()
            });
            if let Some(l) = hit {
                pl = moved(&w, &pl, it, l).unwrap();
                used.push(l);
                out.push(format!("{}={item}", w.locs[l].name));
            }
        }
        out
    }

    #[test]
    fn key_items_at_every_spot_class_generate_and_build_into_one_mod_with_their_packs() {
        use crate::slice::generate::{generate_data, GenOpts, Generated};
        let Some(root) = real() else {
            return;
        };
        let d: Data = Data::load().unwrap();
        let s: Settings = wide();
        let want: [(&str, &str); 5] = [("Boomerang", "free"), ("Mirror Shield", "pot"), ("Megaton Hammer", "crate"), ("Lens of Truth", "skull"), ("Iron Boots", "skull")];
        let list: Vec<String> = plando_for(&d, &s, &want);
        assert!(list.len() >= 4, "{list:?}");
        let o: GenOpts = GenOpts { spoiler: true, plando: list.iter().map(|x| crate::slice::plando::parse(x).unwrap()).collect(), ..GenOpts::default() };
        let g: Generated = generate_data(&d, &s, "wp4 key spots", &o).unwrap();
        let again: Generated = generate_data(&d, &s, "wp4 key spots", &o).unwrap();
        assert_eq!(g.seed_file.to_text(), again.seed_file.to_text());
        let sp: String = g.spoiler.clone().unwrap();
        assert!(sp.contains("Key items at pickup spots"), "{sp}");
        for x in &list {
            let (l, i) = x.split_once('=').unwrap();
            assert_eq!(g.seed_file.placements.get(l).map(|v| v.as_str()), Some(i));
            assert!(sp.contains(&format!("  {l}: {i} as ")), "{l}");
        }
        let load: PB = std::env::temp_dir().join(format!("ootrr_seed_packs_{}", std::process::id()));
        let _ = fs::remove_dir_all(&load);
        let b: Built = crate::slice::edit::build_for(&g.seed_file, &root, &load, "wp4 keys", Host::Hookshot).unwrap();
        assert!(b.report.contains("pickup packs for key items"), "{}", b.report);
        let romfs: PB = b.root.join("romfs");
        let order: Verdict = fixed_actors(&d, &g.seed_file).unwrap();
        let wants: BM<String, Want> = need(&d, &g.seed_file, &order.assign).unwrap();
        assert!(wants.len() >= list.len().min(4), "{:?}", wants.keys());
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        assert_eq!(audit(&mut rf, &romfs, &wants).unwrap().len(), wants.len());
        let token: Vec<crate::sarc::Entry> = kit::token_pack(&mut rf).unwrap();
        for w in wants.values() {
            assert!(romfs.join(pack_of(&w.name)).is_file(), "{}", w.name);
            if w.kind == Kind::Skull {
                let es: Vec<crate::sarc::Entry> = crate::sarc::parse(&rf.zs.decompress(&fs::read(romfs.join(pack_of(&w.name))).unwrap()).unwrap()).unwrap();
                assert_eq!(kit::verify(&es, &w.name, &token).unwrap().len(), 3, "{}", w.name);
            }
        }
        let first: Vec<(String, Vec<u8>)> = files(&romfs);
        let b2: Built = crate::slice::edit::build_for(&g.seed_file, &root, &load, "wp4 keys", Host::Hookshot).unwrap();
        assert_eq!(files(&b2.root.join("romfs")), first);
        let _ = fs::remove_dir_all(&load);
    }

    #[test]
    fn tier_items_at_pickup_spots_get_the_tier_the_order_names_and_their_packs() {
        use crate::slice::generate::{generate_data, GenOpts, Generated};
        let Some(root) = real() else {
            return;
        };
        let d: Data = Data::load().unwrap();
        let s: Settings = wide();
        let want: [(&str, &str); 3] = [("Progressive Hookshot", "free"), ("Bow", "pot"), ("Bomb Bag", "skull")];
        let list: Vec<String> = plando_for(&d, &s, &want);
        assert_eq!(list.len(), 3, "{list:?}");
        let o: GenOpts = GenOpts { spoiler: true, plando: list.iter().map(|x| crate::slice::plando::parse(x).unwrap()).collect(), ..GenOpts::default() };
        let g: Generated = generate_data(&d, &s, "wp4 tier spots", &o).unwrap();
        let order: Verdict = fixed_actors(&d, &g.seed_file).unwrap();
        assert!(order.bad.is_empty(), "{:?}", order.bad);
        let wants: BM<String, Want> = need(&d, &g.seed_file, &order.assign).unwrap();
        for x in &list {
            let (l, _) = x.split_once('=').unwrap();
            let tier: &str = order.assign.get(l).map(|t| t.as_str()).unwrap();
            assert!(wants.values().any(|w| w.spots.iter().any(|s| s == l) && w.pick.content == tier), "{l} {tier}");
        }
        assert!(wants.values().any(|w| w.kind == Kind::Skull && w.name.starts_with("Obj_BombBag_")));
        for w in wants.values().filter(|w| w.pick.content == "Item_Range_Bow") {
            assert!(w.spec().unwrap().1.ammo, "{}", w.name);
        }
        let load: PB = std::env::temp_dir().join(format!("ootrr_seed_packs_tier_{}", std::process::id()));
        let _ = fs::remove_dir_all(&load);
        let b: Built = crate::slice::edit::build_for(&g.seed_file, &root, &load, "wp4 tiers", Host::Hookshot).unwrap();
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        assert_eq!(audit(&mut rf, &b.root.join("romfs"), &wants).unwrap().len(), wants.len());
        let _ = fs::remove_dir_all(&load);
    }

    fn files(root: &P) -> Vec<(String, Vec<u8>)> {
        let mut out: Vec<(String, Vec<u8>)> = Vec::new();
        let mut stack: Vec<PB> = vec![root.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in fs::read_dir(&d).unwrap() {
                let p: PB = e.unwrap().path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    out.push((p.strip_prefix(root).unwrap().to_string_lossy().to_string(), fs::read(&p).unwrap()));
                }
            }
        }
        out.sort();
        out
    }

    #[test]
    fn a_seed_with_no_new_item_builds_the_same_mod_as_before() {
        use crate::slice::generate::{generate_data, GenOpts, Generated};
        let Some(root) = real() else {
            return;
        };
        let d: Data = Data::load().unwrap();
        let g: Generated = generate_data(&d, &Settings::default(), "wp4 plain", &GenOpts::default()).unwrap();
        assert!(need(&d, &g.seed_file, &BM::new()).unwrap().is_empty());
        let load: PB = std::env::temp_dir().join(format!("ootrr_seed_packs_plain_{}", std::process::id()));
        let _ = fs::remove_dir_all(&load);
        let a: Built = crate::slice::edit::build_for(&g.seed_file, &root, &load.join("a"), "m", Host::Hookshot).unwrap();
        let b: Built = build_with(&g.seed_file, &root, &load.join("b"), "m", Host::Hookshot, &Extra::default()).unwrap();
        assert_eq!(files(&a.root.join("romfs")), files(&b.root.join("romfs")));
        assert!(!a.report.contains("pickup packs for key items"));
        let _ = fs::remove_dir_all(&load);
    }
}
