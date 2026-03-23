// SPDX-License-Identifier: MIT
//! # Ralph Loop — Autonomous Iteration with Stop-Hook
//!
//! `/ralph-loop` with `--max-iterations` and `--completion-promise`.
//! Stop-hook-driven autonomous iteration that prevents session exit.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoopStatus {
    Running,
    Paused,
    Completed,
    MaxIterationsReached,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RalphLoopConfig {
    pub max_iterations: u32,
    pub completion_promise: String,
    #[serde(default = "default_cooldown")]
    pub cooldown_ms: u64,
    #[serde(default)]
    pub stop_on_error: bool,
}

fn default_cooldown() -> u64 {
    2000
}

impl Default for RalphLoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            completion_promise: "Task is fully complete".into(),
            cooldown_ms: 2000,
            stop_on_error: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationRecord {
    pub iteration: u32,
    pub prompt: String,
    pub result_summary: String,
    pub duration_ms: u64,
    pub files_changed: Vec<String>,
}

pub struct RalphLoop {
    config: RalphLoopConfig,
    status: LoopStatus,
    iterations_completed: u32,
    history: Vec<IterationRecord>,
    started_at: Option<Instant>,
}

impl RalphLoop {
    pub fn new(config: RalphLoopConfig) -> Self {
        Self {
            config,
            status: LoopStatus::Running,
            iterations_completed: 0,
            history: Vec::new(),
            started_at: None,
        }
    }

    /// Start the loop.
    pub fn start(&mut self) {
        self.started_at = Some(Instant::now());
        self.status = LoopStatus::Running;
    }

    /// Check if another iteration should run.
    pub fn should_continue(&self) -> bool {
        self.status == LoopStatus::Running && self.iterations_completed < self.config.max_iterations
    }

    /// Generate the next iteration prompt.
    pub fn next_prompt(&self) -> String {
        if self.iterations_completed == 0 {
            format!(
                "Begin working towards: {}\nCompletion criteria: {}\nMax iterations: {}",
                self.config.completion_promise,
                self.config.completion_promise,
                self.config.max_iterations
            )
        } else {
            let prev = self
                .history
                .last()
                .map(|r| r.result_summary.as_str())
                .unwrap_or("No previous output");
            format!(
                "Continue working. Iteration {}/{}.\nPrevious result: {prev}\nCompletion criteria: {}\nIf complete, say DONE.",
                self.iterations_completed + 1,
                self.config.max_iterations,
                self.config.completion_promise
            )
        }
    }

    /// Record an iteration result.
    pub fn record_iteration(
        &mut self,
        result_summary: &str,
        files_changed: Vec<String>,
        duration_ms: u64,
    ) {
        self.iterations_completed += 1;
        self.history.push(IterationRecord {
            iteration: self.iterations_completed,
            prompt: self.next_prompt(),
            result_summary: result_summary.to_string(),
            duration_ms,
            files_changed,
        });

        // Check for completion.
        if result_summary.contains("DONE") || result_summary.contains("complete") {
            self.status = LoopStatus::Completed;
        } else if self.iterations_completed >= self.config.max_iterations {
            self.status = LoopStatus::MaxIterationsReached;
        }
    }

    /// Record an error.
    pub fn record_error(&mut self) {
        if self.config.stop_on_error {
            self.status = LoopStatus::Error;
        }
    }

    /// Pause the loop.
    pub fn pause(&mut self) {
        self.status = LoopStatus::Paused;
    }
    /// Resume the loop.
    pub fn resume(&mut self) {
        if self.status == LoopStatus::Paused {
            self.status = LoopStatus::Running;
        }
    }

    pub fn status(&self) -> LoopStatus {
        self.status
    }
    pub fn iterations(&self) -> u32 {
        self.iterations_completed
    }
    pub fn history(&self) -> &[IterationRecord] {
        &self.history
    }
    pub fn elapsed(&self) -> Duration {
        self.started_at.map(|s| s.elapsed()).unwrap_or_default()
    }
}
