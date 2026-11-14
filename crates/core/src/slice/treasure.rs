use crate::byml::{Byml, Node};
use crate::error::Res;
use crate::placement::hash_of;
use crate::settings::Settings;
use crate::slice::banc::set_drop_ev;
use crate::slice::data::{Data, Ev};

pub const SETTING: &str = "shuffle_treasure_reward";

pub const LOC: &str = "Market Treasure Chest Game Reward";

pub const FIRST: u64 = 16881481538337649070;

pub const REPEAT: u64 = 6781536951709667179;

pub const VIEW: &str = "DropViewActor";

pub const VANILLA: &str = "Obj_Heart_Piece";

pub const NOTE: &str = "The Treasure Chest Game reward is shuffled. The first win gives the shuffled item and every later win gives the vanilla red rupee. Only unique items whose second copy changes nothing can be placed there.";

pub const ITEMS: [&str; 17] = [
    "Boomerang",
    "Lens of Truth",
    "Megaton Hammer",
    "Iron Boots",
    "Hover Boots",
    "Mirror Shield",
    "Stone of Agony",
    "Gerudo Membership Card",
    "Fire Arrows",
    "Ice Arrows",
    "Light Arrows",
    "Dins Fire",
    "Farores Wind",
    "Nayrus Love",
    "Piece of Heart",
    "Piece of Heart (Out of Logic)",
    "Piece of Heart (Treasure Chest Game)",
];

const VIEWS: [(&str, &str); 1] = [("Obj_Heart_Piece", "Obj_Heart_Piece_MiniGame")];

pub fn mode(s: &Settings, bad: &mut Vec<String>) -> bool {
    s.flag(SETTING).unwrap_or_else(|e| {
        bad.push(e.to_string());
        false
    })
}

pub fn opens(on: bool, name: &str) -> bool {
    on && name == LOC
}

pub fn at(on: bool, name: &str, item: &str) -> Option<bool> {
    if !on || name != LOC {
        return None;
    }
    Some(ITEMS.contains(&item))
}

pub fn with(mut d: Data, on: bool) -> Data {
    if on {
        d.events.keep.insert(FIRST);
    }
    d
}

pub fn view_of(actor: &str) -> String {
    match VIEWS.iter().find(|(a, _)| *a == actor) {
        Some((_, v)) => v.to_string(),
        None => actor.to_string(),
    }
}

pub fn set_view(y: &mut Byml, hash: u64, actor: &str) -> usize {
    if hash != FIRST {
        return 0;
    }
    let Some(list) = y.root.get_mut("Actors").and_then(|a| a.as_arr_mut()) else {
        return 0;
    };
    let mut n: usize = 0;
    for a in list.iter_mut() {
        if hash_of(a) != Some(hash) {
            continue;
        }
        let Some(d) = a.get_mut("Dynamic") else {
            continue;
        };
        if d.get(VIEW).is_none() {
            continue;
        }
        d.set(VIEW, Node::Str(view_of(actor)));
        n += 1;
    }
    n
}

