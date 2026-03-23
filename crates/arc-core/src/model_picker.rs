// SPDX-License-Identifier: MIT
//! Interactive model picker for the setup wizard.

use crate::models::ModelRegistry;
use dialoguer::{Select, theme::ColorfulTheme};

/// Prompt the user to select a model from the discovered registry.
pub fn pick_model(registry: &ModelRegistry) -> Option<String> {
    if registry.models.is_empty() {
        eprintln!("No models discovered. Please configure at least one provider first.");
        return None;
    }

    let items: Vec<String> = registry
        .models
        .iter()
        .map(|m| {
            format!(
                "[{}] {} ({}k ctx)",
                m.provider,
                m.display_name,
                m.context_window / 1000
            )
        })
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select your default model")
        .items(&items)
        .default(0)
        .interact_opt()
        .ok()
        .flatten();

    selection.map(|idx| registry.models[idx].id.clone())
}

/// Pick a routing strategy interactively.
pub fn pick_routing_strategy() -> crate::config::RoutingStrategy {
    let strategies = [
        "fallback-chain — Try providers in order, fall back on failure",
        "latency — Race providers, use fastest response",
        "cost-optimized — Prefer cheapest provider first",
        "round-robin — Distribute evenly across providers",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose your routing strategy")
        .items(&strategies)
        .default(0)
        .interact()
        .unwrap_or(0);

    match selection {
        0 => crate::config::RoutingStrategy::FallbackChain,
        1 => crate::config::RoutingStrategy::Latency,
        2 => crate::config::RoutingStrategy::CostOptimized,
        3 => crate::config::RoutingStrategy::RoundRobin,
        _ => crate::config::RoutingStrategy::FallbackChain,
    }
}
