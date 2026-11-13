use std::collections::VecDeque;
use std::fs;
use std::path::{Path as P, PathBuf as PB};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime};

use ootrr_core::TITLE_ID;
use ootrr_core::error::{Error, Res};
use ootrr_core::tracker::peek::{Peek, peek};

pub const TICK: Duration = Duration::from_millis(500);
pub const RELIST: Duration = Duration::from_secs(5);
pub const SETTLE: Duration = Duration::from_millis(250);
pub const CAPTION_GRACE: Duration = Duration::from_secs(1);
const SEARCH_DEPTH: usize = 7;
const SEARCH_DIRS: usize = 4000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sig {
    pub plen: u64,
    pub pmod: Option<SystemTime>,
    pub clen: Option<u64>,
    pub cmod: Option<SystemTime>,
}

#[derive(Debug, Clone)]
struct Slot {
    name: String,
    dir: PB,
    sig: Option<Sig>,
    seen: Instant,
    read: Option<Sig>,
    changed: Option<Instant>,
}

#[derive(Debug)]
pub enum Out {
    Read { slot: String, peek: Box<Peek>, at: Option<SystemTime> },
    Fail { slot: String, msg: String },
}

pub struct Watcher {
    pub root: PB,
    pub pinned: Option<String>,
    slots: Vec<Slot>,
    listed: Option<Instant>,
    pub active: Option<String>,
}

fn stat(p: &P) -> Option<(u64, Option<SystemTime>)> {
    let m: fs::Metadata = fs::metadata(p).ok()?;
    if !m.is_file() {
        return None;
    }
    Some((m.len(), m.modified().ok()))
}

fn sig_of(d: &P) -> Option<Sig> {
    let (plen, pmod): (u64, Option<SystemTime>) = stat(&d.join("progress.sav"))?;
    let c: Option<(u64, Option<SystemTime>)> = stat(&d.join("caption.sav"));
    Some(Sig { plen, pmod, clen: c.map(|x| x.0), cmod: c.and_then(|x| x.1) })
}

fn is_slot(name: &str) -> bool {
    name.starts_with("slot_") && name.len() <= 16
}

fn slot_dirs(d: &P) -> Vec<(String, PB)> {
    let mut out: Vec<(String, PB)> = Vec::new();
    if let Ok(rd) = fs::read_dir(d) {
        for e in rd.flatten() {
            let n: String = e.file_name().to_string_lossy().into_owned();
            if is_slot(&n) && e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                out.push((n, e.path()));
            }
        }
    }
    out.sort();
    out
}

pub fn resolve(p: &P) -> Res<PB> {
    if !p.is_dir() {
        return Err(Error::Msg("That save folder does not exist.".to_string()));
    }
    if p.join("progress.sav").is_file() || !slot_dirs(p).is_empty() {
        return Ok(p.to_path_buf());
    }
    let mut q: VecDeque<(PB, usize)> = VecDeque::new();
    q.push_back((p.to_path_buf(), 0));
    let mut seen: usize = 0;
    let mut hits: Vec<PB> = Vec::new();
    while let Some((d, depth)) = q.pop_front() {
        seen += 1;
        if seen > SEARCH_DIRS {
            break;
        }
        if !slot_dirs(&d).is_empty() {
            hits.push(d);
            continue;
        }
        if depth >= SEARCH_DEPTH {
            continue;
        }
        let Ok(rd) = fs::read_dir(&d) else {
            continue;
        };
        let mut kids: Vec<PB> = rd
            .flatten()
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|e| e.path())
            .collect();
        kids.sort();
        for k in kids {
            q.push_back((k, depth + 1));
        }
    }
    if let Some(h) = hits.iter().find(|h| h.to_string_lossy().to_ascii_uppercase().contains(TITLE_ID)) {
        return Ok(h.clone());
    }
    hits.into_iter()
        .next()
        .ok_or_else(|| Error::Msg("No save slots were found in that folder. Pick the folder that holds slot_00, slot_01 and so on.".to_string()))
}

impl Watcher {
    pub fn new(root: PB, pinned: Option<String>) -> Watcher {
        Watcher { root, pinned, slots: Vec::new(), listed: None, active: None }
    }

    pub fn names(&self) -> Vec<String> {
        self.slots.iter().map(|s| s.name.clone()).collect()
    }

    fn relist(&mut self, now: Instant) {
        let mut found: Vec<(String, PB)> = slot_dirs(&self.root);
        if found.is_empty() && self.root.join("progress.sav").is_file() {
            let n: String = self.root.file_name().map(|x| x.to_string_lossy().into_owned()).unwrap_or_else(|| "save".to_string());
            found.push((n, self.root.clone()));
        }
        let old: Vec<Slot> = std::mem::take(&mut self.slots);
        for (name, dir) in found {
            match old.iter().find(|s| s.name == name) {
                Some(s) => self.slots.push(s.clone()),
                None => self.slots.push(Slot { name, dir, sig: None, seen: now, read: None, changed: None }),
            }
        }
        self.listed = Some(now);
    }

