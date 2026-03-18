//! Interactive Setup Wizard — guides users through provider configuration.

use crate::auth;
use crate::config::{ArcConfig, ProviderEntry};
use crate::credentials::Provider;
use crate::error::ArcResult;
use crate::model_picker;
use crate::models::ModelRegistry;
use console::style;
use dialoguer::{MultiSelect, Select, theme::ColorfulTheme};

/// Run the full interactive setup wizard.
pub async fn run_setup_wizard() -> ArcResult<()> {
    println!("\n{}", style("╔══════════════════════════════════════╗").cyan());
    println!("{}", style("║     ARC — Setup Wizard               ║").cyan());
    println!("{}", style("║     Configure your AI providers      ║").cyan());
    println!("{}", style("╚══════════════════════════════════════╝").cyan());
    println!();

    let mut config = ArcConfig::load().unwrap_or_default();

    // Step 1: Select providers
    let provider_names = &["Anthropic (Claude)", "OpenAI (GPT/o-series)", "Google Gemini", "Ollama (Local)"];
    let selections = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select your AI providers (Space to toggle, Enter to confirm)")
        .items(provider_names)
        .defaults(&[true, false, false, false])
        .interact()
        .unwrap_or_default();

    let selected_providers: Vec<Provider> = selections
        .iter()
        .filter_map(|&idx| match idx {
            0 => Some(Provider::Anthropic),
            1 => Some(Provider::OpenAI),
            2 => Some(Provider::Gemini),
            3 => Some(Provider::Ollama),
            _ => None,
        })
        .collect();

    if selected_providers.is_empty() {
        println!("{}", style("⚠ No providers selected. Exiting setup.").yellow());
        return Ok(());
    }

    // Step 2: Authenticate each provider
    for &provider in &selected_providers {
        println!("\n{}", style(format!("── Configuring {provider} ──")).bold());

        let auth_method = if provider == Provider::Gemini {
            let methods = &["API Key (AI Studio)", "Google OAuth (Browser Sign-in)"];
            let choice = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Authentication method")
                .items(methods)
                .default(0)
                .interact()
                .unwrap_or(0);

            if choice == 1 { "oauth" } else { "api_key" }
        } else if provider == Provider::Ollama {
            // Ollama needs no auth
            println!("  {} Ollama uses local inference — no API key needed.", style("ℹ").blue());
            config.providers.ollama.enabled = true;
            continue;
        } else {
            "api_key"
        };

        auth::authenticate_provider(provider, auth_method).await?;

        // Update config
        match provider {
            Provider::Anthropic => {
                config.providers.anthropic = ProviderEntry {
                    enabled: true,
                    auth_method: auth_method.to_string(),
                };
            }
            Provider::OpenAI => {
                config.providers.openai = ProviderEntry {
                    enabled: true,
                    auth_method: auth_method.to_string(),
                };
            }
            Provider::Gemini => {
                config.providers.gemini = ProviderEntry {
                    enabled: true,
                    auth_method: auth_method.to_string(),
                };
            }
            Provider::Ollama => {
                config.providers.ollama.enabled = true;
            }
        }

        println!("  {} {provider} configured!", style("✓").green());
    }

    // Step 3: Model discovery
    println!("\n{}", style("── Discovering available models ──").bold());
    let http_client = reqwest::Client::new();
    let registry = ModelRegistry::discover_all(&http_client).await;
    println!(
        "  {} Found {} models across all providers",
        style("✓").green(),
        registry.models.len()
    );

    // Step 4: Pick default model
    if let Some(model_id) = model_picker::pick_model(&registry) {
        config.general.default_model = Some(model_id.clone());
        println!("  {} Default model: {model_id}", style("✓").green());
    }

    // Step 5: Routing strategy
    println!();
    config.routing.strategy = model_picker::pick_routing_strategy();

    // Step 6: Set fallback chain from selected providers
    config.routing.fallback_chain = selected_providers
        .iter()
        .map(|p| p.as_str().to_string())
        .collect();

    // Save
    config.save()?;

    println!("\n{}", style("╔══════════════════════════════════════╗").green());
    println!("{}", style("║     ✅ Setup complete!               ║").green());
    println!("{}", style("║     Run `arc` to start chatting      ║").green());
    println!("{}", style("╚══════════════════════════════════════╝").green());

    Ok(())
}
