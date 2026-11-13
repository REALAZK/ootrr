use std::collections::{BTreeMap as BM, HashMap as HM};
use std::fmt::Write as W;
use std::sync::{Mutex, OnceLock};

use crate::error::{Error as E, Res};
use crate::pickup_pack::catalog::{self, Entry, Status};
use crate::pickup_pack::kit::name_of;
use crate::pickup_pack::{default_name, Layers, Spec};
use crate::progressive::{family_of, tiers};
use crate::slice::data::{Data, Item};
use crate::world::{LocId, Placement, World};

pub const NEVER: [&str; 2] = ["Milk", "Buy Magic Bean"];
pub const TOP_ONLY: [&str; 2] = ["NutBag", "StickBag"];
pub const REFUSED: [&str; 2] = ["in_game_first", "not_for_placement"];
pub const AMMO: &str = "--ammo";
pub const SHIELDS: [(&str, &str); 2] = [("Deku Shield", "Obj_Shield_Deku"), ("Hylian Shield", "Obj_Shield_Hylian")];
pub const LOST: &str = "Key items in pots and flying pots can be lost: the pot is saved as broken when it breaks, so an item that is not picked up, falls into a pit or is left behind on a room change is gone for good. Crates break the same way.";
pub const NOTE: &str = "Key items can sit at shuffled pickup spots. Every item whose pickup pack is built can lie on the ground, in a pot, a crate or at a Gold Skulltula, the seed build writes those packs into the mod. Upgrade tiers at these spots always go low to high in the order the player can reach them, and nut and stick capacity there is always the top tier. A spot that exists once per age only takes items that can be found twice. Experimental, untested in the game.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Spot {
    Free,
    Pot,
    Skull,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pick {
    pub content: String,
    pub pack: String,
    pub args: Vec<String>,
}

impl Pick {
    pub fn spec(&self) -> Res<(Spec, Layers)> {
        catalog::spec(&self.content, &self.args)
    }

    pub fn kit(&self) -> String {
        name_of(&self.pack)
    }

    pub fn kit_spec(&self) -> Res<Spec> {
        let (s, l): (Spec, Layers) = self.spec()?;
        if !l.plain() {
            return Err(E::Msg(format!("{} needs a pack layer, a skulltula kit cannot carry it", self.pack)));
        }
        Ok(Spec { name: self.kit(), ..s })
    }
}

fn entries() -> &'static [Entry] {
    static C: OnceLock<Vec<Entry>> = OnceLock::new();
    C.get_or_init(|| catalog::load().unwrap_or_default())
}

pub fn chosen(e: &Entry) -> Option<Vec<String>> {
    if e.flag.as_deref().is_some_and(|f| REFUSED.contains(&f)) || !e.item.starts_with("Item_") {
        return None;
    }
    let ammo: Option<&catalog::Variant> = e.variants.iter().find(|v| v.status == Status::Built && v.args.iter().any(|a| a == AMMO));
    match ammo {
        Some(v) => Some(v.args.clone()),
        None if e.status == Status::Built => Some(e.args.clone()),
        None => None,
    }
}

pub fn pick_in(list: &[Entry], content: &str) -> Option<Pick> {
    let e: &Entry = list.iter().find(|e| e.item == content)?;
    let args: Vec<String> = chosen(e)?;
    let (s, _): (Spec, Layers) = catalog::spec(content, &args).ok()?;
    let named: bool = args.iter().any(|a| a == "--name");
    if Some(&s.name) != e.pack.as_ref() || (!named && s.name != default_name(content)) {
        return None;
    }
    Some(Pick { content: content.to_string(), pack: s.name, args })
}

fn picks() -> &'static BM<String, Pick> {
    static M: OnceLock<BM<String, Pick>> = OnceLock::new();
    M.get_or_init(|| entries().iter().filter_map(|e| pick_in(entries(), &e.item)).map(|p| (p.content.clone(), p)).collect())
}

pub fn pick(content: &str) -> Option<Pick> {
    picks().get(content).cloned()
}

fn family(i: &Item) -> Option<&'static str> {
    family_of(&i.tiers)
}

pub fn contents(d: &Data, item: &str) -> Vec<String> {
    let Some(i) = d.item(item) else {
        return Vec::new();
    };
    if NEVER.contains(&item) || i.kind != "Item" || i.dungeon.is_some() {
        return Vec::new();
    }
    match family(i) {
        Some(f) if TOP_ONLY.contains(&f) => i.tiers.last().cloned().into_iter().collect(),
        Some(_) => i.tiers.clone(),
        None => i.actor.clone().into_iter().collect(),
    }
}

pub fn placeable(d: &Data, item: &str, at: Spot) -> bool {
    static M: OnceLock<Mutex<HM<(String, Spot), bool>>> = OnceLock::new();
    let m: &Mutex<HM<(String, Spot), bool>> = M.get_or_init(|| Mutex::new(HM::new()));
    let k: (String, Spot) = (item.to_string(), at);
    if let Some(v) = m.lock().ok().and_then(|g| g.get(&k).copied()) {
        return v;
    }
    let v: bool = judge(d, item, at);
    if let Ok(mut g) = m.lock() {
        g.insert(k, v);
    }
    v
}

