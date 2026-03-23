// SPDX-License-Identifier: MIT
//! # Hook System — Event-Driven Extensibility Engine
//!
//! Implements the 10+ event lifecycle hooks inspired by Claude Code:
//! `PreToolUse`, `PostToolUse`, `Stop`, `SessionStart`, `SessionEnd`,
//! `StopFailure`, `PostCompact`, `InstructionsLoaded`, `ConfigChange`, `Elicitation`.
//!
//! Hooks can be Python scripts, Bash scripts, or HTTP webhooks.
//! Each hook receives JSON on stdin and returns JSON on stdout.
//! Exit code 0 = allow, exit code 2 = block.

#![allow(clippy::unwrap_used)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

// ── Event Types ──────────────────────────────────────────────────────────────

/// All lifecycle events the hook system supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    Stop,
    SessionStart,
    SessionEnd,
    StopFailure,
    PostCompact,
    InstructionsLoaded,
    ConfigChange,
    Elicitation,
    ElicitationResult,
    Prompt,
}

impl HookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PreToolUse => "pre_tool_use",
            Self::PostToolUse => "post_tool_use",
            Self::Stop => "stop",
            Self::SessionStart => "session_start",
            Self::SessionEnd => "session_end",
            Self::StopFailure => "stop_failure",
            Self::PostCompact => "post_compact",
            Self::InstructionsLoaded => "instructions_loaded",
            Self::ConfigChange => "config_change",
            Self::Elicitation => "elicitation",
            Self::ElicitationResult => "elicitation_result",
            Self::Prompt => "prompt",
        }
    }
}

// ── Hook Action ──────────────────────────────────────────────────────────────

/// Action a hook can take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookAction {
    /// Allow the operation to proceed (show message as warning).
    Warn,
    /// Block the operation from executing.
    Block,
}

impl Default for HookAction {
    fn default() -> Self {
        Self::Warn
    }
}

// ── Hook Executor Type ───────────────────────────────────────────────────────

/// How to execute a hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookExecutor {
    /// Run a shell command (bash/python/etc).
    Command {
        command: String,
        #[serde(default = "default_timeout")]
        timeout_ms: u64,
    },
    /// POST JSON to an HTTP endpoint and receive JSON response.
    Http {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        #[serde(default = "default_http_timeout")]
        timeout_ms: u64,
    },
}

fn default_timeout() -> u64 {
    10_000
}

fn default_http_timeout() -> u64 {
    5_000
}

// ── Hook Definition ──────────────────────────────────────────────────────────

/// A registered hook binding an event to an executor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDefinition {
    /// Unique name for this hook.
    pub name: String,
    /// Which event triggers this hook.
    pub event: HookEvent,
    /// How to execute the hook.
    pub executor: HookExecutor,
    /// Whether this hook is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Optional description.
    #[serde(default)]
    pub description: String,
    /// Source of this hook (settings, plugin, skill).
    #[serde(default)]
    pub source: HookSource,
}

fn default_true() -> bool {
    true
}

/// Where a hook was registered from.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookSource {
    #[default]
    Settings,
    Plugin,
    Skill,
    Managed,
}

// ── Hook Input / Output ──────────────────────────────────────────────────────

/// Input payload sent to hook executors via stdin (JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookInput {
    pub event: HookEvent,
    pub session_id: String,
    #[serde(default)]
    pub tool_name: String,
    #[serde(default)]
    pub tool_input: serde_json::Value,
    #[serde(default)]
    pub file_path: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub transcript_path: String,
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub agent_type: String,
    #[serde(default)]
    pub worktree: Option<WorktreeInfo>,
}

/// Worktree context passed to hooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeInfo {
    pub name: String,
    pub path: String,
    pub branch: String,
    pub original_dir: String,
}

/// Output returned by hook executors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookOutput {
    /// Whether to continue (true) or stop (false).
    #[serde(default = "default_true")]
    pub r#continue: bool,
    /// Optional message to display to user.
    #[serde(default)]
    pub message: String,
    /// Optional reason for stopping.
    #[serde(default)]
    pub stop_reason: String,
}

// ── Hookify Rule (Markdown + YAML) ──────────────────────────────────────────

/// A hookify-style rule defined in markdown with YAML frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookifyRule {
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Maps to HookEvent variant name: "bash", "file", "stop", "prompt", "all".
    pub event: String,
    /// Regex pattern for simple rules.
    #[serde(default)]
    pub pattern: String,
    /// Action: warn or block.
    #[serde(default)]
    pub action: HookAction,
    /// Advanced condition list.
    #[serde(default)]
    pub conditions: Vec<HookifyCondition>,
    /// The markdown body (displayed as message when triggered).
    #[serde(skip)]
    pub message: String,
}

