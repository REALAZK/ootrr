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
    rng: &mut Rng,
) -> Res<Placement> {
    let mut place: Placement = fixed.clone();
    let mut items: Vec<ItemId> = pool.to_vec();
    rng.shuffle(&mut items);
    let room = |i: &ItemId| free.iter().filter(|l| allow(*i, **l)).count();
    items.sort_by_key(room);
    let prog: Vec<ItemId> = items.iter().copied().filter(|i| w.items.adv[*i]).collect();
    let mut open: Vec<LocId> = free.to_vec();
    let mut base: Search = Search::new(w);
    base.settle(&place, true);
    let mut assumed: Search = base.clone();
    assumed.add_all(&prog);
    for it in items {
        rng.shuffle(&mut open);
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
) -> Res<Placement> {
    let mut fixed: Placement = vanilla.clone();
    let mut pool: Vec<ItemId> = Vec::with_capacity(free.len());
    for l in free {
        if let Some(i) = fixed.get(*l) {
            pool.push(i);
        }
        fixed.item[*l] = None;
    }
    if !explore(w, vanilla, &[]).won() {
        return Err(E::Format("the vanilla placement does not reach the goal under this logic"));
    }
    let mut won: Vec<Placement> = Vec::new();
    let mut last: E = E::Format("no fill attempts were made");
    let base: u32 = tries.max(1);
    for n in 0..base.saturating_mul(RETRY) {
        if n >= base && !won.is_empty() {
            break;
        }
        match once(w, &fixed, free, &pool, allow, rng) {
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
        return Err(last);
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
    use crate::slice::logic::world;

    #[test]
    fn the_vanilla_game_reaches_the_goal() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let p: Placement = vanilla_place(&w, &d).unwrap();
        assert!(explore(&w, &p, &[]).won());
    }
}