    fn settled(s: &Slot, now: Instant) -> bool {
        let Some(g) = s.sig else {
            return false;
        };
        let quiet: Duration = now.saturating_duration_since(s.seen);
        if quiet < SETTLE {
            return false;
        }
        let cap_ok: bool = match (g.cmod, g.pmod) {
            (Some(c), Some(p)) => c >= p,
            _ => false,
        };
        cap_ok || quiet >= CAPTION_GRACE
    }

    fn choose(&self) -> Option<usize> {
        if let Some(p) = &self.pinned {
            return self.slots.iter().position(|s| &s.name == p);
        }
        let live: Option<usize> = self
            .slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.changed.is_some() && s.sig.is_some())
            .max_by_key(|(_, s)| s.changed)
            .map(|(i, _)| i);
        live.or_else(|| {
            self.slots
                .iter()
                .enumerate()
                .filter(|(_, s)| s.sig.is_some())
                .max_by_key(|(_, s)| s.sig.and_then(|g| g.pmod))
                .map(|(i, _)| i)
        })
    }

    pub fn tick(&mut self, now: Instant) -> Option<Out> {
        if self.listed.is_none_or(|t| now.saturating_duration_since(t) >= RELIST) {
            let first: bool = self.listed.is_none();
            self.relist(now);
            if first {
                for s in self.slots.iter_mut() {
                    s.sig = sig_of(&s.dir);
                    s.seen = now;
                }
            }
        }
        for s in self.slots.iter_mut() {
            let g: Option<Sig> = sig_of(&s.dir);
            if g != s.sig {
                s.sig = g;
                s.seen = now;
                s.changed = Some(now);
            }
        }
        let i: usize = self.choose()?;
        self.active = Some(self.slots[i].name.clone());
        let s: &Slot = &self.slots[i];
        if !Watcher::settled(s, now) || s.read == s.sig {
            return None;
        }
        let name: String = s.name.clone();
        let g: Option<Sig> = s.sig;
        let b: Vec<u8> = match fs::read(s.dir.join("progress.sav")) {
            Ok(b) => b,
            Err(e) => return Some(Out::Fail { slot: name, msg: format!("the save could not be read yet ({e})") }),
        };
        match peek(&b) {
            Ok(p) => {
                self.slots[i].read = g;
                Some(Out::Read { slot: name, peek: Box::new(p), at: g.and_then(|x| x.pmod) })
            }
            Err(e) => Some(Out::Fail { slot: name, msg: e.to_string() }),
        }
    }
}

