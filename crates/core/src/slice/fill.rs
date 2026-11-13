use crate::error::{Error as E, Res};
use crate::logic_items::ItemId;
use crate::rng::Rng;
use crate::search::{explore, Search};
use crate::slice::data::Data;
use crate::world::{LocId, Placement, World};

pub fn vanilla_place(w: &World, d: &Data) -> Res<Placement> {
    let mut p: Placement = Placement::new(w.locs.len());
    for l in &w.real {
        let Some(name) = d.vanilla_of(&w.locs[*l].name) else {
            continue;
        };
        let i: ItemId = w
            .item(name)
            .ok_or_else(|| E::Msg(format!("vanilla item {name} of {} is not in the item list", w.locs[*l].name)))?;
        p.put(*l, i);
    }
    Ok(p)
}

pub const WINS: u32 = 16;
pub const RETRY: u32 = 8;

fn once(
    w: &World,
    fixed: &Placement,
    free: &[LocId],
    pool: &[ItemId],
    allow: &dyn Fn(ItemId, LocId) -> bool,
    order: &dyn Fn(ItemId, LocId) -> bool,
    rng: &mut Rng,
    rank: Option<&dyn Fn(LocId) -> usize>,
) -> Res<Placement> {
    let mut place: Placement = fixed.clone();
    let mut items: Vec<ItemId> = pool.to_vec();
    rng.shuffle(&mut items);
    let room = |i: &ItemId| free.iter().filter(|l| order(*i, **l)).count();
    items.sort_by_key(room);
    let prog: Vec<ItemId> = items.iter().copied().filter(|i| w.items.adv[*i]).collect();
    let mut open: Vec<LocId> = free.to_vec();
    let mut base: Search = Search::new(w);
    base.settle(&place, true);
    let mut assumed: Search = base.clone();
    assumed.add_all(&prog);
    for it in items {
        rng.shuffle(&mut open);
        if let Some(r) = rank {
            open.sort_by_key(|l| r(*l));
        }
        let pick: Option<usize> = if w.items.adv[it] {
            assumed.remove(it);
            let mut mx: Search = assumed.clone();
            mx.settle(&place, true);
            open.iter().position(|l| allow(it, *l) && mx.loc_access(*l))
        } else {
            open.iter().position(|l| allow(it, *l))
        };
        match pick {
            Some(k) => {
                let l: LocId = open.swap_remove(k);
                place.put(l, it);
            }
            None => {
                return Err(E::Msg(format!(
                    "no spot for {} among {} free locations",
                    w.items.names[it],
                    open.len()
                )));
            }
        }
    }
    if !explore(w, &place, &[]).won() {
        return Err(E::Format("the game is not beatable after the fill"));
    }
    Ok(place)
}

pub fn changed(fixed: &Placement, got: &Placement, free: &[LocId]) -> usize {
    free.iter().filter(|l| fixed.get(**l) != got.get(**l)).count()
}

pub fn fill(
    w: &World,
    vanilla: &Placement,
    free: &[LocId],
    allow: &dyn Fn(ItemId, LocId) -> bool,
    rng: &mut Rng,
    tries: u32,
    plan: &[(LocId, ItemId)],
    check_vanilla: bool,
) -> Res<Placement> {
    fill_ranked(w, vanilla, free, allow, rng, tries, plan, check_vanilla, None)
}

pub fn fill_ranked(
    w: &World,
    vanilla: &Placement,
    free: &[LocId],
    allow: &dyn Fn(ItemId, LocId) -> bool,
    rng: &mut Rng,
    tries: u32,
    plan: &[(LocId, ItemId)],
    check_vanilla: bool,
    rank: Option<&dyn Fn(LocId) -> usize>,
) -> Res<Placement> {
    fill_ordered(w, vanilla, free, allow, None, rng, tries, plan, check_vanilla, rank)
}

pub fn fill_ordered(
    w: &World,
    vanilla: &Placement,
    free: &[LocId],
    allow: &dyn Fn(ItemId, LocId) -> bool,
    order: Option<&dyn Fn(ItemId, LocId) -> bool>,
    rng: &mut Rng,
    tries: u32,
    plan: &[(LocId, ItemId)],
    check_vanilla: bool,
    rank: Option<&dyn Fn(LocId) -> usize>,
) -> Res<Placement> {
    let mut fixed: Placement = vanilla.clone();
    let mut pool: Vec<ItemId> = Vec::with_capacity(free.len());
    for l in free {
        if let Some(i) = fixed.get(*l) {
            pool.push(i);
        }
        fixed.item[*l] = None;
    }
    for (l, i) in plan {
        let at: usize = pool
            .iter()
            .position(|x| x == i)
            .ok_or_else(|| E::Msg(format!("plando: {} has no copy left to place at {}", w.items.names[*i], w.locs[*l].name)))?;
        pool.swap_remove(at);
        fixed.put(*l, *i);
    }
    let open: Vec<LocId> = free.iter().copied().filter(|l| !plan.iter().any(|(p, _)| p == l)).collect();
    if check_vanilla && !explore(w, vanilla, &[]).won() {
        return Err(E::Format("the vanilla placement does not reach the goal under this logic"));
    }
    if !plan.is_empty() && !explore(w, &fixed, &pool).won() {
        return Err(E::Format("plando: the seed cannot be beaten with these fixed placements"));
    }
    let mut won: Vec<Placement> = Vec::new();
    let mut last: E = E::Format("no fill attempts were made");
    let base: u32 = tries.max(1);
    for n in 0..base.saturating_mul(RETRY) {
        if n >= base && !won.is_empty() {
            break;
        }
        match once(w, &fixed, &open, &pool, allow, order.unwrap_or(allow), rng, rank) {
            Ok(p) => {
                won.push(p);
                if won.len() >= WINS as usize {
                    break;
                }
            }
            Err(e) => last = e,
        }
    }
    if won.is_empty() {
        if plan.is_empty() {
            return Err(last);
        }
        return Err(E::Msg(format!("plando: no seed could be made with these fixed placements, last problem: {last}")));
    }
    let moved: Vec<usize> = (0..won.len()).filter(|i| changed(vanilla, &won[*i], free) > 0).collect();
    let k: usize = if moved.is_empty() {
        rng.below(won.len() as u64) as usize
    } else {
        moved[rng.below(moved.len() as u64) as usize]
    };
    Ok(won.swap_remove(k))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slice::logic::{world, world_with};

    #[test]
    fn the_vanilla_game_reaches_the_goal() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let p: Placement = vanilla_place(&w, &d).unwrap();
        assert!(explore(&w, &p, &[]).won());
    }

    #[test]
    fn no_small_key_is_given_for_free() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        assert!(w.start.iter().all(|(i, _)| !w.items.names[*i].starts_with("Small Key")));
    }

    #[test]
    fn each_vanilla_key_layout_needs_its_own_slack() {
        let d: Data = Data::load().unwrap();
        for k in ["fire_key_layout", "water_key_layout", "spirit_key_layout"] {
            let w: World = world_with(&d, &[(k, "shuffled")]).unwrap();
            let p: Placement = vanilla_place(&w, &d).unwrap();
            assert!(!explore(&w, &p, &[]).won(), "{k}");
        }
    }
}
