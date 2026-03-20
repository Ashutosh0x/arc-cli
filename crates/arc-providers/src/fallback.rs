//! Policy-Driven Fallback Handler
//!
//! Classifies failure kinds and resolves fallback chains.
//! Supports intents: retry_always, retry_once, stop, upgrade.

use crate::availability::{ModelAvailabilityService, UnavailabilityReason};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureKind {
    Quota,
    Capacity,
    Transient,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FallbackIntent {
    RetryAlways,
    RetryOnce,
    Stop,
    RetryLater,
    Upgrade,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FallbackAction {
    Silent,
    Ask,
}

#[derive(Debug, Clone)]
pub struct FallbackPolicy {
    pub model: String,
    pub is_last_resort: bool,
    pub action_on_quota: FallbackAction,
    pub action_on_capacity: FallbackAction,
    pub action_on_transient: FallbackAction,
}

pub struct FallbackResult {
    pub success: bool,
    pub switched_to: Option<String>,
    pub intent: Option<FallbackIntent>,
}

/// Classify the kind of failure from an error message or status code.
pub fn classify_failure(error_msg: &str, status_code: Option<u16>) -> FailureKind {
    if let Some(code) = status_code {
        match code {
            429 => return FailureKind::Quota,
            503 | 502 => return FailureKind::Capacity,
            _ => {},
        }
    }

    let lower = error_msg.to_lowercase();
    if lower.contains("quota") || lower.contains("rate limit") || lower.contains("429") {
        FailureKind::Quota
    } else if lower.contains("capacity") || lower.contains("overloaded") || lower.contains("503") {
        FailureKind::Capacity
    } else if lower.contains("timeout") || lower.contains("connection") {
        FailureKind::Transient
    } else {
        FailureKind::Unknown
    }
}

/// Resolve the action for a given failure kind and policy.
pub fn resolve_policy_action(kind: &FailureKind, policy: &FallbackPolicy) -> FallbackAction {
    match kind {
        FailureKind::Quota => policy.action_on_quota.clone(),
        FailureKind::Capacity => policy.action_on_capacity.clone(),
        FailureKind::Transient => policy.action_on_transient.clone(),
        FailureKind::Unknown => FallbackAction::Ask,
    }
}

/// Apply the availability transition for a failed model.
pub fn apply_availability_transition(
    availability: &mut ModelAvailabilityService,
    model: &str,
    kind: &FailureKind,
) {
    match kind {
        FailureKind::Quota => {
            availability.mark_terminal(model, UnavailabilityReason::Quota);
        },
        FailureKind::Capacity => {
            availability.mark_terminal(model, UnavailabilityReason::Capacity);
        },
        FailureKind::Transient => {
            availability.mark_retry_once_per_turn(model);
        },
        FailureKind::Unknown => {
            availability.mark_retry_once_per_turn(model);
        },
    }
}

/// Handle a model failure with a fallback chain.
pub fn handle_fallback(
    availability: &mut ModelAvailabilityService,
    failed_model: &str,
    chain: &[FallbackPolicy],
    intent: FallbackIntent,
    error_msg: &str,
    status_code: Option<u16>,
) -> FallbackResult {
    let failure_kind = classify_failure(error_msg, status_code);

    // Find candidates (everything except the failed model)
    let candidates: Vec<&FallbackPolicy> =
        chain.iter().filter(|p| p.model != failed_model).collect();

    if candidates.is_empty() {
        return FallbackResult {
            success: false,
            switched_to: None,
            intent: Some(intent),
        };
    }

    // Select first available
    let candidate_models: Vec<String> = candidates.iter().map(|p| p.model.clone()).collect();
    let selection = availability.select_first_available(&candidate_models);

    let fallback_model = selection.selected_model.or_else(|| {
        candidates
            .iter()
            .find(|p| p.is_last_resort)
            .map(|p| p.model.clone())
    });

    let Some(fallback) = fallback_model else {
        return FallbackResult {
            success: false,
            switched_to: None,
            intent: Some(intent),
        };
    };

    match intent {
        FallbackIntent::RetryAlways | FallbackIntent::RetryOnce => {
            apply_availability_transition(availability, failed_model, &failure_kind);
            FallbackResult {
                success: true,
                switched_to: Some(fallback),
                intent: Some(intent),
            }
        },
        FallbackIntent::Stop | FallbackIntent::RetryLater => FallbackResult {
            success: false,
            switched_to: None,
            intent: Some(intent),
        },
        FallbackIntent::Upgrade => FallbackResult {
            success: false,
            switched_to: None,
            intent: Some(FallbackIntent::Upgrade),
        },
    }
}
