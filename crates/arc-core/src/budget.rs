use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenBudget {
    pub session_limit_tokens: Option<u64>,
    pub daily_limit_usd: Option<f64>,
    pub monthly_limit_usd: Option<f64>,

    pub session_tokens_used: u64,
    pub session_cost_usd: f64,
}

impl TokenBudget {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the upcoming request might exceed limits. The requested_tokens is an estimate.
    pub fn can_spend(&self, estimated_next_cost_usd: f64, estimated_next_tokens: u64) -> bool {
        if let Some(limit) = self.session_limit_tokens {
            if self.session_tokens_used + estimated_next_tokens > limit {
                return false;
            }
        }

        if let Some(limit) = self.daily_limit_usd {
            if self.session_cost_usd + estimated_next_cost_usd > limit {
                return false;
            }
        }

        // Add monthly limits tracking against a persistent store if necessary

        true
    }

    pub fn consume(&mut self, actual_tokens: u64, cost_usd: f64) {
        self.session_tokens_used += actual_tokens;
        self.session_cost_usd += cost_usd;
    }
}
