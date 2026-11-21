use std::collections::{BTreeMap as BM, BTreeSet as BS};
use std::path::{Path as P, PathBuf as PB};

use crate::TITLE_ID;
use crate::byml::Byml;
use crate::dungeon_credit::KEY_DUNGEONS;
use crate::flow_locations::{self, Choice, Opts as FlowOpts, Stage};
use crate::error::{Error as E, Res};
use crate::modpack::{Host, Pack};
use crate::pickup_persist::Extra;
use crate::placement::banc_files;
use crate::progressive::{stage as stage_upgrades, Size};
use crate::romfs::Romfs;
use crate::rsizetable::{self, Rst};
use crate::settings::seed::SeedFile;
use crate::slice::support::Songs;
use crate::special;
use crate::slice::banc::fingerprint;
use crate::slice::data::Data;
use crate::slice::flow_wrap::{grants as wrap_grants, wraps as flow_wraps};
use crate::slice::pickups as pu;
use crate::slice::support::plan;
use crate::zs;

pub use crate::slice::banc_edit::{Edited, edit_file, edit_file_with};
pub use crate::slice::chest_targets::{Target, is_sale};
pub(crate) use crate::slice::chest_targets::targets_with;
#[cfg(test)]
pub(crate) use crate::slice::chest_targets::{actors_for, targets};
pub(crate) use crate::slice::build_report::{coop_word, describe, lines};
#[cfg(test)]
pub(crate) use crate::slice::build_report::coords;

pub const MARGIN: u32 = 0x400;
pub const LISTED: usize = 10;


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
    if p.gates.kakariko {
        ops.extend(crate::start_state::kakariko(&skipped_items(d, seed, &crate::start_state::KAKARIKO_LOCATIONS)?));
    }
    if p.intro {
        ops.extend(crate::start_state::first_visit());
    }
    if p.gates.forest == crate::slice::support::Forest::Open {
        ops.extend(crate::story_skip::deku());
    }
    if p.story {
        ops.extend(crate::story_skip::start());
    }
    let seen: crate::start_state::Ops = crate::start_state::visits(&ops);
    ops.extend(seen);
    Ok(ops)
}

/// The locations whose items a plan gives at new game instead of in the world.
pub fn skipped_locations(p: &crate::slice::support::Plan) -> Vec<&'static str> {
    if p.gates.kakariko { crate::start_state::KAKARIKO_LOCATIONS.to_vec() } else { Vec::new() }
}

