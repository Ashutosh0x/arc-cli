// ARC CLI — Main entry point (v3.0 Production)
// Supports both CLI subcommands AND interactive TUI mode.
//
//   arc-tui                         → Launch TUI (default)
//   arc-tui setup <model>           → Auto-start Ollama + pull model + save config
//   arc-tui ollama start            → Start Ollama server
//   arc-tui ollama status           → Check Ollama health + available models
//   arc-tui ollama pull <model>     → Pull a model
//   arc-tui ollama use <model>      → Set active model in config
//
// Wires: UI ↔ Orchestrator ↔ Agents ↔ LLM Providers
// Real async pipeline with event-driven state updates.

mod agents;
mod app;
mod config;
mod diff;
mod llm;
mod models;
mod ollama_manager;
mod state;
mod theme;
mod ui;

use std::io;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use tokio::sync::mpsc;

use app::{App, classify_prompt, PromptMode};
use config::ArcConfig;
use llm::LLMProvider;
use models::OrchestratorEvent;

// =====================================================================
//  CLI argument parsing (clap)
// =====================================================================

#[derive(Parser)]
#[command(name = "arc-tui", version = "3.0.0")]
#[command(about = "ARC CLI — Agentic AI Runtime for Developers")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Auto-setup: start Ollama + pull model + save as default
    Setup {
        /// Model to setup (e.g. gemma3:latest, llama3:8b)
        model: String,
    },
    /// Manage Ollama server lifecycle
    Ollama {
        #[command(subcommand)]
        action: OllamaCmd,
    },
}

#[derive(Subcommand)]
enum OllamaCmd {
    /// Start the Ollama server (if not already running)
    Start,
    /// Check Ollama health and list available models
    Status,
    /// Pull a model from the Ollama registry
    Pull {
        /// Model name to pull (e.g. gemma3:latest)
        model: String,
    },
    /// Set the active model (persisted to config)
    Use {
        /// Model name to use (e.g. gemma3:latest)
        model: String,
    },
}

// =====================================================================
//  Main entry point — routes between CLI and TUI
// =====================================================================

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Setup { model }) => cmd_setup(&model).await,
        Some(Commands::Ollama { action }) => cmd_ollama(action).await,
        None => run_tui().await,
    }
}

// =====================================================================
//  CLI: arc-tui setup <model>
// =====================================================================

async fn cmd_setup(model: &str) -> color_eyre::Result<()> {
    println!("[ARC] Setting up model: {}", model);

    let config = ArcConfig::load();
    let base_url = &config.ollama_host;

    // Step 1: Start Ollama if needed
    print!("[ARC] Checking Ollama server... ");
    match ollama_manager::auto_start(base_url).await {
        Ok(true) => println!("started!"),
        Ok(false) => println!("already running."),
        Err(e) => {
            println!("FAILED");
            eprintln!("[ERROR] {}", e);
            return Ok(());
        }
    }

    // Step 2: Pull model if not available
    if !ollama_manager::is_model_available(base_url, model).await {
        println!("[ARC] Pulling model '{}'... (this may take a while)", model);
        match ollama_manager::pull_model(base_url, model, None).await {
            Ok(_) => println!("[ARC] Model '{}' ready!", model),
            Err(e) => {
                eprintln!("[ERROR] Failed to pull model: {}", e);
                return Ok(());
            }
        }
    } else {
        println!("[ARC] Model '{}' already available.", model);
    }

    // Step 3: Save to config
    let new_config = ArcConfig {
        provider: "ollama".to_string(),
        model: model.to_string(),
        ollama_host: base_url.clone(),
    };
    match new_config.save() {
        Ok(_) => println!("[ARC] Config saved. Default model: {}", model),
        Err(e) => eprintln!("[WARN] Could not save config: {}", e),
    }

    println!();
    println!("[ARC] Ready! Run `arc-tui` to launch the TUI.");
    Ok(())
}

// =====================================================================
//  CLI: arc-tui ollama {start|status|pull|use}
// =====================================================================

