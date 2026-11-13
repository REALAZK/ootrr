use std::collections::VecDeque;
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub const KEEP: usize = 256;
pub const MAX_WAIT: Duration = Duration::from_secs(25);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ev {
    Loc { i: usize, fresh: bool },
    Source,
    Reset,
}

struct Inner {
    seq: u64,
    list: VecDeque<(u64, Ev)>,
}

pub struct Events {
    inner: Mutex<Inner>,
    cv: Condvar,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Batch {
    pub seq: u64,
    pub reset: bool,
    pub items: Vec<(u64, Ev)>,
}

impl Events {
    pub fn new() -> Events {
        Events { inner: Mutex::new(Inner { seq: 0, list: VecDeque::new() }), cv: Condvar::new() }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn seq(&self) -> u64 {
        self.lock().seq
    }

    pub fn push(&self, evs: &[Ev]) -> u64 {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        for e in evs {
            g.seq += 1;
            let s: u64 = g.seq;
            g.list.push_back((s, *e));
            while g.list.len() > KEEP {
                g.list.pop_front();
            }
        }
        let s: u64 = g.seq;
        drop(g);
        self.cv.notify_all();
        s
    }

    fn take(g: &Inner, after: u64) -> Batch {
        let oldest: u64 = g.list.front().map(|x| x.0).unwrap_or(g.seq + 1);
        if after > g.seq || after + 1 < oldest {
            return Batch { seq: g.seq, reset: after != g.seq, items: Vec::new() };
        }
        Batch { seq: g.seq, reset: false, items: g.list.iter().filter(|x| x.0 > after).copied().collect() }
    }

    pub fn wait(&self, after: u64, wait: Duration) -> Batch {
        let end: Instant = Instant::now() + wait.min(MAX_WAIT);
        let mut g: MutexGuard<'_, Inner> = self.lock();
        loop {
            if g.seq != after {
                return Events::take(&g, after);
            }
            let now: Instant = Instant::now();
            if now >= end {
                return Batch { seq: g.seq, reset: false, items: Vec::new() };
            }
            g = match self.cv.wait_timeout(g, end - now) {
                Ok((x, _)) => x,
                Err(e) => e.into_inner().0,
            };
        }
    }
}

impl Default for Events {
    fn default() -> Events {
        Events::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn events_get_sequence_numbers_in_order() {
        let e: Events = Events::new();
        assert_eq!(e.push(&[Ev::Source, Ev::Loc { i: 3, fresh: true }]), 2);
        let b: Batch = e.wait(0, Duration::from_millis(1));
        assert_eq!(b.items, vec![(1, Ev::Source), (2, Ev::Loc { i: 3, fresh: true })]);
        assert_eq!(e.wait(1, Duration::from_millis(1)).items.len(), 1);
    }

    #[test]
    fn only_the_last_256_are_kept_and_a_late_page_must_reset() {
        let e: Events = Events::new();
        let many: Vec<Ev> = (0..300).map(|i| Ev::Loc { i, fresh: false }).collect();
        e.push(&many);
        let b: Batch = e.wait(10, Duration::from_millis(1));
        assert!(b.reset && b.items.is_empty());
        assert_eq!(b.seq, 300);
        let c: Batch = e.wait(44, Duration::from_millis(1));
        assert!(!c.reset);
        assert_eq!(c.items.len(), KEEP);
        assert!(e.wait(999, Duration::from_millis(1)).reset);
    }

    #[test]
    fn a_long_poll_times_out_empty_and_wakes_on_a_push() {
        let e: Arc<Events> = Arc::new(Events::new());
        let t0: Instant = Instant::now();
        let b: Batch = e.wait(0, Duration::from_millis(120));
        assert!(b.items.is_empty() && !b.reset);
        assert!(t0.elapsed() >= Duration::from_millis(110));
        let e2: Arc<Events> = Arc::clone(&e);
        let h = thread::spawn(move || e2.wait(0, Duration::from_secs(5)));
        thread::sleep(Duration::from_millis(50));
        e.push(&[Ev::Reset]);
        let got: Batch = h.join().unwrap();
        assert_eq!(got.items, vec![(1, Ev::Reset)]);
    }
}
