#![forbid(unsafe_code)]

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use anyhow::Result;
use clap::{Parser, CommandFactory};

mod cli;
pub mod commands;
mod telemetry;
pub mod repl;

use arc_core::config::ArcConfig;

fn get_config() -> &'static ArcConfig {
    ArcConfig::global()
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().unwrap_or(());
    
    let log_dir = arc_core::config::ArcConfig::dir().unwrap_or_else(|_| std::path::PathBuf::from(".arc")).join("logs");
    let _telemetry_guard = telemetry::init_telemetry(log_dir).unwrap_or_else(|e| {
        eprintln!("Failed to initialize telemetry: {}", e);
        std::process::exit(1);
    });

    let cli = cli::Cli::parse();
    // Load config early to handle auth/doctor seamlessly
    let config = get_config();

    let profile_dir = ArcConfig::dir().ok();
    let mut memory = arc_core::memory::MemoryManager::new(config.memory.clone(), profile_dir)?;

    match cli.command {
        Some(cli::Command::Setup) => {
            arc_core::setup_wizard::run_setup_wizard().await?;
        }
        Some(cli::Command::Stats) => {
            let data_dir = arc_core::config::ArcConfig::dir().unwrap_or_else(|_| std::path::PathBuf::from(".arc"));
            commands::stats::run(&data_dir).await?;
        }
        Some(cli::Command::Doctor) => {
            let data_dir = arc_core::config::ArcConfig::dir().unwrap_or_else(|_| std::path::PathBuf::from(".arc"));
            let config_path = data_dir.join("config.toml");
            commands::doctor::run(&data_dir, &config_path).await?;
        }
        Some(cli::Command::Init) => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            commands::init::run(&cwd).await?;
        }
        Some(cli::Command::Fix { max_iterations }) => {
            commands::fix::run(max_iterations).await?;
        }
        Some(cli::Command::Completions { shell }) => {
            let mut cmd = cli::Cli::command();
            let name = cmd.get_name().to_string();
            clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
        }
        Some(cli::Command::Update) => {
            commands::update::run().await?;
        }
        Some(cli::Command::Review { base }) => {
            commands::review::run(&base).await?;
        }
        Some(cli::Command::Auth { action }) => match action {
            cli::AuthAction::Status => print_auth_status(config)?,
            cli::AuthAction::Login => {
                arc_core::auth::oauth_google::authenticate_with_oauth(arc_core::credentials::Provider::Gemini).await?;
            }
            cli::AuthAction::Logout => logout_all()?,
            cli::AuthAction::SetKey { provider } => set_single_key(&provider).await?,
        },
        Some(cli::Command::Session { action }) => match action {
            cli::SessionAction::List => handle_history(&mut memory, None, None).await?,
            cli::SessionAction::Resume { id } => handle_history(&mut memory, None, Some(id)).await?,
            cli::SessionAction::Delete { id } => handle_history(&mut memory, Some(id), None).await?,
        },
        Some(cli::Command::History { delete, resume }) => {
            handle_history(&mut memory, delete, resume).await?;
        }
        Some(cli::Command::Memory { action }) => match action {
            cli::MemoryAction::Inspect => {
                let ctx = memory.get_context().await;
                println!("Memory context elements: {}", ctx.len());
                for msg in ctx {
                    println!("[{}] ({} tokens)\n{}", msg.role, msg.token_count, msg.content);
                }
            }
            cli::MemoryAction::Clear => {
                println!("Working memory cleared.");
            }
        },
        Some(cli::Command::Chat) | None => {
            if !arc_core::credentials::auth_status_all().iter().any(|s| s.has_api_key || s.has_oauth_access) && !config.providers.ollama.enabled {
                println!("👋 Welcome to ARC! Let's set up your providers first.\n");
                arc_core::setup_wizard::run_setup_wizard().await?;
            }
            if let Some(prompt) = cli.prompt {
                if cli.headless && cli.output_format == "json" {
                    use arc_providers::traits::Provider;
                    use arc_providers::message::{Message, Role};
                    
                    let client = reqwest::Client::builder().http2_prior_knowledge().build()?;
                    let provider = arc_providers::anthropic::AnthropicProvider::new(client, "".to_string());
                    let msgs = vec![Message { 
                        role: Role::User, 
                        content: prompt.clone(),
                        tool_calls: vec![],
                        tool_call_id: None,
                    }];
                    
                    let response_text = provider.generate_text("claude-3-5-sonnet-20241022", &msgs).await.unwrap_or_else(|e| e.to_string());
                    let json_out = serde_json::json!({
                        "prompt": prompt,
                        "status": "success",
                        "response": response_text
                    });
                    println!("{}", json_out.to_string());
                } else {
                    println!("Running one-shot prompt: {}", prompt);
                    let spinner = arc_tui::spinner::Spinner::new().message("Thinking").start();
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    spinner.finish("Done").await;
                }
            } else {
                repl::run_repl("".to_string()).await?;
            }
        }
        Some(cli::Command::Serve { port }) => {
            println!("Starting ARC server on port {port}");
        }
        Some(cli::Command::Config) => {
            println!("Config management not implemented yet.");
        }
    }

    Ok(())
}

async fn handle_history(memory: &mut arc_core::memory::MemoryManager, delete: Option<String>, resume: Option<String>) -> anyhow::Result<()> {
    if let Some(id) = delete {
        memory.delete_session(&id)?;
        println!("✅ Session {} deleted.", id);
    } else if let Some(id) = resume {
        println!("Resuming session {}...", id);
        memory.load_session(&id).await?;
        println!("✅ Session loaded! Ready for use.");
    } else {
        println!("📜 Past Sessions:");
        let sessions = memory.list_sessions()?;
        if sessions.is_empty() {
            println!("  No past sessions found.");
        } else {
            for session in sessions {
                println!(
                    "  [{}] {} - {} messages ({})",
                    session.id,
                    session.updated_at.format("%Y-%m-%d %H:%M"),
                    session.message_count,
                    session.summary
                );
            }
        }
    }
    Ok(())
}

fn print_auth_status(_config: &ArcConfig) -> anyhow::Result<()> {
    use console::style;

    println!("\n{}", style("ARC Authentication Status").bold().underlined());

    let statuses = arc_core::credentials::auth_status_all();
    for status in statuses {
        let mut parts = Vec::new();
        if status.has_api_key { parts.push("API Key"); }
        if status.has_oauth_access { parts.push("OAuth Access"); }
        if status.has_oauth_refresh { parts.push("OAuth Refresh"); }

        if parts.is_empty() {
            println!("  ✘ {:<12}: {}", status.provider.as_str(), style("Not authenticated").dim());
        } else {
            println!("  ✔ {:<12}: {}", status.provider.as_str(), style(parts.join(", ")).green());
        }
    }

    println!();
    Ok(())
}

fn logout_all() -> anyhow::Result<()> {
    arc_core::credentials::logout_all()?;
    println!("🗑️  All credentials removed from OS keyring.");
    Ok(())
}

async fn set_single_key(provider_str: &str) -> anyhow::Result<()> {
    let provider = arc_core::credentials::Provider::from_str(provider_str)
        .ok_or_else(|| anyhow::anyhow!("Unknown provider: {provider_str}. Use anthropic, openai, gemini, or ollama."))?;

    arc_core::auth::authenticate_provider(provider, "api_key").await?;
    println!("  ✔ Key updated for {provider}");
    Ok(())
}

