//! Tool Output Masking Service — Hybrid Backward-Scanned FIFO
//!
//! Manages context window efficiency by masking bulky tool outputs:
//! 1. **Protection Window**: Protects the newest 50k tool tokens from pruning
//! 2. **Global Aggregation**: Scans backwards past the protection window
//! 3. **Batch Trigger**: Only masks if prunable tokens exceed 30k threshold

use std::collections::HashSet;
use std::path::{Path, PathBuf};

// ── Defaults ────────────────────────────────────────────────────────────────
pub const DEFAULT_TOOL_PROTECTION_THRESHOLD: usize = 50_000;
pub const DEFAULT_MIN_PRUNABLE_TOKENS_THRESHOLD: usize = 30_000;
pub const DEFAULT_PROTECT_LATEST_TURN: bool = true;
pub const MASKING_INDICATOR_TAG: &str = "tool_output_masked";
pub const TOOL_OUTPUTS_DIR: &str = "tool-outputs";

/// Tools whose outputs should never be masked.
fn exempt_tools() -> HashSet<&'static str> {
    let mut s = HashSet::new();
    s.insert("ask_user");
    s.insert("memory");
    s.insert("enter_plan_mode");
    s.insert("exit_plan_mode");
    s.insert("activate_skill");
    s
}

// ── Config ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MaskingConfig {
    pub enabled: bool,
    pub tool_protection_threshold: usize,
    pub min_prunable_tokens_threshold: usize,
    pub protect_latest_turn: bool,
}

impl Default for MaskingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            tool_protection_threshold: DEFAULT_TOOL_PROTECTION_THRESHOLD,
            min_prunable_tokens_threshold: DEFAULT_MIN_PRUNABLE_TOKENS_THRESHOLD,
            protect_latest_turn: DEFAULT_PROTECT_LATEST_TURN,
        }
    }
}

// ── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub tool_name: String,
    pub call_id: String,
    pub content: String,
    pub token_estimate: usize,
}

#[derive(Debug, Clone)]
pub struct MaskedOutput {
    pub tool_name: String,
    pub call_id: String,
    pub masked_snippet: String,
    pub offload_path: PathBuf,
    pub tokens_saved: usize,
}

#[derive(Debug)]
pub struct MaskingResult {
    pub masked_count: usize,
    pub tokens_saved: usize,
    pub masked_outputs: Vec<MaskedOutput>,
}

// ── Service ─────────────────────────────────────────────────────────────────

pub struct ToolOutputMaskingService {
    config: MaskingConfig,
    exempt: HashSet<&'static str>,
}

impl ToolOutputMaskingService {
    pub fn new(config: MaskingConfig) -> Self {
        Self {
            config,
            exempt: exempt_tools(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(MaskingConfig::default())
    }

    /// Estimate token count for a string (~4 chars per token).
    fn estimate_tokens(s: &str) -> usize {
        s.len() / 4
    }

    /// Determine which tool outputs to mask from a history of outputs.
    /// Returns the indices and masking details.
    pub fn mask(
        &self,
        history: &[ToolOutput],
        output_dir: &Path,
    ) -> Result<MaskingResult, std::io::Error> {
        if !self.config.enabled || history.is_empty() {
            return Ok(MaskingResult {
                masked_count: 0,
                tokens_saved: 0,
                masked_outputs: Vec::new(),
            });
        }

        let mut cumulative_tool_tokens: usize = 0;
        let mut protection_boundary_reached = false;
        let mut total_prunable_tokens: usize = 0;

        struct PrunableItem {
            idx: usize,
            tokens: usize,
            tool_name: String,
            call_id: String,
            content: String,
        }
        let mut prunable: Vec<PrunableItem> = Vec::new();

        // Determine scan start (skip latest turn if configured)
        let scan_start = if self.config.protect_latest_turn && history.len() > 1 {
            history.len() - 2
        } else {
            history.len().saturating_sub(1)
        };

        // Backward scan
        for i in (0..=scan_start).rev() {
            let output = &history[i];

            // Skip exempt tools
            if self.exempt.contains(output.tool_name.as_str()) {
                continue;
            }

            // Skip already-masked
            if output
                .content
                .contains(&format!("<{MASKING_INDICATOR_TAG}"))
            {
                continue;
            }

            let tokens = Self::estimate_tokens(&output.content);

            if !protection_boundary_reached {
                cumulative_tool_tokens += tokens;
                if cumulative_tool_tokens > self.config.tool_protection_threshold {
                    protection_boundary_reached = true;
                    total_prunable_tokens += tokens;
                    prunable.push(PrunableItem {
                        idx: i,
                        tokens,
                        tool_name: output.tool_name.clone(),
                        call_id: output.call_id.clone(),
                        content: output.content.clone(),
                    });
                }
            } else {
                total_prunable_tokens += tokens;
                prunable.push(PrunableItem {
                    idx: i,
                    tokens,
                    tool_name: output.tool_name.clone(),
                    call_id: output.call_id.clone(),
                    content: output.content.clone(),
                });
            }
        }

        // Batch threshold check
        if total_prunable_tokens < self.config.min_prunable_tokens_threshold {
            return Ok(MaskingResult {
                masked_count: 0,
                tokens_saved: 0,
                masked_outputs: Vec::new(),
            });
        }

        // Perform masking
        std::fs::create_dir_all(output_dir)?;
        let mut masked_outputs = Vec::new();
        let mut total_saved: usize = 0;

        for item in &prunable {
            let safe_name = item.tool_name.replace(|c: char| !c.is_alphanumeric(), "_");
            let safe_id = item.call_id.replace(|c: char| !c.is_alphanumeric(), "_");
            let file_name = format!("{}_{}.txt", safe_name, safe_id);
            let file_path = output_dir.join(&file_name);

            std::fs::write(&file_path, &item.content)?;

            let preview = Self::generate_preview(&item.content, &item.tool_name);
            let total_lines = item.content.lines().count();
            let file_size_kb = item.content.len() / 1024;

            let masked_snippet = format!(
                "<{tag}>\n{preview}\n\nOutput too large ({total_lines} lines, {file_size_kb}KB). \
                 Full output saved to: {path}\n</{tag}>",
                tag = MASKING_INDICATOR_TAG,
                path = file_path.display()
            );

            let masked_tokens = Self::estimate_tokens(&masked_snippet);
            let savings = item.tokens.saturating_sub(masked_tokens);

            if savings > 0 {
                total_saved += savings;
                masked_outputs.push(MaskedOutput {
                    tool_name: item.tool_name.clone(),
                    call_id: item.call_id.clone(),
                    masked_snippet,
                    offload_path: file_path,
                    tokens_saved: savings,
                });
            }
        }

        Ok(MaskingResult {
            masked_count: masked_outputs.len(),
            tokens_saved: total_saved,
            masked_outputs,
        })
    }

    fn generate_preview(content: &str, tool_name: &str) -> String {
        if tool_name == "shell" {
            // Shell: show head + tail with exit code preserved
            Self::head_tail_preview(content, 10)
        } else if content.len() > 500 {
            // General: head + tail (250 chars each)
            format!(
                "{}\n... [TRUNCATED] ...\n{}",
                &content[..250],
                &content[content.len() - 250..]
            )
        } else {
            content.to_string()
        }
    }

    fn head_tail_preview(content: &str, n: usize) -> String {
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() <= n * 2 {
            return content.to_string();
        }
        let head = lines[..n].join("\n");
        let tail = lines[lines.len() - n..].join("\n");
        format!(
            "{head}\n\n... [{} lines omitted] ...\n\n{tail}",
            lines.len() - 2 * n
        )
    }
}
