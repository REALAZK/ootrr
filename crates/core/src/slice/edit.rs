use std::collections::{BTreeMap as BM, BTreeSet as BS};
use std::fmt::Write as W;
use std::path::{Path as P, PathBuf as PB};

use crate::TITLE_ID;
use crate::boss_heart::{heart_ids, stage as stage_hearts, swap, CHEST, HEART};
use crate::byml::Byml;
use crate::flow_locations::{self, Choice, Opts as FlowOpts, Stage};
use crate::error::{Error as E, Res};
use crate::modpack::Pack;
use crate::placement::banc_files;
use crate::progressive::{needs_every_holder, route, stage as stage_upgrades, stage_grants, tiers, Size, FAMILIES};
use crate::romfs::Romfs;
use crate::rsizetable::{self, Rst};
use crate::settings::seed::SeedFile;
use crate::slice::support::Songs;
use crate::{hints, songs, special};
use crate::slice::banc::{find, fingerprint, set_drop_ev, Found};
use crate::slice::data::{Data, Ev, State};
use crate::slice::support::plan;
use crate::zs;

pub const MARGIN: u32 = 0x400;
pub const LISTED: usize = 10;

#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub name: String,
    pub hash: u64,
    pub files: Vec<String>,
    pub old_item: String,
    pub new_item: String,
    pub old_actor: String,
    pub new_actor: String,
    pub ev: Option<Ev>,
    pub family: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChestLine {
    pub name: String,
    pub area: String,
    pub pos: Vec<f32>,
    pub old_item: String,
    pub new_item: String,
    pub old_actor: String,
    pub new_actor: String,
    pub copies: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildCounts {
    pub placed: usize,
    pub changed: usize,
    pub unchanged: usize,
    pub left_vanilla: usize,
    pub files: usize,
    pub grown: usize,
    pub flows: usize,
}

#[derive(Debug, Clone)]
pub struct Built {
    pub root: PB,
    pub counts: BuildCounts,
    pub chests: Vec<ChestLine>,
    pub report: String,
}

fn tier_rank(d: &Data, item: &str, actor: &str) -> usize {
    d.item(item).and_then(|i| i.tiers.iter().position(|t| t == actor)).unwrap_or(usize::MAX)
}

fn vanilla_actor(d: &Data, loc: &str) -> Res<String> {
    if is_sale(d, loc) {
        if let Some(a) = d.flow.by_name(loc).and_then(|l| l.item.clone()) {
            return Ok(a);
        }
    }
    if let Some(l) = d.cat.by_name(loc) {
        return Ok(l.item.clone());
    }
    d.flow
        .by_name(loc)
        .and_then(|l| l.item.clone())
        .ok_or_else(|| E::Msg(format!("{loc} is not in the catalog")))
}

pub fn is_sale(d: &Data, loc: &str) -> bool {
    d.flow.by_name(loc).is_some() && d.cat.by_name(loc).is_none_or(|c| c.kind == "scrub")
}

fn actors_for(d: &Data, seed: &SeedFile) -> Res<BM<String, String>> {
    let mut pool: BM<String, Vec<String>> = BM::new();
    let mut dests: BM<String, Vec<String>> = BM::new();
    for (loc, item) in &seed.placements {
        let vanilla: &str = d.vanilla_of(loc).ok_or_else(|| E::Msg(format!("{loc} has no vanilla item")))?;
        pool.entry(vanilla.to_string()).or_default().push(vanilla_actor(d, loc)?);
        dests.entry(item.clone()).or_default().push(loc.clone());
    }
    let mut out: BM<String, String> = BM::new();
    for (item, locs) in &dests {
        let mut acts: Vec<String> = pool.get(item).cloned().unwrap_or_default();
        if acts.len() != locs.len() {
            return Err(E::Msg(format!(
                "the seed places {} copies of {item} but its chests hold {}, this seed does not belong to these game files",
                locs.len(),
                acts.len()
            )));
        }
        acts.sort_by(|x, y| (tier_rank(d, item, x), x).cmp(&(tier_rank(d, item, y), y)));
        for (loc, actor) in locs.iter().zip(acts) {
            out.insert(loc.clone(), actor);
        }
    }
    Ok(out)
}

fn targets(d: &Data, seed: &SeedFile) -> Res<(Vec<Target>, usize, Vec<Choice>)> {
    let mut out: Vec<Target> = Vec::new();
    let mut flows: Vec<Choice> = Vec::new();
    let mut same: usize = 0;
    let given: BM<String, String> = actors_for(d, seed)?;
    for (loc, item) in &seed.placements {
        if is_sale(d, loc) {
            if !d.flow_ok(loc) && !d.song_ok(loc) {
                return Err(E::Msg(format!("{loc} cannot be placed, its reward event is not supported")));
            }
            let actor: String = given.get(loc).cloned().ok_or_else(|| E::Msg(format!("{loc} was given no item actor")))?;
            if Some(actor.as_str()) == d.flow.by_name(loc).and_then(|l| l.item.as_deref()) {
                same += 1;
            } else {
                flows.push(Choice { location: loc.clone(), item: actor });
            }
            continue;
        }
        if let Some(l) = d.cat.by_name(loc).filter(|l| l.kind == "bossheart") {
            let new: &crate::slice::data::Item = d.item(item).ok_or_else(|| E::Msg(format!("unknown item {item}")))?;
            if new.actor.is_none() {
                return Err(E::Msg(format!("{item} has no remake item actor and cannot be placed")));
            }
            let new_actor: String = given.get(loc).cloned().ok_or_else(|| E::Msg(format!("{loc} was given no item actor")))?;
            if new_actor == l.item {
                same += 1;
                continue;
            }
            let vanilla: &str = d.vanilla_of(loc).ok_or_else(|| E::Msg(format!("{loc} has no vanilla item")))?;
            for r in &l.refs {
                out.push(Target {
                    name: loc.clone(),
                    hash: r.hash,
                    files: r.files.clone(),
                    old_item: vanilla.to_string(),
                    new_item: item.clone(),
                    old_actor: l.item.clone(),
                    new_actor: new_actor.clone(),
                    ev: None,
                    family: None,
                });
            }
            continue;
        }
        match d.state_of(loc) {
            Some(State::Eligible) => {}
            Some(State::Event) if d.event_ok(loc) => {}
            Some(s) => {
                return Err(E::Msg(format!("{loc} cannot be placed, it stays vanilla because {}", s.why())));
            }
            None => return Err(E::Msg(format!("{loc} is not a chest this build knows"))),
        }
        let vanilla: &str = d.vanilla_of(loc).ok_or_else(|| E::Msg(format!("{loc} has no vanilla item")))?;
        let new: &crate::slice::data::Item = d.item(item).ok_or_else(|| E::Msg(format!("unknown item {item}")))?;
        if new.actor.is_none() {
            return Err(E::Msg(format!("{item} has no remake item actor and cannot be placed")));
        }
        let family: Option<&'static str> = d
            .family(item)
            .filter(|f| !needs_every_holder(f) || d.holders(f).iter().all(|h| seed.placements.contains_key(*h) || d.wrappable(h)));
        let new_actor: String = match family {
            Some(f) => tiers(f)?[0].to_string(),
            None => given.get(loc).cloned().ok_or_else(|| E::Msg(format!("{loc} was given no item actor")))?,
        };
        let l: &crate::catalog::Loc = d.cat.by_name(loc).ok_or_else(|| E::Msg(format!("{loc} is not in the catalog")))?;
        if family.is_none() && new_actor == l.item {
            same += 1;
            continue;
        }
        for r in &l.refs {
            out.push(Target {
                name: loc.clone(),
                hash: r.hash,
                files: r.files.clone(),
                old_item: vanilla.to_string(),
                new_item: item.clone(),
                old_actor: l.item.clone(),
                new_actor: new_actor.clone(),
                ev: d.event_mode(r.hash),
                family,
            });
        }
    }
    Ok((out, same, flows))
}

pub struct Edited {
    pub file: String,
    pub old_len: usize,
    pub bytes: Vec<u8>,
    pub hits: Vec<(usize, Found)>,
    pub used: Vec<(String, Size)>,
    pub hearts: Vec<(u64, Vec<String>)>,
    pub wraps: Vec<(u64, String)>,
    pub touched: Vec<u64>,
}

pub fn edit_file(file: &str, raw: &[u8], ts: &[Target]) -> Res<Option<Edited>> {
    let mut y: Byml = Byml::parse(raw)?;
    let mut hits: Vec<(usize, Found)> = Vec::new();
    let mut used: Vec<(String, Size)> = Vec::new();
    let mut hearts: Vec<(u64, Vec<String>)> = Vec::new();
    let mut wraps: Vec<(u64, String)> = Vec::new();
    let mut touched: Vec<u64> = Vec::new();
    for (i, t) in ts.iter().enumerate() {
        let n: usize = if t.old_actor == HEART {
            let n: usize = swap(&mut y, t.hash, &t.new_actor, CHEST);
            if n > 0 {
                hearts.push((t.hash, heart_ids(&y, t.hash)));
            }
            n
        } else {
            set_drop_ev(&mut y, t.hash, &t.old_actor, &t.new_actor, t.ev)?
        };
        if n > 0 {
            if special::of(t.hash).is_some() {
                touched.push(t.hash);
            }
            if let Some(f) = t.family {
                if special::of(t.hash).is_some() {
                    used.push((f.to_string(), Size::Big));
                    wraps.push((t.hash, f.to_string()));
                } else {
                    let size: Size = route(&mut y, t.hash, f).ok_or(E::Format("edited actor vanished"))?;
                    used.push((f.to_string(), size));
                }
            }
            let f: Found = find(&y, file, t.hash).into_iter().next().ok_or(E::Format("edited actor vanished"))?;
            hits.push((i, f));
        }
    }
    if hits.is_empty() {
        return Ok(None);
    }
    Ok(Some(Edited { file: file.to_string(), old_len: raw.len(), bytes: y.write()?, hits, used, hearts, wraps, touched }))
}

fn area(file: &str) -> String {
    file.split('/').nth(1).unwrap_or("").to_string()
}

fn lines(ts: &[Target], edits: &[Edited]) -> Vec<ChestLine> {
    let mut by: BM<String, ChestLine> = BM::new();
    for e in edits {
        for (i, f) in &e.hits {
            let t: &Target = &ts[*i];
            let c: &mut ChestLine = by.entry(t.name.clone()).or_insert_with(|| ChestLine {
                name: t.name.clone(),
                area: area(&e.file),
                pos: f.pos.clone(),
                old_item: t.old_item.clone(),
                new_item: t.new_item.clone(),
                old_actor: t.old_actor.clone(),
                new_actor: t.new_actor.clone(),
                copies: 0,
            });
            c.copies += 1;
        }
    }
    let mut v: Vec<ChestLine> = by.into_values().collect();
    v.sort_by(|a, b| (&a.area, &a.name).cmp(&(&b.area, &b.name)));
    v
}

fn coords(p: &[f32]) -> String {
    match p {
        [x, y, z] => format!("({x:.1}, {y:.1}, {z:.1})"),
        _ => "(unknown)".to_string(),
    }
}

fn describe(c: &BuildCounts, root: &P, chests: &[ChestLine], notes: &[String]) -> String {
    let mut out: String = String::new();
    let _ = writeln!(out, "Mod folder: {}", root.display());
    let _ = writeln!(
        out,
        "{} chests in the seed, {} changed, {} unchanged. {} candidate chests stay vanilla.",
        c.placed, c.changed, c.unchanged, c.left_vanilla
    );
    let _ = writeln!(out, "{} files written, {} needed a resource size table update.", c.files, c.grown);
    if !chests.is_empty() {
        let _ = writeln!(out, "Check these in game ({} of {} changed chests):", chests.len().min(LISTED), chests.len());
        for ch in chests.iter().take(LISTED) {
            let _ = writeln!(
                out,
                "  {} | area {} | at {} | vanilla {} ({}) | now {} ({})",
                ch.name,
                ch.area,
                coords(&ch.pos),
                ch.old_item,
                ch.old_actor,
                ch.new_item,
                ch.new_actor
            );
        }
    }
    for n in notes {
        let _ = writeln!(out, "{n}");
    }
    out
}

pub fn build(seed: &SeedFile, romfs: &P, load: &P, mod_name: &str) -> Res<Built> {
    let d: Data = Data::load()?;
    let loaded = seed.settings()?;
    let p = plan(&loaded.settings)?;
    let mut rf: Romfs = Romfs::open(romfs)?;
    let have: String = fingerprint(&rf, &d.scan.files)?;
    if have != seed.game {
        return Err(E::Msg(format!(
            "this seed was made for the game {} but the folder {} is {}",
            seed.game,
            romfs.display(),
            have
        )));
    }
    let (ts, same, flows): (Vec<Target>, usize, Vec<Choice>) = targets(&d, seed)?;
    let mut edits: Vec<Edited> = Vec::new();
    for f in banc_files(&rf)? {
        let raw: Vec<u8> = rf.read_zs(&f)?;
        if let Some(e) = edit_file(&f, &raw, &ts)? {
            edits.push(e);
        }
    }
    for (i, t) in ts.iter().enumerate() {
        let got: Vec<&str> = edits
            .iter()
            .filter(|e| e.hits.iter().any(|(j, _)| *j == i))
            .map(|e| e.file.as_str())
            .collect();
        for f in &t.files {
            if !got.contains(&f.as_str()) {
                return Err(E::Msg(format!("{} was not found in {f}, the game files differ", t.name)));
            }
        }
    }
    Pack::clean(load, TITLE_ID, mod_name)?;
    let pk: Pack = Pack::create(load, TITLE_ID, mod_name)?;
    let mut rst: Option<Rst> = None;
    let mut grown: usize = 0;
    for e in &edits {
        let z: Vec<u8> = zs::compress_raw(&e.bytes, Some(3))?;
        pk.write_romfs(&e.file, &z)?;
        if e.bytes.len() > e.old_len {
            if rst.is_none() {
                rst = Some(rsizetable::load(&mut rf)?);
            }
            if let Some(t) = rst.as_mut() {
                t.grow(&e.file, e.old_len as u64, e.bytes.len() as u64, MARGIN)?;
                grown += 1;
            }
        }
    }
    let mut upgrades: BS<(String, Size)> = BS::new();
    for e in &edits {
        upgrades.extend(e.used.iter().cloned());
    }
    let mut extra: usize = 0;
    let wraps: Vec<(u64, String)> = edits.iter().flat_map(|e| e.wraps.clone()).collect();
    let touched: Vec<u64> = edits.iter().flat_map(|e| e.touched.clone()).collect();
    let (special_outs, kids): (Vec<crate::staging::Out>, Vec<(String, String)>) = if wraps.is_empty() && !touched.contains(&special::DAMPE_HOOK) {
        (Vec::new(), Vec::new())
    } else {
        special::stage(&mut rf, &wraps, &touched)?
    };
    if !upgrades.is_empty() {
        let mut outs: Vec<crate::staging::Out> = stage_upgrades(&mut rf, &upgrades, None, &kids)?;
        let fams: BS<&str> = upgrades.iter().map(|u| u.0.as_str()).collect();
        let mut gs: Vec<crate::progressive::Grant> = Vec::new();
        for f in FAMILIES.iter().filter(|x| fams.contains(x.0)) {
            gs.extend(d.grants(f.0));
        }
        outs.extend(stage_grants(&mut rf, &gs)?);
        for o in outs {
            pk.write_romfs(&o.path, &o.z)?;
            extra += 1;
            if o.new > o.old {
                if rst.is_none() {
                    rst = Some(rsizetable::load(&mut rf)?);
                }
                if let Some(t) = rst.as_mut()
                    && t.grow(&o.path, o.old as u64, o.new as u64, MARGIN)?.is_some()
                {
                    grown += 1;
                }
            }
        }
    }
    let staged: Stage = flow_locations::stage(&mut rf, &d.flow, &flows, &FlowOpts { songs: p.songs != Songs::Off, ..FlowOpts::default() })?;
    for f in &staged.files {
        pk.write_romfs(&f.path, &f.z)?;
        if f.new > f.old {
            if rst.is_none() {
                rst = Some(rsizetable::load(&mut rf)?);
            }
            if let Some(t) = rst.as_mut()
                && t.grow(&f.path, f.old as u64, f.new as u64, MARGIN)?.is_some()
            {
                grown += 1;
            }
        }
    }
    let mut more: Vec<crate::staging::Out> = special_outs;
    let hearts: Vec<(u64, Vec<String>)> = edits.iter().flat_map(|e| e.hearts.clone()).collect();
    if !hearts.is_empty() {
        more.extend(stage_hearts(&mut rf, &d, &hearts, true)?);
    }
    if p.songs != Songs::Off {
        more.push(songs::stage(&mut rf)?);
    }
    if p.songs == Songs::Anywhere {
        more.extend(crate::song_models::stage(&mut rf)?);
    }
    if p.hints {
        more.extend(hints::stage(&mut rf, &d, seed)?);
    }
    more.extend(crate::bomb_lift::stage(&mut rf)?);
    if let Some((_, actor)) = crate::start_reward::pick(&seed.seed, p.start) {
        more.push(crate::start_reward::stage(&mut rf, actor)?);
    }
    for o in more {
        pk.write_romfs(&o.path, &o.z)?;
        extra += 1;
        if o.new > o.old {
            if rst.is_none() {
                rst = Some(rsizetable::load(&mut rf)?);
            }
            if let Some(t) = rst.as_mut()
                && t.grow(&o.path, o.old as u64, o.new as u64, MARGIN)?.is_some()
            {
                grown += 1;
            }
        }
    }
    if p.fast {
        extra += crate::fast::apply(&mut rf, &pk)?;
    }
    if let Some(t) = &rst {
        pk.write_romfs(rsizetable::TABLE_PATH, &t.to_zs()?)?;
    }
    let chests: Vec<ChestLine> = lines(&ts, &edits);
    let counts: BuildCounts = BuildCounts {
        placed: seed.placements.len(),
        changed: chests.len(),
        unchanged: same,
        left_vanilla: d.scan.rows.len().saturating_sub(seed.placements.len()),
        files: edits.len() + staged.files.len() + extra,
        grown,
        flows: flows.len(),
    };
    let mut notes: Vec<String> = p.notes.clone();
    if !upgrades.is_empty() {
        let n: usize = ts.iter().filter(|t| t.family.is_some()).map(|t| t.name.as_str()).collect::<BS<&str>>().len();
        notes.push(format!(
            "{n} chests hold upgrade items that the game resolves when the chest opens, so the first one found is always the first tier. This replaces the chest event flows, so remove any other mod that changes Dm_SY_Treasurebox or Dm_SY_TreasureboxSmall."
        ));
    }
    if !flows.is_empty() {
        notes.push(format!("{} reward events changed in {} flow files, none of these edits is tested in the game yet:", flows.len(), staged.files.len()));
        for c in staged.changes.iter().take(LISTED) {
            notes.push(format!("  {} | {} | {} ({}) | {} -> {}", c.location, c.file, c.event, c.index, c.old, c.new));
        }
    }
    let report: String = describe(&counts, &pk.root, &chests, &notes);
    Ok(Built { root: pk.root, counts, chests, report })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byml::Node;

    fn actor(hash: u64, drop: &str) -> Node {
        Node::Map(vec![
            (
                "Dynamic".to_string(),
                Node::Map(vec![("Drop__DropActor".to_string(), Node::Str(drop.to_string()))]),
            ),
            ("Gyaml".to_string(), Node::Str("Obj_TBox_A_02".to_string())),
            ("Hash".to_string(), Node::U64(hash)),
            ("Translate".to_string(), Node::Arr(vec![Node::F32(1.0), Node::F32(2.0), Node::F32(3.0)])),
        ])
    }

    fn raw(list: Vec<Node>) -> Vec<u8> {
        Byml { root: Node::Map(vec![("Actors".to_string(), Node::Arr(list))]), version: 7, big: false }
            .write()
            .unwrap()
    }

    fn target(hash: u64, old: &str, new: &str) -> Target {
        Target {
            name: "Chest".to_string(),
            hash,
            files: vec!["Banc/Area/a.bcett.byml.zs".to_string()],
            old_item: "Old".to_string(),
            new_item: "New".to_string(),
            old_actor: old.to_string(),
            new_actor: new.to_string(),
            ev: None,
            family: None,
        }
    }

    #[test]
    fn edits_only_the_named_actor_and_reports_it() {
        let b: Vec<u8> = raw(vec![actor(1, "Item_A"), actor(2, "Item_B")]);
        let e: Edited = edit_file("Banc/Area/a.bcett.byml.zs", &b, &[target(2, "Item_B", "Item_C")]).unwrap().unwrap();
        let back: Byml = Byml::parse(&e.bytes).unwrap();
        assert_eq!(find(&back, "f", 1)[0].drop.as_deref(), Some("Item_A"));
        assert_eq!(find(&back, "f", 2)[0].drop.as_deref(), Some("Item_C"));
        assert_eq!(e.hits.len(), 1);
        assert_eq!(e.hits[0].1.pos, vec![1.0, 2.0, 3.0]);
        let l: Vec<ChestLine> = lines(&[target(2, "Item_B", "Item_C")], &[e]);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].area, "Area");
    }

    #[test]
    fn a_file_without_the_hash_is_untouched() {
        let b: Vec<u8> = raw(vec![actor(1, "Item_A")]);
        assert!(edit_file("Banc/Area/a.bcett.byml.zs", &b, &[target(5, "Item_A", "X")]).unwrap().is_none());
    }

    #[test]
    fn a_changed_game_is_refused() {
        let b: Vec<u8> = raw(vec![actor(1, "Item_Other")]);
        assert!(edit_file("Banc/Area/a.bcett.byml.zs", &b, &[target(1, "Item_A", "X")]).is_err());
    }

    #[test]
    fn same_length_names_keep_the_file_size() {
        let b: Vec<u8> = raw(vec![actor(1, "Item_A")]);
        let e: Edited = edit_file("Banc/Area/a.bcett.byml.zs", &b, &[target(1, "Item_A", "Item_Z")]).unwrap().unwrap();
        assert_eq!(e.bytes.len(), b.len());
    }

    #[test]
    fn coordinates_print_with_one_decimal() {
        assert_eq!(coords(&[1.0, 2.26, -3.0]), "(1.0, 2.3, -3.0)");
        assert_eq!(coords(&[]), "(unknown)");
    }

    #[test]
    fn items_move_with_their_own_actors_so_progressive_tiers_stay_distinct() {
        use crate::settings::Settings;
        use crate::slice::generate::{generate_data, GenOpts};
        let d: Data = Data::load().unwrap();
        for n in 0..3 {
            let g = generate_data(&d, &Settings::default(), &format!("tiers {n}"), &GenOpts::default()).unwrap();
            let given: BM<String, String> = actors_for(&d, &g.seed_file).unwrap();
            let mut before: Vec<String> = Vec::new();
            let mut after: Vec<String> = Vec::new();
            for (loc, actor) in &given {
                before.push(d.cat.by_name(loc).unwrap().item.clone());
                after.push(actor.clone());
            }
            before.sort();
            after.sort();
            assert_eq!(before, after, "seed tiers {n}");
            let (ts, same, flows): (Vec<Target>, usize, Vec<Choice>) = targets(&d, &g.seed_file).unwrap();
            let moved: std::collections::BTreeSet<&str> = ts.iter().map(|t| t.name.as_str()).collect();
            assert_eq!(moved.len() + same + flows.len(), g.seed_file.placements.len());
        }
    }

    #[test]
    fn upgrade_chests_route_through_the_wrappers_unless_exact_tiers_matter() {
        use crate::settings::Settings;
        use crate::slice::generate::{generate_data, GenOpts};
        let d: Data = Data::load().unwrap();
        assert!(d.holders("Strength").contains(&"GC Darunias Joy"));
        let mut seen: BS<&'static str> = BS::new();
        for n in 0..4 {
            let g = generate_data(&d, &Settings::default(), &format!("wrap {n}"), &GenOpts::default()).unwrap();
            let (ts, _, _): (Vec<Target>, usize, Vec<Choice>) = targets(&d, &g.seed_file).unwrap();
            for t in &ts {
                match (t.family, d.family(&t.new_item)) {
                    (Some(f), _) => {
                        seen.insert(f);
                        assert_eq!(t.new_actor, tiers(f).unwrap()[0]);
                        assert!(
                            !needs_every_holder(f) || d.holders(f).iter().all(|h| g.seed_file.placements.contains_key(*h) || d.wrappable(h)),
                            "{f}"
                        );
                    }
                    (None, Some(f)) => {
                        assert!(needs_every_holder(f), "{} {f}", t.name);
                        assert!(!d.holders(f).iter().all(|h| g.seed_file.placements.contains_key(*h) || d.wrappable(h)), "{} {f}", t.name);
                    }
                    (None, None) => {}
                }
            }
        }
        assert!(!seen.is_empty());
    }

    #[test]
    fn reward_events_become_flow_choices_with_accepted_actors() {
        use crate::settings::{Settings, Value};
        use crate::slice::generate::{generate_data, GenOpts};
        let d: Data = Data::load().unwrap();
        let mut st: Settings = Settings::default();
        st.set("shuffle_rewards", Value::s("supported")).unwrap();
        let g = generate_data(&d, &st, "rewards", &GenOpts::default()).unwrap();
        let (_, _, flows): (Vec<Target>, usize, Vec<Choice>) = targets(&d, &g.seed_file).unwrap();
        assert!(!flows.is_empty());
        for c in &flows {
            assert!(d.flow.usable(&c.item), "{} -> {}", c.location, c.item);
            assert!(d.flow_ok(&c.location), "{}", c.location);
        }
        let off = generate_data(&d, &Settings::default(), "rewards", &GenOpts::default()).unwrap();
        let (_, _, none): (Vec<Target>, usize, Vec<Choice>) = targets(&d, &off.seed_file).unwrap();
        assert!(none.is_empty());
    }
}
