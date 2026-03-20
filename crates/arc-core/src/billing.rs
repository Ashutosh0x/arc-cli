//! Billing & Quota Management
//!
//! Tracks API credit balance per provider, overage strategy,
//! and cost estimation.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverageStrategy {
    Ask,
    Always,
    Never,
}

impl Default for OverageStrategy {
    fn default() -> Self {
        Self::Ask
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditBalance {
    pub provider: String,
    pub credit_type: String,
    pub amount: i64,
    pub currency: String,
}

pub const MIN_CREDIT_BALANCE: i64 = 50;

/// Check if credits should be automatically used.
pub fn should_auto_use_credits(strategy: &OverageStrategy, balance: Option<i64>) -> bool {
    matches!(strategy, OverageStrategy::Always)
        && balance.map_or(false, |b| b >= MIN_CREDIT_BALANCE)
}

/// Check if the overage prompt menu should be shown.
pub fn should_show_overage_menu(strategy: &OverageStrategy, balance: Option<i64>) -> bool {
    matches!(strategy, OverageStrategy::Ask) && balance.map_or(false, |b| b >= MIN_CREDIT_BALANCE)
}

/// Check if the empty wallet message should be shown.
pub fn should_show_empty_wallet(strategy: &OverageStrategy, balance: Option<i64>) -> bool {
    !matches!(strategy, OverageStrategy::Never) && balance.map_or(false, |b| b < MIN_CREDIT_BALANCE)
}

/// Estimate token cost for a request.
pub fn estimate_cost(input_tokens: u64, output_tokens: u64, model: &str) -> f64 {
    let (input_rate, output_rate) = match model {
        m if m.contains("gpt-4") => (0.03, 0.06), // per 1K tokens
        m if m.contains("gpt-3.5") => (0.0015, 0.002),
        m if m.contains("claude-3-opus") => (0.015, 0.075),
        m if m.contains("claude-3-sonnet") => (0.003, 0.015),
        m if m.contains("claude-3-haiku") => (0.00025, 0.00125),
        m if m.contains("gemini-pro") => (0.00125, 0.005),
        m if m.contains("gemini-flash") => (0.000075, 0.0003),
        _ => (0.001, 0.002), // default conservative estimate
    };
    (input_tokens as f64 / 1000.0 * input_rate) + (output_tokens as f64 / 1000.0 * output_rate)
}

/// Format a cost estimate for display.
pub fn format_cost(cost: f64) -> String {
    if cost < 0.01 {
        format!("<$0.01")
    } else {
        format!("${cost:.2}")
    }
}

#[derive(Debug, Clone, Default)]
pub struct SessionCostTracker {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost: f64,
    pub request_count: u32,
}

impl SessionCostTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, input_tokens: u64, output_tokens: u64, model: &str) {
        self.total_input_tokens += input_tokens;
        self.total_output_tokens += output_tokens;
        self.total_cost += estimate_cost(input_tokens, output_tokens, model);
        self.request_count += 1;
    }

    pub fn summary(&self) -> String {
        format!(
            "Session: {} requests, {} input tokens, {} output tokens, {}",
            self.request_count,
            self.total_input_tokens,
            self.total_output_tokens,
            format_cost(self.total_cost)
        )
    }
}
