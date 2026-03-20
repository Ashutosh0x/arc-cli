use async_trait::async_trait;
use serde_json::json;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::fs;

static ACCEPT_ALL: AtomicBool = AtomicBool::new(false);
static DENY_ALL: AtomicBool = AtomicBool::new(false);

use crate::traits::Tool;

pub struct FileEditTool;

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &'static str {
        "file_edit"
    }

    fn description(&self) -> &'static str {
        "Edit a file by replacing a specific block of text. For Phase 1, it expects exact string matching search/replace."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to edit"
                },
                "search": {
                    "type": "string",
                    "description": "The exact multi-line string to find in the file"
                },
                "replace": {
                    "type": "string",
                    "description": "The exact replacement string"
                }
            },
            "required": ["path", "search", "replace"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, anyhow::Error> {
        let path_str = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path'"))?;
        let search = args
            .get("search")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'search'"))?;
        let replace = args
            .get("replace")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'replace'"))?;

        let path = Path::new(path_str);
        if !path.exists() {
            return Ok(format!("Error: File not found at '{path_str}'"));
        }

        let content = fs::read_to_string(path).await?;

        // AST-aware diffing and formatting would go here via tree-sitter.
        // For MVP Phase 1 (and robust edit reliability), we just run string replacement.
        if !content.contains(search) {
            return Ok("Error: Search block not found in file. Ensure exact whitespace and line-endings match.".into());
        }

        let new_content = content.replace(search, replace);

        // ── Diff Preview rendering via `similar` and `console` ──
        // ── Diff Preview rendering via `similar` and `syntect` ──
        let diff = similar::TextDiff::from_lines(&content, &new_content);
        println!(
            "\n  {}",
            console::style(format!("Diff Preview: {}", path_str))
                .bold()
                .underlined()
        );

        // Pre-highlight both buffers to guarantee chronological parser state
        fn highlight_buffer(text: &str, ext: Option<&str>) -> Vec<String> {
            let ps = syntect::parsing::SyntaxSet::load_defaults_newlines();
            let ts = syntect::highlighting::ThemeSet::load_defaults();
            let syntax = ext
                .and_then(|e| ps.find_syntax_by_extension(e))
                .unwrap_or_else(|| ps.find_syntax_plain_text());
            let mut h = syntect::easy::HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

            syntect::util::LinesWithEndings::from(text)
                .map(|line| {
                    let regions = h.highlight_line(line, &ps).unwrap_or_default();
                    syntect::util::as_24_bit_terminal_escaped(&regions[..], false) + "\x1b[0m"
                })
                .collect()
        }

        let ext = path.extension().and_then(|s| s.to_str());
        let hl_old = highlight_buffer(&content, ext);
        let hl_new = highlight_buffer(&new_content, ext);

        for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
            if idx > 0 {
                println!("    {}", console::style("@@ ... @@").dim());
            }
            for op in group {
                for change in diff.iter_changes(op) {
                    let (_sign, styled_sign, hl_line): (
                        &'static str,
                        console::StyledObject<&'static str>,
                        String,
                    ) = match change.tag() {
                        similar::ChangeTag::Delete => {
                            let line = hl_old
                                .get(change.old_index().unwrap())
                                .cloned()
                                .unwrap_or_default();
                            ("-", console::style("-").red(), line)
                        },
                        similar::ChangeTag::Insert => {
                            let line = hl_new
                                .get(change.new_index().unwrap())
                                .cloned()
                                .unwrap_or_default();
                            ("+", console::style("+").green(), line)
                        },
                        similar::ChangeTag::Equal => {
                            let line = hl_new
                                .get(change.new_index().unwrap())
                                .cloned()
                                .unwrap_or_default();
                            (" ", console::style(" ").dim(), line)
                        },
                    };

                    print!("{} {}", styled_sign, hl_line);
                    if !hl_line.ends_with('\n') && !hl_line.contains("\n\x1b[0m") {
                        println!();
                    }
                }
            }
        }
        println!();

        // ── Auth-Gate the autonomous File Write via `console::Term::read_key()` ──
        if std::env::var("ARC_HEADLESS").is_ok() || ACCEPT_ALL.load(Ordering::Relaxed) {
            fs::write(path, new_content).await?;
            return Ok(format!("Successfully edited file '{path_str}'"));
        }
        if DENY_ALL.load(Ordering::Relaxed) {
            return Ok(format!(
                "Auto-rejected edit for '{path_str}' due to Deny All active."
            ));
        }

        let term = console::Term::stdout();
        term.write_line(&format!(
            "Accept autonomous edits to {}? [Y/n/a/d/e/s/?]",
            path_str
        ))
        .unwrap_or(());

        let confirm = loop {
            if let Ok(key) = term.read_key() {
                match key {
                    console::Key::Enter | console::Key::Char('y') | console::Key::Char('Y') => {
                        term.write_line("  ✅ Accepted.").unwrap_or(());
                        break true;
                    },
                    console::Key::Escape | console::Key::Char('n') | console::Key::Char('N') => {
                        term.write_line("  ❌ Rejected.").unwrap_or(());
                        break false;
                    },
                    console::Key::Char('a') | console::Key::Char('A') => {
                        term.write_line("  🚀 Accept All enabled.").unwrap_or(());
                        ACCEPT_ALL.store(true, Ordering::Relaxed);
                        break true;
                    },
                    console::Key::Char('d') | console::Key::Char('D') => {
                        term.write_line("  🛑 Deny All enabled.").unwrap_or(());
                        DENY_ALL.store(true, Ordering::Relaxed);
                        break false;
                    },
                    console::Key::Char('s') | console::Key::Char('S') => {
                        term.write_line("  ⏭️ Skipped.").unwrap_or(());
                        return Ok(format!("User skipped editing '{path_str}'."));
                    },
                    console::Key::Char('e') | console::Key::Char('E') => {
                        term.write_line("  📝 Opening in $EDITOR (Not fully implemented, accepting by default for now)...").unwrap_or(());
                        break true;
                    },
                    console::Key::Char('j') | console::Key::Char('k') => {
                        term.write_line("  (Use terminal scrollback for diff viewport)")
                            .unwrap_or(());
                    },
                    console::Key::Char('?') => {
                        term.write_line("Help:\n  Enter/y: Accept\n  Esc/n: Reject\n  a: Accept All (this session)\n  d: Deny All (this session)\n  s: Skip file\n  e: Edit manually\n  j/k: Scroll diff").unwrap_or(());
                    },
                    _ => {},
                }
            }
        };

        if !confirm {
            return Ok(format!(
                "User definitively REJECTED the edit for '{}'. You MUST revise your approach.",
                path_str
            ));
        }

        fs::write(path, new_content).await?;
        Ok(format!("Successfully edited file '{path_str}'"))
    }
}
