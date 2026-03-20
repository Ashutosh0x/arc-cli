//! # Cron Scheduling — /loop Recurring Prompts Within Session
//!
//! `/loop 5m check deploy` — schedule recurring prompts.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub interval: Duration,
    pub prompt: String,
    pub max_iterations: Option<u32>,
    pub iterations_run: u32,
    pub enabled: bool,
    #[serde(skip)]
    pub last_run: Option<Instant>,
}

impl CronJob {
    pub fn new(id: &str, interval: Duration, prompt: &str) -> Self {
        Self {
            id: id.to_string(),
            interval,
            prompt: prompt.to_string(),
            max_iterations: None,
            iterations_run: 0,
            enabled: true,
            last_run: None,
        }
    }

    pub fn should_run(&self) -> bool {
        if !self.enabled {
            return false;
        }
        if let Some(max) = self.max_iterations {
            if self.iterations_run >= max {
                return false;
            }
        }
        match self.last_run {
            Some(last) => last.elapsed() >= self.interval,
            None => true,
        }
    }

    pub fn record_run(&mut self) {
        self.iterations_run += 1;
        self.last_run = Some(Instant::now());
    }
}

pub struct CronScheduler {
    jobs: Vec<CronJob>,
    enabled: bool,
}

impl CronScheduler {
    pub fn new() -> Self {
        Self {
            jobs: Vec::new(),
            enabled: true,
        }
    }

    /// Parse interval string like "5m", "30s", "1h".
    pub fn parse_interval(s: &str) -> Result<Duration, String> {
        let s = s.trim();
        let (num_str, unit) = s.split_at(s.len().saturating_sub(1));
        let num: u64 = num_str
            .parse()
            .map_err(|_| format!("Invalid interval: {s}"))?;
        match unit {
            "s" => Ok(Duration::from_secs(num)),
            "m" => Ok(Duration::from_secs(num * 60)),
            "h" => Ok(Duration::from_secs(num * 3600)),
            _ => Err(format!("Unknown unit '{unit}', use s/m/h")),
        }
    }

    /// Add a cron job from /loop command: `/loop 5m check deploy`.
    pub fn add_loop(&mut self, interval_str: &str, prompt: &str) -> Result<String, String> {
        let interval = Self::parse_interval(interval_str)?;
        let id = format!("loop-{}", self.jobs.len());
        self.jobs.push(CronJob::new(&id, interval, prompt));
        Ok(id)
    }

    /// Get jobs ready to run.
    pub fn pending_jobs(&self) -> Vec<&CronJob> {
        if !self.enabled {
            return Vec::new();
        }
        self.jobs.iter().filter(|j| j.should_run()).collect()
    }

    /// Record a run for a job.
    pub fn mark_run(&mut self, id: &str) {
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
            job.record_run();
        }
    }

    /// Cancel a job.
    pub fn cancel(&mut self, id: &str) {
        self.jobs.retain(|j| j.id != id);
    }

    /// Cancel all jobs.
    pub fn cancel_all(&mut self) {
        self.jobs.clear();
    }

    /// Disable all scheduling.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn jobs(&self) -> &[CronJob] {
        &self.jobs
    }
}

impl Default for CronScheduler {
    fn default() -> Self {
        Self::new()
    }
}
