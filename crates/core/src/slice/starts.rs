use crate::error::{Error as E, Res};
use crate::logic_items::ItemId;
use crate::settings::seed::SeedFile;
use crate::settings::Loaded;
use crate::slice::data::Data;
use crate::slice::fill::vanilla_place;
use crate::slice::logic::world_with;
use crate::slice::support::{plan, Plan};
use crate::start_reward::pick as pick_start;
use crate::world::{Placement, World};

pub fn add(d: &Data, p: &Plan, w: &mut World, seed: &str) -> Res<()> {
    if let Some((name, _)) = pick_start(seed, p.start) {
        let i: ItemId = w.item(name).ok_or_else(|| E::Msg(format!("{name} is not in the logic item list")))?;
        w.start.push((i, 1));
    }
    for name in crate::slice::dungeon_items::start_items(d, p) {
        let i: ItemId = w.item(&name).ok_or_else(|| E::Msg(format!("{name} is not in the logic item list")))?;
        w.start.push((i, 1));
    }
    for name in &p.gear {
        let i: ItemId = w.item(name).ok_or_else(|| E::Msg(format!("{name} is not in the logic item list")))?;
        if !w.start.iter().any(|(id, _)| *id == i) {
            w.start.push((i, 1));
        }
    }
    Ok(())
}

pub fn rebuild(d: &Data, seed: &SeedFile) -> Res<(World, Placement)> {
    let loaded: Loaded = seed.settings()?;
    let p: Plan = plan(&loaded.settings)?;
    let mut w: World = world_with(d, &p.logic())?;
    add(d, &p, &mut w, &seed.seed)?;
    let mut place: Placement = vanilla_place(&w, d)?;
    for (loc, item) in &seed.placements {
        let (Some(l), Some(i)) = (w.loc(loc), w.item(item)) else {
            continue;
        };
        place.put(l, i);
    }
    Ok((w, place))
}
