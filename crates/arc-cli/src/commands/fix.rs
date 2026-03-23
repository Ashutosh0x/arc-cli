// SPDX-License-Identifier: MIT
use anyhow::Result;
use arc_providers::message::{Message, Role};
use arc_providers::traits::Provider;
use console::style;
use std::process::Command;
// use arc_providers::anthropic::AnthropicProvider;
use std::env;

/// Executes a persistent test-fix heuristic feedback loop natively against local build invariants.
pub async fn run(max_iters: u32) -> Result<()> {
    println!(
        "\n  {}",
        style("ARC Test-Fix Loop (Autonomous Subagent)")
            .bold()
            .magenta()
    );
    println!("  Executing diagnostics up to {} times...\n", max_iters);

    let client = reqwest::Client::builder().http2_prior_knowledge().build()?;
    let api_key = env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| {
        keyring::Entry::new("arc_cli", "anthropic_api_key")
            .unwrap_or_else(|_| keyring::Entry::new("arc_cli", "fallback").unwrap())
            .get_password()
            .unwrap_or_default()
    });

    let provider = arc_providers::anthropic::AnthropicProvider::new(client, api_key);
    let mut iter = 1;

    let project_rules =
        std::fs::read_to_string("ARC.md").unwrap_or_else(|_| "No ARC.md found.".to_string());

    while iter <= max_iters {
        println!(
            "  {} {} {}",
            style("Iteration").cyan(),
            iter,
            style("Running `cargo test`...").cyan()
        );

        // 1. Execute cargo test
        let output = Command::new("cargo").arg("test").output()?;

        if output.status.success() {
            println!(
                "  {} {}",
                style("✓").green(),
                style("All tests passed natively!").bold()
            );
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined_trace = format!("{}\n{}", stdout, stderr);

        println!(
            "  {} Test bounds failed. Parsing diagnostic trace and launching subagent...",
            style("✗").red()
        );

        // 2. Format LLM fix request
        let prompt = format!(
            "The following `cargo test` sequence failed on my project.\n{}\n\nTrace Output:\n```rust\n{}\n```\nAnalyze the error precisely, find the associated file, and write a concrete fix. Be extremely concise.",
            project_rules, combined_trace
        );

        let messages = vec![Message {
            role: Role::User,
            content: prompt,
            tool_calls: vec![],
            tool_call_id: None,
        }];

        // 3. Delegate to Agent
        println!(
            "  🤖 {} Analyzing failure map natively...",
            style("Subagent").magenta()
        );
        let response = provider
            .generate_text("claude-3-5-sonnet-20241022", &messages)
            .await?;

        println!("\n  {}", style("Subagent Diagnosis:").bold());
        println!("{}", response);

        println!(
            "\n  {} Waiting 3 seconds before next loop...",
            style("ℹ").yellow()
        );
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        iter += 1;
    }

    println!(
        "  {} Reached maximum iterations ({}) without a passing state. Halting autonomous feedback loops.",
        style("✗").red(),
        max_iters
    );
    Ok(())
}
