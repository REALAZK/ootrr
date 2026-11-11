use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ootrr_core::json::Json;

use crate::guard::Guard;
use crate::jobs::Jobs;

pub type Cached = Mutex<Option<(Instant, Duration, Json)>>;

pub struct Shared {
    pub guard: Guard,
    pub jobs: Arc<Jobs>,
    pub updates: bool,
    pub cache: Cached,
}

impl Shared {
    pub fn new(guard: Guard, updates: bool) -> Shared {
        Shared { guard, jobs: Jobs::new(), updates, cache: Mutex::new(None) }
    }
}
