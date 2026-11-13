use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use ootrr_core::json::Json;
use ootrr_core::slice::watch::Live;

use crate::error::{Fail, Hr};
use crate::token;

pub const MAX_GENERATING: usize = 4;
pub const MAX_TICKET: usize = 64;

struct Run {
    ticket: String,
    start: Instant,
    live: Arc<Live>,
}

#[derive(Default)]
pub struct Gens {
    list: Mutex<Vec<Run>>,
}

pub struct Slot<'a> {
    gens: &'a Gens,
    pub live: Arc<Live>,
}

impl Drop for Slot<'_> {
    fn drop(&mut self) {
        self.gens.lock().retain(|r| !Arc::ptr_eq(&r.live, &self.live));
    }
}

pub fn ticket_ok(t: &str) -> bool {
    !t.is_empty() && t.len() <= MAX_TICKET && t.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

impl Gens {
    fn lock(&self) -> MutexGuard<'_, Vec<Run>> {
        self.list.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn begin(&self, ticket: Option<&str>) -> Hr<Slot<'_>> {
        let t: String = match ticket {
            Some(t) if ticket_ok(t) => t.to_string(),
            Some(_) => return Err(Fail::bad("the ticket must be up to 64 letters, digits, dashes or underscores")),
            None => token::token(),
        };
        let mut l: MutexGuard<'_, Vec<Run>> = self.lock();
        if l.len() >= MAX_GENERATING {
            return Err(Fail::new(429, "too many seeds are being generated at once, wait for one to finish"));
        }
        if l.iter().any(|r| r.ticket == t) {
            return Err(Fail::new(409, "a seed with this ticket is already being generated"));
        }
        let live: Arc<Live> = Arc::new(Live::default());
        l.push(Run { ticket: t, start: Instant::now(), live: Arc::clone(&live) });
        Ok(Slot { gens: self, live })
    }

    pub fn progress(&self, ticket: &str) -> Json {
        let l: MutexGuard<'_, Vec<Run>> = self.lock();
        match l.iter().find(|r| r.ticket == ticket) {
            Some(r) => Json::obj()
                .with("running", true.into())
                .with("seconds", Json::Int(r.start.elapsed().as_secs() as i64))
                .with("attempts", Json::Int(r.live.tries() as i64))
                .with("fallback", r.live.in_fallback().into()),
            None => Json::obj().with("running", false.into()),
        }
    }

    pub fn cancel(&self, ticket: &str) -> bool {
        let l: MutexGuard<'_, Vec<Run>> = self.lock();
        match l.iter().find(|r| r.ticket == ticket) {
            Some(r) => {
                r.live.cancel();
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slot_shows_progress_until_it_is_dropped() {
        let g: Gens = Gens::default();
        {
            let s: Slot = g.begin(Some("abc-1")).unwrap();
            s.live.tries.fetch_add(3, std::sync::atomic::Ordering::Relaxed);
            let p: Json = g.progress("abc-1");
            assert_eq!(p.get("running").and_then(|x| x.as_bool()), Some(true));
            assert_eq!(p.get("attempts").and_then(|x| x.as_i64()), Some(3));
            assert_eq!(g.begin(Some("abc-1")).err().unwrap().status, 409);
            assert!(g.cancel("abc-1"));
            assert!(s.live.stop.load(std::sync::atomic::Ordering::Relaxed));
        }
        assert_eq!(g.progress("abc-1").get("running").and_then(|x| x.as_bool()), Some(false));
        assert!(!g.cancel("abc-1"));
    }

    #[test]
    fn bad_tickets_and_too_many_runs_are_refused() {
        let g: Gens = Gens::default();
        assert_eq!(g.begin(Some("a b")).err().unwrap().status, 400);
        assert_eq!(g.begin(Some(&"x".repeat(65))).err().unwrap().status, 400);
        let held: Vec<Slot> = (0..MAX_GENERATING).map(|_| g.begin(None).unwrap()).collect();
        assert_eq!(g.begin(None).err().unwrap().status, 429);
        drop(held);
        assert!(g.begin(None).is_ok());
    }
}