fn judge(d: &Data, item: &str, at: Spot) -> bool {
    let cs: Vec<String> = contents(d, item);
    !cs.is_empty()
        && cs.iter().all(|c| match pick(c) {
            Some(p) if at == Spot::Skull => p.kit_spec().is_ok(),
            Some(_) => true,
            None => false,
        })
}

pub fn shield(item: &str) -> Option<&'static str> {
    SHIELDS.iter().find(|s| s.0 == item).map(|s| s.1)
}

pub fn top_only(d: &Data, item: &str) -> bool {
    d.family(item).is_some_and(|f| TOP_ONLY.contains(&f))
}

pub fn resolve(d: &Data, item: &str, tier: Option<&str>) -> Res<Pick> {
    let i: &Item = d.item(item).ok_or_else(|| E::Msg(format!("unknown item {item}")))?;
    let content: String = match (tier, family(i)) {
        (Some(t), _) => t.to_string(),
        (None, Some(f)) if TOP_ONLY.contains(&f) => i.tiers.last().cloned().unwrap_or_default(),
        (None, Some(_)) => return Err(E::Msg(format!("{item} is an upgrade, the tier order names no tier for its spot"))),
        (None, None) => i.actor.clone().unwrap_or_default(),
    };
    if !contents(d, item).contains(&content) {
        return Err(E::Msg(format!("{item} as {content} has no pickup pack this build can place yet")));
    }
    pick(&content).ok_or_else(|| E::Msg(format!("{item} has no pickup pack this build can place yet ({content})")))
}

pub fn top(f: &str) -> Option<&'static str> {
    tiers(f).ok().and_then(|t| t.last().copied())
}

pub fn widened(d: &Data, name: &str, item: &str) -> bool {
    if crate::slice::skulls::is_skull(d, name) {
        return !crate::slice::skulls::shared().is_some_and(|t| crate::slice::skulls::pickup(t, d, item).is_some());
    }
    if d.pots.has(name) {
        return crate::pot_slice::pickup::of(item).is_none();
    }
    d.cat.by_name(name).is_some_and(crate::slice::pickups::is_world) && crate::slice::pickups::world_actor(item).is_none()
}

pub fn class(d: &Data, name: &str) -> Option<&'static str> {
    if crate::slice::skulls::is_skull(d, name) {
        return Some("Gold Skulltula drop, a kit pickup");
    }
    if let Some(l) = d.pots.get(name) {
        return Some(match l.kind.as_str() {
            "crate" => "crate, given once",
            _ => "pot, can be lost if not picked up",
        });
    }
    if d.cat.by_name(name).is_some_and(crate::slice::pickups::is_world) {
        return Some("on the ground, given once");
    }
    None
}

