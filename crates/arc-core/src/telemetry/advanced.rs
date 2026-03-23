// SPDX-License-Identifier: MIT
//! Advanced Telemetry — Activity Monitor, Memory Monitor, Startup Profiler

use std::time::{Duration, Instant};

// ── Activity Detector ───────────────────────────────────────────────────────

pub struct ActivityDetector {
    last_activity: Instant,
    idle_threshold: Duration,
}

impl ActivityDetector {
    pub fn new(idle_threshold: Duration) -> Self {
        Self {
            last_activity: Instant::now(),
            idle_threshold,
        }
    }

    pub fn record_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn is_idle(&self) -> bool {
        self.last_activity.elapsed() > self.idle_threshold
    }

    pub fn idle_duration(&self) -> Duration {
        self.last_activity.elapsed()
    }
}

// ── Memory Monitor ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct MemorySnapshot {
    pub rss_bytes: u64,
    pub heap_bytes: u64,
    pub timestamp_ms: u64,
}

pub struct MemoryMonitor {
    samples: Vec<MemorySnapshot>,
    high_water_mark: u64,
    warning_threshold_bytes: u64,
}

impl MemoryMonitor {
    pub fn new(warning_threshold_mb: u64) -> Self {
        Self {
            samples: Vec::new(),
            high_water_mark: 0,
            warning_threshold_bytes: warning_threshold_mb * 1024 * 1024,
        }
    }

    pub fn record(&mut self, snapshot: MemorySnapshot) {
        if snapshot.rss_bytes > self.high_water_mark {
            self.high_water_mark = snapshot.rss_bytes;
        }
        self.samples.push(snapshot);
    }

    pub fn high_water_mark_mb(&self) -> f64 {
        self.high_water_mark as f64 / (1024.0 * 1024.0)
    }

    pub fn is_above_threshold(&self) -> bool {
        self.high_water_mark > self.warning_threshold_bytes
    }

    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    pub fn average_rss_mb(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let total: u64 = self.samples.iter().map(|s| s.rss_bytes).sum();
        (total as f64 / self.samples.len() as f64) / (1024.0 * 1024.0)
    }
}

// ── Startup Profiler ────────────────────────────────────────────────────────

pub struct StartupProfiler {
    start: Instant,
    checkpoints: Vec<(String, Duration)>,
}

impl StartupProfiler {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            checkpoints: Vec::new(),
        }
    }

    pub fn checkpoint(&mut self, name: &str) {
        self.checkpoints
            .push((name.to_string(), self.start.elapsed()));
    }

    pub fn total_elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn report(&self) -> String {
        let mut lines = vec![format!(
            "Startup Profile (total: {:?})",
            self.total_elapsed()
        )];
        let mut prev = Duration::ZERO;
        for (name, elapsed) in &self.checkpoints {
            let delta = *elapsed - prev;
            lines.push(format!("  {name}: +{delta:?} (at {elapsed:?})"));
            prev = *elapsed;
        }
        lines.join("\n")
    }
}

impl Default for StartupProfiler {
    fn default() -> Self {
        Self::new()
    }
}

// ── High Water Mark Tracker ─────────────────────────────────────────────────

pub struct HighWaterMarkTracker {
    marks: std::collections::HashMap<String, f64>,
}

impl HighWaterMarkTracker {
    pub fn new() -> Self {
        Self {
            marks: std::collections::HashMap::new(),
        }
    }

    pub fn record(&mut self, metric: &str, value: f64) {
        let entry = self.marks.entry(metric.to_string()).or_insert(0.0);
        if value > *entry {
            *entry = value;
        }
    }

    pub fn get(&self, metric: &str) -> Option<f64> {
        self.marks.get(metric).copied()
    }

    pub fn all(&self) -> &std::collections::HashMap<String, f64> {
        &self.marks
    }
}

impl Default for HighWaterMarkTracker {
    fn default() -> Self {
        Self::new()
    }
}
