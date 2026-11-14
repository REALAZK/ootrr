use std::collections::{BTreeMap as BM, BTreeSet as BS};
use std::fmt::Write as W;
use std::path::{Path as P, PathBuf as PB};

use crate::TITLE_ID;
use crate::boss_heart::{heart_ids, stage as stage_hearts, swap, CHEST, HEART};
use crate::byml::Byml;
use crate::dungeon_credit::{key_dungeon, place as place_key, stage_pouch, sweep, KEY_DUNGEONS};
use crate::flow_locations::{self, Choice, Opts as FlowOpts, Stage};
use crate::error::{Error as E, Res};
use crate::modpack::{Host, Pack};
use crate::pickup_persist::Extra;
use crate::placement::banc_files;
use crate::progressive::{needs_every_holder, route, stage as stage_upgrades, Size};
use crate::romfs::Romfs;
use crate::rsizetable::{self, Rst};
use crate::settings::seed::SeedFile;
use crate::slice::support::Songs;
use crate::{hints, songs, special};
use crate::slice::banc::{find, fingerprint, set_drop_ev, Found};
use crate::slice::data::{Data, Ev, State};
use crate::slice::flow_pool::{dest_form, pool_actor};
use crate::slice::flow_wrap::{grants as wrap_grants, wraps as flow_wraps};
use crate::slice::pickups as pu;
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
    pub key: Option<&'static str>,
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
    if let Some(a) = crate::slice::skull_edit::vanilla_actor(d, loc) {
        return Ok(a);
    }
    if let Some(a) = pu::pool_actor(d, loc) {
        return Ok(a);
    }
    if is_sale(d, loc) {
        if let Some(a) = d.flow.by_name(loc).and_then(|l| l.item.clone()) {
            return Ok(pool_actor(d, loc, &a));
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
        if crate::pot_slice::targets::is_loc(d, loc) {
            continue;
        }
        let vanilla: &str = d.vanilla_of(loc).ok_or_else(|| E::Msg(format!("{loc} has no vanilla item")))?;
        pool.entry(vanilla.to_string()).or_default().push(vanilla_actor(d, loc)?);
        dests.entry(item.clone()).or_default().push(loc.clone());
    }
    let stripped: bool = crate::start_items::pool::active(&plan(&seed.settings()?.settings)?);
    let mut out: BM<String, String> = BM::new();
    for (item, locs) in &dests {
        let mut acts: Vec<String> = pool.get(item).cloned().unwrap_or_default();
        crate::pot_slice::targets::balance(d, seed, item, &mut acts);
        if stripped {
            crate::start_items::pool::fit_actors(d, item, locs.len(), &mut acts);
        }
        if acts.len() != locs.len() {
            return Err(E::Msg(format!(
                "the seed places {} copies of {item} but its chests hold {}, this seed does not belong to these game files",
                locs.len(),
                acts.len()
            )));
        }
        acts.sort_by(|x, y| (tier_rank(d, item, x), x).cmp(&(tier_rank(d, item, y), y)));
        for (loc, actor) in locs.iter().zip(acts) {
            out.insert(loc.clone(), dest_form(d, item, loc, actor));
        }
    }
    Ok(out)
}

#[cfg(test)]
fn targets(d: &Data, seed: &SeedFile) -> Res<(Vec<Target>, usize, Vec<Choice>)> {
    let v: crate::slice::tier_order::Verdict = crate::slice::tier_order::fixed_actors(d, seed)?;
    targets_with(d, seed, false, &v.assign, &Extra::default())
}