async fn cmd_ollama(action: OllamaCmd) -> color_eyre::Result<()> {
    let config = ArcConfig::load();
    let base_url = &config.ollama_host;

    match action {
        OllamaCmd::Start => {
            print!("[ARC] Starting Ollama... ");
            match ollama_manager::auto_start(base_url).await {
                Ok(true) => println!("started!"),
                Ok(false) => println!("already running."),
                Err(e) => {
                    println!("FAILED");
                    eprintln!("[ERROR] {}", e);
                }
            }
        }
        OllamaCmd::Status => {
            let running = ollama_manager::is_running(base_url).await;
            println!("[ARC] Ollama Status");
            println!("  Host:    {}", base_url);
            println!("  Status:  {}", if running { "UP" } else { "DOWN" });

            if running {
                // List available models
                let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
                if let Ok(resp) = reqwest::get(&url).await {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(models) = json.get("models").and_then(|m| m.as_array()) {
                            println!("  Models:");
                            for m in models {
                                if let Some(name) = m.get("name").and_then(|n| n.as_str()) {
                                    let size = m
                                        .get("size")
                                        .and_then(|s| s.as_u64())
                                        .map(|s| format!(" ({:.1}GB)", s as f64 / 1_073_741_824.0))
                                        .unwrap_or_default();
                                    let active = if name == config.model || name.starts_with(&config.model) {
                                        " <-- active"
                                    } else {
                                        ""
                                    };
                                    println!("    - {}{}{}", name, size, active);
                                }
                            }
                        }
                    }
                }
            }

            println!();
            println!("  Config:  provider={}, model={}", config.provider, config.model);
        }
        OllamaCmd::Pull { model } => {
            if !ollama_manager::is_running(base_url).await {
                println!("[ARC] Ollama not running. Starting...");
                if let Err(e) = ollama_manager::auto_start(base_url).await {
                    eprintln!("[ERROR] {}", e);
                    return Ok(());
                }
            }
            println!("[ARC] Pulling model '{}'...", model);
            match ollama_manager::pull_model(base_url, &model, None).await {
                Ok(_) => println!("[ARC] Model '{}' ready!", model),
                Err(e) => eprintln!("[ERROR] {}", e),
            }
        }
        OllamaCmd::Use { model } => {
            let mut new_config = config;
            new_config.model = model.clone();
            new_config.provider = "ollama".to_string();
            match new_config.save() {
                Ok(_) => println!("[ARC] Active model set to: {}", model),
                Err(e) => eprintln!("[ERROR] Could not save config: {}", e),
            }
        }
    }

    Ok(())
}

// =====================================================================
//  TUI Mode — the full interactive terminal UI
// =====================================================================

