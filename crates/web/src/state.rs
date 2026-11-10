use std::sync::Arc;

use crate::guard::Guard;
use crate::jobs::Jobs;

pub struct Shared {
    pub guard: Guard,
    pub jobs: Arc<Jobs>,
}