pub fn test_edit(y: &mut Byml, actor: &str, view: bool) -> Res<usize> {
    let n: usize = set_drop_ev(y, FIRST, VANILLA, actor, Some(Ev::Keep))?;
    if view {
        set_view(y, FIRST, actor);
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap as BM;
    use std::path::PathBuf as PB;

    use super::*;
    use crate::json::{self, Json};
    use crate::romfs::Romfs;
    use crate::search::explore;
    use crate::settings::Value;
    use crate::settings::seed::SeedFile;
    use crate::slice::data::{State, EVENTS_JSON};
    use crate::slice::edit::{build_with, edit_file, Edited, Target};
    use crate::slice::fill::vanilla_place;
    use crate::slice::generate::{allowed, generate_data, GenOpts, Generated};
    use crate::slice::logic::world;
    use crate::slice::pick::pick;
    use crate::slice::support::{plan, Plan};
    use crate::world::{LocId, Placement, World};

    const FILE: &str = "Banc/Market/Market_Child_Static.bcett.byml.zs";

    fn on() -> Settings {
        let mut s: Settings = Settings::default();
        s.set(SETTING, Value::Bool(true)).unwrap();
        s
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

    fn dynamic(y: &Byml, hash: u64) -> Node {
        let list: &[Node] = y.root.get("Actors").and_then(|a| a.as_arr()).unwrap_or(&[]);
        list.iter().find(|a| hash_of(a) == Some(hash)).and_then(|a| a.get("Dynamic")).cloned().unwrap()
    }

    fn text(n: &Node, k: &str) -> String {
        n.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
    }

    fn target(new: &str) -> Target {
        Target {
            name: LOC.to_string(),
            hash: FIRST,
            files: vec![FILE.to_string()],
            old_item: "Piece of Heart (Treasure Chest Game)".to_string(),
            new_item: "Megaton Hammer".to_string(),
            old_actor: "Obj_Heart_Piece".to_string(),
            new_actor: new.to_string(),
            ev: Some(Ev::Keep),
            family: None,
            key: None,
        }
    }

    #[test]
    fn the_event_data_matches_the_two_stacked_reward_chests() {
        let j: Json = json::parse(EVENTS_JSON).unwrap();
        let rows: &[Json] = j.get("chests").and_then(|v| v.as_arr()).unwrap();
        let row = |h: u64| rows.iter().find(|r| r.get("hash").and_then(|v| v.as_str()) == Some(h.to_string().as_str())).unwrap();
        let a: &Json = row(FIRST);
        let b: &Json = row(REPEAT);
        assert_eq!(a.get("location").and_then(|v| v.as_str()), Some(LOC));
        assert_eq!(a.get("drop").and_then(|v| v.as_str()), Some("Obj_Heart_Piece"));
        assert_eq!(b.get("drop").and_then(|v| v.as_str()), Some("TBox_Rupee_Red"));
        assert_eq!(a.get("translate"), b.get("translate"));
        for r in [a, b] {
            assert_eq!(r.get("event").and_then(|v| v.as_str()), Some("MiniGame_ChoiceTreasure_OpenClearTreasureBox"));
            assert_eq!(r.get("verdict").and_then(|v| v.as_str()), Some("keep_vanilla"));
        }
        assert_eq!(a.get("dynamic_extra").and_then(|v| v.get(VIEW)).and_then(|v| v.as_str()), Some("Obj_Heart_Piece_MiniGame"));
        let d: Data = Data::load().unwrap();
        assert_eq!(d.cat.by_name(LOC).map(|l| l.refs.iter().map(|r| r.hash).collect::<Vec<u64>>()), Some(vec![FIRST]));
        assert_eq!(d.state_of(LOC), Some(State::Event));
        assert!(!d.event_ok(LOC));
        assert!(with(Data::load().unwrap(), true).event_ok(LOC));
        assert!(!with(Data::load().unwrap(), false).event_ok(LOC));
    }

    #[test]
    fn the_allowed_items_are_unique_untiered_and_exist() {
        let d: Data = Data::load().unwrap();
        for n in ITEMS {
            let i: &crate::slice::data::Item = d.item(n).unwrap_or_else(|| panic!("{n}"));
            assert!(i.tiers.is_empty() && d.family(n).is_none(), "{n}");
            assert_eq!(i.kind, "Item", "{n}");
            let a: &str = i.actor.as_deref().unwrap();
            assert!(a.starts_with("Item_") || a == "Obj_Heart_Piece", "{n} {a}");
        }
        assert!(ITEMS.contains(&d.vanilla_of(LOC).unwrap()));
        for n in ["Progressive Hookshot", "Bottle", "Rupees (200)", "Bombchus (10)", "Small Key (Treasure Chest Game)", "Pocket Egg", "Deku Nuts (5)", "Heart Container"] {
            assert!(!ITEMS.contains(&n), "{n}");
        }
        assert_eq!(view_of("Obj_Heart_Piece"), "Obj_Heart_Piece_MiniGame");
        assert_eq!(view_of("Item_Hammer"), "Item_Hammer");
    }

    #[test]
    fn the_logic_needs_the_lens_and_the_vanilla_world_reaches_the_reward() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let l: LocId = w.loc(LOC).unwrap();
        let mut p: Placement = vanilla_place(&w, &d).unwrap();
        assert!(explore(&w, &p, &[]).loc_access(l));
        let lens: LocId = (0..w.locs.len()).find(|x| p.get(*x).is_some_and(|i| w.items.names[i] == "Lens of Truth")).unwrap();
        p.put(lens, w.item("Rupees (20)").unwrap());
        assert!(!explore(&w, &p, &[]).loc_access(l));
    }

    #[test]
    fn off_keeps_the_reward_vanilla_and_on_frees_only_the_first_win_chest() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let off: Plan = plan(&Settings::default()).unwrap();
        assert!(!off.treasure);
        assert!(!pick(&d, &w, &off).free.contains(&w.loc(LOC).unwrap()));
        let p: Plan = plan(&on()).unwrap();
        assert!(p.treasure);
        assert!(p.notes.iter().any(|n| n == NOTE));
        let free: Vec<LocId> = pick(&d, &w, &p).free;
        assert!(free.contains(&w.loc(LOC).unwrap()));
        let market: Vec<&str> = free.iter().map(|l| w.locs[*l].name.as_str()).filter(|n| n.starts_with("Market Treasure Chest Game")).collect();
        assert_eq!(market, vec![LOC]);
        let mut s: Settings = on();
        s.set("shuffle_chests", Value::s("dungeons")).unwrap();
        assert!(!pick(&d, &w, &plan(&s).unwrap()).free.contains(&w.loc(LOC).unwrap()));
    }

    #[test]
    fn seeds_with_the_setting_put_only_allowed_items_on_the_reward() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let s: Settings = on();
        let p: Plan = plan(&s).unwrap();
        let mut seen: BM<String, usize> = BM::new();
        for n in 0..12 {
            let g: Generated = generate_data(&d, &s, &format!("treasure {n}"), &GenOpts::default()).unwrap();
            let it: &str = &g.seed_file.placements[LOC];
            assert!(ITEMS.contains(&it), "{it}");
            assert!(allowed(&d, &w, &p, w.item(it).unwrap(), w.loc(LOC).unwrap()));
            *seen.entry(it.to_string()).or_insert(0) += 1;
            assert!(g.report.contains(NOTE));
        }
        assert!(seen.len() > 1, "{seen:?}");
        let l: LocId = w.loc(LOC).unwrap();
        for bad in ["Progressive Hookshot", "Bottle", "Rupees (200)", "Bomb Bag", "Deku Nuts (5)"] {
            assert!(!allowed(&d, &w, &p, w.item(bad).unwrap(), l), "{bad}");
        }
        let off: Generated = generate_data(&d, &Settings::default(), "treasure off", &GenOpts::default()).unwrap();
        assert!(!off.seed_file.placements.contains_key(LOC));
    }

    #[test]
    fn a_plando_hookshot_is_refused_and_a_hammer_is_honoured() {
        let d: Data = Data::load().unwrap();
        let go = |x: &str| GenOpts { plando: vec![crate::slice::plando::parse(x).unwrap()], ..GenOpts::default() };
        let e: String = generate_data(&d, &on(), "p", &go(&format!("{LOC}=Progressive Hookshot"))).err().unwrap().to_string();
        assert!(e.contains("cannot be placed"), "{e}");
        let g: Generated = generate_data(&d, &on(), "p", &go(&format!("{LOC}=Megaton Hammer"))).unwrap();
        assert_eq!(g.seed_file.placements[LOC], "Megaton Hammer");
    }

    #[test]
    fn the_dump_edit_changes_only_the_first_win_drop_and_its_preview() {
        let Some(root) = real() else { return };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read_zs(FILE).unwrap();
        let before: Byml = Byml::parse(&raw).unwrap();
        let first: Node = dynamic(&before, FIRST);
        let repeat: Node = dynamic(&before, REPEAT);
        assert_eq!(text(&first, "Drop__DropActor"), "Obj_Heart_Piece");
        assert_eq!(text(&first, VIEW), "Obj_Heart_Piece_MiniGame");
        assert_eq!(text(&repeat, "Drop__DropActor"), "TBox_Rupee_Red");
        assert_eq!(text(&repeat, VIEW), "Obj_OrnamentalRupee_Red_MiniGame");
        assert!(edit_file(FILE, &raw, &[Target { ev: None, ..target("Item_Hammer") }]).is_err());
        let e: Edited = edit_file(FILE, &raw, &[target("Item_Hammer")]).unwrap().unwrap();
        assert_eq!(e.hits.len(), 1);
        let after: Byml = Byml::parse(&e.bytes).unwrap();
        let a: Node = dynamic(&after, FIRST);
        assert_eq!(text(&a, "Drop__DropActor"), "Item_Hammer");
        assert_eq!(text(&a, VIEW), "Item_Hammer");
        assert_eq!(text(&a, "GetEventName"), "MiniGame_ChoiceTreasure_OpenClearTreasureBox");
        assert_eq!(a.get("IsEnableRepeatOpen"), first.get("IsEnableRepeatOpen"));
        assert_eq!(dynamic(&after, REPEAT), repeat);
        let old: &[Node] = before.root.get("Actors").and_then(|x| x.as_arr()).unwrap();
        let new: &[Node] = after.root.get("Actors").and_then(|x| x.as_arr()).unwrap();
        assert_eq!(old.len(), new.len());
        let diff: Vec<Option<u64>> = old.iter().zip(new).filter(|(x, y)| x != y).map(|(x, _)| hash_of(x)).collect();
        assert_eq!(diff, vec![Some(FIRST)]);
        let mut keys: Vec<&str> = match (&first, &a) {
            (Node::Map(x), Node::Map(y)) => x.iter().zip(y).filter(|(p, q)| p != q).map(|(p, _)| p.0.as_str()).collect(),
            _ => Vec::new(),
        };
        keys.sort();
        assert_eq!(keys, vec![VIEW, "Drop__DropActor"]);
        let mut c: Byml = before.clone();
        c.root.set("Actors", Node::Arr(new.to_vec()));
        assert_eq!(c, after);
        let mut back: Byml = after.clone();
        assert_eq!(set_view(&mut back, REPEAT, "Item_Hammer"), 0);
        assert_eq!(set_view(&mut back, FIRST, "Obj_Heart_Piece"), 1);
        assert_eq!(text(&dynamic(&back, FIRST), VIEW), "Obj_Heart_Piece_MiniGame");
        let mut m: Byml = before.clone();
        assert_eq!(test_edit(&mut m, "Item_Hammer", true).unwrap(), 1);
        assert_eq!(m, after);
        let mut k: Byml = before.clone();
        assert_eq!(test_edit(&mut k, "TBox_Rupee_Gold", false).unwrap(), 1);
        assert_eq!(text(&dynamic(&k, FIRST), "Drop__DropActor"), "TBox_Rupee_Gold");
        assert_eq!(text(&dynamic(&k, FIRST), VIEW), "Obj_Heart_Piece_MiniGame");
        assert_eq!(dynamic(&k, REPEAT), repeat);
    }

    #[test]
    fn a_generated_seed_builds_with_the_hammer_in_the_first_win_chest() {
        let Some(root) = real() else { return };
        let d: Data = Data::load().unwrap();
        let o: GenOpts = GenOpts { plando: vec![crate::slice::plando::parse(&format!("{LOC}=Megaton Hammer")).unwrap()], ..GenOpts::default() };
        let g: Generated = generate_data(&d, &on(), "treasure build", &o).unwrap();
        let seed: SeedFile = g.seed_file;
        let out: PB = std::env::temp_dir().join(format!("ootrr_treasure_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        let b: crate::slice::edit::Built = build_with(&seed, &root, &out, "treasure", crate::modpack::Host::Eden, &crate::pickup_persist::Extra::default()).unwrap();
        assert!(b.chests.iter().any(|c| c.name == LOC && c.new_actor == "Item_Hammer"), "{}", b.report);
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let f: PB = b.root.join("romfs").join(FILE.replace('/', std::path::MAIN_SEPARATOR_STR));
        let z: Vec<u8> = std::fs::read(&f).unwrap();
        let y: Byml = Byml::parse(&rf.zs.decompress(&z).unwrap()).unwrap();
        let a: Node = dynamic(&y, FIRST);
        assert_eq!(text(&a, "Drop__DropActor"), "Item_Hammer");
        assert_eq!(text(&a, VIEW), "Item_Hammer");
        assert_eq!(text(&dynamic(&y, REPEAT), "Drop__DropActor"), "TBox_Rupee_Red");
        assert_eq!(text(&dynamic(&y, REPEAT), VIEW), "Obj_OrnamentalRupee_Red_MiniGame");
        let _ = std::fs::remove_dir_all(&out);
    }
}