async fn run_tui() -> color_eyre::Result<()> {
    // Force-enable VT processing on Windows by toggling raw mode once
    #[cfg(windows)]
    {
        let _ = enable_raw_mode();
        let _ = disable_raw_mode();
    }

    // Initialize state store
    let store = state::StateStore::new();
    let _ = store.init();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Run app
    let result = run_app(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {err:?}");
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> color_eyre::Result<()> {
    let mut app = App::new();

    // Drain any stale key events (e.g. the Enter press that launched this exe)
    while event::poll(Duration::from_millis(50))? {
        let _ = event::read()?;
    }

    // Load persisted config and apply default model selection
    let config = ArcConfig::load();

    // Initialize LLM router (with config's Ollama host)
    let llm_router = llm::LLMRouter::with_host(Some(config.ollama_host.clone()));

    // Run health checks in background
    {
        let ollama_provider = llm::ollama::OllamaProvider::new(Some(config.ollama_host.clone()));
        let health = ollama_provider.check_health().await;
        app.ollama_healthy = Some(health);
    }
    app.openai_healthy = Some(std::env::var("ARC_OPENAI_KEY").is_ok());

    // Channel for receiving orchestrator events
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<OrchestratorEvent>();

    loop {
        // ── Process all pending orchestrator events ──
        while let Ok(event) = event_rx.try_recv() {
            match event {
                OrchestratorEvent::TaskUpdate(task) => {
                    // Update existing task or insert new
                    if let Some(existing) = app.tasks.iter_mut().find(|t| t.id == task.id) {
                        *existing = task;
                    } else {
                        app.tasks.push(task);
                    }
                }
                OrchestratorEvent::Log(log) => {
                    app.agent_logs.push(log);
                }
                OrchestratorEvent::DiffProduced(diff) => {
                    app.current_diff = Some(diff);
                }
                OrchestratorEvent::Usage(usage) => {
                    app.llm_usage.push(usage);
                }
                OrchestratorEvent::Token(token) => {
                    if token == "[DONE]" {
                        app.streaming = false;
                    } else {
                        app.response_text.push_str(&token);
                    }
                }
                OrchestratorEvent::PipelineComplete => {
                    app.pipeline_running = false;
                    app.pipeline_complete = true;
                    app.streaming = false;
                }
                OrchestratorEvent::PipelineFailed(reason) => {
                    app.pipeline_running = false;
                    app.pipeline_failed = true;
                    app.pipeline_error = Some(reason.clone());
                    app.streaming = false;
                    app.agent_logs.push(models::AgentLog::error(
                        models::AgentKind::RepoMap,
                        format!("Pipeline FAILED: {}", reason),
                    ));
                }
            }
        }

        // ── Draw ──
        terminal.draw(|frame| ui::draw(frame, &app))?;

        // ── Handle input ──
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // ── EDITING MODE: typing into the prompt box ──
                if app.editing {
                    match key.code {
                        KeyCode::Esc => {
                            app.editing = false;
                        }
                        KeyCode::Backspace => {
                            app.prompt_text.pop();
                        }
                        KeyCode::Enter => {
                            let prompt = app.prompt_text.clone();
                            if prompt.trim().is_empty() {
                                continue;
                            }

                            // Classify prompt
                            let mode = classify_prompt(&prompt);

                            // Common reset
                            app.editing = false;
                            app.response_text.clear();
                            app.streaming = true;
                            app.pipeline_running = true;
                            app.pipeline_complete = false;
                            app.pipeline_failed = false;
                            app.pipeline_error = None;
                            app.scroll_offset = 0;
                            app.agent_logs.clear();
                            app.tasks.clear();
                            app.current_diff = None;
                            app.llm_usage.clear();
                            app.mode = mode;

                            let model = app::MODELS[app.selected_model].ollama_model.to_string();
                            let tag = app::MODELS[app.selected_model].tag;
                            let event_tx_clone = event_tx.clone();

                            // Get provider via the router
                            let provider: Arc<dyn llm::LLMProvider> = match llm_router.get_provider(tag) {
                                Ok(p) => p,
                                Err(err_msg) => {
                                    let _ = event_tx_clone.send(OrchestratorEvent::PipelineFailed(err_msg));
                                    continue;
                                }
                            };

                            // We already verified Ollama health at startup —
                            // skip ensure_ready() on each prompt to eliminate
                            // 2 redundant HTTP roundtrips (~1-2s saved).
                            let ollama_verified = app.ollama_healthy == Some(true);

                            match mode {
                                PromptMode::Chat => {
                                    // ── CHAT MODE: direct LLM → Output ──
                                    app.screen = app::Screen::Output;

                                    tokio::spawn(async move {
                                        if !ollama_verified && (tag == "Local" || tag == "OSS") {
                                            match ollama_manager::ensure_ready(&model, Some(&event_tx_clone)).await {
                                                Ok(_) => {}
                                                Err(e) => {
                                                    let _ = event_tx_clone.send(OrchestratorEvent::PipelineFailed(
                                                        format!("Ollama setup failed: {}", e),
                                                    ));
                                                    return;
                                                }
                                            }
                                        }

                                        run_chat(prompt, model, provider, event_tx_clone).await;
                                    });
                                }
                                PromptMode::FastCode => {
                                    // ── FAST CODE: skip Architect, direct to Coder → Output ──
                                    app.screen = app::Screen::Output;

                                    tokio::spawn(async move {
                                        if !ollama_verified && (tag == "Local" || tag == "OSS") {
                                            match ollama_manager::ensure_ready(&model, Some(&event_tx_clone)).await {
                                                Ok(_) => {}
                                                Err(e) => {
                                                    let _ = event_tx_clone.send(OrchestratorEvent::PipelineFailed(
                                                        format!("Ollama setup failed: {}", e),
                                                    ));
                                                    return;
                                                }
                                            }
                                        }

                                        run_fast_code(prompt, model, provider, event_tx_clone).await;
                                    });
                                }
                                PromptMode::Agent => {
                                    // ── AGENT MODE: full pipeline, but show Output for live streaming ──
                                    app.screen = app::Screen::Output;

                                    tokio::spawn(async move {
                                        if !ollama_verified && (tag == "Local" || tag == "OSS") {
                                            match ollama_manager::ensure_ready(&model, Some(&event_tx_clone)).await {
                                                Ok(_) => {}
                                                Err(e) => {
                                                    let _ = event_tx_clone.send(OrchestratorEvent::PipelineFailed(
                                                        format!("Ollama setup failed: {}", e),
                                                    ));
                                                    return;
                                                }
                                            }
                                        }

                                        agents::orchestrator::run_pipeline(
                                            prompt,
                                            model,
                                            provider,
                                            event_tx_clone,
                                        )
                                        .await;
                                    });
                                }
                            }
                        }
                        KeyCode::Char(c) => {
                            app.prompt_text.push(c);
                        }
                        _ => {}
                    }
                    continue;
                }

                // ── NORMAL MODE ──
                match key.code {
                    // Quit
                    KeyCode::Char('q') | KeyCode::Esc => {
                        // Save session before quitting
                        let store = state::StateStore::new();
                        let session = state::Session {
                            id: uuid::Uuid::new_v4().to_string(),
                            started_at: chrono::Utc::now(),
                            tasks: app.tasks.clone(),
                            logs: app.agent_logs.clone(),
                            diffs: app.current_diff.iter().cloned().collect(),
                            llm_usage: app.llm_usage.clone(),
                        };
                        let _ = store.save(&session);
                        app.running = false;
                        return Ok(());
                    }

                    // Enter editing mode (prompt screen only)
                    KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('i') => {
                        if app.screen == app::Screen::Prompt {
                            app.editing = true;
                        }
                    }

                    // Navigate screens
                    KeyCode::Tab => {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            app.prev_screen();
                        } else {
                            app.next_screen();
                        }
                    }
                    KeyCode::BackTab => {
                        app.prev_screen();
                    }

                    // Scroll / navigate (j/k vim-style)
                    KeyCode::Char('j') | KeyCode::Down => {
                        match app.screen {
                            app::Screen::Prompt => app.next_model(),
                            app::Screen::AgentView => {
                                app.agent_log_scroll = app.agent_log_scroll.saturating_add(1);
                            }
                            app::Screen::DiffView => {
                                app.diff_scroll = app.diff_scroll.saturating_add(1);
                            }
                            app::Screen::Output => {
                                app.scroll_offset = app.scroll_offset.saturating_add(1);
                            }
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        match app.screen {
                            app::Screen::Prompt => app.prev_model(),
                            app::Screen::AgentView => {
                                app.agent_log_scroll = app.agent_log_scroll.saturating_sub(1);
                            }
                            app::Screen::DiffView => {
                                app.diff_scroll = app.diff_scroll.saturating_sub(1);
                            }
                            app::Screen::Output => {
                                app.scroll_offset = app.scroll_offset.saturating_sub(1);
                            }
                        }
                    }

                    _ => {}
                }
            }
        }

        app.tick();
    }
}

