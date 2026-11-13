use std::collections::BTreeSet as BS;

use crate::error::{Error as E, Res};
use crate::logic_items::ItemId;
use crate::slice::data::Data;
use crate::slice::support::{Keep, Plan, Songs};
use crate::slice::tier_order::{native, Give};
use crate::start_items::give::Take;
use crate::start_items::table::Strip;
use crate::world::{LocId, Placement, World};

pub const JUNK: &str = "Rupee (1)";

fn gone_kinds(p: &Plan) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    if p.map == Keep::Start {
        out.extend(["Map", "Compass"]);
    }
    if p.boss == Keep::Start {
        out.push("BossKey");
    }
    if p.ganon == Keep::Start {
        out.push("GanonBossKey");
    }
    out
}

pub fn active(p: &Plan) -> bool {
    !gone_kinds(p).is_empty() || p.kit.take.iter().any(|t| t.strip != Strip::Keep && t.count > 0)
}

fn kind_of<'a>(d: &'a Data, w: &World, it: ItemId) -> &'a str {
    d.item(&w.items.names[it]).map(|i| i.kind.as_str()).unwrap_or("")
}

fn is_song(d: &Data, w: &World, it: ItemId) -> bool {
    d.item(&w.items.names[it]).and_then(|i| i.actor.as_deref()).is_some_and(crate::slice::generate::is_song)
}

fn held_tier(d: &Data, w: &World, it: ItemId, l: LocId) -> Option<usize> {
    let f: &str = d.family(&w.items.names[it])?;
    match native(d, f, &w.locs[l].name) {
        Give::Fixed(k) => Some(k),
        _ => None,
    }
}

struct Ctx<'a> {
    d: &'a Data,
    w: &'a World,
    p: &'a Plan,
    free: BS<LocId>,
    junk: ItemId,
}

fn strip_one(c: &Ctx, t: &Take, base: &mut Placement, notes: &mut Vec<String>) -> Res<()> {
    let (d, w, p, free, junk): (&Data, &World, &Plan, &BS<LocId>, ItemId) = (c.d, c.w, c.p, &c.free, c.junk);
    let it: ItemId = w.item(t.logic).ok_or_else(|| E::Msg(format!("{} is not in the logic item list", t.logic)))?;
    let mut left: u32 = t.count;
    if matches!(t.strip, Strip::Unique | Strip::Family) {
        for l in w.real.iter().copied().filter(|l| !free.contains(l)) {
            if left == 0 {
                break;
            }
            if base.get(l) != Some(it) {
                continue;
            }
            let tier: Option<usize> = if t.strip == Strip::Unique { Some(0) } else { held_tier(d, w, it, l) };
            if tier.is_some_and(|k| (k as u32) < t.count) {
                base.put(l, junk);
                left -= 1;
            }
        }
    }
    if p.songs == Songs::Own && is_song(d, w, it) {
        if left > 0 {
            notes.push(format!("{} stays among the song lessons because songs only shuffle among themselves, so it can be learned a second time.", t.logic));
        }
        return Ok(());
    }
    for l in free.iter().copied() {
        if left == 0 {
            break;
        }
        if base.get(l) == Some(it) {
            base.put(l, junk);
            left -= 1;
        }
    }
    Ok(())
}

pub fn fit_actors(d: &Data, item: &str, want: usize, acts: &mut Vec<String>) {
    if item == JUNK
        && let Some(a) = d.item(JUNK).and_then(|i| i.actor.clone())
    {
        while acts.len() < want {
            acts.push(a.clone());
        }
    }
    if acts.len() > want {
        let f: Option<&str> = d.family(item);
        acts.sort_by_key(|a| f.and_then(|f| crate::slice::tier_order::rank(f, a)).unwrap_or(0));
        acts.truncate(want);
    }
}

