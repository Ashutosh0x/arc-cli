use anyhow::Result;
use futures::StreamExt;
use reqwest::Client;
use std::io::{self, Write};

use arc_providers::anthropic::{self, AnthropicProvider};
use arc_providers::message::{Message, Role, StreamEvent, ToolCall};
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
                "/compact",
                "/fork",
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

/// Physical entrypoint connecting the Terminal to the active Agent model.
pub async fn run_repl(api_key: String) -> Result<()> {
    println!(">>> ARC Agent — Tool-Use Loop Active");
    println!(">>> Tools: file_read, file_edit, shell");
    println!(">>> Type /exit to quit.\n");

    let client = Client::builder().http2_prior_knowledge().build()?;
    let provider = AnthropicProvider::new(client.clone(), api_key.clone());

    // Initialize tool registry
    let tool_registry = ToolRegistry::new();
    let tool_definitions = tool_registry.definitions();

    let model = "claude-sonnet-4-20250514";

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
                println!("  Ctrl+C. Type /exit to quit.");
                continue;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                println!("  EOF. Exiting...");
                input_str = "/exit".to_string();
            }
            Err(e) => {
                eprintln!("  REPL Error: {:?}", e);
                break;
            }
        }

        let input = input_str.trim();

        if input == "/exit" {
            let _ = rl.save_history(&history_file);
            break;
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
                    println!("  {} Attached {}", style("+").green(), style(path_str).bold());
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

        // ── Agent Tool-Use Loop ──
        // Keep looping until the model produces a text response without tool calls.
        loop {
            // Step 1: Stream the response
            let mut stream = provider
                .stream(model, &session_messages, &tool_definitions)
                .await?;

            print!("|ARC|: ");
            io::stdout().flush()?;

            let mut full_response = String::new();
            let mut _got_tool_use_stop = false;

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
                                // This shouldn't happen in streaming mode with current parser
                                // Tool use is detected via stop_reason
                                _got_tool_use_stop = true;
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

            // Step 2: Check if the model wants to use tools
            // We need to make a non-streaming call to get complete tool_use blocks
            // because streaming only gives us partial JSON fragments for tool inputs.
            let complete_response = anthropic::fetch_complete_response(
                &client,
                &api_key,
                model,
                &session_messages,
                &tool_definitions,
            )
            .await;

            match complete_response {
                Ok(response_body) => {
                    let stop_reason = response_body
                        .get("stop_reason")
                        .and_then(|v| v.as_str())
                        .unwrap_or("end_turn");

                    let content_blocks = response_body
                        .get("content")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();

                    // Extract text and tool_use blocks
                    let mut text_content = String::new();
                    let mut tool_calls: Vec<ToolCall> = Vec::new();

                    for block in &content_blocks {
                        let block_type = block.get("type").and_then(|v| v.as_str());
                        match block_type {
                            Some("text") => {
                                if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                                    text_content.push_str(text);
                                }
                            }
                            Some("tool_use") => {
                                let id = block
                                    .get("id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let name = block
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let input = block
                                    .get("input")
                                    .cloned()
                                    .unwrap_or(serde_json::json!({}));

                                tool_calls.push(ToolCall {
                                    id,
                                    name,
                                    arguments: input,
                                });
                            }
                            _ => {}
                        }
                    }

                    if stop_reason == "tool_use" && !tool_calls.is_empty() {
                        // Print the text part if any (LLM often explains before tool calls)
                        if !text_content.is_empty() && full_response.is_empty() {
                            print!("|ARC|: {}", text_content);
                            println!();
                        }

                        // Record the assistant message with tool calls
                        session_messages.push(Message {
                            role: Role::Assistant,
                            content: text_content.clone(),
                            tool_calls: tool_calls.clone(),
                            tool_call_id: None,
                        });

                        // Step 3: Execute each tool call
                        for tc in &tool_calls {
                            println!(
                                "  [tool] {} ({})",
                                console::style(&tc.name).cyan().bold(),
                                &tc.id[..8.min(tc.id.len())]
                            );

                            let result = tool_registry
                                .execute(&tc.name, tc.arguments.clone())
                                .await;

                            let result_text = match result {
                                Ok(output) => output,
                                Err(e) => format!("Tool execution error: {}", e),
                            };

                            // Truncate very long outputs
                            let truncated = if result_text.len() > 50_000 {
                                format!(
                                    "{}...\n[Truncated: {} total bytes]",
                                    &result_text[..50_000],
                                    result_text.len()
                                )
                            } else {
                                result_text
                            };

                            // Add tool result to messages
                            session_messages.push(Message {
                                role: Role::Tool,
                                content: truncated,
                                tool_calls: vec![],
                                tool_call_id: Some(tc.id.clone()),
                            });
                        }

                        // Loop back — let the model process tool results
                        continue;
                    } else {
                        // No tool calls — record the text response and break
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
                }
                Err(e) => {
                    // If the non-streaming call fails, just record the streamed text
                    eprintln!("  [Warning: Could not verify tool use: {}]", e);
                    session_messages.push(Message {
                        role: Role::Assistant,
                        content: full_response,
                        tool_calls: vec![],
                        tool_call_id: None,
                    });
                    break;
                }
            }
        }

        // Desktop notification
        let _ = notify_rust::Notification::new()
            .summary("ARC Agent")
            .body("Task completed.")
            .show();
    }

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
