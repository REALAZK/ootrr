use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::error::{Error as E, Res};

pub const STOPPED: &str = "generation was cancelled";
pub const OVER: &str = "no seed was found within the time limit";

#[derive(Debug, Default)]
pub struct Live {
    pub tries: AtomicU64,
    pub fallback: AtomicBool,
    pub stop: AtomicBool,
}

impl Live {
    pub fn tries(&self) -> u64 {
        self.tries.load(Ordering::Relaxed)
    }

    pub fn in_fallback(&self) -> bool {
        self.fallback.load(Ordering::Relaxed)
    }

    pub fn cancel(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone)]
pub struct Watch {
    pub start: Instant,
    pub limit: Option<Duration>,
    pub live: Option<Arc<Live>>,
    fell: Cell<bool>,
}

impl Watch {
    pub fn new(limit: Option<Duration>, live: Option<Arc<Live>>) -> Watch {
        Watch { start: Instant::now(), limit, live, fell: Cell::new(false) }
    }

    pub fn idle() -> Watch {
        Watch::new(None, None)
    }

    pub fn check(&self) -> Res<()> {
        if let Some(l) = &self.live
            && l.stop.load(Ordering::Relaxed)
        {
            return Err(E::Msg(STOPPED.to_string()));
        }
        if let Some(t) = self.limit
            && self.start.elapsed() >= t
        {
            return Err(E::Msg(format!("{OVER} of {} seconds", t.as_secs())));
        }
        Ok(())
    }

    pub fn tick(&self) -> Res<()> {
        self.check()?;
        if let Some(l) = &self.live {
            l.tries.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }

    pub fn fell(&self) -> bool {
        self.fell.get()
    }

    pub fn falling_back(&self) {
        self.fell.set(true);
        if let Some(l) = &self.live {
            l.fallback.store(true, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zero_limit_stops_at_once_and_no_limit_never_does() {
        let w: Watch = Watch::new(Some(Duration::ZERO), None);
        assert!(w.tick().unwrap_err().to_string().contains("time limit"));
        assert!(Watch::idle().tick().is_ok());
    }

    #[test]
    fn a_fallback_is_remembered_for_the_rest_of_the_run() {
        let l: Arc<Live> = Arc::new(Live::default());
        let w: Watch = Watch::new(None, Some(Arc::clone(&l)));
        assert!(!w.fell() && !l.in_fallback());
        w.falling_back();
        assert!(w.fell() && l.in_fallback());
    }

    #[test]
    fn ticks_are_counted_and_a_cancel_stops_the_next_one() {
        let l: Arc<Live> = Arc::new(Live::default());
        let w: Watch = Watch::new(None, Some(Arc::clone(&l)));
        w.tick().unwrap();
        w.tick().unwrap();
        assert_eq!(l.tries(), 2);
        l.cancel();
        assert_eq!(w.tick().unwrap_err().to_string(), STOPPED);
        assert_eq!(l.tries(), 2);
    }
}
