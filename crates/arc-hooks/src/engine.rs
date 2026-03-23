// SPDX-License-Identifier: MIT
//! The hook engine: central coordinator that integrates with the ARC agent loop.
//!
//! Usage:
//!   let engine = HookEngine::load(project_root)?;
//!   // Before a tool call:
//!   let decision = engine.fire(PreToolUse { ... }).await;
//!   if decision.is_blocked() { /* skip the tool call */ }

use std::path::Path;
use std::sync::Arc;

use dashmap::DashMap;
use tracing::{debug, info, warn};

use crate::config::HooksConfig;
use crate::events::HookEvent;
use crate::executor::{self, HookOutcome, HookResult};
use crate::security_presets;

/// Decision made by the hook engine after processing all matching hooks.
#[derive(Debug, Clone)]
pub struct HookDecision {
    pub results: Vec<HookResult>,
    pub action: HookAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookAction {
    /// Proceed with the original action.
    Proceed,
    /// Block the action. Contains the reason from the blocking hook.
    Block { reason: String, hook_name: String },
}

impl HookDecision {
    pub fn is_blocked(&self) -> bool {
        matches!(self.action, HookAction::Block { .. })
    }

    pub fn is_allowed(&self) -> bool {
        matches!(self.action, HookAction::Proceed)
    }
}

/// The hook engine manages all hook configurations and dispatches events.
pub struct HookEngine {
    config: HooksConfig,
    /// Cache of hook execution stats for performance monitoring.
    stats: Arc<DashMap<String, HookStats>>,
}

#[derive(Debug, Default, Clone)]
pub struct HookStats {
    pub invocation_count: u64,
    pub total_duration_ms: u64,
    pub block_count: u64,
    pub error_count: u64,
}

impl HookEngine {
    /// Load hooks from the project directory, merging global + local + security defaults.
    pub fn load(project_root: &Path) -> Self {
        let mut config = HooksConfig::load_merged(project_root);

        // Merge in security defaults (can be overridden by user hooks)
        let security = security_presets::default_security_hooks();
        for (name, hook) in security.hooks {
            config
                .hooks
                .entry(name)
                .or_insert(hook);
        }

        let enabled_count = config.hooks.values().filter(|h| h.enabled).count();
        info!(
            total_hooks = config.hooks.len(),
            enabled_hooks = enabled_count,
            "Hook engine loaded"
        );

        Self {
            config,
            stats: Arc::new(DashMap::new()),
        }
    }

    /// Create an engine with a specific config (useful for testing).
    pub fn with_config(config: HooksConfig) -> Self {
        Self {
            config,
            stats: Arc::new(DashMap::new()),
        }
    }

    /// Fire an event through the hook system.
    ///
    /// For blocking events (PreToolUse, Stop, UserPromptSubmit):
    ///   - Hooks run sequentially by priority
    ///   - First Block outcome stops the chain
    ///
    /// For non-blocking events (PostToolUse, SessionStart, Notification, etc.):
    ///   - Hooks run in parallel
    ///   - Block outcomes are logged but don't affect anything
    pub async fn fire(&self, event: HookEvent) -> HookDecision {
        let matching = self.config.matching_hooks(&event);

        if matching.is_empty() {
            return HookDecision {
                results: vec![],
                action: HookAction::Proceed,
            };
        }

        debug!(
            event = event.event_name(),
            hook_count = matching.len(),
            "Firing hooks"
        );

        let is_blocking_event = matches!(
            event,
            HookEvent::PreToolUse(_)
                | HookEvent::Stop(_)
                | HookEvent::UserPromptSubmit(_)
                | HookEvent::PermissionRequest(_)
        );

        if is_blocking_event {
            let (results, blocked) =
                executor::execute_hook_chain(matching, &event).await;

            // Update stats
            for result in &results {
                self.update_stats(result);
            }

            let action = if blocked {
                let blocking_result = results
                    .iter()
                    .find(|r| matches!(r.outcome, HookOutcome::Block { .. }))
                    .unwrap();

                let reason = match &blocking_result.outcome {
                    HookOutcome::Block { reason } => reason.clone(),
                    _ => "Unknown".into(),
                };

                warn!(
                    hook = %blocking_result.hook_name,
                    reason = %reason,
                    event = event.event_name(),
                    "Hook BLOCKED action"
                );

                HookAction::Block {
                    reason,
                    hook_name: blocking_result.hook_name.clone(),
                }
            } else {
                HookAction::Proceed
            };

            HookDecision { results, action }
        } else {
            // Non-blocking: run in parallel
            let hooks: Vec<_> = matching
                .into_iter()
                .map(|(name, hook)| (name.to_string(), hook.clone()))
                .collect();

            let results = executor::execute_hooks_parallel(hooks, &event).await;

            for result in &results {
                self.update_stats(result);
            }

            HookDecision {
                results,
                action: HookAction::Proceed,
            }
        }
    }

    fn update_stats(&self, result: &HookResult) {
        let mut entry = self
            .stats
            .entry(result.hook_name.clone())
            .or_insert_with(HookStats::default);
        entry.invocation_count += 1;
        entry.total_duration_ms += result.duration_ms;
        if matches!(result.outcome, HookOutcome::Block { .. }) {
            entry.block_count += 1;
        }
        if matches!(result.outcome, HookOutcome::Error { .. }) {
            entry.error_count += 1;
        }
    }

    /// Get execution statistics for all hooks.
    pub fn get_stats(&self) -> Vec<(String, HookStats)> {
        self.stats
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Add a hook dynamically at runtime (e.g., from a plugin installation).
    pub fn register_hook(&mut self, name: String, hook: crate::config::HookDefinition) {
        info!(hook_name = %name, "Dynamically registered hook");
        self.config.hooks.insert(name, hook);
    }

    /// Remove a hook by name.
    pub fn unregister_hook(&mut self, name: &str) -> bool {
        self.config.hooks.remove(name).is_some()
    }

    /// Disable all hooks from a specific plugin.
    pub fn disable_plugin_hooks(&mut self, plugin_name: &str) {
        for hook in self.config.hooks.values_mut() {
            if hook.installed_by_plugin.as_deref() == Some(plugin_name) {
                hook.enabled = false;
            }
        }
    }
}