pub fn build_for(seed: &SeedFile, romfs: &P, load: &P, mod_name: &str, host: Host) -> Res<Built> {
    let b: Built = crate::slice::seed_packs::build(seed, romfs, load, mod_name, host)?;
    crate::shelf::seed::after(seed, romfs, load, mod_name, host, b)
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
    let gift_keys: Vec<crate::gift_keys::Gift> = crate::gift_keys::gifts(&d, seed)?;
    let mut keys: Vec<&'static str> = if roam { KEY_DUNGEONS.to_vec() } else { Vec::new() };
    for k in crate::gift_keys::small_dungeons(&gift_keys) {
        if !keys.contains(&k) {
            keys.push(k);
        }
    }
    let order: crate::slice::tier_order::Verdict = crate::slice::tier_order::fixed_actors(&d, seed)?;
    let (ts, same, flows): (Vec<Target>, usize, Vec<Choice>) = targets_with(&d, seed, roam, &order.assign, extra)?;
    let skull_drops: Vec<crate::slice::skull_edit::Drop> = crate::slice::skull_edit::drops_with(&d, seed, &order.assign)?;
    crate::slice::skull_kit::check(&mut rf, &skull_drops, extra)?;
    let kit_pool: &Extra = extra;
    crate::slice::swap_guard::check(&mut rf, &d, &ts, extra)?;
    let gr: crate::dungeon_grant::Grants = crate::slice::dungeon_items::grants(&d, seed, &p)?;
    let silver: BM<u64, &'static str> = crate::slice::silver::grants(&d, seed)?;
    let mut passes: crate::slice::map_passes::State = crate::slice::map_passes::State::new(&silver, &gr, &skull_drops);
    let mut edits: Vec<Edited> = Vec::new();
    let size_ctx: crate::chest_size::Ctx = crate::chest_size::Ctx { chus: p.chus, owned: p.start.owned.clone() };
    let sizes: BM<u64, crate::chest_size::Want> = if p.chest_sizes { crate::chest_size::wanted(&d, seed, &size_ctx, crate::slice::dungeon_items::credited(p.keys.map)) } else { BM::new() };
    let small_hearts: BS<u64> = if p.chest_sizes { crate::chest_size::small_hearts(&d, seed, &size_ctx, crate::slice::dungeon_items::credited(p.keys.map)) } else { BS::new() };
    let moves: crate::chest_size::Moves = if sizes.is_empty() { crate::chest_size::Moves::default() } else { crate::chest_size::moves(&crate::chest_size::scan(&mut rf, &sizes)?, &sizes) };
    let mut resized_n: usize = 0;
    // Freestanding spots swapped to a seed pickup stand higher, so the turning item model clears
    // the ground (crate::pickup_pack::lift).
    let lifts: BM<String, f32> = {
        let info: crate::byml::Byml = rf.read_byml(crate::pickup_pack::rows::ACTOR_INFO)?;
        ts.iter().filter(|t| extra.has(&t.new_actor)).filter_map(|t| crate::pickup_pack::lift::for_pack(&info, &t.new_actor).map(|dy| (t.new_actor.clone(), dy))).collect()
    };
    for f in banc_files(&rf)? {
        let vanilla: Vec<u8> = rf.read_zs(&f)?;
        let resized: Option<Vec<u8>> = crate::chest_size::resize(&vanilla, &f, &sizes, &moves)?.map(|(b, n)| {
            resized_n += n;
            b
        });
        let raw: &[u8] = resized.as_deref().unwrap_or(&vanilla);
        let e: Option<Edited> = edit_file_with(&f, raw, &ts, roam, &small_hearts, &lifts)?;
        let e: Option<Edited> = match (e, &resized) {
            (Some(mut e), Some(_)) => {
                e.old_len = vanilla.len();
                Some(e)
            }
            (None, Some(b)) => Some(Edited {
                file: f.clone(),
                old_len: vanilla.len(),
                bytes: b.clone(),
                hits: Vec::new(),
                used: Vec::new(),
                hearts: Vec::new(),
                wraps: Vec::new(),
                touched: Vec::new(),
            }),
            (e, None) => e,
        };
        if let Some(e) = crate::slice::map_passes::run(&f, raw, e, &mut passes)? {
            edits.push(e);
        }
    }
    crate::slice::map_passes::finish(&passes)?;
    let pot_ts: Vec<crate::pot_slice::targets::Give> = crate::pot_slice::targets::targets_with(&d, seed, &order.assign)?;
    crate::pot_slice::apply::apply(&mut rf, &d, &pot_ts, &mut edits)?;
    let scrub_jobs: Vec<crate::scrubs_b::Job> = crate::scrubs_b::jobs(&d, seed)?;
    crate::scrubs_b::stage_banc(&mut rf, &scrub_jobs, &mut edits)?;
    let gate_notes: Vec<String> = crate::entr::build::apply(&mut rf, seed, &mut edits)?;
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
    let retarget: Vec<(u64, Size)> = crate::chest_size::retargets(&sizes, &moves);
    let (mut special_outs, kids): (Vec<crate::staging::Out>, Vec<(String, String)>) = if wraps.is_empty() && retarget.is_empty() && !touched.contains(&special::DAMPE_HOOK) {
        (Vec::new(), Vec::new())
    } else {
        special::stage(&mut rf, &wraps, &retarget, &touched)?
    };
    if !kids.is_empty() && upgrades.is_empty() && !roam {
        // No upgrade stage writes the analyzed table, so the resized chests list their open event here.
        special_outs.push(crate::staging::rewrite(&mut rf, crate::progressive::ANALYZED_TABLE, |raw| {
            let mut y: Byml = Byml::parse(raw)?;
            crate::progressive::add_children(&mut y, &kids);
            y.write()
        })?);
    }
    let placed: BM<String, &'static str> = flow_wraps(&d, seed);
    let chest_fams: BS<&str> = upgrades.iter().map(|u| u.0.as_str()).collect();
    let gs: Vec<crate::progressive::Grant> = wrap_grants(&d, seed, &chest_fams, &placed);
    if !upgrades.is_empty() || roam || gr.active() || !passes.silver_used.is_empty() {
        let mut outs: Vec<crate::staging::Out> = if !upgrades.is_empty() || roam { stage_upgrades(&mut rf, &upgrades, None, &kids, &keys)? } else { Vec::new() };
        crate::dungeon_grant::stage(&mut rf, &mut outs, &gr, &passes.marks.used)?;
        crate::slice::silver::stage(&mut rf, &mut outs, &passes.silver_used)?;
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
    crate::scrub_prices::stage(&mut rf, &d, seed, p.rewards.scrub_prices, &mut staged.files)?;
    crate::merchant_prices::stage_medigoron(&mut rf, seed, p.rewards.merchant_prices, p.rewards.medigoron, &mut staged.files)?;
    if p.songs != Songs::Off {
        crate::song_lessons::stage_flow(&mut rf, &mut staged.files)?;
    }
    // Dungeon items in reward events: window-only copies of their give routines and the credit
    // (crate::gift_keys). The Light Arrows item get wants the copy of its routine too.
    let arrows_key: bool = gift_keys.iter().any(|g| {
        (p.story && (crate::story_skip::DROPPED.contains(&g.location.as_str()) || crate::story_skip::FAIRIES.iter().any(|f| f.0 == g.location))) || (p.endings && crate::story_skip::REWARD_SCENES.iter().any(|r| r.0 == g.location))
    });
    let windows: Vec<(&str, &str)> = if arrows_key { vec![crate::story_skip::ITEM_GET] } else { Vec::new() };
    crate::gift_keys::apply(&mut rf, &mut staged.files, &gift_keys, &windows)?;
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
    let quiet: Vec<&str> = crate::bomb_pickup::wanted(pot_ts.iter().map(|t| t.actor.as_str()).chain(ts.iter().map(|t| t.new_actor.as_str())).chain(skull_drops.iter().map(|x| x.actor.as_str())));
    let work: PB = pk.work.join("romfs");
    let pots: Vec<u64> = crate::pot_slice::apply::hashes(&pot_ts);
    let mut marked: BS<(crate::dungeon_grant::Kind, &'static str)> = passes.marks.used.clone();
    marked.extend(crate::gift_keys::marked(&gift_keys));
    let c: crate::slice::stages::Ctx = crate::slice::stages::Ctx {
        d: &d,
        seed,
        p: &p,
        roam,
        keys: &keys,
        hearts: &hearts,
        marked: &marked,
        silver: &passes.silver_used,
        quiet: &quiet,
        work: &work,
        pots: &pots,
        scrub_jobs: &scrub_jobs,
        scrub_rows,
    };
    crate::slice::stages::run(&mut rf, &c, &mut more)?;
    for o in more {
        pk.write_romfs(&o.path, &o.z)?;
        extra += 1;
        if let Some(peer) = crate::song_models::size_peer(&o.path) {
            if rst.is_none() {
                rst = Some(rsizetable::load(&mut rf)?);
            }
            if let Some(t) = rst.as_mut()
                && let Some(n) = t.get(&peer)
            {
                t.set_size(&o.path, n)?;
                grown += 1;
            }
            continue;
        }
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
    let ocarina_held: bool = crate::ocarina_hold::needed(&d, seed);
    if p.fast {
        extra += crate::fast::apply(&mut rf, &pk)?;
    }
    let mut state: crate::start_state::Ops = start_state(&d, seed, &p)?;
    if !state.items.is_empty() {
        let l: crate::flags::FlagList = crate::flags_build::load(&mut rf)?;
        crate::start_state::add_is_get(&mut state, |n| l.by_hash(crate::flags::hash_name(n) as u64).is_some());
    }
    let clock: bool = p.gates.kakariko || p.open.time;
    if p.intro || !state.is_empty() || p.opening_step().is_some() || clock {
        let o: crate::intro_skip::Opts = if p.intro { crate::intro_skip::Opts::FULL } else { crate::intro_skip::Opts::default() };
        let (mut outs, _): (Vec<crate::staging::Out>, crate::intro_skip::Report) = crate::intro_skip::stage_with(&mut rf, o, &state)?;
        if let Some(st) = p.opening_step() {
            crate::start_defaults::opening::stage(&mut rf, &mut outs, st)?;
        }
        if p.gates.kakariko {
            outs.push(crate::start_state::stage_free_quest(&mut rf)?);
        }
        if clock {
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
    let difficulty_notes: Vec<String> = crate::difficulty::worked(&mut rf, &pk, &p.difficulty, &seed.seed, &mut late)?;
    crate::merchant_prices::stage_bean(&mut rf, seed, p.rewards.merchant_prices, p.rewards.bean_sale, &mut late)?;
    // Scenes that are dropped: Link runs the item get with what their check holds.
    let give_for = |loc: &str| -> crate::story_skip::SceneGive {
        let vanilla: String = d.flow.by_name(loc).and_then(|l| l.item.clone()).unwrap_or_default();
        crate::story_skip::SceneGive {
            actor: flows.iter().find(|c| c.location == loc).map(|c| c.item.clone()).unwrap_or(vanilla),
            // An unshuffled prize keeps the tier check its guard gave it (flow_wrap::PRIZES).
            family: placed.get(loc).copied().or_else(|| {
                if seed.placements.contains_key(loc) {
                    None
                } else {
                    crate::slice::flow_wrap::PRIZES.iter().find(|x| x.0 == loc).map(|x| x.1)
                }
            }),
            key: gift_keys.iter().find(|g| g.location == loc).map(|g| (g.item, g.dungeon)),
            line: if p.debug_names { crate::debug_names::scene_line(&d, loc) } else { None },
        }
    };
    if p.story {
        late.extend(crate::story_skip::worked(&mut rf, &pk)?);
        crate::story_skip::stage_arrows_give(&mut rf, &mut late, &give_for(crate::lacs_key::LOC))?;
        crate::story_skip::stage_saria_give(&mut rf, &mut late, &give_for(crate::slice::flow_rules::SARIA))?;
        crate::story_skip::stage_requiem(&mut rf, &mut late, &work)?;
        crate::story_skip::stage_arrows_row(&mut rf, &mut late, &work)?;
        crate::story_skip::stage_rauru_give(&mut rf, &mut late, &give_for(crate::story_skip::RAURU_LOC))?;
        for (loc, flow) in crate::story_skip::FAIRIES {
            crate::story_skip::stage_fairy_give(&mut rf, &mut late, &work, flow, &give_for(loc))?;
        }
        // Song lessons: the six played from other flows are dropped for the item get, and the
        // grant Sheik's six cut scenes jump to becomes the item get.
        for t in &crate::story_skip::SONG_TRIGGERS {
            let writes: Vec<crate::story_skip::Write> = crate::story_skip::scene_writes(&mut rf, t.give)?;
            let mut gives: Vec<crate::story_skip::SceneGive> = t.also.iter().map(|l| give_for(l)).collect();
            gives.push(give_for(t.loc));
            crate::story_skip::stage_late(&mut rf, &mut late, &work, t.flow, |b| crate::story_skip::patch_song_trigger(b, t, &writes, &gives))?;
        }
    }
    if p.endings {
        late.extend(crate::story_skip::worked_endings(&mut rf, &pk)?);
        let gives: Vec<(&str, &[&str], crate::story_skip::SceneGive)> = crate::story_skip::REWARD_SCENES.iter().map(|(loc, scene, extra)| (*scene, *extra, give_for(loc))).collect();
        crate::story_skip::stage_reward_gives(&mut rf, &mut late, &gives)?;
        crate::story_skip::stage_late(&mut rf, &mut late, &work, crate::story_skip::ARROWS_FLOW, crate::story_skip::patch_last_sage)?;
    }
    crate::fairy_prices::stage(&mut rf, &mut late, &work, p.rewards.fairy_prices, seed)?;
    if p.bridge {
        late.extend(crate::deku_bridge::worked(&mut rf, &pk)?);
    }
    if p.shops.on() {
        late.push(crate::shops::door::worked(&mut rf, &pk)?);
    }
    if p.rainbow.is_some() {
        crate::story_skip::stage_late(&mut rf, &mut late, &work, "AllSage", crate::rainbow_bridge::patch_lacs_step)?;
    }
    // A Bow or Slingshot from any gift or pickup brings its starting bag, as a chest's does.
    crate::starter_bags::stage(&mut rf, &mut late, &work)?;
    if p.short {
        crate::short_get::stage(&mut rf, &mut late, &work)?;
    }
    crate::open_world::stage_last(&mut rf, &mut late, &work)?;
    let mut debug_missed: Vec<String> = Vec::new();
    let mut paths: BS<&str> = BS::new();
    if let Some(o) = late.iter().find(|o| !paths.insert(o.path.as_str())) {
        return Err(E::Msg(format!("build: {} is edited twice in the late files, and the later edit would drop the earlier one", o.path)));
    }
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
    if p.difficulty.backup {
        if rst.is_none() {
            rst = Some(rsizetable::load(&mut rf)?);
        }
        if let Some(t) = rst.as_mut() {
            crate::difficulty::backup::size(&mut rf, t)?;
        }
    }
    // The merged carpenter schedules need their parsed size, not their file growth.
    if rst.is_none() {
        rst = Some(rsizetable::load(&mut rf)?);
    }
    if let Some(t) = rst.as_mut() {
        grown += crate::open_world::fortress::budget(&mut rf, t)?;
    }
    extra += crate::slice::skull_kit::write(&mut rf, &pk, &mut rst, &skull_drops, kit_pool)?;
    let (cow_files, cow_lines): (usize, Vec<String>) = crate::cow_clone::write(&mut rf, &pk, &mut rst)?;
    extra += cow_files;
    let leever_classes: Vec<&str> = if p.difficulty.gate_list().iter().any(|g| g.new_kind == crate::difficulty::gates::LEEVER_GIANT) { vec![crate::difficulty::leevers::SMALL_CLASS, crate::difficulty::leevers::GIANT_CLASS] } else { Vec::new() };
    extra += crate::difficulty::leevers::write(&mut rf, &pk, &mut rst, &leever_classes)?;
    if p.difficulty.encounters {
        extra += crate::difficulty::beamos::write(&mut rf, &pk, &mut rst)?;
    }
    let volvagia_lines: Vec<String> = if p.difficulty.volvagia {
        let (n, l): (usize, Vec<String>) = crate::difficulty::volvagia::write(&mut rf, &pk, &mut rst)?;
        extra += n;
        l
    } else {
        Vec::new()
    };
    let mega_lines: Vec<String> = if p.difficulty.mega_like {
        let (n, l): (usize, Vec<String>) = crate::difficulty::mega_like::write(&mut rf, &pk, &mut rst)?;
        extra += n;
        l
    } else {
        Vec::new()
    };
    let size_lines: Vec<String> = if p.difficulty.sizes.on() && p.difficulty.sized_health {
        let (n, l): (usize, Vec<String>) = crate::difficulty::sizes::write(&mut rf, &pk, &mut rst)?;
        extra += n;
        l
    } else {
        Vec::new()
    };
    if p.open.epona {
        extra += crate::open_world::write_epona_event(&mut rf, &pk, &mut rst)?;
    }
    if p.debug_names {
        let mut tail: Vec<crate::staging::Out> = Vec::new();
        let sales: Vec<(String, String)> = scrub_jobs.iter().map(|j| (j.sale.entry.clone(), j.scrub.location.clone())).collect();
        let mut scenes: BS<&str> = BS::new();
        if p.story {
            scenes.extend([crate::lacs_key::LOC, crate::slice::flow_rules::SARIA, crate::story_skip::RAURU_LOC]);
            scenes.extend(crate::story_skip::FAIRIES.iter().map(|f| f.0));
            for t in &crate::story_skip::SONG_TRIGGERS {
                scenes.insert(t.loc);
                scenes.extend(t.also.iter().copied());
            }
        }
        if p.endings {
            scenes.extend(crate::story_skip::REWARD_SCENES.iter().map(|r| r.0));
        }
        debug_missed = crate::debug_names::stage_flows(&mut rf, &d, &work, &mut tail, &scenes, &sales)?;
        crate::debug_names::stage_sweep(&mut rf, &d, &work, &mut tail)?;
        for o in tail {
            pk.write_romfs(&o.path, &o.z)?;
            extra += 1;
            if rst.is_none() {
                rst = Some(rsizetable::load(&mut rf)?);
            }
            if let Some(t) = rst.as_mut() {
                grown += crate::staging::grow_out(&mut rf, t, &o)?;
            }
        }
    }
    let loads: Option<(crate::staging::Out, crate::flow_extra::analyzed::Added)> = crate::flow_extra::analyzed::worked(&mut rf, &pk.work.join("romfs"))?;
    if let Some((o, _)) = &loads {
        pk.write_romfs(&o.path, &o.z)?;
        extra += 1;
        if o.new > o.old {
            if rst.is_none() {
                rst = Some(rsizetable::load(&mut rf)?);
            }
            if let Some(t) = rst.as_mut() {
                grown += crate::staging::grow_out(&mut rf, t, o)?;
            }
        }
    }
    if host.one_table() && rst.is_none() {
        rst = Some(rsizetable::load(&mut rf)?);
    }
    let mut fold: Option<rsizetable::Fold> = None;
    if let Some(t) = &mut rst {
        if host.one_table() {
            fold = Some(rsizetable::fold_mods(t, load, mod_name)?);
        }
        pk.write_romfs(rsizetable::TABLE_PATH, &t.to_zs()?)?;
    }
    let chests: Vec<ChestLine> = lines(&ts, &edits);
    let side: usize = seed.placements.keys().filter(|k| crate::pot_slice::targets::is_loc(&d, k) || crate::slice::skulls::is_skull(&d, k) || crate::shops::seed::skip(k)).count();
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
    notes.extend(difficulty_notes);
    if !debug_missed.is_empty() {
        notes.push(format!("Debug location names could not hook: {}", debug_missed.join(", ")));
    }
    if ocarina_held {
        notes.push(crate::ocarina_hold::NOTE.to_string());
    }
    if p.chest_sizes {
        notes.push(crate::chest_size::report(resized_n + small_hearts.len(), moves.tight.len()));
    }
    if p.world.on() && !seed.placements.keys().any(|k| pu::listed(&d, k)) {
        notes.retain(|n| !pu::is_note(n));
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
    notes.extend(gate_notes);
    if !upgrades.is_empty() {
        let n: usize = ts.iter().filter(|t| t.family.is_some()).map(|t| t.name.as_str()).collect::<BS<&str>>().len();
        notes.push(format!(
            "{n} chests hold upgrade items that the game resolves when the chest opens, so the first one found is always the first tier. This replaces the chest event flows, so remove any other mod that changes Dm_SY_Treasurebox or Dm_SY_TreasureboxSmall."
        ));
    }
    if !pot_ts.is_empty() {
        let n: usize = pot_ts.iter().filter(|t| !t.kind.wild()).map(|t| t.name.as_str()).collect::<BS<&str>>().len();
        let w: usize = pot_ts.iter().filter(|t| t.kind.wild()).map(|t| t.name.as_str()).collect::<BS<&str>>().len();
        if w == 0 {
            notes.push(format!("{n} pots, crates, flying pots or beehives hold other items, none of these edits is tested in the game yet:"));
        } else {
            notes.push(format!("{n} pots, crates, flying pots or beehives and {w} rocks or grass tufts hold other items, none of these edits is tested in the game yet:"));
        }
        notes.extend(crate::pot_slice::apply::lines(&pot_ts, LISTED));
        notes.push(crate::slice::key_spots::LOST.to_string());
    }
    if !flows.is_empty() {
        notes.push(format!("{} reward events changed in {} flow files:", flows.len(), staged.files.len()));
        for c in staged.changes.iter().take(LISTED) {
            notes.push(format!("  {} | {} | {} ({}) | {} -> {}", c.location, c.file, c.event, c.index, c.old, c.new));
        }
    }
    if !cow_lines.is_empty() {
        notes.push("Cows made into actor classes of their own, untested in the game:".to_string());
        notes.extend(cow_lines.iter().map(|l| format!("  {l}")));
    }
    if !volvagia_lines.is_empty() {
        notes.push("Volvagia roams, files made:".to_string());
        notes.extend(volvagia_lines.iter().map(|l| format!("  {l}")));
    }
    if !mega_lines.is_empty() {
        notes.push("Mega Like Likes, files made:".to_string());
        notes.extend(mega_lines.iter().map(|l| format!("  {l}")));
    }
    if !size_lines.is_empty() {
        notes.push("Random enemy sizes, files made:".to_string());
        notes.extend(size_lines.iter().map(|l| format!("  {l}")));
    }
    if let Some((_, a)) = &loads {
        notes.push(format!("{} events now load the flows their edited events call:", a.len()));
        notes.extend(crate::flow_extra::analyzed::lines(a));
    }
    if host == Host::Hookshot {
        notes.push("Written for Hookshot, under mods/<name>/romfs. Name the mod randomizer-<seed> so co-op reads coop-seed.txt. This mod keeps the larger size-table entry from every other mod that is on.".to_string());
    }
    if host == Host::Longshot {
        notes.push("Written for Longshot, under mods/<name>/romfs beside longshot.exe. This mod keeps the larger size-table entry from every other mod that is on, and Longshot adds the button prompts' entries when the game starts.".to_string());
    }
    if let Some(f) = &fold {
        if !f.unread.is_empty() {
            notes.push(format!(
                "Could not read the size table in: {}. Those entries were not copied, so turn this mod off or fix that table before you play.",
                f.unread.join(", ")
            ));
        }
        if host == Host::Longshot && !f.later.is_empty() {
            notes.push(format!(
                "The game uses the size table of the mod whose folder name sorts last, and {} sorts after this one. Rename this mod so it sorts last (for example zz-{mod_name}), or the larger files here may not load.",
                f.later.join(", ")
            ));
        }
    }
    if host == Host::Nx2navi {
        notes.push("Written for NX2navi, under mods/<name>/romfs. Turn it on in the launcher. The higher row in the load order wins a file both mods change.".to_string());
    }
    if !placed.is_empty() {
        notes.push(format!("{} flow locations hand out upgrade items that the game resolves by ownership.", placed.len()));
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
    use crate::boss_heart::{CHEST, HEART};
    use crate::progressive::{guarded, needs_every_holder};
    use crate::slice::banc::find;
    use crate::slice::data::State;
    use crate::byml::Node;

    fn real() -> Option<PB> {
        let p: PB = ootrr_game::test_romfs::path();
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Some(p)
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    #[test]
    fn a_longshot_build_folds_the_other_mods_tables_and_writes_no_coop_file() {
        let Some(root) = real() else {
            return;
        };
        let d: Data = Data::load().unwrap();
        let g = crate::slice::generate::generate_data(&d, &crate::settings::Settings::default(), "longshot build", &crate::slice::generate::GenOpts::default()).unwrap();
        let load: PB = PB::from(env!("CARGO_MANIFEST_DIR")).join("../../target/longshot_mods");
        let _ = std::fs::remove_dir_all(&load);
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut other: Rst = rsizetable::load(&mut rf).unwrap();
        other.set_size("OotrrTest/only_in_other.bin", 4321).unwrap();
        for m in ["00-other", "zz-late"] {
            let p: PB = load.join(m).join("romfs").join(rsizetable::TABLE_PATH);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, other.to_zs().unwrap()).unwrap();
        }
        let b: Built = build_for(&g.seed_file, &root, &load, "ootrr-longshot", Host::Longshot).unwrap();
        assert_eq!(b.root, load.join("ootrr-longshot"));
        assert!(!b.root.join("coop-seed.txt").exists());
        let ours: Rst = Rst::parse(&rf.zs.decompress(&std::fs::read(b.root.join("romfs").join(rsizetable::TABLE_PATH)).unwrap()).unwrap()).unwrap();
        assert_eq!(ours.get("OotrrTest/only_in_other.bin"), Some(4321));
        assert!(b.report.contains("Written for Longshot"), "{}", b.report);
        assert!(b.report.contains("zz-late sorts after this one"), "{}", b.report);
        let _ = std::fs::remove_dir_all(&load);
    }

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
            heart_chest: false,
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
    fn a_boss_heart_holding_a_minor_item_becomes_a_small_chest() {
        let heart = |h: u64| -> Node {
            Node::Map(vec![
                ("Gyaml".to_string(), Node::Str(HEART.to_string())),
                ("Hash".to_string(), Node::U64(h)),
                ("Translate".to_string(), Node::Arr(vec![Node::F32(1.0), Node::F32(2.0), Node::F32(3.0)])),
            ])
        };
        let b: Vec<u8> = raw(vec![heart(1), heart(2)]);
        let chest = |h: u64, new: &str| Target { heart_chest: true, ..target(h, HEART, new) };
        let ts: Vec<Target> = vec![chest(1, "TBox_Rupee_Red"), chest(2, "Item_Hookshot")];
        let e: Edited = edit_file_with("Banc/Area/a.bcett.byml.zs", &b, &ts, false, &BS::from([1]), &BM::new()).unwrap().unwrap();
        let back: Byml = Byml::parse(&e.bytes).unwrap();
        let g: Vec<&str> = back.root.get("Actors").unwrap().as_arr().unwrap().iter().map(|a| a.get("Gyaml").unwrap().as_str().unwrap()).collect();
        assert_eq!(g, vec!["Obj_TBox_A_01", CHEST]);
        assert_eq!(e.hearts.len(), 2);
    }

    #[test]
    fn a_boss_heart_with_a_ground_pickup_becomes_that_pickup_and_needs_no_unlock() {
        let heart = |h: u64| -> Node {
            Node::Map(vec![
                ("Gyaml".to_string(), Node::Str(HEART.to_string())),
                ("Hash".to_string(), Node::U64(h)),
                ("Translate".to_string(), Node::Arr(vec![Node::F32(1.0), Node::F32(2.0), Node::F32(3.0)])),
            ])
        };
        let b: Vec<u8> = raw(vec![heart(1)]);
        let e: Edited = edit_file_with("Banc/Area/a.bcett.byml.zs", &b, &[target(1, HEART, "Obj_Hookshot")], false, &BS::new(), &BM::new()).unwrap().unwrap();
        let back: Byml = Byml::parse(&e.bytes).unwrap();
        let a: &Node = &back.root.get("Actors").unwrap().as_arr().unwrap()[0];
        assert_eq!(a.get("Gyaml").and_then(|v| v.as_str()), Some("Obj_Hookshot"));
        assert!(a.get("Dynamic").and_then(|d| d.get("Drop__DropActor")).is_none());
        assert!(e.hearts.is_empty());
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
                // The Gerudo Valley waterfall piece has an actor of its own, and moves as the plain one.
                let v: &str = &d.cat.by_name(loc).unwrap().item;
                before.push(if v == "Obj_GerudoValley_Heart_Piece" { "Obj_Heart_Piece".to_string() } else { v.to_string() });
                after.push(actor.clone());
            }
            before.sort();
            after.sort();
            assert_eq!(before, after, "seed tiers {n}");
            let (ts, same, flows): (Vec<Target>, usize, Vec<Choice>) = targets(&d, &g.seed_file).unwrap();
            let moved: std::collections::BTreeSet<&str> = ts.iter().map(|t| t.name.as_str()).filter(|n| g.seed_file.placements.contains_key(*n)).collect();
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
            // Freestanding spots hand out a fixed tier (slice::tier_order), not a chest wrapper.
            for t in ts.iter().filter(|t| !d.cat.by_name(&t.name).is_some_and(crate::slice::pickups::is_world)) {
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
            let moved: BS<&str> = ts.iter().map(|t| t.name.as_str()).filter(|n| g.seed_file.placements.contains_key(*n)).collect();
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
            // Reward events hand out any item a chest gives, through its chest content actor (#24).
            let allowed: bool = g.seed_file.placements.get(&c.location).and_then(|it| d.item(it)).is_some_and(|i| crate::slice::flow_rules::allows(&d, &c.location, i));
            assert!(d.flow.usable(&c.item) || crate::slice::flow_rules::ANY_POUCH.contains(&c.location.as_str()) || crate::ocarina_hold::is_form(&c.item) || allowed, "{} -> {}", c.location, c.item);
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
            let moved: BS<&str> = ts.iter().map(|t| t.name.as_str()).filter(|n| g.seed_file.placements.contains_key(*n)).collect();
            assert_eq!(moved.len() + same + flows.len() + wrapped.len(), g.seed_file.placements.len(), "seed split {n}");
            for (loc, f) in &wrapped {
                assert_eq!(d.family(&g.seed_file.placements[loc]), Some(*f));
                assert!(d.flow_ok(loc));
            }
            for c in &flows {
                // Reward events hand out any item a chest gives, through its chest content actor (#24).
            let allowed: bool = g.seed_file.placements.get(&c.location).and_then(|it| d.item(it)).is_some_and(|i| crate::slice::flow_rules::allows(&d, &c.location, i));
            assert!(d.flow.usable(&c.item) || crate::slice::flow_rules::ANY_POUCH.contains(&c.location.as_str()) || crate::ocarina_hold::is_form(&c.item) || allowed, "{} -> {}", c.location, c.item);
            }
            let given: BM<String, String> = actors_for(&d, &g.seed_file).unwrap();
            for (loc, actor) in &given {
                if !is_sale(&d, loc) {
                    assert!(!actor.starts_with("Obj_Rupee"), "{loc} {actor}");
                }
            }
        }
    }

    #[test]
    fn a_bomb_chest_always_takes_the_bomb_bag_guard() {
        use crate::settings::Settings;
        let d: Data = Data::load().unwrap();
        let homes: Vec<&crate::catalog::Loc> = d.cat.locs.iter().filter(|l| l.kind == "chest" && guarded(&l.item) && d.state_of(&l.name) == Some(State::Eligible)).collect();
        assert!(!homes.is_empty());
        for l in &homes {
            let (home, item): (&str, &str) = (l.name.as_str(), d.vanilla_of(&l.name).unwrap());
            let m: BM<String, String> = [(home.to_string(), item.to_string())].into_iter().collect();
            let s: SeedFile = SeedFile::new("s", &Settings::default(), &d.scan.game, m).unwrap();
            let (ts, _, _): (Vec<Target>, usize, Vec<Choice>) = targets(&d, &s).unwrap();
            assert!(ts.iter().any(|t| t.name == home && guarded(&t.new_actor)), "{item}");
        }
        let s: SeedFile = SeedFile::new("s", &Settings::default(), &d.scan.game, BM::new()).unwrap();
        let (ts, _, _): (Vec<Target>, usize, Vec<Choice>) = targets(&d, &s).unwrap();
        for l in &homes {
            assert!(ts.iter().any(|t| t.name == l.name && t.new_actor == l.item), "{}", l.name);
        }
    }
}
