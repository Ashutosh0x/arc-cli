// SPDX-License-Identifier: MIT
use anyhow::Result;
use futures::StreamExt;
use reqwest::Client;
use std::io::{self, Write};
use std::sync::Arc;

use arc_providers::anthropic::{self, AnthropicProvider};
use arc_providers::message::{Message, Role, StreamEvent, ToolCall};
use arc_providers::openai_compat::{self as oai_compat, OpenAICompatProvider};
use arc_providers::traits::Provider;
use arc_tools::registry::ToolRegistry;

use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{Context, Helper};

struct ArcHelper {
    file_completer: FilenameCompleter,
}

impl Completer for ArcHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let start = line[..pos]
            .rfind(|c| c == ' ' || c == '\t')
            .map(|i| i + 1)
            .unwrap_or(0);
        let word = &line[start..pos];

        if word.starts_with('/') {
            let cmds = vec![
                "/exit",
                "/add",
                "/undo",
                "/save",
                "/status",
                "/plan",
                "/checkpoint",
                "/rewind",
                "/compact",
                "/fork",
                "/provider",
                "/model",
            ];
            let matches: Vec<Pair> = cmds
                .into_iter()
                .filter(|c| c.starts_with(word))
                .map(|c| Pair {
                    display: c.to_string(),
                    replacement: c.to_string(),
                })
                .collect();
            return Ok((start, matches));
        } else if word.starts_with('@') {
            let path_query = &word[1..];
            if let Ok((_file_start, mut file_matches)) =
                self.file_completer
                    .complete(path_query, pos - start - 1, ctx)
            {
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
    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
        None
    }
}
impl Highlighter for ArcHelper {}
impl Validator for ArcHelper {
    fn validate(&self, _ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        Ok(ValidationResult::Valid(None))
    }
    fn validate_while_typing(&self) -> bool {
        false
    }
}
impl Helper for ArcHelper {}

/// Supported provider types.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ProviderKind {
    Anthropic,
    Groq,
    Xai,
    OpenAI,
}

struct ActiveProvider {
    kind: ProviderKind,
    provider: Arc<dyn Provider>,
    model: String,
    base_url: String,
    api_key: String,
}

/// Detect available provider from environment variables.
fn detect_provider(client: &Client) -> Option<ActiveProvider> {
    // Priority: ANTHROPIC > GROQ > XAI > OPENAI
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        if !key.is_empty() {
            return Some(ActiveProvider {
                kind: ProviderKind::Anthropic,
                provider: Arc::new(AnthropicProvider::new(client.clone(), key.clone())),
                model: "claude-sonnet-4-20250514".to_string(),
                base_url: "https://api.anthropic.com/v1".to_string(),
                api_key: key,
            });
        }
    }
    if let Ok(key) = std::env::var("GROQ_API_KEY") {
        if !key.is_empty() {
            return Some(ActiveProvider {
                kind: ProviderKind::Groq,
                provider: Arc::new(OpenAICompatProvider::groq(client.clone(), key.clone())),
                model: "llama-3.3-70b-versatile".to_string(),
                base_url: "https://api.groq.com/openai/v1".to_string(),
                api_key: key,
            });
        }
    }
    if let Ok(key) = std::env::var("XAI_API_KEY") {
        if !key.is_empty() {
            return Some(ActiveProvider {
                kind: ProviderKind::Xai,
                provider: Arc::new(OpenAICompatProvider::xai(client.clone(), key.clone())),
                model: "grok-4-1-fast-non-reasoning".to_string(),
                base_url: "https://api.x.ai/v1".to_string(),
                api_key: key,
            });
        }
    }
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        if !key.is_empty() {
            return Some(ActiveProvider {
                kind: ProviderKind::OpenAI,
                provider: Arc::new(OpenAICompatProvider::openai(client.clone(), key.clone())),
                model: "gpt-4o".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: key,
            });
        }
    }
    None
}