// =====================================================================
//  Chat Mode — direct LLM streaming (no agent pipeline)
// =====================================================================

async fn run_chat(
    prompt: String,
    model: String,
    provider: Arc<dyn LLMProvider>,
    event_tx: mpsc::UnboundedSender<OrchestratorEvent>,
) {
    use models::AgentLog;

    let _ = event_tx.send(OrchestratorEvent::Log(
        AgentLog::info(models::AgentKind::RepoMap, "[CHAT] Direct LLM streaming started"),
    ));

    // Create a token channel for the LLM provider
    let (token_tx, mut token_rx) = mpsc::unbounded_channel::<String>();

    let llm_clone = provider.clone();
    let model_clone = model.clone();
    let prompt_clone = prompt.clone();

    // Spawn LLM generation in background
    let llm_handle = tokio::spawn(async move {
        llm_clone.generate(&prompt_clone, &model_clone, token_tx).await
    });

    // Forward tokens to the UI event channel
    while let Some(token) = token_rx.recv().await {
        if token == "[DONE]" {
            break;
        }
        let _ = event_tx.send(OrchestratorEvent::Token(token));
    }

    // Collect usage stats
    match llm_handle.await {
        Ok(Ok(usage)) => {
            let _ = event_tx.send(OrchestratorEvent::Log(
                AgentLog::info(
                    models::AgentKind::RepoMap,
                    format!("[CHAT] Complete ({}ms, {} tokens)", usage.latency_ms, usage.total_tokens),
                ),
            ));
            let _ = event_tx.send(OrchestratorEvent::Usage(usage));
        }
        Ok(Err(e)) => {
            let _ = event_tx.send(OrchestratorEvent::PipelineFailed(
                format!("LLM error: {}", e),
            ));
            return;
        }
        Err(e) => {
            let _ = event_tx.send(OrchestratorEvent::PipelineFailed(
                format!("Task error: {}", e),
            ));
            return;
        }
    }

    let _ = event_tx.send(OrchestratorEvent::PipelineComplete);
}