pub fn spoiler(d: &Data, w: &World, free: &[LocId], got: &Placement, assign: &BM<String, String>) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut pots: bool = false;
    for l in free {
        let name: &str = &w.locs[*l].name;
        let Some(c) = class(d, name) else {
            continue;
        };
        let Some(i) = got.get(*l) else {
            continue;
        };
        let item: &str = &w.items.names[i];
        if d.vanilla_of(name) == Some(item) {
            continue;
        }
        let Ok(p) = resolve(d, item, assign.get(name).map(|s| s.as_str())) else {
            continue;
        };
        let actor: String = if crate::slice::skulls::is_skull(d, name) { p.kit() } else { p.pack.clone() };
        pots |= c.starts_with("pot");
        lines.push(format!("  {name}: {item} as {actor} ({c})"));
    }
    let mut out: String = String::new();
    if lines.is_empty() {
        return out;
    }
    let _ = writeln!(out, "Key items at pickup spots, {}", lines.len());
    for x in lines {
        let _ = writeln!(out, "{x}");
    }
    if pots {
        let _ = writeln!(out, "{LOST}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d() -> Data {
        Data::load().unwrap()
    }

    #[test]
    fn the_catalog_names_a_pack_for_each_built_key_item_and_bow_takes_the_ammo_layer() {
        let hook: Pick = pick("Item_Hookshot").unwrap();
        assert_eq!(hook.pack, "Obj_Hookshot");
        assert_eq!(pick("Item_Longshot").unwrap().pack, "Obj_Longshot");
        let bow: Pick = pick("Item_Range_Bow").unwrap();
        assert!(bow.args.iter().any(|a| a == AMMO), "{:?}", bow.args);
        assert!(bow.spec().unwrap().1.ammo);
        assert!(pick("Item_Range_SlingShot").unwrap().spec().unwrap().1.ammo);
        assert_eq!(pick("Item_Bottle_Letter").unwrap().pack, "Obj_Bottle_LetterLand");
        assert!(pick("Item_Gauntlet_Goron").is_none());
        assert!(pick("Item_NutBag_Medium").is_none());
        assert!(pick("Item_NutBag_Large").is_some());
        assert!(pick("Item_Bombchu").is_none());
        assert!(pick("Item_BombchuBundle_Small").is_none());
        assert!(pick("Item_Shield_Deku").is_none());
        assert!(pick("Item_Ocarina_Fairy").is_none());
        assert!(pick("Item_Nothing").is_none());
    }

    #[test]
    fn every_pick_maps_item_to_obj_unless_the_catalog_renames_it() {
        let mut n: usize = 0;
        for e in catalog::load().unwrap() {
            let Some(p) = pick(&e.item) else {
                continue;
            };
            n += 1;
            assert_eq!(Some(&p.pack), e.pack.as_ref(), "{}", e.item);
            if !p.args.iter().any(|a| a == "--name") {
                assert_eq!(p.pack, default_name(&e.item), "{}", e.item);
            }
            assert_eq!(p.kit(), format!("{}_GS", p.pack));
        }
        assert!(n >= 50, "{n}");
    }

    #[test]
    fn a_flagged_or_unbuilt_entry_is_refused_and_a_renamed_pack_must_say_so() {
        let one = |status: &str, flag: &str, pack: &str, args: &str| {
            let t: String = format!(r#"{{"packs":[{{"group":"A","item":"Item_X","pack":"{pack}","args":[{args}],"status":"{status}","identity":"Item_X","flag":{flag},"notes":""}}]}}"#);
            pick_in(&catalog::parse(&t).unwrap(), "Item_X")
        };
        assert!(one("built", "null", "Obj_X", "").is_some());
        assert!(one("dry_ok", "null", "Obj_X", "").is_none());
        assert!(one("built", "\"in_game_first\"", "Obj_X", "").is_none());
        assert!(one("built", "\"not_for_placement\"", "Obj_X", "").is_none());
        assert!(one("built", "null", "Obj_Y", "").is_none());
        assert!(one("built", "null", "Obj_Y", r#""--name","Obj_Y""#).is_some());
    }

    #[test]
    fn logic_items_resolve_to_their_tier_packs_with_the_nut_and_stick_top_tier_rule() {
        let d: Data = d();
        for item in ["Boomerang", "Megaton Hammer", "Lens of Truth", "Mirror Shield", "Light Arrows", "Bottle", "Iron Boots", "Hover Boots", "Progressive Hookshot", "Bomb Bag", "Bow"] {
            for at in [Spot::Free, Spot::Pot] {
                assert!(placeable(&d, item, at), "{item} {at:?}");
            }
        }
        assert!(!placeable(&d, "Bow", Spot::Skull));
        assert!(!placeable(&d, "Slingshot", Spot::Skull));
        assert!(!placeable(&d, "Rutos Letter", Spot::Skull));
        assert!(placeable(&d, "Rutos Letter", Spot::Free));
        assert!(placeable(&d, "Progressive Hookshot", Spot::Skull));
        assert!(!placeable(&d, "Progressive Strength Upgrade", Spot::Free));
        assert!(!placeable(&d, "Ocarina", Spot::Free));
        assert!(!placeable(&d, "Milk", Spot::Free));
        assert!(!placeable(&d, "Small Key (Forest Temple)", Spot::Free));
        assert!(!placeable(&d, "Rupees (5)", Spot::Free));
        assert!(placeable(&d, "Deku Nut Capacity", Spot::Pot));
        assert_eq!(contents(&d, "Deku Nut Capacity"), vec!["Item_NutBag_Large".to_string()]);
        assert_eq!(resolve(&d, "Deku Stick Capacity", None).unwrap().pack, "Obj_StickBag_Large");
        assert_eq!(resolve(&d, "Progressive Hookshot", Some("Item_Longshot")).unwrap().pack, "Obj_Longshot");
        assert!(resolve(&d, "Progressive Hookshot", None).is_err());
        assert!(resolve(&d, "Deku Nut Capacity", Some("Item_NutBag_Medium")).is_err());
        assert_eq!(resolve(&d, "Bomb Bag", Some("Item_BombBag_Small")).unwrap().pack, "Obj_BombBag_Small");
        assert_eq!(resolve(&d, "Boomerang", None).unwrap().kit(), "Obj_Boomerang_GS");
        assert!(top_only(&d, "Deku Nut Capacity") && !top_only(&d, "Bomb Bag"));
        assert_eq!(top("NutBag"), Some("Item_NutBag_Large"));
        assert_eq!(shield("Deku Shield"), Some("Obj_Shield_Deku"));
    }

    #[test]
    fn a_skulltula_kit_keeps_the_heap_and_refuses_a_layer() {
        let k: Spec = pick("Item_Hookshot").unwrap().kit_spec().unwrap();
        assert_eq!(k.name, "Obj_Hookshot_GS");
        assert_eq!(k.item, "Item_Hookshot");
        assert_eq!(k.heap, Some(199928));
        assert!(pick("Item_Range_Bow").unwrap().kit_spec().is_err());
    }
}