pub fn base(d: &Data, w: &World, p: &Plan, vanilla: &Placement, free: &[LocId]) -> Res<(Placement, Vec<String>)> {
    let mut out: Placement = vanilla.clone();
    let mut notes: Vec<String> = Vec::new();
    if !active(p) {
        return Ok((out, notes));
    }
    let junk: ItemId = w.item(JUNK).ok_or_else(|| E::Msg(format!("{JUNK} is not in the logic item list")))?;
    let set: BS<LocId> = free.iter().copied().collect();
    let kinds: Vec<&str> = gone_kinds(p);
    if !kinds.is_empty() {
        let mut n: usize = 0;
        for l in free {
            if out.get(*l).is_some_and(|i| kinds.contains(&kind_of(d, w, i))) {
                out.put(*l, junk);
                n += 1;
            }
        }
        if n > 0 {
            notes.push(format!("{n} removed dungeon items (Maps, Compasses or Boss Keys whose setting is Removed or Start with) were taken out of the shuffled locations and replaced by {JUNK}."));
        }
    }
    let c: Ctx = Ctx { d, w, p, free: set, junk };
    for t in p.kit.take.iter().filter(|t| t.strip != Strip::Keep && t.count > 0) {
        strip_one(&c, t, &mut out, &mut notes)?;
    }
    Ok((out, notes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Settings, Value};
    use crate::slice::fill::vanilla_place;
    use crate::slice::logic::world_with;
    use crate::slice::pick::{pick, Pick};
    use crate::slice::support::plan;

    fn with(pairs: &[(&str, Value)]) -> Settings {
        let mut s: Settings = Settings::default();
        for (k, v) in pairs {
            s.set(k, v.clone()).unwrap();
        }
        s
    }

    fn count(w: &World, pl: &Placement, ls: &[LocId], name: &str) -> usize {
        let it: ItemId = w.item(name).unwrap();
        ls.iter().filter(|l| pl.get(**l) == Some(it)).count()
    }

    fn setup(s: &Settings) -> (Data, World, Plan, Placement, Pick) {
        let d: Data = Data::load().unwrap();
        let p: Plan = plan(s).unwrap();
        let mut w: World = world_with(&d, &p.logic()).unwrap();
        crate::slice::starts::add(&d, &p, &mut w, "pool").unwrap();
        let v: Placement = vanilla_place(&w, &d).unwrap();
        let pk: Pick = pick(&d, &w, &p);
        (d, w, p, v, pk)
    }

    #[test]
    fn defaults_change_nothing() {
        let (d, w, p, v, pk) = setup(&Settings::default());
        assert!(!active(&p));
        let (b, n): (Placement, Vec<String>) = base(&d, &w, &p, &v, &pk.free).unwrap();
        assert_eq!(b.item, v.item);
        assert!(n.is_empty());
    }

    #[test]
    fn removed_maps_and_boss_keys_leave_the_shuffled_locations() {
        let s: Settings = with(&[("shuffle_mapcompass", Value::s("remove")), ("shuffle_bosskeys", Value::s("remove")), ("shuffle_ganon_bosskey", Value::s("remove"))]);
        let (d, w, p, v, pk) = setup(&s);
        let (b, n): (Placement, Vec<String>) = base(&d, &w, &p, &v, &pk.free).unwrap();
        let gone = |pl: &Placement| pk.free.iter().filter(|l| pl.get(**l).is_some_and(|i| ["Map", "Compass", "BossKey", "GanonBossKey"].contains(&kind_of(&d, &w, i)))).count();
        assert!(gone(&v) > 20);
        assert_eq!(gone(&b), 0);
        assert_eq!(count(&w, &b, &pk.free, JUNK), count(&w, &v, &pk.free, JUNK) + gone(&v));
        assert_eq!(n.len(), 1);
    }

    #[test]
    fn a_starting_item_loses_one_pool_copy_and_tiers_lose_as_many() {
        let s: Settings = with(&[("starting_items", Value::set(&["hookshot", "boomerang", "bow"]))]);
        let (d, w, p, v, pk) = setup(&s);
        let (b, _): (Placement, Vec<String>) = base(&d, &w, &p, &v, &pk.free).unwrap();
        let all: Vec<LocId> = w.real.clone();
        assert_eq!(count(&w, &b, &all, "Boomerang") + 1, count(&w, &v, &all, "Boomerang"));
        assert_eq!(count(&w, &b, &all, "Progressive Hookshot") + 1, count(&w, &v, &all, "Progressive Hookshot"));
        assert_eq!(count(&w, &b, &all, "Bow") + 1, count(&w, &v, &all, "Bow"));
        let l: Settings = with(&[("starting_items", Value::set(&["longshot"]))]);
        let (d, w, p, v, pk) = setup(&l);
        let (b, _): (Placement, Vec<String>) = base(&d, &w, &p, &v, &pk.free).unwrap();
        assert_eq!(count(&w, &b, &w.real, "Progressive Hookshot"), count(&w, &v, &w.real, "Progressive Hookshot") - 2);
    }

    fn beatable(d: &Data, s: &Settings, seed: &str) -> crate::slice::generate::Generated {
        use crate::slice::generate::{generate_data, GenOpts};
        let g = generate_data(d, s, seed, &GenOpts::default()).unwrap_or_else(|e| panic!("{seed}: {e}"));
        let (w, pl): (World, Placement) = crate::slice::starts::rebuild(d, &g.seed_file).unwrap();
        assert!(crate::search::explore(&w, &pl, &[]).won(), "{seed} is not beatable");
        g
    }

    #[test]
    fn seeds_with_many_starting_items_generate_and_stay_beatable() {
        let d: Data = Data::load().unwrap();
        let spread: Vec<&str> = vec!["kokiri_sword", "big_bomb_bag", "longshot", "double_magic", "bottle", "iron_boots", "ocarina", "biggest_quiver", "golden_scale", "silver_gauntlets", "giant_wallet", "lens_of_truth", "deku_nuts", "big_stick_bag", "bombchus", "magic_beans", "gerudo_card"];
        let cases: Vec<(&str, Settings)> = vec![
            ("spread", with(&[("starting_items", Value::set(&spread)), ("starting_songs", Value::set(&["song_of_time", "zeldas_lullaby"])), ("starting_hearts", Value::Int(6))])),
            ("all", with(&[("starting_items", Value::set(&crate::start_items::table::OPTS.iter().map(|o| o.id).collect::<Vec<&str>>())), ("starting_songs", Value::set(&crate::start_items::table::SONGS.iter().map(|o| o.0).collect::<Vec<&str>>()))])),
            ("songs anywhere", with(&[("starting_items", Value::set(&["hookshot", "bow"])), ("starting_songs", Value::set(&["eponas_song"])), ("shuffle_songs", Value::s("anywhere"))])),
            ("songs own", with(&[("starting_songs", Value::set(&["sarias_song", "suns_song"])), ("shuffle_songs", Value::s("own"))])),
            ("chests off", with(&[("starting_items", Value::set(&["hookshot", "boomerang", "magic_meter"])), ("shuffle_chests", Value::s("off"))])),
        ];
        for (tag, s) in &cases {
            let p: Plan = plan(s).unwrap();
            for n in 0..3 {
                let g = beatable(&d, s, &format!("start {tag} {n}"));
                let w: World = world_with(&d, &p.logic()).unwrap();
                for (loc, item) in &g.seed_file.placements {
                    assert!(w.loc(loc).is_some() && w.item(item).is_some(), "{loc} {item}");
                }
                if *tag == "all" {
                    for x in ["Progressive Hookshot", "Boomerang", "Megaton Hammer", "Lens of Truth", "Bomb Bag", "Bow", "Progressive Strength Upgrade"] {
                        assert!(!g.seed_file.placements.values().any(|v| v == x), "{tag} {n} still places {x}");
                    }
                }
            }
        }
    }

    #[test]
    fn removed_dungeon_items_never_show_up_in_a_seed() {
        let d: Data = Data::load().unwrap();
        for v in ["remove", "startwith"] {
            let s: Settings = with(&[("shuffle_mapcompass", Value::s(v)), ("shuffle_bosskeys", Value::s("remove")), ("shuffle_ganon_bosskey", Value::s("remove"))]);
            for n in 0..3 {
                let g = beatable(&d, &s, &format!("gone {v} {n}"));
                for (loc, item) in &g.seed_file.placements {
                    let k: &str = d.item(item).map(|i| i.kind.as_str()).unwrap_or("");
                    assert!(!["Map", "Compass", "BossKey", "GanonBossKey"].contains(&k), "{v} {n} {loc} {item}");
                }
                assert!(g.report.contains("replaced by Rupee (1)"));
            }
        }
    }

    #[test]
    fn an_unshuffled_duplicate_is_junk_for_the_logic_and_a_shuffled_higher_tier_stays() {
        let s: Settings = with(&[("starting_items", Value::set(&["hookshot", "boomerang"])), ("shuffle_chests", Value::s("off")), ("shuffle_rewards", Value::s("off"))]);
        let (d, w, p, v, pk) = setup(&s);
        let (b, _): (Placement, Vec<String>) = base(&d, &w, &p, &v, &pk.free).unwrap();
        assert_eq!(count(&w, &b, &w.real, "Boomerang"), 0);
        let hook: LocId = w.loc("Graveyard Dampe Race Hookshot Chest").unwrap();
        let long: LocId = w.loc("Water Temple Longshot Chest").unwrap();
        assert_eq!(b.get(hook), w.item(JUNK));
        assert_eq!(b.get(long), w.item("Progressive Hookshot"));
    }
}
