use anyhow::Result;
use std::io::{self, Write};
use futures::StreamExt;
use reqwest::Client;

use arc_providers::message::{Message, Role};
use arc_providers::traits::Provider;
use arc_providers::anthropic::AnthropicProvider;

use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::{Validator, ValidationResult, ValidationContext};
use rustyline::{Helper, Context};

struct ArcHelper {
    file_completer: FilenameCompleter,
}

impl Completer for ArcHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        let start = line[..pos].rfind(|c| c == ' ' || c == '\t').map(|i| i + 1).unwrap_or(0);
        let word = &line[start..pos];
        
        if word.starts_with('/') {
            let cmds = vec!["/exit", "/add", "/undo", "/save", "/status"];
            let matches: Vec<Pair> = cmds.into_iter()
                .filter(|c| c.starts_with(word))
                .map(|c| Pair { display: c.to_string(), replacement: c.to_string() })
                .collect();
            return Ok((start, matches));
        } else if word.starts_with('@') {
            let path_query = &word[1..];
            if let Ok((_file_start, mut file_matches)) = self.file_completer.complete(path_query, pos - start - 1, ctx) {
                for m in &mut file_matches {
                    m.replacement = format!("@{}", m.replacement);
                }
                return Ok((start, file_matches));
            }
        }
        
        Ok((0, Vec::with_capacity(0)))
    }
}

impl Hinter for ArcHelper {
    type Hint = String;
    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> { None }
}
impl Highlighter for ArcHelper {}
impl Validator for ArcHelper {
    fn validate(&self, _ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        Ok(ValidationResult::Valid(None))
    }
    fn validate_while_typing(&self) -> bool { false }
}
impl Helper for ArcHelper {}