fn targets_with(d: &Data, seed: &SeedFile, roam: bool, fixed: &BM<String, String>, extra: &Extra) -> Res<(Vec<Target>, usize, Vec<Choice>)> {
    let mut out: Vec<Target> = Vec::new();
    let mut flows: Vec<Choice> = Vec::new();
    let mut same: usize = 0;
    let given: BM<String, String> = actors_for(d, seed)?;
    let wrapped: BM<String, &'static str> = flow_wraps(d, seed);
    for (loc, item) in &seed.placements {
        if crate::pot_slice::targets::is_loc(d, loc) || crate::slice::skulls::is_skull(d, loc) {
            continue;
        }
        if is_sale(d, loc) {
            if !d.flow_ok(loc) && !d.song_ok(loc) {
                return Err(E::Msg(format!("{loc} cannot be placed, its reward event is not supported")));
            }
            if crate::scrubs_a::home(d, loc, item) {
                same += 1;
                continue;
            }
            if wrapped.contains_key(loc) {
                continue;
            }
            let actor: String = fixed.get(loc).or_else(|| given.get(loc)).cloned().ok_or_else(|| E::Msg(format!("{loc} was given no item actor")))?;
            if Some(actor.as_str()) == d.flow.by_name(loc).and_then(|l| l.item.as_deref()) {
                same += 1;
            } else {
                flows.push(Choice { location: loc.clone(), item: actor });
            }
            continue;
        }
        if let Some(l) = d.cat.by_name(loc).filter(|l| pu::is_world(l)) {
            let t: Vec<Target> = pu::targets_with(d, l, item, extra, fixed.get(loc).map(|a| a.as_str()))?;
            if t.is_empty() {
                same += 1;
            } else {
                out.extend(t);
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
                    key: None,
                });
            }
            continue;
        }
        match d.state_of(loc) {
            Some(State::Eligible) => {}
            Some(State::Event) if d.event_ok(loc) => {}
            None if pu::is_held(loc) => {}
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
            Some(f) => crate::pregrant::chest_drop(f)?.to_string(),
            None => fixed.get(loc).or_else(|| given.get(loc)).cloned().ok_or_else(|| E::Msg(format!("{loc} was given no item actor")))?,
        };
        let key: Option<&'static str> = if roam && new.kind == "SmallKey" {
            let name: &str = new.dungeon.as_deref().unwrap_or("");
            Some(key_dungeon(name).ok_or_else(|| E::Msg(format!("{item} cannot be placed in a chest, no key count belongs to it")))?)
        } else {
            None
        };
        let l: &crate::catalog::Loc = d.cat.by_name(loc).ok_or_else(|| E::Msg(format!("{loc} is not in the catalog")))?;
        if family.is_none() && key.is_none() && new_actor == l.item {
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
                key,
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
    edit_file_with(file, raw, ts, false)
}

pub fn edit_file_with(file: &str, raw: &[u8], ts: &[Target], keys: bool) -> Res<Option<Edited>> {
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
            match pu::swap_gyaml(&mut y, t.hash, &t.old_actor, &t.new_actor) {
                0 => set_drop_ev(&mut y, t.hash, &t.old_actor, &t.new_actor, t.ev)?,
                k => k,
            }
        };
        if n > 0 {
            crate::slice::treasure::set_view(&mut y, t.hash, &t.new_actor);
            if let Some(k) = t.key {
                place_key(&mut y, t.hash, k).ok_or(E::Format("edited key chest vanished"))?;
            }
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
    let swept: usize = if keys { sweep(&mut y, file)? } else { 0 };
    if hits.is_empty() && swept == 0 {
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

fn coop_word(seed: &str) -> String {
    let s: String = seed
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(63)
        .collect();
    if s.is_empty() { "seed".to_string() } else { s }
}

pub fn build(seed: &SeedFile, romfs: &P, load: &P, mod_name: &str) -> Res<Built> {
    build_for(seed, romfs, load, mod_name, Host::Eden)
}

/// The item actors the skipped story would have handed out at `locs`: the placed item, or the
/// vanilla one when the location is not shuffled. Each must have an actor the pouch can take.
/// A progressive item given more than once steps through its tiers, so two ocarinas become the
/// fairy one and the Ocarina of Time.
pub fn skipped_items(d: &Data, seed: &SeedFile, locs: &[&str]) -> Res<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    let mut given: BM<String, usize> = BM::new();
    for loc in locs {
        let l = d.flow.by_name(loc).ok_or_else(|| E::Msg(format!("{loc} is not in the flow location list")))?;
        let Some(name) = seed.placements.get(*loc) else {
            out.push(l.item.clone().ok_or_else(|| E::Msg(format!("{loc} has no vanilla item actor")))?);
            continue;
        };
        let i = d.item(name).ok_or_else(|| E::Msg(format!("{loc} holds {name}, which is not an item")))?;
        let k: usize = *given.get(name).unwrap_or(&0);
        let actor: Option<String> = if i.tiers.is_empty() { i.actor.clone() } else { i.tiers.get(k).cloned() };
        let actor: String = actor.ok_or_else(|| E::Msg(format!("{loc} holds {name}, which cannot be given when a new game starts")))?;
        given.insert(name.clone(), k + 1);
        out.push(actor);
    }
    Ok(out)
}

/// The story state a plan writes at new game. The Door of Time is not driven this way: moving
/// its quest past Zelda's escape would turn the Ocarina of Time into a starting item, so the
/// door keeps its logic edits instead. Zora's Fountain is crate::open_world's (flag list default
/// plus flow guards), so start_state::fountain is not used here.
pub fn start_state(d: &Data, seed: &SeedFile, p: &crate::slice::support::Plan) -> Res<crate::start_state::Ops> {
    let mut ops: crate::start_state::Ops = crate::start_state::Ops::default();
    if p.kakariko {
        ops.extend(crate::start_state::kakariko(&skipped_items(d, seed, &crate::start_state::KAKARIKO_LOCATIONS)?));
    }
    if p.intro {
        ops.extend(crate::start_state::first_visit());
    }
    Ok(ops)
}

/// The locations whose items a plan gives at new game instead of in the world.
pub fn skipped_locations(p: &crate::slice::support::Plan) -> Vec<&'static str> {
    if p.kakariko { crate::start_state::KAKARIKO_LOCATIONS.to_vec() } else { Vec::new() }
}