/// Physical entrypoint connecting the Terminal to the active Agent model.
pub async fn run_repl(api_key: String) -> Result<()> {
    let client = Client::builder().http2_prior_knowledge().build()?;

    // Auto-detect provider or fallback to Anthropic with provided key
    let mut active = detect_provider(&client).unwrap_or_else(|| ActiveProvider {
        kind: ProviderKind::Anthropic,
        provider: Arc::new(AnthropicProvider::new(client.clone(), api_key.clone())),
        model: "claude-sonnet-4-20250514".to_string(),
        base_url: "https://api.anthropic.com/v1".to_string(),
        api_key: api_key.clone(),
    });

    println!(">>> ARC Agent — Tool-Use Loop Active");
    println!(
        ">>> Provider: {} | Model: {}",
        active.provider.name(),
        active.model
    );
    println!(">>> Tools: file_read, file_edit, shell");
    println!(">>> Type /exit to quit, /provider to switch.\n");

    // Initialize tool registry
    let tool_registry = ToolRegistry::new();
    let tool_definitions = tool_registry.definitions();

    // Session persistence
    let arc_home = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("arc");
    if !arc_home.exists() {
        std::fs::create_dir_all(&arc_home)?;
    }
    let checkpoint_dir = arc_home.join("checkpoints");
    if !checkpoint_dir.exists() {
        std::fs::create_dir_all(&checkpoint_dir)?;
    }
    let session_id = uuid::Uuid::new_v4().to_string();

    let mut system_prompt =
        "You are ARC, a terminal-native autonomous coding agent written in Rust. You have access to tools: file_read (read files), file_edit (search/replace edits), and shell (run commands). Use them to help the user with their coding tasks. Always read files before editing them. Be concise in your responses.".to_string();

    // Load project-specific instructions from ARC.md
    for filename in ["ARC.md", ".arc.md", "arc.md"] {
        if let Ok(content) = std::fs::read_to_string(filename) {
            system_prompt.push_str(&format!(
                "\n\n=== Project Instructions ({}) ===\n{}",
                filename, content
            ));
            println!(">>> Loaded project context from {}", filename);
            break;
        }
    }

    let mut session_messages = vec![Message {
        role: Role::System,
        content: system_prompt,
        tool_calls: vec![],
        tool_call_id: None,
    }];

    // Check for existing session to resume
    if let Ok(entries) = std::fs::read_dir(&checkpoint_dir) {
        let mut checkpoints: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        checkpoints
            .sort_by_key(|e| std::cmp::Reverse(e.metadata().ok().and_then(|m| m.modified().ok())));
        if let Some(latest) = checkpoints.first() {
            println!(
                ">>> Found previous session: {}",
                latest.file_name().to_string_lossy()
            );
            println!(">>> Type /rewind to restore, or continue for new session.\n");
        }
    }

    let mut rl = rustyline::Editor::<ArcHelper, rustyline::history::DefaultHistory>::new()?;
    rl.set_helper(Some(ArcHelper {
        file_completer: FilenameCompleter::new(),
    }));

    let history_file = arc_home.join("repl_history.txt");
    let _ = rl.load_history(&history_file);

    let mut checkpoint_count = 0u32;

    loop {
        let readline = rl.readline("arc> ");
        let input_str;

        match readline {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                input_str = line;
            },
            Err(rustyline::error::ReadlineError::Interrupted) => {
                println!("  Ctrl+C. Type /exit to quit.");
                continue;
            },
            Err(rustyline::error::ReadlineError::Eof) => {
                println!("  EOF. Exiting...");
                input_str = "/exit".to_string();
            },
            Err(e) => {
                eprintln!("  REPL Error: {:?}", e);
                break;
            },
        }

        let input = input_str.trim();

        if input == "/exit" {
            // Auto-checkpoint on exit
            save_checkpoint(
                &checkpoint_dir,
                &session_id,
                &session_messages,
                checkpoint_count,
            );
            let _ = rl.save_history(&history_file);
            break;
        }

        if input == "/status" {
            println!(
                "  Provider: {} | Model: {} | Messages: {} | Checkpoints: {}",
                active.provider.name(),
                active.model,
                session_messages.len(),
                checkpoint_count
            );
            continue;
        }

        if input == "/checkpoint" {
            checkpoint_count += 1;
            save_checkpoint(
                &checkpoint_dir,
                &session_id,
                &session_messages,
                checkpoint_count,
            );
            println!(
                "  Checkpoint #{} saved ({} messages)",
                checkpoint_count,
                session_messages.len()
            );
            continue;
        }

        if input == "/rewind" || input.starts_with("/rewind ") {
            let target = input.strip_prefix("/rewind").unwrap_or("").trim();
            match load_checkpoint(&checkpoint_dir, &session_id, target) {
                Some(restored) => {
                    let count = restored.len();
                    session_messages = restored;
                    println!("  Rewound to {} messages", count);
                },
                None => {
                    println!("  No checkpoint found to rewind to.");
                    // Try listing available checkpoints
                    if let Ok(entries) = std::fs::read_dir(&checkpoint_dir) {
                        let files: Vec<_> = entries
                            .filter_map(|e| e.ok())
                            .map(|e| e.file_name().to_string_lossy().to_string())
                            .filter(|name| name.ends_with(".json"))
                            .collect();
                        if !files.is_empty() {
                            println!("  Available checkpoints: {:?}", files);
                        }
                    }
                },
            }
            continue;
        }

        if input == "/provider" || input.starts_with("/provider ") {
            let provider_name = input.strip_prefix("/provider").unwrap_or("").trim();
            if provider_name.is_empty() {
                println!("  Current: {} ({})", active.provider.name(), active.model);
                println!("  Available: anthropic, groq, xai, openai");
                println!("  Usage: /provider groq");
                continue;
            }
            match switch_provider(&client, provider_name) {
                Some(new_active) => {
                    println!(
                        "  Switched to {} ({})",
                        new_active.provider.name(),
                        new_active.model
                    );
                    active = new_active;
                },
                None => {
                    println!("  Failed: set the API key env var first (e.g. GROQ_API_KEY)");
                },
            }
            continue;
        }

        if input == "/model" || input.starts_with("/model ") {
            let model_name = input.strip_prefix("/model").unwrap_or("").trim();
            if model_name.is_empty() {
                println!("  Current model: {}", active.model);
                println!("  Available: {:?}", active.provider.models());
                continue;
            }
            active.model = model_name.to_string();
            println!("  Model switched to: {}", active.model);
            continue;
        }

        if input == "/compact" {
            let before = session_messages.len();
            compact_messages(&mut session_messages);
            println!(
                "  Compacted: {} -> {} messages",
                before,
                session_messages.len()
            );
            continue;
        }

        if input == "/add" {
            use dialoguer::{FuzzySelect, theme::ColorfulTheme};

            println!("  Scanning workspace for files...");
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
                println!("  No valid files found.");
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
                    println!(
                        "  {} Attached {}",
                        style("+").green(),
                        style(path_str).bold()
                    );
                    session_messages.push(Message {
                        role: Role::User,
                        content: format!("\n\n```{}\n{}\n```", path_str, content),
                        tool_calls: vec![],
                        tool_call_id: None,
                    });
                } else {
                    println!("  Failed to read {}", path_str);
                }
            } else {
                println!("  Selection aborted.");
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

        // Agent Tool-Use Loop
        loop {
            // Stream the response
            let stream_result = active
                .provider
                .stream(&active.model, &session_messages, &tool_definitions)
                .await;

            let mut stream = match stream_result {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("  [Provider Error]: {}", e);
                    break;
                },
            };

            print!("|ARC|: ");
            io::stdout().flush()?;

            let mut full_response = String::new();

            let cancel_token = tokio_util::sync::CancellationToken::new();
            let token_clone = cancel_token.clone();

            let ctrl_c_task = tokio::spawn(async move {
                let _ = tokio::signal::ctrl_c().await;
                token_clone.cancel();
            });

            loop {
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        println!("\n  [Interrupted]");
                        break;
                    }
                    chunk = stream.next() => {
                        match chunk {
                            Some(Ok(StreamEvent::TextDelta(text))) => {
                                print!("{}", text);
                                io::stdout().flush().unwrap_or(());
                                full_response.push_str(&text);
                            }
                            Some(Ok(StreamEvent::Done)) => {
                                break;
                            }
                            Some(Ok(StreamEvent::ToolUse { .. })) => {
                                break;
                            }
                            Some(Err(e)) => {
                                eprintln!("\n[Stream Error]: {}", e);
                                break;
                            }
                            None => break,
                        }
                    }
                }
            }

            ctrl_c_task.abort();
            println!();

            // Check for tool use — fetch complete response
            let complete_response = match active.kind {
                ProviderKind::Anthropic => {
                    anthropic::fetch_complete_response(
                        &client,
                        &active.api_key,
                        &active.model,
                        &session_messages,
                        &tool_definitions,
                    )
                    .await
                },
                _ => {
                    oai_compat::fetch_complete_response(
                        &client,
                        &active.api_key,
                        &active.base_url,
                        &active.model,
                        &session_messages,
                        &tool_definitions,
                    )
                    .await
                },
            };

            match complete_response {
                Ok(response_body) => {
                    let (stop_reason, tool_calls, text_content) = match active.kind {
                        ProviderKind::Anthropic => parse_anthropic_response(&response_body),
                        _ => parse_openai_response(&response_body),
                    };

                    if (stop_reason == "tool_use" || stop_reason == "tool_calls")
                        && !tool_calls.is_empty()
                    {
                        if !text_content.is_empty() && full_response.is_empty() {
                            print!("|ARC|: {}", text_content);
                            println!();
                        }

                        session_messages.push(Message {
                            role: Role::Assistant,
                            content: text_content.clone(),
                            tool_calls: tool_calls.clone(),
                            tool_call_id: None,
                        });

                        for tc in &tool_calls {
                            println!(
                                "  [tool] {} ({})",
                                console::style(&tc.name).cyan().bold(),
                                &tc.id[..8.min(tc.id.len())]
                            );

                            let result =
                                tool_registry.execute(&tc.name, tc.arguments.clone()).await;

                            let result_text = match result {
                                Ok(output) => output,
                                Err(e) => format!("Tool execution error: {}", e),
                            };

                            let truncated = if result_text.len() > 50_000 {
                                format!(
                                    "{}...\n[Truncated: {} total bytes]",
                                    &result_text[..50_000],
                                    result_text.len()
                                )
                            } else {
                                result_text
                            };

                            session_messages.push(Message {
                                role: Role::Tool,
                                content: truncated,
                                tool_calls: vec![],
                                tool_call_id: Some(tc.id.clone()),
                            });
                        }

                        continue;
                    } else {
                        if full_response.is_empty() {
                            full_response = text_content;
                        }
                        session_messages.push(Message {
                            role: Role::Assistant,
                            content: full_response,
                            tool_calls: vec![],
                            tool_call_id: None,
                        });
                        break;
                    }
                },
                Err(e) => {
                    eprintln!("  [Warning: Could not verify tool use: {}]", e);
                    session_messages.push(Message {
                        role: Role::Assistant,
                        content: full_response,
                        tool_calls: vec![],
                        tool_call_id: None,
                    });
                    break;
                },
            }
        }

        // Auto-checkpoint every 5 turns
        let user_turn_count = session_messages
            .iter()
            .filter(|m| m.role == Role::User)
            .count();
        if user_turn_count % 5 == 0 && user_turn_count > 0 {
            checkpoint_count += 1;
            save_checkpoint(
                &checkpoint_dir,
                &session_id,
                &session_messages,
                checkpoint_count,
            );
        }

        let _ = notify_rust::Notification::new()
            .summary("ARC Agent")
            .body("Task completed.")
            .show();
    }

    let _ = rl.save_history(&history_file);
    Ok(())
}

