use std::env;
use std::path::PathBuf as PB;
use std::sync::{Arc, MutexGuard};
use std::time::Duration;

use ootrr_core::json::Json;
use ootrr_core::tracker::index::Index;
use ootrr_core::settings::seed::SeedFile;
use ootrr_core::tracker::targets::KINDS;

use crate::api_validate::{self, Report};
use crate::body;
use crate::error::{Fail, Hr};
use crate::events::{Batch, MAX_WAIT};
use crate::jobs::Ctl;
use crate::pathsafe;
use crate::req::Req;
use crate::state::Shared;
use crate::tracker::{Mode, Opts, Tracker};
use crate::tracker_view;

const PENDING: u64 = 0;

fn start_index(sh: &Shared, game: PB) -> Hr<u64> {
    let rep: Report = api_validate::inspect(&game);
    if !rep.ok {
        return Err(Fail::new(422, &format!("Game folder: {}", rep.problems.join(" "))));
    }
    let romfs: PB = rep.romfs.clone().unwrap_or_else(|| game.clone());
    let t: Arc<Tracker> = Arc::clone(&sh.tracker);
    t.lock().job = Some(PENDING);
    let id: u64 = sh.jobs.start(move |c: &Ctl| {
        c.begin("Read the map data from your game folder and place every location");
        match t.load_or_build(&romfs) {
            Ok(ix) => {
                let ix: Index = ix;
                let n: usize = ix.locs.iter().filter(|l| !l.pins.is_empty()).count();
                c.ok(100);
                t.set_index(game, ix);
                c.finish("done", &format!("{n} locations have a place on a map"));
            }
            Err(e) => {
                c.mark("failed");
                t.index_failed(&e.to_string());
                c.finish("failed", &e.to_string());
            }
        }
    })?;
    let mut g = sh.tracker.lock();
    if g.job == Some(PENDING) {
        g.job = Some(id);
    }
    Ok(id)
}

pub fn ensure(sh: &Shared) {
    let _held: MutexGuard<'_, ()> = sh.tracker.boot();
    let (game, save, slot): (Option<PB>, Option<PB>, Option<String>) = sh.tracker.load_saved();
    if let Some(s) = save
        && let Err(e) = sh.tracker.watch(s, slot)
    {
        sh.tracker.lock().src.error = Some(e.to_string());
    }
    if let Some(g) = game
        && let Err(f) = start_index(sh, g)
    {
        sh.tracker.lock().index_error = Some(f.msg);
    }
}