pub fn build_for(seed: &SeedFile, romfs: &P, load: &P, mod_name: &str, host: Host) -> Res<Built> {
    crate::slice::seed_packs::build(seed, romfs, load, mod_name, host)
}

pub fn build_with(seed: &SeedFile, romfs: &P, load: &P, mod_name: &str, host: Host, extra: &Extra) -> Res<Built> {
    let d: Data = Data::load()?;
    let loaded = seed.settings()?;
    let p = plan(&loaded.settings)?;
    let d: Data = crate::slice::treasure::with(d, p.treasure);
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
    let roam: bool = p.keys_roam();
    let keys: Vec<&'static str> = if roam { KEY_DUNGEONS.to_vec() } else { Vec::new() };
    let order: crate::slice::tier_order::Verdict = crate::slice::tier_order::fixed_actors(&d, seed)?;
    let (ts, same, flows): (Vec<Target>, usize, Vec<Choice>) = targets_with(&d, seed, roam, &order.assign, extra)?;
    let skull_drops: Vec<crate::slice::skull_edit::Drop> = crate::slice::skull_edit::drops_with(&d, seed, &order.assign)?;
    crate::slice::skull_kit::check(&mut rf, &skull_drops, extra)?;
    let kit_pool: &Extra = extra;
    crate::slice::swap_guard::check(&mut rf, &d, &ts, extra)?;
    let mut skull_hit: BS<u64> = BS::new();
    let gr: crate::dungeon_grant::Grants = crate::slice::dungeon_items::grants(&d, seed, &p)?;
    let mut marks: crate::dungeon_grant::Marks = crate::dungeon_grant::Marks::default();
    let mut edits: Vec<Edited> = Vec::new();
    for f in banc_files(&rf)? {
        let raw: Vec<u8> = rf.read_zs(&f)?;
        let e: Option<Edited> = edit_file_with(&f, &raw, &ts, roam)?;
        let e: Option<Edited> = crate::slice::dungeon_items::apply(&f, &raw, e, &gr, &mut marks)?;
        if let Some(e) = crate::slice::skull_edit::apply(&f, &raw, e, &skull_drops, &mut skull_hit)? {
            edits.push(e);
        }
    }
    crate::slice::skull_edit::all_found(&skull_drops, &skull_hit)?;
    crate::dungeon_grant::all_marked(&gr, &marks)?;
    let pot_ts: Vec<crate::pot_slice::targets::Give> = crate::pot_slice::targets::targets_with(&d, seed, &order.assign)?;
    crate::pot_slice::apply::apply(&mut rf, &d, &pot_ts, &mut edits)?;
    let scrub_jobs: Vec<crate::scrubs_b::Job> = crate::scrubs_b::jobs(&d, seed)?;
    crate::scrubs_b::stage_banc(&mut rf, &scrub_jobs, &mut edits)?;
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
    let pk: Pack = Pack::begin_for(load, TITLE_ID, mod_name, host)?;
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
    let mut scrub_rows: usize = 0;
    let wraps: Vec<(u64, String)> = edits.iter().flat_map(|e| e.wraps.clone()).collect();
    let touched: Vec<u64> = edits.iter().flat_map(|e| e.touched.clone()).collect();
    let (special_outs, kids): (Vec<crate::staging::Out>, Vec<(String, String)>) = if wraps.is_empty() && !touched.contains(&special::DAMPE_HOOK) {
        (Vec::new(), Vec::new())
    } else {
        special::stage(&mut rf, &wraps, &touched)?
    };
    let placed: BM<String, &'static str> = flow_wraps(&d, seed);
    let chest_fams: BS<&str> = upgrades.iter().map(|u| u.0.as_str()).collect();
    let gs: Vec<crate::progressive::Grant> = wrap_grants(&d, seed, &chest_fams, &placed);
    if !upgrades.is_empty() || roam || gr.active() {
        let mut outs: Vec<crate::staging::Out> = if !upgrades.is_empty() || roam { stage_upgrades(&mut rf, &upgrades, None, &kids, &keys)? } else { Vec::new() };
        crate::dungeon_grant::stage(&mut rf, &mut outs, &gr, &marks.used)?;
        scrub_rows = crate::scrubs_b::stage_rows(&mut rf, &scrub_jobs, &mut outs, false)?;
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
    let mut staged: Stage = flow_locations::stage_merged(&mut rf, &d.flow, &flows, &gs, &FlowOpts { songs: p.songs != Songs::Off, ..FlowOpts::default() })?;
    crate::scrubs_b::stage_flow(&mut rf, &scrub_jobs, &mut staged.files)?;
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
    if p.hints || roam || p.npc.any() {
        more.extend(hints::stage(&mut rf, &d, seed, p.hints, &keys, &p.npc)?);
    }
    crate::dungeon_grant::stage_text(&mut rf, &mut more, &marks.used)?;
    if roam {
        more.push(stage_pouch(&mut rf)?);
    }
    more.extend(crate::bomb_lift::stage(&mut rf)?);
    more.extend(crate::door_fix::stage(&mut rf)?);
    if p.door {
        more.extend(crate::door_open::stage(&mut rf)?);
    }
    let reward: Option<&str> = crate::start_reward::pick(&seed.seed, p.start).map(|(_, a)| a);
    let flags: Vec<(String, bool)> = p.start_flags();
    let mut give: Vec<&str> = p.pouch.clone();
    if let Some(a) = reward {
        give.push(a);
    }
    if !give.is_empty() || !flags.is_empty() {
        more.push(crate::start_reward::stage_all(&mut rf, &give, &flags)?);
    }
    crate::start_items::stage::stage(&mut rf, &mut more, &p.kit)?;
    crate::pot_slice::rows::stage(&mut rf, &mut more, &crate::pot_slice::apply::hashes(&pot_ts))?;
    crate::start_defaults::stage(&mut rf, &mut more, &crate::start_defaults::presets::chosen(&p.defaults())?)?;
    if scrub_rows == 0 {
        crate::scrubs_b::stage_rows(&mut rf, &scrub_jobs, &mut more, true)?;
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
    let mut state: crate::start_state::Ops = start_state(&d, seed, &p)?;
    if !state.items.is_empty() {
        let l: crate::flags::FlagList = crate::flags_build::load(&mut rf)?;
        crate::start_state::add_is_get(&mut state, |n| l.by_hash(crate::flags::hash_name(n) as u64).is_some());
    }
    if p.intro || !state.is_empty() || p.opening_step().is_some() {
        let o: crate::intro_skip::Opts = if p.intro { crate::intro_skip::Opts::FULL } else { crate::intro_skip::Opts::default() };
        let (mut outs, _): (Vec<crate::staging::Out>, crate::intro_skip::Report) = crate::intro_skip::stage_with(&mut rf, o, &state)?;
        if let Some(st) = p.opening_step() {
            crate::start_defaults::opening::stage(&mut rf, &mut outs, st)?;
        }
        if p.kakariko {
            outs.push(crate::start_state::stage_free_quest(&mut rf)?);
            outs.push(crate::start_state::stage_field_clock(&mut rf)?);
        }
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
    let mut late: Vec<crate::staging::Out> = if p.qol.any() { crate::qol::worked(&mut rf, &pk, &p.qol)? } else { Vec::new() };
    late.extend(crate::open_world::worked(&mut rf, &pk, &p.open)?);
    for o in late {
        pk.write_romfs(&o.path, &o.z)?;
        extra += 1;
        if o.new > o.old || o.path.ends_with(".pack.zs") {
            if rst.is_none() {
                rst = Some(rsizetable::load(&mut rf)?);
            }
            if let Some(t) = rst.as_mut() {
                grown += crate::staging::grow_out(&mut rf, t, &o)?;
            }
        }
    }
    extra += crate::slice::skull_kit::write(&mut rf, &pk, &mut rst, &skull_drops, kit_pool)?;
    if host == Host::Hookshot && rst.is_none() {
        rst = Some(rsizetable::load(&mut rf)?);
    }
    let mut unread: Vec<String> = Vec::new();
    if let Some(t) = &mut rst {
        if host == Host::Hookshot {
            let (_n, missed): (usize, Vec<String>) = rsizetable::fold_mods(t, load, mod_name)?;
            unread = missed;
        }
        pk.write_romfs(rsizetable::TABLE_PATH, &t.to_zs()?)?;
    }
    let chests: Vec<ChestLine> = lines(&ts, &edits);
    let side: usize = seed.placements.keys().filter(|k| crate::pot_slice::targets::is_loc(&d, k) || crate::slice::skulls::is_skull(&d, k)).count();
    let counts: BuildCounts = BuildCounts {
        placed: seed.placements.len() - side,
        changed: chests.len(),
        unchanged: same,
        left_vanilla: d.scan.rows.len().saturating_sub(seed.placements.len() - side),
        files: edits.len() + staged.files.len() + extra,
        grown,
        flows: flows.len() + placed.len(),
    };
    let mut notes: Vec<String> = p.notes.clone();
    if p.world != crate::slice::support::Mode::Off && !seed.placements.keys().any(|k| pu::listed(&d, k)) {
        notes.retain(|n| n != pu::NOTE);
    }
    if !state.is_empty() {
        notes.push(format!(
            "A new game writes the skipped story: {} quest steps, {} quest states, {} flags, and these items: {}.",
            state.steps.len(),
            state.enums.len(),
            state.flags.len(),
            state.items.join(", ")
        ));
    }
    notes.extend(crate::slice::skull_edit::notes(&skull_drops));
    for b in &order.bad {
        notes.push(format!("Upgrade tier warning, generate this seed again with this build: {b}"));
    }
    notes.extend(crate::scrubs_b::notes(&scrub_jobs));
    if !upgrades.is_empty() {
        let n: usize = ts.iter().filter(|t| t.family.is_some()).map(|t| t.name.as_str()).collect::<BS<&str>>().len();
        notes.push(format!(
            "{n} chests hold upgrade items that the game resolves when the chest opens, so the first one found is always the first tier. This replaces the chest event flows, so remove any other mod that changes Dm_SY_Treasurebox or Dm_SY_TreasureboxSmall."
        ));
    }
    if !pot_ts.is_empty() {
        let n: usize = pot_ts.iter().map(|t| t.name.as_str()).collect::<BS<&str>>().len();
        notes.push(format!("{n} pots, crates or flying pots hold other items and give them once, none of these edits is tested in the game yet:"));
        notes.extend(crate::pot_slice::apply::lines(&pot_ts, LISTED));
        if pot_ts.iter().any(|t| crate::pot_slice::pickup::of(&t.item).is_none() && t.kind != crate::pot_slice::data::Kind::Crate) {
            notes.push(crate::slice::key_spots::LOST.to_string());
        }
    }
    if !flows.is_empty() {
        notes.push(format!("{} reward events changed in {} flow files, none of these edits is tested in the game yet:", flows.len(), staged.files.len()));
        for c in staged.changes.iter().take(LISTED) {
            notes.push(format!("  {} | {} | {} ({}) | {} -> {}", c.location, c.file, c.event, c.index, c.old, c.new));
        }
    }
    if host == Host::Hookshot {
        notes.push("Written for Hookshot, under mods/<name>/romfs. Name the mod randomizer-<seed> so co-op reads coop-seed.txt. This mod keeps the larger size-table entry from every other mod that is on.".to_string());
        if !unread.is_empty() {
            notes.push(format!(
                "Could not read the size table in: {}. Those entries were not copied, so turn this mod off or fix that table before you play.",
                unread.join(", ")
            ));
        }
    }
    if host == Host::Nx2navi {
        notes.push("Written for NX2navi, under mods/<name>/romfs. Turn it on in the launcher. The higher row in the load order wins a file both mods change.".to_string());
    }
    if !placed.is_empty() {
        notes.push(format!("{} flow locations hand out upgrade items that the game resolves by ownership, none of these edits is tested in the game yet.", placed.len()));
    }
    pk.commit()?;
    if host == Host::Hookshot {
        std::fs::write(pk.root.join("coop-seed.txt"), format!("{}\n", coop_word(&seed.seed)))?;
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
            key: None,
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
        let mut s: Settings = Settings::default();
        s.set("shuffle_rewards", crate::settings::Value::s("off")).unwrap();
        for n in 0..3 {
            let g = generate_data(&d, &s, &format!("tiers {n}"), &GenOpts::default()).unwrap();
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
                        assert_eq!(t.new_actor, crate::pregrant::chest_drop(f).unwrap());
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
    fn pot_placements_leave_the_chest_actor_pools_balanced() {
        use crate::settings::{Settings, Value};
        use crate::slice::generate::{generate_data, GenOpts};
        let d: Data = Data::load().unwrap();
        let mut s: Settings = Settings::default();
        s.set("shuffle_rewards", Value::s("off")).unwrap();
        s.set("shuffle_pots", Value::s("all")).unwrap();
        for n in 0..3 {
            let g = generate_data(&d, &s, &format!("potpool {n}"), &GenOpts::default()).unwrap();
            let given: BM<String, String> = actors_for(&d, &g.seed_file).unwrap();
            let pots: usize = g.seed_file.placements.keys().filter(|k| d.pots.has(k)).count();
            assert!(pots > 20);
            assert!(g.seed_file.placements.keys().filter(|k| d.pots.has(k)).all(|k| !given.contains_key(k.as_str())));
            let (ts, same, flows): (Vec<Target>, usize, Vec<Choice>) = targets(&d, &g.seed_file).unwrap();
            let moved: BS<&str> = ts.iter().map(|t| t.name.as_str()).collect();
            assert_eq!(moved.len() + same + flows.len() + pots, g.seed_file.placements.len());
        }
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
        let mut vanilla: Settings = Settings::default();
        vanilla.set("shuffle_rewards", Value::s("off")).unwrap();
        let off = generate_data(&d, &vanilla, "rewards", &GenOpts::default()).unwrap();
        let (_, _, none): (Vec<Target>, usize, Vec<Choice>) = targets(&d, &off.seed_file).unwrap();
        assert!(none.is_empty());
    }

    #[test]
    fn every_flow_placement_is_a_choice_or_a_wrap_and_rupee_forms_follow_the_destination() {
        use crate::settings::{Settings, Value};
        use crate::slice::generate::{generate_data, GenOpts};
        let d: Data = Data::load().unwrap();
        let mut st: Settings = Settings::default();
        st.set("shuffle_chests", Value::s("all")).unwrap();
        st.set("shuffle_rewards", Value::s("supported")).unwrap();
        for n in 0..3 {
            let g = generate_data(&d, &st, &format!("split {n}"), &GenOpts::default()).unwrap();
            let (ts, same, flows): (Vec<Target>, usize, Vec<Choice>) = targets(&d, &g.seed_file).unwrap();
            let wrapped: BM<String, &'static str> = flow_wraps(&d, &g.seed_file);
            let moved: BS<&str> = ts.iter().map(|t| t.name.as_str()).collect();
            assert_eq!(moved.len() + same + flows.len() + wrapped.len(), g.seed_file.placements.len(), "seed split {n}");
            for (loc, f) in &wrapped {
                assert_eq!(d.family(&g.seed_file.placements[loc]), Some(*f));
                assert!(d.flow_ok(loc));
            }
            for c in &flows {
                assert!(d.flow.usable(&c.item), "{} -> {}", c.location, c.item);
            }
            let given: BM<String, String> = actors_for(&d, &g.seed_file).unwrap();
            for (loc, actor) in &given {
                if !is_sale(&d, loc) {
                    assert!(!actor.starts_with("Obj_Rupee"), "{loc} {actor}");
                }
            }
        }
    }
}