fn parse_anthropic_response(body: &serde_json::Value) -> (String, Vec<ToolCall>, String) {
    let stop_reason = body
        .get("stop_reason")
        .and_then(|v| v.as_str())
        .unwrap_or("end_turn")
        .to_string();

    let content_blocks = body
        .get("content")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut text_content = String::new();
    let mut tool_calls = Vec::new();

    for block in &content_blocks {
        match block.get("type").and_then(|v| v.as_str()) {
            Some("text") => {
                if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                    text_content.push_str(text);
                }
            },
            Some("tool_use") => {
                tool_calls.push(ToolCall {
                    id: block
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    name: block
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    arguments: block.get("input").cloned().unwrap_or(serde_json::json!({})),
                });
            },
            _ => {},
        }
    }

    (stop_reason, tool_calls, text_content)
}

fn parse_openai_response(body: &serde_json::Value) -> (String, Vec<ToolCall>, String) {
    let choice = body
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first());

    let finish_reason = choice
        .and_then(|c| c.get("finish_reason"))
        .and_then(|v| v.as_str())
        .unwrap_or("stop")
        .to_string();

    let message = choice.and_then(|c| c.get("message"));

    let text_content = message
        .and_then(|m| m.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let tool_calls: Vec<ToolCall> = message
        .and_then(|m| m.get("tool_calls"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|tc| {
                    let id = tc.get("id").and_then(|v| v.as_str())?.to_string();
                    let function = tc.get("function")?;
                    let name = function.get("name").and_then(|v| v.as_str())?.to_string();
                    let args_str = function
                        .get("arguments")
                        .and_then(|v| v.as_str())
                        .unwrap_or("{}");
                    let arguments: serde_json::Value =
                        serde_json::from_str(args_str).unwrap_or(serde_json::json!({}));
                    Some(ToolCall {
                        id,
                        name,
                        arguments,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // OpenAI uses "tool_calls" as finish_reason
    let stop_reason = if finish_reason == "tool_calls" {
        "tool_calls".to_string()
    } else {
        finish_reason
    };

    (stop_reason, tool_calls, text_content)
}

fn switch_provider(client: &Client, name: &str) -> Option<ActiveProvider> {
    match name {
        "anthropic" => {
            let key = std::env::var("ANTHROPIC_API_KEY").ok()?;
            Some(ActiveProvider {
                kind: ProviderKind::Anthropic,
                provider: Arc::new(AnthropicProvider::new(client.clone(), key.clone())),
                model: "claude-sonnet-4-20250514".to_string(),
                base_url: "https://api.anthropic.com/v1".to_string(),
                api_key: key,
            })
        },
        "groq" => {
            let key = std::env::var("GROQ_API_KEY").ok()?;
            Some(ActiveProvider {
                kind: ProviderKind::Groq,
                provider: Arc::new(OpenAICompatProvider::groq(client.clone(), key.clone())),
                model: "llama-3.3-70b-versatile".to_string(),
                base_url: "https://api.groq.com/openai/v1".to_string(),
                api_key: key,
            })
        },
        "xai" | "grok" => {
            let key = std::env::var("XAI_API_KEY").ok()?;
            Some(ActiveProvider {
                kind: ProviderKind::Xai,
                provider: Arc::new(OpenAICompatProvider::xai(client.clone(), key.clone())),
                model: "grok-4-1-fast-non-reasoning".to_string(),
                base_url: "https://api.x.ai/v1".to_string(),
                api_key: key,
            })
        },
        "openai" => {
            let key = std::env::var("OPENAI_API_KEY").ok()?;
            Some(ActiveProvider {
                kind: ProviderKind::OpenAI,
                provider: Arc::new(OpenAICompatProvider::openai(client.clone(), key.clone())),
                model: "gpt-4o".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: key,
            })
        },
        _ => None,
    }
}

fn save_checkpoint(dir: &std::path::Path, session_id: &str, messages: &[Message], count: u32) {
    let filename = dir.join(format!("{}_{}.json", session_id, count));
    if let Ok(json) = serde_json::to_string_pretty(messages) {
        let _ = std::fs::write(filename, json);
    }
}

fn load_checkpoint(dir: &std::path::Path, session_id: &str, target: &str) -> Option<Vec<Message>> {
    // If target is a number, load that specific checkpoint
    if let Ok(num) = target.parse::<u32>() {
        let filename = dir.join(format!("{}_{}.json", session_id, num));
        if filename.exists() {
            let data = std::fs::read_to_string(filename).ok()?;
            return serde_json::from_str(&data).ok();
        }
    }

    // Otherwise load the latest checkpoint for this session
    let mut latest: Option<(u32, std::path::PathBuf)> = None;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(session_id) && name.ends_with(".json") {
                if let Some(num_str) = name
                    .strip_prefix(&format!("{}_", session_id))
                    .and_then(|s| s.strip_suffix(".json"))
                {
                    if let Ok(num) = num_str.parse::<u32>() {
                        if latest.as_ref().map_or(true, |(prev, _)| num > *prev) {
                            latest = Some((num, entry.path()));
                        }
                    }
                }
            }
        }
    }

    // Try loading any .json checkpoint
    if latest.is_none() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut all: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            all.sort_by_key(|e| {
                std::cmp::Reverse(e.metadata().ok().and_then(|m| m.modified().ok()))
            });
            if let Some(entry) = all.first() {
                if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                    let data = std::fs::read_to_string(entry.path()).ok()?;
                    return serde_json::from_str(&data).ok();
                }
            }
        }
    }

    if let Some((_, path)) = latest {
        let data = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    } else {
        None
    }
}