pub fn open(sh: &Shared, b: &Json) -> Hr<Json> {
    ensure(sh);
    let slot: Option<String> = body::opt_text(b, "slot")?.filter(|s| !s.is_empty()).map(|s| s.to_string());
    if let Some(s) = &slot
        && (!s.starts_with("slot_") || s.len() > 16 || !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
    {
        return Err(Fail::bad("slot must look like slot_00"));
    }
    let mut out: Json = Json::obj();
    if let Some(s) = body::opt_text(b, "save_dir")?.filter(|s| !s.is_empty()) {
        let p: PB = pathsafe::abs(s)?;
        let root: PB = sh.tracker.watch(p, slot).map_err(|e| Fail::new(422, &format!("Save folder: {e}")))?;
        out.put("save_folder", pathsafe::text(&root).into());
    }
    if let Some(s) = body::opt_text(b, "game_dir")?.filter(|s| !s.is_empty()) {
        let id: u64 = start_index(sh, pathsafe::abs(s)?)?;
        out.put("job_id", Json::Int(id as i64));
    }
    Ok(out.with("ok", true.into()))
}

pub fn mode(sh: &Shared, b: &Json) -> Hr<Json> {
    let mut o: Opts = sh.tracker.lock().opts.clone();
    if let Some(m) = body::opt_text(b, "mode")? {
        o.mode = Mode::parse(m).ok_or_else(|| Fail::bad("mode must be hidden or all"))?;
    }
    match b.get("kinds") {
        None | Some(Json::Null) => {}
        Some(Json::Arr(a)) => {
            let mut ks: Vec<String> = Vec::new();
            for k in a {
                let s: &str = k.as_str().filter(|s| KINDS.contains(s)).ok_or_else(|| Fail::bad("kinds holds an unknown kind"))?;
                if !ks.iter().any(|x| x == s) {
                    ks.push(s.to_string());
                }
            }
            o.kinds = ks;
        }
        Some(_) => return Err(Fail::bad("kinds must be a list")),
    }
    match b.get("dream") {
        None | Some(Json::Null) => {}
        Some(Json::Bool(d)) => o.dream = *d,
        Some(_) => return Err(Fail::bad("dream must be true or false")),
    }
    sh.tracker.set_opts(o);
    Ok(Json::obj().with("ok", true.into()))
}

pub fn seed(sh: &Shared, b: &Json) -> Hr<Json> {
    ensure(sh);
    let text: Option<&str> = body::opt_text(b, "seed_text")?.filter(|s| !s.is_empty());
    let sf: Option<SeedFile> = match text {
        Some(t) => Some(SeedFile::parse(t).map_err(|e| Fail::new(422, &format!("That file is not a seed file this version can read. ({e})")))?),
        None => None,
    };
    let s = sh.tracker.set_seed(sf.as_ref()).map_err(|e| Fail::new(500, &e.to_string()))?;
    Ok(Json::obj().with("ok", true.into()).with("loaded", s.is_some().into()).with("rewired", Json::Int(s.map(|x| x.rewired as i64).unwrap_or(0))))
}

pub fn mark(sh: &Shared, b: &Json) -> Hr<Json> {
    let id: &str = body::short(b, "id")?;
    let on: bool = b.get("on").and_then(|x| x.as_bool()).ok_or_else(|| Fail::bad("on must be true or false"))?;
    {
        let g = sh.tracker.lock();
        let i: usize = g.track.find(id).ok_or_else(|| Fail::new(404, "no such location"))?;
        if !tracker_view::visible(&g, i) {
            return Err(Fail::new(404, "no such location"));
        }
    }
    let changed: bool = sh.tracker.user(id, on)?;
    Ok(Json::obj().with("ok", true.into()).with("changed", changed.into()))
}

fn hosts() -> Json {
    let mut out: Vec<Json> = Vec::new();
    let eden: Option<PB> = env::var("APPDATA")
        .ok()
        .and_then(|a| pathsafe::abs(&a).ok())
        .map(|a| a.join("eden").join("nand").join("user").join("save").join("0000000000000000"))
        .filter(|p| p.is_dir());
    out.push(
        Json::obj()
            .with("host", "eden".into())
            .with("label", "Lichii / Eden".into())
            .with("path", eden.map(|p| Json::from(pathsafe::text(&p))).unwrap_or(Json::Null))
            .with("hint", "The folder is found for you inside %APPDATA%\\eden\\nand\\user\\save.".into()),
    );
    out.push(
        Json::obj()
            .with("host", "hookshot".into())
            .with("label", "Hookshot".into())
            .with("path", Json::Null)
            .with("hint", "Pick your Hookshot folder. The tracker finds user\\save\\<user> and its slot folders inside it.".into()),
    );
    out.push(
        Json::obj()
            .with("host", "longshot".into())
            .with("label", "Longshot".into())
            .with("path", longshot_saves().map(|p| Json::from(pathsafe::text(&p))).unwrap_or(Json::Null))
            .with("hint", "Pick your Longshot game folder (the one with bin\\longshot.exe). On Linux the saves are in ~/.local/share/longshot/save.".into()),
    );
    Json::Arr(out)
}

/// Longshot's save folder where its builder puts it: `<game folder>\user\save` on Windows, the XDG data folder elsewhere.
fn longshot_saves() -> Option<PB> {
    let base: Option<PB> = if cfg!(windows) {
        env::var("USERPROFILE").ok().map(|h| PB::from(h).join("Games").join("Longshot").join("user"))
    } else {
        env::var("XDG_DATA_HOME")
            .ok()
            .filter(|x| !x.is_empty())
            .map(PB::from)
            .or_else(|| env::var("HOME").ok().map(|h| PB::from(h).join(".local").join("share")))
            .map(|d| d.join("longshot"))
    };
    base.and_then(|b| pathsafe::abs(&b.join("save").to_string_lossy()).ok()).filter(|p| p.is_dir())
}

pub fn index(sh: &Shared) -> Json {
    ensure(sh);
    let g = sh.tracker.lock();
    tracker_view::index_json(&g)
}

pub fn state(sh: &Shared) -> Json {
    ensure(sh);
    let seq: u64 = sh.tracker.events.seq();
    let g = sh.tracker.lock();
    tracker_view::state_json(&g, seq).with("hosts", hosts())
}

pub fn events(sh: &Shared, r: &Req) -> Hr<Json> {
    ensure(sh);
    let after: u64 = match r.param("after") {
        None | Some("") => 0,
        Some(s) => s.parse::<u64>().map_err(|_| Fail::bad("after must be a number"))?,
    };
    let cap: Duration = sh.tracker.poll.min(MAX_WAIT);
    let wait: Duration = match r.param("wait") {
        None | Some("") => cap,
        Some(s) => Duration::from_millis(s.parse::<u64>().map_err(|_| Fail::bad("wait must be a number"))?).min(cap),
    };
    let b: Batch = sh.tracker.events.wait(after, wait);
    let g = sh.tracker.lock();
    Ok(tracker_view::events_json(&g, &b))
}
