use std::fs;
use std::path::{Path as P, PathBuf as PB};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ootrr_core::error::{Error, Res};
use ootrr_core::json::{self, Json};
use ootrr_core::tracker::grid;
use ootrr_core::tracker::index::{self, Index};
use ootrr_core::tracker::peek::{Peek, Place};
use ootrr_core::tracker::rules::Rule;
use ootrr_core::tracker::seed;
use ootrr_core::tracker::state::{By, S, Track};
use ootrr_core::items::load;
use ootrr_core::settings::seed::SeedFile;
use ootrr_core::tracker::targets::{self, KINDS, OFF_BY_DEFAULT, Target};

use crate::datadir;
use crate::events::{Ev, Events, MAX_WAIT};
use crate::icon_store::{self, Icons};
use crate::pathsafe;
use crate::watch::{self, Out, Watcher};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Hidden,
    All,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Hidden => "hidden",
            Mode::All => "all",
        }
    }

    pub fn parse(s: &str) -> Option<Mode> {
        match s {
            "hidden" => Some(Mode::Hidden),
            "all" => Some(Mode::All),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Opts {
    pub mode: Mode,
    pub kinds: Vec<String>,
    pub dream: bool,
}

impl Default for Opts {
    fn default() -> Opts {
        Opts {
            mode: Mode::Hidden,
            kinds: KINDS.iter().filter(|k| !OFF_BY_DEFAULT.contains(k)).map(|k| k.to_string()).collect(),
            dream: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Source {
    pub asked: Option<PB>,
    pub folder: Option<PB>,
    pub pinned: Option<String>,
    pub slot: Option<String>,
    pub slots: Vec<String>,
    pub read_at: Option<u64>,
    pub keys: usize,
    pub error: Option<String>,
    pub reads: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Seeded {
    pub seed: String,
    pub checksum: String,
    pub rewired: usize,
}

pub struct Inner {
    pub seeded: Option<Seeded>,
    pub peek: Option<Peek>,
    pub index: Arc<Index>,
    pub game_dir: Option<PB>,
    pub job: Option<u64>,
    pub index_error: Option<String>,
    pub track: Track,
    pub opts: Opts,
    pub src: Source,
    pub place: Option<Place>,
    pub loaded: bool,
    pub items: Option<Vec<grid::St>>,
}

pub struct Tracker {
    inner: Mutex<Inner>,
    pub events: Events,
    pub data: Option<PB>,
    watch: Mutex<Option<watch::Handle>>,
    boot: Mutex<()>,
    pub poll: Duration,
    pub icons: Icons,
}

pub fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn secs(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn write_atomic(p: &P, text: &str) -> Res<()> {
    if let Some(d) = p.parent() {
        fs::create_dir_all(d)?;
    }
    let tmp: PB = p.with_extension("tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, p)?;
    Ok(())
}

fn path_of(j: &Json, k: &str) -> Option<PB> {
    j.get(k).and_then(|x| x.as_str()).and_then(|s| pathsafe::abs(s).ok())
}

impl Tracker {
    pub fn new(data: Option<PB>) -> Tracker {
        let ts: &[Target] = targets::all().unwrap_or(&[]);
        let ix: Index = index::empty().unwrap_or(Index { game: String::new(), table: 0, maps: Vec::new(), locs: Vec::new(), shapes: Vec::new() });
        Tracker {
            inner: Mutex::new(Inner {
                seeded: None,
                peek: None,
                index: Arc::new(ix),
                game_dir: None,
                job: None,
                index_error: None,
                track: Track::new(ts),
                opts: Opts::default(),
                src: Source::default(),
                place: None,
                loaded: false,
                items: None,
            }),
            events: Events::new(),
            data: data.map(|d| datadir::cache_root(&d)),
            watch: Mutex::new(None),
            boot: Mutex::new(()),
            poll: MAX_WAIT,
            icons: Icons::new(icon_store::disk_on()),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn boot(&self) -> MutexGuard<'_, ()> {
        self.boot.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn settings_file(&self) -> Option<PB> {
        self.data.as_ref().map(|d| d.join("settings.json"))
    }

    fn marks_file(&self) -> Option<PB> {
        self.data.as_ref().map(|d| d.join("profile-vanilla.json"))
    }

    pub fn index_file(&self, game: &str) -> Option<PB> {
        self.data.as_ref().map(|d| d.join(datadir::game_dir_name(game)).join("index.json"))
    }

    fn save_settings(&self, g: &Inner) {
        let Some(p) = self.settings_file() else {
            return;
        };
        let s = |x: &Option<PB>| x.as_ref().map(|p| Json::from(pathsafe::text(p))).unwrap_or(Json::Null);
        let j: Json = Json::obj()
            .with("format", Json::Int(1))
            .with("game_dir", s(&g.game_dir))
            .with("save_dir", s(&g.src.asked))
            .with("slot", g.src.pinned.as_deref().map(Json::from).unwrap_or(Json::Null))
            .with("mode", g.opts.mode.name().into())
            .with("kinds", Json::strs(g.opts.kinds.iter().map(|k| k.as_str())))
            .with("dream", g.opts.dream.into());
        let _ = write_atomic(&p, &j.pretty());
    }

    fn save_marks(&self, g: &Inner) {
        if let Some(p) = self.marks_file() {
            let _ = write_atomic(&p, &g.track.to_json().to_string());
        }
    }

    pub fn load_saved(&self) -> (Option<PB>, Option<PB>, Option<String>) {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        if g.loaded {
            return (None, None, None);
        }
        g.loaded = true;
        if let Some(t) = self.marks_file().and_then(|p| fs::read_to_string(p).ok())
            && let Ok(j) = json::parse(&t)
        {
            g.track.load(&j);
        }
        let Some(t) = self.settings_file().and_then(|p| fs::read_to_string(p).ok()) else {
            return (None, None, None);
        };
        let Ok(j) = json::parse(&t) else {
            return (None, None, None);
        };
        if let Some(m) = j.get("mode").and_then(|x| x.as_str()).and_then(Mode::parse) {
            g.opts.mode = m;
        }
        if let Some(a) = j.get("kinds").and_then(|x| x.as_arr()) {
            g.opts.kinds = a.iter().filter_map(|x| x.as_str()).filter(|k| KINDS.contains(k)).map(|k| k.to_string()).collect();
        }
        if let Some(d) = j.get("dream").and_then(|x| x.as_bool()) {
            g.opts.dream = d;
        }
        let slot: Option<String> = j.get("slot").and_then(|x| x.as_str()).map(|s| s.to_string());
        (path_of(&j, "game_dir"), path_of(&j, "save_dir"), slot)
    }

    pub fn set_index(&self, game_dir: PB, ix: Index) {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        g.index = Arc::new(ix);
        g.game_dir = Some(game_dir);
        g.job = None;
        g.index_error = None;
        self.save_settings(&g);
        drop(g);
        self.events.push(&[Ev::Reset]);
    }

    pub fn index_failed(&self, msg: &str) {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        g.job = None;
        g.index_error = Some(msg.to_string());
        drop(g);
        self.events.push(&[Ev::Source]);
    }

    pub fn load_or_build(&self, romfs: &P) -> Res<Index> {
        let fp: String = index::fingerprint(romfs)?;
        let file: Option<PB> = self.index_file(&fp);
        if let Some(f) = &file
            && let Ok(t) = fs::read_to_string(f)
            && let Ok(ix) = index::from_json(&t)
            && ix.game == fp
        {
            return Ok(ix);
        }
        let ix: Index = index::build(romfs)?;
        if let Some(f) = &file {
            write_atomic(f, &ix.to_json().to_string())?;
        }
        Ok(ix)
    }

    pub fn publish(&self, slot: &str, p: &Peek, at: Option<SystemTime>, names: Vec<String>) {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        let quiet: bool = g.src.reads == 0;
        let base: u64 = self.events.seq() + 1;
        let changed: Vec<usize> = g.track.apply(p, base);
        g.src.reads += 1;
        g.src.slot = Some(slot.to_string());
        g.src.slots = names;
        g.src.read_at = at.map(secs);
        g.src.keys = p.keys.len();
        g.src.error = None;
        g.place = Some(p.place.clone());
        g.items = Some(grid::read(p));
        g.peek = Some(p.clone());
        let mut evs: Vec<Ev> = changed
            .iter()
            .map(|i| {
                let m = g.track.marks[*i];
                Ev::Loc { i: *i, fresh: !quiet && m.s == S::Found && m.by == Some(By::Save) && m.at == base }
            })
            .collect();
        evs.push(Ev::Source);
        if !changed.is_empty() {
            self.save_marks(&g);
        }
        drop(g);
        self.events.push(&evs);
    }

    pub fn set_seed(&self, sf: Option<&SeedFile>) -> Res<Option<Seeded>> {
        let ts: &[Target] = targets::all()?;
        let over: Vec<Option<Rule>> = match sf {
            Some(f) => seed::rules(ts, &f.placements, &load::load()?),
            None => Vec::new(),
        };
        let mut g: MutexGuard<'_, Inner> = self.lock();
        g.track.rewire(ts, &over);
        g.seeded = sf.map(|f| Seeded { seed: f.seed.clone(), checksum: f.checksum(), rewired: seed::rewired(&over) });
        let base: u64 = self.events.seq() + 1;
        if let Some(p) = g.peek.clone() {
            g.track.apply(&p, base);
        }
        self.save_marks(&g);
        let out: Option<Seeded> = g.seeded.clone();
        drop(g);
        self.events.push(&[Ev::Reset]);
        Ok(out)
    }

    pub fn fail(&self, slot: &str, msg: &str) {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        let m: String = format!("{slot}: {msg}");
        if g.src.error.as_deref() == Some(m.as_str()) {
            return;
        }
        g.src.error = Some(m);
        drop(g);
        self.events.push(&[Ev::Source]);
    }

    pub fn user(&self, id: &str, on: bool) -> Res<bool> {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        let i: usize = g.track.find(id).ok_or_else(|| Error::Msg("no such location".to_string()))?;
        let seq: u64 = self.events.seq() + 1;
        if !g.track.user(i, on, seq) {
            return Ok(false);
        }
        self.save_marks(&g);
        drop(g);
        self.events.push(&[Ev::Loc { i, fresh: false }, Ev::Source]);
        Ok(true)
    }

    pub fn set_opts(&self, o: Opts) {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        g.opts = o;
        self.save_settings(&g);
        drop(g);
        self.events.push(&[Ev::Reset]);
    }

    pub fn stop_watch(&self) {
        let h: Option<watch::Handle> = self.watch.lock().unwrap_or_else(|e| e.into_inner()).take();
        drop(h);
    }

    pub fn watch(self: &Arc<Self>, asked: PB, pinned: Option<String>) -> Res<PB> {
        let root: PB = watch::resolve(&asked)?;
        self.stop_watch();
        {
            let mut g: MutexGuard<'_, Inner> = self.lock();
            g.src = Source { asked: Some(asked), folder: Some(root.clone()), pinned: pinned.clone(), ..Source::default() };
            self.save_settings(&g);
        }
        let me: Arc<Tracker> = Arc::clone(self);
        let w: Watcher = Watcher::new(root.clone(), pinned);
        let h: watch::Handle = watch::spawn(w, move |o: Out, w: &Watcher| match o {
            Out::Read { slot, peek, at } => me.publish(&slot, &peek, at, w.names()),
            Out::Fail { slot, msg } => me.fail(&slot, &msg),
        })?;
        *self.watch.lock().unwrap_or_else(|e| e.into_inner()) = Some(h);
        self.events.push(&[Ev::Source]);
        Ok(root)
    }
}

impl Drop for Tracker {
    fn drop(&mut self) {
        self.stop_watch();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watch::tests::{put, save_bytes, tmp};
    use ootrr_core::murmur::murmur3_32;
    use ootrr_core::tracker::peek::{Lim, peek_with};
    use std::thread;
    use std::time::Instant;

    fn pick(kind: &str) -> (usize, &'static Target) {
        targets::all().unwrap().iter().enumerate().find(|(_, t)| t.kind == kind && !t.keys.is_empty() || t.kind == kind && !t.flags.is_empty()).unwrap()
    }

    #[test]
    fn publish_ticks_sticky_kinds_and_only_later_reads_are_fresh() {
        let t: Tracker = Tracker::new(None);
        let (pi, pot): (usize, &Target) = pick("pot");
        let (ci, chest): (usize, &Target) = pick("chest");
        let fl: u32 = murmur3_32(chest.flags[0].as_bytes(), 0);
        let a: Peek = peek_with(&save_bytes(&[(fl, 1)], &[pot.keys[0]]), &Lim::ANY).unwrap();
        t.publish("slot_00", &a, None, vec!["slot_00".into()]);
        let first = t.events.wait(0, Duration::from_millis(1));
        assert!(first.items.iter().all(|x| !matches!(x.1, Ev::Loc { fresh: true, .. })));
        let b: Peek = peek_with(&save_bytes(&[], &[]), &Lim::ANY).unwrap();
        let s0: u64 = t.events.seq();
        t.publish("slot_00", &b, None, vec!["slot_00".into()]);
        {
            let g = t.lock();
            assert_eq!((g.track.marks[pi].s, g.track.marks[pi].by), (S::Found, Some(By::Sticky)));
            assert_eq!(g.track.marks[ci].s, S::Lost);
        }
        t.publish("slot_00", &a, None, vec!["slot_00".into()]);
        let later = t.events.wait(s0, Duration::from_millis(1));
        assert!(later.items.iter().any(|x| x.1 == Ev::Loc { i: ci, fresh: true }));
    }

    #[test]
    fn a_seed_rewires_the_gift_rows_and_leaving_it_restores_them() {
        use ootrr_core::settings::Settings;
        use std::collections::BTreeMap;
        let t: Tracker = Tracker::new(None);
        let ts: &[Target] = targets::all().unwrap();
        let impa: usize = ts.iter().position(|x| x.ootr.as_deref() == Some("Song from Impa")).unwrap();
        let sheik: usize = ts.iter().position(|x| x.ootr.as_deref() == Some("Sheik in Forest")).unwrap();
        let fl: u32 = murmur3_32(b"HavePlayedGetEvent.Item_Melody_ForestMinuet", 0);
        let a: Peek = peek_with(&save_bytes(&[(fl, 1)], &[]), &Lim::ANY).unwrap();
        t.publish("slot_00", &a, None, vec!["slot_00".into()]);
        assert_eq!(t.lock().track.marks[sheik].s, S::Found);
        let mut p: BTreeMap<String, String> = BTreeMap::new();
        p.insert("Song from Impa".into(), "Minuet of Forest".into());
        p.insert("Sheik in Forest".into(), "Zeldas Lullaby".into());
        let sf: SeedFile = SeedFile::new("abc", &Settings::default(), "oot-remake:test", p).unwrap();
        let s: Seeded = t.set_seed(Some(&sf)).unwrap().unwrap();
        assert_eq!(s.rewired, 2);
        {
            let g = t.lock();
            assert_eq!(g.track.marks[impa].s, S::Found);
            assert_eq!(g.track.marks[sheik].s, S::Open);
        }
        assert_eq!(t.set_seed(None).unwrap(), None);
        let g = t.lock();
        assert_eq!(g.track.marks[impa].s, S::Open);
        assert_eq!(g.track.marks[sheik].s, S::Found);
    }

    #[test]
    fn settings_and_marks_persist_in_the_data_folder() {
        let d: PB = tmp("persist");
        let t: Tracker = Tracker::new(Some(d.clone()));
        let (ci, chest): (usize, &Target) = pick("chest");
        t.load_saved();
        t.user(&chest.id, true).unwrap();
        t.set_opts(Opts { mode: Mode::All, kinds: vec!["chest".into()], dream: true });
        let u: Tracker = Tracker::new(Some(d.clone()));
        u.load_saved();
        let g = u.lock();
        assert_eq!(g.track.marks[ci].by, Some(By::User));
        assert_eq!(g.opts, Opts { mode: Mode::All, kinds: vec!["chest".into()], dream: true });
        drop(g);
        assert!(d.join("ootrr").join("tracker-cache").join("settings.json").is_file());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn the_watch_thread_publishes_a_rewritten_save() {
        let d: PB = tmp("live");
        let (ci, chest): (usize, &Target) = pick("chest");
        let fl: u32 = murmur3_32(chest.flags[0].as_bytes(), 0);
        put(&d.join("slot_00"), &save_bytes(&[], &[]), 1_700_000_000);
        let t: Arc<Tracker> = Arc::new(Tracker::new(None));
        t.watch(d.clone(), None).unwrap();
        let t0: Instant = Instant::now();
        while t.lock().src.reads == 0 && t0.elapsed() < Duration::from_secs(5) {
            thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(t.lock().src.reads, 1);
        put(&d.join("slot_00"), &save_bytes(&[(fl, 1)], &[]), 1_700_000_100);
        while t.lock().track.marks[ci].s != S::Found && t0.elapsed() < Duration::from_secs(8) {
            thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(t.lock().track.marks[ci].s, S::Found);
        t.stop_watch();
        let _ = fs::remove_dir_all(&d);
    }
}
