use anyhow::{Context, Result};
use console::style;
use futures::StreamExt;
use std::env;
use std::io::{self, Write};
use std::process::Command as OsCommand;

use arc_providers::anthropic::AnthropicProvider;
use arc_providers::message::{Message, Role, StreamEvent};
use arc_providers::traits::Provider;
use arc_tui::spinner::Spinner;

pub async fn run(base: &str) -> Result<()> {
    println!(
        "\n  {}",
        style("ARC PR Auto-Review (Autonomous Subagent)")
            .bold()
            .magenta()
    );

    let spinner = Spinner::new()
        .message(format!("Fetching and diffing against {}...", base))
        .start();

    // Attempt git fetch (ignore failures as user might be offline)
    let _ = OsCommand::new("git").arg("fetch").arg("origin").output();

    let output = OsCommand::new("git")
        .arg("diff")
        .arg(base)
        .output()
        .context("Failed to execute git diff. Are you in a git repository?")?;

    spinner.finish("✅ Diff captured").await;

    let diff_str = String::from_utf8_lossy(&output.stdout);
    if diff_str.trim().is_empty() {
        println!(
            "  {}",
            style("No differences found. Your branch is up to date!").green()
        );
        return Ok(());
    }

    println!(
        "  {} Analyzing {} bytes of architectural changes...",
        style("🤖").magenta(),
        diff_str.len()
    );

    let client = reqwest::Client::builder().http2_prior_knowledge().build()?;
    let api_key = env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| {
        keyring::Entry::new("arc_cli", "anthropic_api_key")
            .unwrap_or_else(|_| keyring::Entry::new("arc_cli", "fallback").unwrap())
            .get_password()
            .unwrap_or_default()
    });

    if api_key.is_empty() {
        return Err(anyhow::anyhow!(
            "No ANTHROPIC_API_KEY found. Please run `arc auth login` or set the environment variable."
        ));
    }

    let provider = AnthropicProvider::new(client, api_key);

    let prompt = format!(
        "You are ARC, an expert Principal Software Engineer. Provide a comprehensive architectural PR review of the following Git diff.\n\
        Catch bugs, highlight security vectors, and suggest ergonomic improvements.\n\
        Output in dense, readable Markdown. Do not hallucinate.\n\n\
        ```diff\n{}\n```",
        diff_str
    );

    let messages = vec![Message {
        role: Role::User,
        content: prompt,
        tool_calls: vec![],
        tool_call_id: None,
    }];

    let mut stream = provider
        .stream("claude-3-5-sonnet-20241022", &messages, &[])
        .await?;

    println!("\n{}\n", style("── PR Auto-Review ──").bold().underlined());

    let cancel_token = tokio_util::sync::CancellationToken::new();
    let token_clone = cancel_token.clone();

    let ctrl_c_task = tokio::spawn(async move {
        if let Ok(_) = tokio::signal::ctrl_c().await {
            token_clone.cancel();
        }
    });

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                println!("\n  ⚠️ [Agent Interrupt] Stream cancelled by user.");
                break;
            }
            chunk = stream.next() => {
                match chunk {
                    Some(Ok(StreamEvent::TextDelta(text))) => {
                        print!("{}", text);
                        io::stdout().flush().unwrap_or(());
                    }
                    Some(Ok(StreamEvent::Done)) => break,
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        eprintln!("\n[Stream Disconnect Error]: {}", e);
                        break;
                    }
                    None => break,
                }
            }
        }
    }

    ctrl_c_task.abort();
    println!("\n\n{}\n", style("────────────────────").bold());

    // Fallback best-effort desktop notification
    let _ = notify_rust::Notification::new()
        .summary("ARC PR Review")
        .body("Code review subagent has finished architectural critique.")
        .show();

    Ok(())
}
