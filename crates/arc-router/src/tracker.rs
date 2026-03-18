use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct UsageTracker {
    requests_today: DashMap<String, AtomicUsize>,
}

impl UsageTracker {
    pub fn new() -> Self {
        Self {
            requests_today: DashMap::new(),
        }
    }

    pub fn record_usage(&self, provider: &str) {
        self.requests_today
            .entry(provider.to_string())
            .or_insert_with(|| AtomicUsize::new(0))
            .fetch_add(1, Ordering::SeqCst);
    }

    pub fn get_usage(&self, provider: &str) -> usize {
        self.requests_today
            .get(provider)
            .map(|val| val.value().load(Ordering::SeqCst))
            .unwrap_or(0)
    }
}