// =====================================================================
//  Fast Code Mode — direct to Coder, skip Architect (5-10x faster)
// =====================================================================

async fn run_fast_code(
    prompt: String,
    model: String,
    provider: Arc<dyn LLMProvider>,
    event_tx: mpsc::UnboundedSender<OrchestratorEvent>,
) {
    use models::AgentLog;

    let _ = event_tx.send(OrchestratorEvent::Log(
        AgentLog::info(models::AgentKind::Coder, "[FAST] Direct code generation (no Architect)"),
    ));

    // Build a direct code prompt (no Architect plan needed)
    let code_prompt = format!(
        "You are an expert developer. Write code for the following request.\n\n\
         ## Request\n{}\n\n\
         ## Rules\n\
         - Output ONLY code inside ```language fenced blocks\n\
         - Mark each file with: `// FILE: path/to/file.ext` at the top\n\
         - Clean, idiomatic code with error handling\n\
         - NO explanations outside code blocks\n\
         - Be concise",
        prompt
    );

    let (token_tx, mut token_rx) = mpsc::unbounded_channel::<String>();

    let llm_clone = provider.clone();
    let model_clone = model.clone();

    let llm_handle = tokio::spawn(async move {
        llm_clone.generate(&code_prompt, &model_clone, token_tx).await
    });

    // Stream tokens live to UI
    let mut full_response = String::new();
    while let Some(token) = token_rx.recv().await {
        if token == "[DONE]" {
            break;
        }
        full_response.push_str(&token);
        let _ = event_tx.send(OrchestratorEvent::Token(token));
    }

    // Generate diff from the response
    if !full_response.is_empty() {
        let diff_result = crate::diff::compute_diff("generated_code", "", &full_response);
        let _ = event_tx.send(OrchestratorEvent::Log(
            AgentLog::info(
                models::AgentKind::Coder,
                format!("[FAST] +{} lines generated", diff_result.additions),
            ),
        ));
        let _ = event_tx.send(OrchestratorEvent::DiffProduced(diff_result));
    }

    // Collect usage
    match llm_handle.await {
        Ok(Ok(usage)) => {
            let _ = event_tx.send(OrchestratorEvent::Log(
                AgentLog::info(
                    models::AgentKind::Coder,
                    format!("[FAST] Complete ({}ms, {} tokens)", usage.latency_ms, usage.total_tokens),
                ),
            ));
            let _ = event_tx.send(OrchestratorEvent::Usage(usage));
        }
        Ok(Err(e)) => {
            let _ = event_tx.send(OrchestratorEvent::PipelineFailed(
                format!("LLM error: {}", e),
            ));
            return;
        }
        Err(e) => {
            let _ = event_tx.send(OrchestratorEvent::PipelineFailed(
                format!("Task error: {}", e),
            ));
            return;
        }
    }

    let _ = event_tx.send(OrchestratorEvent::PipelineComplete);
}