/// Simple context compaction: keep system + last N messages.
fn compact_messages(messages: &mut Vec<Message>) {
    let keep_last = 20; // Keep last 20 messages + system
    if messages.len() <= keep_last + 1 {
        return;
    }

    // Extract system message
    let system = messages.iter().find(|m| m.role == Role::System).cloned();

    // Summarize dropped messages
    let dropped_count = messages.len() - keep_last - 1;

    let mut new_messages = Vec::new();
    if let Some(sys) = system {
        new_messages.push(sys);
    }

    new_messages.push(Message {
        role: Role::User,
        content: format!(
            "[Context compacted: {} earlier messages were removed to save tokens]",
            dropped_count
        ),
        tool_calls: vec![],
        tool_call_id: None,
    });

    // Keep last N messages
    let start = messages.len() - keep_last;
    new_messages.extend(messages[start..].to_vec());

    *messages = new_messages;
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
                    println!(
                        "  {} Attached {}",
                        style("+").green(),
                        style(path_str).bold()
                    );
                    resolved_input.push_str(&format!("\n\n```{}\n{}\n```", path_str, content));
                } else {
                    println!("  Failed to read {}", path_str);
                }
            } else if path.exists() && path.is_dir() {
                println!("  @-directories not yet supported: {}", path_str);
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
    let Ok(mut ui) = TerminalUi::new() else {
        eprintln!("Failed to initialize terminal UI");
        return;
    };

    let diffs: Vec<_> = changes
        .iter()
        .map(|ch| compute_diff(&ch.path, &ch.old_content, &ch.new_content))
        .collect();

    let Ok(result) = ui.enter_review(diffs) else {
        eprintln!("Failed to enter diff review");
        return;
    };

    for path in &result.accepted {
        println!("Accepted changes to {}", path);
    }

    for path in &result.rejected {
        tracing::info!("Rejected changes to {}", path);
    }
}