/// Physical entrypoint connecting the Terminal to the active Agent models.
pub async fn run_repl(api_key: String) -> Result<()> {
    println!(">>> ARC Agentic CLI - Autonomous Loop Booted Native.");
    println!(">>> Connected via HTTP/2 stream to Anthropic Opus.");
    println!(">>> Type /exit or /checkpoint to manage history.");

    let client = Client::builder()
        .http2_prior_knowledge()
        .build()?;
        
    let provider = AnthropicProvider::new(client, api_key);
    
    let mut system_prompt = "You are ARC, a terminal-native autonomous Rust-based agent.".to_string();
    
    // Load project-specific instructions from ARC.md or .arc.md
    for filename in ["ARC.md", ".arc.md", "arc.md"] {
        if let Ok(content) = std::fs::read_to_string(filename) {
            system_prompt.push_str(&format!("\n\n=== Project Instructions & Architecture ({}) ===\n{}", filename, content));
            println!(">>> Loaded project context from {}", filename);
            break;
        }
    }

    let mut session_messages = vec![
        Message {
            role: Role::System,
            content: system_prompt,
            tool_calls: vec![],
            tool_call_id: None,
        }
    ];

    let mut rl = rustyline::Editor::<ArcHelper, rustyline::history::DefaultHistory>::new()?;
    rl.set_helper(Some(ArcHelper {
        file_completer: FilenameCompleter::new(),
    }));
    
    let arc_home = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("arc");
    
    if !arc_home.exists() {
        std::fs::create_dir_all(&arc_home)?;
    }
    
    let history_file = arc_home.join("repl_history.txt");
    let _ = rl.load_history(&history_file);

    loop {
        let readline = rl.readline("arc> ");
        let input_str;
        
        match readline {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                input_str = line;
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                println!("  ⚠️ Keyboard Interrupt (Ctrl+C). Type /exit to quit.");
                continue;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                println!("  🚪 EOF received (Ctrl+D). Saving checkpoint and exiting...");
                input_str = "/exit".to_string();
            }
            Err(e) => {
                eprintln!("  ❌ REPL Error: {:?}", e);
                break;
            }
        }
        
        let input = input_str.trim();
        
        if input == "/exit" {
            let _ = rl.save_history(&history_file);
            break;
        }

        if input == "/add" {
            use dialoguer::{theme::ColorfulTheme, FuzzySelect};
            
            println!("  🔍 Scanning workspace for files...");
            let mut files = Vec::new();
            for entry in walkdir::WalkDir::new(".")
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let p = entry.path().display().to_string();
                if !p.contains(".git") && !p.contains("target") {
                    files.push(p);
                }
            }

            if files.is_empty() {
                println!("  ⚠️ No valid files found.");
                continue;
            }

            let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
                .with_prompt("Fuzzy select a file to add to context")
                .items(&files)
                .interact_opt()
                .unwrap_or(None);

            if let Some(idx) = selection {
                let path_str = &files[idx];
                if let Ok(content) = std::fs::read_to_string(path_str) {
                    use console::style;
                    println!("  {} Attached {}", style("📎").cyan(), style(path_str).bold());
                    session_messages.push(Message {
                        role: Role::User,
                        content: format!("\n\n```{}\n{}\n```", path_str, content),
                        tool_calls: vec![],
                        tool_call_id: None,
                    });
                } else {
                    println!("  ⚠️ Failed to read {}", path_str);
                }
            } else {
                println!("  ⚠️ Selection aborted.");
            }
            continue;
        }
        
        if input.is_empty() {
            continue;
        }

        let resolved_prompt = resolve_mentions(input);

        session_messages.push(Message {
            role: Role::User,
            content: resolved_prompt,
            tool_calls: vec![],
            tool_call_id: None,
        });

        let mut stream = provider
            .stream("claude-3-5-sonnet-20241022", &session_messages, &[])
            .await?;

        print!("|ARC|: ");
        io::stdout().flush()?;
        
        let mut full_response = String::new();

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
                        Some(Ok(event)) => {
                            print!("{}", event.text_delta);
                            io::stdout().flush().unwrap_or(());
                            full_response.push_str(&event.text_delta);
                        }
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
        println!(); // Terminate flush boundary cleanly 

        session_messages.push(Message {
            role: Role::Assistant,
            content: full_response,
            tool_calls: vec![],
            tool_call_id: None,
        });

        // ── Fire Desktop Notification ──
        let _ = notify_rust::Notification::new()
            .summary("ARC Agent")
            .body("Autonomous generation sequence completed.")
            .show();
    }
    
    // Try to save before exiting
    // Note: rustc will complain if rl was not available here, but since the binding was moved earlier this works.
    let _ = rl.save_history(&history_file);

    Ok(())
}

fn resolve_mentions(input: &str) -> String {
    let mut resolved_input = input.to_string();
    let words: Vec<&str> = input.split_whitespace().collect();
    
    for word in words {
        if word.starts_with('@') && word.len() > 1 {
            let path_str = &word[1..];
            let path = std::path::Path::new(path_str);
            if path.exists() && path.is_file() {
                if let Ok(content) = std::fs::read_to_string(path) {
                    use console::style;
                    println!("  {} Attached {}", style("📎").cyan(), style(path_str).bold());
                    resolved_input.push_str(&format!("\n\n```{}\n{}\n```", path_str, content));
                } else {
                    println!("  ⚠️ Failed to read {}", path_str);
                }
            } else if path.exists() && path.is_dir() {
                println!("  ⚠️ @-directories not deeply mounted yet: {}", path_str);
            }
        }
    }
    resolved_input
}

pub struct FileChange {
    pub path: String,
    pub old_content: String,
    pub new_content: String,
}

pub async fn handle_agent_output(changes: Vec<FileChange>) {
    use arc_ui::{TerminalUi, diff::compute_diff};
    let mut ui = TerminalUi::new().unwrap();

    let diffs: Vec<_> = changes
        .iter()
        .map(|ch| compute_diff(&ch.path, &ch.old_content, &ch.new_content))
        .collect();

    let result = ui.enter_review(diffs).unwrap();

    for path in &result.accepted {
        println!("Accepted changes to {}", path);
    }

    for path in &result.rejected {
        tracing::info!("Rejected changes to {}", path);
    }
}
