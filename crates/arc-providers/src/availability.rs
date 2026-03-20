//! Model Availability Service
//!
//! Tracks per-model health with terminal vs sticky-retry failure modes.
//! Supports chain selection to find the first available model.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnavailabilityReason {
    Quota,
    Capacity,
    RetryOncePerTurn,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HealthState {
    Terminal { reason: UnavailabilityReason },
    StickyRetry { consumed: bool },
}

#[derive(Debug, Clone)]
pub struct AvailabilitySnapshot {
    pub available: bool,
    pub reason: Option<UnavailabilityReason>,
}

#[derive(Debug, Clone)]
pub struct ModelSelectionResult {
    pub selected_model: Option<String>,
    pub attempts: Option<u32>,
    pub skipped: Vec<(String, UnavailabilityReason)>,
}

pub struct ModelAvailabilityService {
    health: HashMap<String, HealthState>,
}

impl ModelAvailabilityService {
    pub fn new() -> Self {
        Self { health: HashMap::new() }
    }

    pub fn mark_terminal(&mut self, model: &str, reason: UnavailabilityReason) {
        self.health.insert(
            model.to_string(),
            HealthState::Terminal { reason },
        );
    }

    pub fn mark_healthy(&mut self, model: &str) {
        self.health.remove(model);
    }

    pub fn mark_retry_once_per_turn(&mut self, model: &str) {
        let current = self.health.get(model);
        // Don't override terminal with transient
        if matches!(current, Some(HealthState::Terminal { .. })) {
            return;
        }
        let consumed = match current {
            Some(HealthState::StickyRetry { consumed }) => *consumed,
            _ => false,
        };
        self.health.insert(model.to_string(), HealthState::StickyRetry { consumed });
    }

    pub fn consume_sticky_attempt(&mut self, model: &str) {
        if let Some(HealthState::StickyRetry { consumed }) = self.health.get_mut(model) {
            *consumed = true;
        }
    }

    pub fn snapshot(&self, model: &str) -> AvailabilitySnapshot {
        match self.health.get(model) {
            None => AvailabilitySnapshot { available: true, reason: None },
            Some(HealthState::Terminal { reason }) => AvailabilitySnapshot {
                available: false,
                reason: Some(reason.clone()),
            },
            Some(HealthState::StickyRetry { consumed: true }) => AvailabilitySnapshot {
                available: false,
                reason: Some(UnavailabilityReason::RetryOncePerTurn),
            },
            Some(HealthState::StickyRetry { consumed: false }) => AvailabilitySnapshot {
                available: true,
                reason: None,
            },
        }
    }

    /// Select the first available model from a prioritized list.
    pub fn select_first_available(&self, models: &[String]) -> ModelSelectionResult {
        let mut skipped = Vec::new();
        for model in models {
            let snap = self.snapshot(model);
            if snap.available {
                let attempts = match self.health.get(model.as_str()) {
                    Some(HealthState::StickyRetry { .. }) => Some(1),
                    _ => None,
                };
                return ModelSelectionResult {
                    selected_model: Some(model.clone()),
                    attempts,
                    skipped,
                };
            } else {
                skipped.push((model.clone(), snap.reason.unwrap_or(UnavailabilityReason::Unknown)));
            }
        }
        ModelSelectionResult { selected_model: None, attempts: None, skipped }
    }

    /// Reset turn-level state (un-consume sticky retries).
    pub fn reset_turn(&mut self) {
        for state in self.health.values_mut() {
            if let HealthState::StickyRetry { consumed } = state {
                *consumed = false;
            }
        }
    }

    pub fn reset(&mut self) {
        self.health.clear();
    }
}

impl Default for ModelAvailabilityService {
    fn default() -> Self {
        Self::new()
    }
}
