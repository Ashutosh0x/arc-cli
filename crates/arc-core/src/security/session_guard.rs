// SPDX-License-Identifier: MIT
//! Session & context memory safety.
//! Detects behavioral anomalies, multi-stage attacks, and context manipulation.

use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use tracing::warn;

/// Maximum number of turns to retain for behavioral analysis.
const MAX_HISTORY: usize = 100;

/// A record of a single agent turn for anomaly detection.
#[derive(Debug, Clone)]
pub struct TurnRecord {
    pub timestamp: DateTime<Utc>,
    pub role: String,
    pub tool_calls: Vec<String>,
    pub token_count: u32,
    pub flagged: bool,
}

/// Session guard state — tracks behavioral patterns over time.
pub struct SessionGuard {
    history: VecDeque<TurnRecord>,
    consecutive_tool_calls: u32,
    total_tool_calls: u32,
    anomaly_count: u32,
}

impl SessionGuard {
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(MAX_HISTORY),
            consecutive_tool_calls: 0,
            total_tool_calls: 0,
            anomaly_count: 0,
        }
    }

    /// Record a turn and check for anomalies.
    pub fn record_turn(&mut self, record: TurnRecord) -> Vec<String> {
        let mut warnings = Vec::new();

        // Track consecutive tool calls
        if !record.tool_calls.is_empty() {
            self.consecutive_tool_calls += 1;
            self.total_tool_calls += record.tool_calls.len() as u32;

            // Flag if too many consecutive tool calls (possible multi-stage attack)
            if self.consecutive_tool_calls > 5 {
                warnings.push(format!(
                    "High consecutive tool call count: {} in a row",
                    self.consecutive_tool_calls
                ));
                warn!(
                    "Behavioral anomaly: {} consecutive tool calls",
                    self.consecutive_tool_calls
                );
                self.anomaly_count += 1;
            }
        } else {
            self.consecutive_tool_calls = 0;
        }

        // Detect rapid-fire requests (< 1 second between turns)
        if let Some(last) = self.history.back() {
            let elapsed = record.timestamp.signed_duration_since(last.timestamp);
            if elapsed.num_milliseconds() < 500 {
                warnings.push("Suspiciously fast turn detected (<500ms)".to_string());
                self.anomaly_count += 1;
            }
        }

        // Track for escalation patterns
        if record.flagged {
            self.anomaly_count += 1;
        }

        // Enforce history limit
        if self.history.len() >= MAX_HISTORY {
            self.history.pop_front();
        }
        self.history.push_back(record);

        // Hard stop if too many anomalies
        if self.anomaly_count > 10 {
            warnings
                .push("SESSION INTEGRITY THRESHOLD EXCEEDED — recommend termination".to_string());
        }

        warnings
    }

    pub fn anomaly_count(&self) -> u32 {
        self.anomaly_count
    }

    pub fn total_turns(&self) -> usize {
        self.history.len()
    }
}

impl Default for SessionGuard {
    fn default() -> Self {
        Self::new()
    }
}
