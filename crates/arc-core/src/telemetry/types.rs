//! Data types for the telemetry subsystem.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single LLM request record persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestRecord {
    /// Unix timestamp (millis).
    pub timestamp_ms: u64,
    /// Provider name (e.g. `"anthropic"`, `"openai"`).
    pub provider: String,
    /// Model identifier.
    pub model: String,
    /// Input tokens consumed.
    pub input_tokens: u32,
    /// Output tokens generated.
    pub output_tokens: u32,
    /// Wall-clock latency in milliseconds.
    pub latency_ms: u64,
    /// Estimated cost in USD.
    pub cost_usd: f64,
    /// Whether the request succeeded.
    pub success: bool,
    /// Optional error message on failure.
    pub error: Option<String>,
}

/// Aggregated statistics for a single provider.
#[derive(Debug, Clone, Default)]
pub struct ProviderStats {
    pub provider: String,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
    /// Raw latency samples for percentile computation.
    pub latencies_ms: Vec<u64>,
}

impl ProviderStats {
    /// Compute a latency percentile (0.0–100.0).
    ///
    /// Returns `None` if there are no samples.
    pub fn percentile(&self, p: f64) -> Option<u64> {
        if self.latencies_ms.is_empty() {
            return None;
        }
        let mut sorted = self.latencies_ms.clone();
        sorted.sort_unstable();
        let idx = ((p / 100.0) * (sorted.len() as f64 - 1.0))
            .round()
            .max(0.0) as usize;
        sorted.get(idx).copied()
    }

    /// Median latency.
    pub fn p50(&self) -> Option<u64> {
        self.percentile(50.0)
    }
    /// 95th percentile latency.
    pub fn p95(&self) -> Option<u64> {
        self.percentile(95.0)
    }
    /// 99th percentile latency.
    pub fn p99(&self) -> Option<u64> {
        self.percentile(99.0)
    }
    /// Error rate as a percentage.
    pub fn error_rate_pct(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        (self.failed_requests as f64 / self.total_requests as f64) * 100.0
    }
}

/// Full telemetry summary across all providers.
#[derive(Debug, Clone)]
pub struct TelemetrySummary {
    /// Per-provider breakdown.
    pub providers: HashMap<String, ProviderStats>,
    /// Global totals.
    pub total_requests: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
    /// Earliest record timestamp.
    pub first_record_ms: Option<u64>,
    /// Latest record timestamp.
    pub last_record_ms: Option<u64>,
}

impl TelemetrySummary {
    /// Days of data covered.
    pub fn span_days(&self) -> f64 {
        match (self.first_record_ms, self.last_record_ms) {
            (Some(first), Some(last)) if last > first => {
                (last - first) as f64 / (1000.0 * 60.0 * 60.0 * 24.0)
            }
            _ => 0.0,
        }
    }
}