pub struct Handle {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl Handle {
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn spawn<F>(mut w: Watcher, mut sink: F) -> Res<Handle>
where
    F: FnMut(Out, &Watcher) + Send + 'static,
{
    let stop: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let flag: Arc<AtomicBool> = Arc::clone(&stop);
    let join: JoinHandle<()> = thread::Builder::new().name("ootrr-tracker-watch".to_string()).spawn(move || {
        while !flag.load(Ordering::SeqCst) {
            if let Some(o) = w.tick(Instant::now()) {
                sink(o, &w);
            }
            let mut left: Duration = TICK;
            while left > Duration::ZERO && !flag.load(Ordering::SeqCst) {
                let d: Duration = left.min(Duration::from_millis(50));
                thread::sleep(d);
                left = left.saturating_sub(d);
            }
        }
    })?;
    Ok(Handle { stop, join: Some(join) })
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::fs::File;
    use std::time::UNIX_EPOCH;

    pub fn save_bytes(flags: &[(u32, u32)], keys: &[u64]) -> Vec<u8> {
        let pad: usize = 13_000;
        let n: usize = pad + flags.len();
        let end: usize = 0x28 + 8 * n + 8;
        let mut b: Vec<u8> = vec![0; 0x28];
        b[0..4].copy_from_slice(&[4, 3, 2, 1]);
        b[8..12].copy_from_slice(&(end as u32).to_le_bytes());
        for i in 0..pad {
            b.extend(0u32.to_le_bytes());
            b.extend((i as u32).to_le_bytes());
        }
        for (h, v) in flags {
            b.extend(h.to_le_bytes());
            b.extend(v.to_le_bytes());
        }
        b.extend(0xa3db_7114u32.to_le_bytes());
        b.extend((end as u32).to_le_bytes());
        let mut ks: Vec<u64> = keys.to_vec();
        ks.sort();
        for k in ks {
            b.extend(k.to_le_bytes());
        }
        b.extend(0u64.to_le_bytes());
        b
    }

    pub fn tmp(tag: &str) -> PB {
        let p: PB = std::env::temp_dir().join(format!("ootrr_watch_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn at(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    pub fn put(d: &P, b: &[u8], t: u64) {
        fs::create_dir_all(d).unwrap();
        fs::write(d.join("progress.sav"), b).unwrap();
        fs::write(d.join("caption.sav"), b"cap").unwrap();
        File::options().write(true).open(d.join("progress.sav")).unwrap().set_modified(at(t)).unwrap();
        File::options().write(true).open(d.join("caption.sav")).unwrap().set_modified(at(t)).unwrap();
    }

    fn read_slot(o: Option<Out>) -> Option<(String, Peek)> {
        match o {
            Some(Out::Read { slot, peek, .. }) => Some((slot, *peek)),
            _ => None,
        }
    }

    #[test]
    fn a_torn_write_is_never_published() {
        let d: PB = tmp("torn");
        let full: Vec<u8> = save_bytes(&[(7, 1)], &[11, 12]);
        let s: PB = d.join("slot_00");
        put(&s, &full[..full.len() / 2], 1_700_000_000);
        let mut w: Watcher = Watcher::new(d.clone(), None);
        let t0: Instant = Instant::now();
        assert!(w.tick(t0).is_none());
        let o: Option<Out> = w.tick(t0 + Duration::from_millis(600));
        assert!(matches!(o, Some(Out::Fail { .. })), "{o:?}");
        assert!(read_slot(w.tick(t0 + Duration::from_millis(1200))).is_none());
        put(&s, &full, 1_700_000_010);
        assert!(w.tick(t0 + Duration::from_millis(1700)).is_none());
        let (n, p): (String, Peek) = read_slot(w.tick(t0 + Duration::from_millis(2200))).unwrap();
        assert_eq!(n, "slot_00");
        assert_eq!(p.keys, vec![11, 12]);
        assert!(w.tick(t0 + Duration::from_millis(2700)).is_none());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_file_still_changing_is_not_read() {
        let d: PB = tmp("busy");
        let s: PB = d.join("slot_00");
        put(&s, &save_bytes(&[], &[1]), 1_700_000_000);
        let mut w: Watcher = Watcher::new(d.clone(), None);
        let t0: Instant = Instant::now();
        w.tick(t0);
        put(&s, &save_bytes(&[], &[1, 2]), 1_700_000_005);
        assert!(w.tick(t0 + Duration::from_millis(100)).is_none());
        put(&s, &save_bytes(&[], &[1, 2, 3]), 1_700_000_006);
        assert!(w.tick(t0 + Duration::from_millis(200)).is_none());
        let (_, p): (String, Peek) = read_slot(w.tick(t0 + Duration::from_millis(500))).unwrap();
        assert_eq!(p.keys, vec![1, 2, 3]);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn the_newest_slot_wins_at_start_and_a_changed_slot_wins_later() {
        let d: PB = tmp("slots");
        put(&d.join("slot_00"), &save_bytes(&[], &[100]), 1_700_000_000);
        put(&d.join("slot_01"), &save_bytes(&[], &[101]), 1_700_009_000);
        let mut w: Watcher = Watcher::new(d.clone(), None);
        let t0: Instant = Instant::now();
        w.tick(t0);
        let (n, _): (String, Peek) = read_slot(w.tick(t0 + Duration::from_millis(500))).unwrap();
        assert_eq!(n, "slot_01");
        put(&d.join("slot_00"), &save_bytes(&[], &[100, 200]), 1_600_000_000);
        assert!(w.tick(t0 + Duration::from_millis(1000)).is_none());
        let (m, p): (String, Peek) = read_slot(w.tick(t0 + Duration::from_millis(1500))).unwrap();
        assert_eq!(m, "slot_00");
        assert_eq!(p.keys, vec![100, 200]);
        assert_eq!(w.active.as_deref(), Some("slot_00"));
        let mut pin: Watcher = Watcher::new(d.clone(), Some("slot_01".to_string()));
        pin.tick(t0);
        let (k, _): (String, Peek) = read_slot(pin.tick(t0 + Duration::from_millis(500))).unwrap();
        assert_eq!(k, "slot_01");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn an_old_caption_waits_for_the_grace_time() {
        let d: PB = tmp("cap");
        let s: PB = d.join("slot_00");
        put(&s, &save_bytes(&[], &[5]), 1_700_000_000);
        File::options().write(true).open(s.join("caption.sav")).unwrap().set_modified(at(1_600_000_000)).unwrap();
        let mut w: Watcher = Watcher::new(d.clone(), None);
        let t0: Instant = Instant::now();
        w.tick(t0);
        assert!(w.tick(t0 + Duration::from_millis(500)).is_none());
        assert!(read_slot(w.tick(t0 + Duration::from_millis(1000))).is_some());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn the_folder_is_resolved_down_to_the_slots() {
        let d: PB = tmp("resolve");
        let deep: PB = d.join("user").join("save").join("user0011");
        put(&deep.join("slot_00"), &save_bytes(&[], &[]), 1_700_000_000);
        assert_eq!(resolve(&d).unwrap(), deep);
        assert_eq!(resolve(&deep).unwrap(), deep);
        assert_eq!(resolve(&deep.join("slot_00")).unwrap(), deep.join("slot_00"));
        assert!(resolve(&d.join("missing")).is_err());
        let empty: PB = tmp("resolve_empty");
        assert!(resolve(&empty).is_err());
        let mut w: Watcher = Watcher::new(deep.join("slot_00"), None);
        let t0: Instant = Instant::now();
        w.tick(t0);
        assert_eq!(read_slot(w.tick(t0 + Duration::from_millis(500))).unwrap().0, "slot_00");
        let _ = fs::remove_dir_all(&d);
        let _ = fs::remove_dir_all(&empty);
    }
}
