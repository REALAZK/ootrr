use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ootrr_core::json::Json;

use crate::datadir;
use crate::gen_live::Gens;
use crate::guard::Guard;
use crate::jobs::Jobs;
use crate::tracker::Tracker;

pub type Cached = Mutex<Option<(Instant, Duration, Json)>>;

pub struct Shared {
    pub guard: Guard,
    pub jobs: Arc<Jobs>,
    pub updates: bool,
    pub cache: Cached,
    pub tracker: Arc<Tracker>,
    pub gens: Gens,
}

impl Shared {
    pub fn new(guard: Guard, updates: bool) -> Shared {
        Shared::with_tracker(guard, updates, Arc::new(Tracker::new(datadir::base())))
    }

    pub fn with_tracker(guard: Guard, updates: bool, tracker: Arc<Tracker>) -> Shared {
        Shared { guard, jobs: Jobs::new(), updates, cache: Mutex::new(None), tracker, gens: Gens::default() }
    }
}
