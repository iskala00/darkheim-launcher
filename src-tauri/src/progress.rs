use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Default)]
pub struct ProgressCounters {
    pub downloaded: AtomicU64,
    pub total: AtomicU64,
}

impl ProgressCounters {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn add_total(&self, delta: u64) {
        self.total.fetch_add(delta, Ordering::Relaxed);
    }

    pub fn add_downloaded(&self, delta: u64) {
        self.downloaded.fetch_add(delta, Ordering::Relaxed);
    }

    pub fn get(&self) -> (u64, u64) {
        (
            self.downloaded.load(Ordering::Relaxed),
            self.total.load(Ordering::Relaxed),
        )
    }
}