/// A condition within a hookify rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookifyCondition {
    pub field: String,
    pub operator: ConditionOperator,
    pub pattern: String,
}

/// Operators for hookify conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    RegexMatch,
    Contains,
    Equals,
    NotContains,
    StartsWith,
    EndsWith,
}

// ── Hook Registry ────────────────────────────────────────────────────────────

/// Central registry managing all hooks.
pub struct HookRegistry {
    hooks: Vec<HookDefinition>,
    rules: Vec<HookifyRule>,
}

impl HookRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            hooks: Vec::new(),
            rules: Vec::new(),
        }
    }

    /// Register a hook definition.
    pub fn register(&mut self, hook: HookDefinition) {
        self.hooks.push(hook);
    }

    /// Register a hookify rule.
    pub fn register_rule(&mut self, rule: HookifyRule) {
        self.rules.push(rule);
    }

    /// Get all hooks for a given event.
    pub fn hooks_for_event(&self, event: HookEvent) -> Vec<&HookDefinition> {
        self.hooks
            .iter()
            .filter(|h| h.enabled && h.event == event)
            .collect()
    }

    /// Get all enabled hookify rules for a given event string.
    pub fn rules_for_event(&self, event_str: &str) -> Vec<&HookifyRule> {
        self.rules
            .iter()
            .filter(|r| r.enabled && (r.event == event_str || r.event == "all"))
            .collect()
    }

    /// Load hooks from a hooks.json file.
    pub fn load_from_file(&mut self, path: &Path) -> Result<usize, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read hooks file: {e}"))?;
        let hooks: Vec<HookDefinition> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse hooks file: {e}"))?;
        let count = hooks.len();
        for hook in hooks {
            self.register(hook);
        }
        Ok(count)
    }

    /// Discover hookify rule files in a directory (*.local.md pattern).
    pub fn discover_rules(&mut self, dir: &Path) -> Result<usize, String> {
        if !dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        let entries = std::fs::read_dir(dir).map_err(|e| format!("Failed to read dir: {e}"))?;

        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();

            if name.starts_with("hookify.") && name.ends_with(".local.md") {
                if let Ok(rule) = Self::parse_hookify_file(&path) {
                    self.register_rule(rule);
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    /// Parse a hookify markdown file with YAML frontmatter.
    fn parse_hookify_file(path: &Path) -> Result<HookifyRule, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read hookify file: {e}"))?;

        // Split frontmatter from body.
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return Err("Invalid hookify file: missing YAML frontmatter".into());
        }

        let yaml_str = parts[1].trim();
        let message = parts[2].trim().to_string();

        let mut rule: HookifyRule = serde_yaml::from_str(yaml_str)
            .map_err(|e| format!("Failed to parse YAML frontmatter: {e}"))?;
        rule.message = message;
        Ok(rule)
    }

    /// Check if a hookify rule matches the given input.
    pub fn check_rule(rule: &HookifyRule, input: &HookInput) -> bool {
        // Simple pattern check.
        if !rule.pattern.is_empty() {
            let text = match rule.event.as_str() {
                "bash" => &input.tool_input.to_string(),
                "file" => &input.file_path,
                "prompt" => &input.content,
                _ => &input.content,
            };

            if let Ok(re) = regex::Regex::new(&rule.pattern) {
                if !re.is_match(text) {
                    return false;
                }
            }
        }

        // Advanced conditions: ALL must match.
        for cond in &rule.conditions {
            let field_value = match cond.field.as_str() {
                "command" => input
                    .tool_input
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                "file_path" => input.file_path.clone(),
                "new_text" | "content" => input.content.clone(),
                "user_prompt" => input.content.clone(),
                _ => String::new(),
            };

            let matches = match cond.operator {
                ConditionOperator::RegexMatch => regex::Regex::new(&cond.pattern)
                    .map(|re| re.is_match(&field_value))
                    .unwrap_or(false),
                ConditionOperator::Contains => field_value.contains(&cond.pattern),
                ConditionOperator::Equals => field_value == cond.pattern,
                ConditionOperator::NotContains => !field_value.contains(&cond.pattern),
                ConditionOperator::StartsWith => field_value.starts_with(&cond.pattern),
                ConditionOperator::EndsWith => field_value.ends_with(&cond.pattern),
            };

            if !matches {
                return false;
            }
        }

        true
    }

    /// Execute all hooks and rules for an event, returning combined result.
    pub async fn fire_event(&self, input: &HookInput) -> HookResult {
        let mut result = HookResult::default();

        // Check hookify rules first (synchronous pattern matching).
        let event_str = input.event.as_str();
        for rule in self.rules_for_event(event_str) {
            if Self::check_rule(rule, input) {
                match rule.action {
                    HookAction::Block => {
                        result.blocked = true;
                        result.messages.push(rule.message.clone());
                        return result; // Block immediately.
                    },
                    HookAction::Warn => {
                        result.messages.push(rule.message.clone());
                    },
                }
            }
        }

        // Check registered hook definitions.
        for hook in self.hooks_for_event(input.event) {
            match &hook.executor {
                HookExecutor::Command {
                    command,
                    timeout_ms,
                } => match Self::execute_command(command, input, *timeout_ms).await {
                    Ok(output) => {
                        if !output.r#continue {
                            result.blocked = true;
                            if !output.message.is_empty() {
                                result.messages.push(output.message);
                            }
                            return result;
                        }
                        if !output.message.is_empty() {
                            result.messages.push(output.message);
                        }
                    },
                    Err(e) => {
                        result
                            .messages
                            .push(format!("Hook '{}' error: {e}", hook.name));
                    },
                },
                HookExecutor::Http {
                    url,
                    headers,
                    timeout_ms,
                } => match Self::execute_http(url, headers, input, *timeout_ms).await {
                    Ok(output) => {
                        if !output.r#continue {
                            result.blocked = true;
                            if !output.message.is_empty() {
                                result.messages.push(output.message);
                            }
                            return result;
                        }
                        if !output.message.is_empty() {
                            result.messages.push(output.message);
                        }
                    },
                    Err(e) => {
                        result
                            .messages
                            .push(format!("Hook '{}' HTTP error: {e}", hook.name));
                    },
                },
            }
        }

        result
    }

    /// Execute a command-based hook by piping JSON to stdin.
    async fn execute_command(
        command: &str,
        input: &HookInput,
        timeout_ms: u64,
    ) -> Result<HookOutput, String> {
        let _input_json = serde_json::to_string(input)
            .map_err(|e| format!("Failed to serialize hook input: {e}"))?;

        let result = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            tokio::process::Command::new(if cfg!(target_os = "windows") {
                "cmd"
            } else {
                "sh"
            })
            .arg(if cfg!(target_os = "windows") {
                "/C"
            } else {
                "-c"
            })
            .arg(command)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output(),
        )
        .await
        .map_err(|_| format!("Hook timed out after {timeout_ms}ms"))?
        .map_err(|e| format!("Hook execution failed: {e}"))?;

        match result.status.code() {
            Some(0) => {
                // Allow — parse stdout for optional message.
                let stdout = String::from_utf8_lossy(&result.stdout);
                if stdout.trim().is_empty() {
                    Ok(HookOutput {
                        r#continue: true,
                        message: String::new(),
                        stop_reason: String::new(),
                    })
                } else {
                    serde_json::from_str(stdout.trim()).unwrap_or(HookOutput {
                        r#continue: true,
                        message: stdout.trim().to_string(),
                        stop_reason: String::new(),
                    });
                    Ok(HookOutput {
                        r#continue: true,
                        message: stdout.trim().to_string(),
                        stop_reason: String::new(),
                    })
                }
            },
            Some(2) => {
                // Block.
                let stderr = String::from_utf8_lossy(&result.stderr);
                Ok(HookOutput {
                    r#continue: false,
                    message: stderr.trim().to_string(),
                    stop_reason: "blocked_by_hook".to_string(),
                })
            },
            Some(code) => Err(format!("Hook exited with unexpected code {code}")),
            None => Err("Hook was terminated by signal".to_string()),
        }
    }

    /// Execute an HTTP webhook hook.
    async fn execute_http(
        url: &str,
        headers: &HashMap<String, String>,
        input: &HookInput,
        timeout_ms: u64,
    ) -> Result<HookOutput, String> {
        let client = reqwest::Client::new();
        let mut req = client
            .post(url)
            .timeout(Duration::from_millis(timeout_ms))
            .json(input);

        for (key, value) in headers {
            req = req.header(key.as_str(), value.as_str());
        }

        let response = req
            .send()
            .await
            .map_err(|e| format!("HTTP hook failed: {e}"))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read hook response: {e}"))?;

        if status.is_success() {
            serde_json::from_str(&body).map_err(|e| format!("Invalid hook response JSON: {e}"))
        } else {
            Ok(HookOutput {
                r#continue: false,
                message: format!("HTTP hook returned {status}: {body}"),
                stop_reason: "http_error".to_string(),
            })
        }
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Aggregated result from firing hooks for an event.
#[derive(Debug, Clone, Default)]
pub struct HookResult {
    /// Whether the operation was blocked.
    pub blocked: bool,
    /// Messages from all hooks that fired.
    pub messages: Vec<String>,
}

impl HookResult {
    /// Whether the operation should proceed.
    pub fn should_proceed(&self) -> bool {
        !self.blocked
    }
}
