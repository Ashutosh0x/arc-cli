// SPDX-License-Identifier: MIT
//! # Permission System — 3-Tier Tool Access Control
//!
//! Implements `allow/ask/deny` permission rules per tool, with compound bash
//! command parsing, enterprise managed policies, and per-subcommand prefix matching.
//! Inspired by Claude Code's permission system.

use serde::{Deserialize, Serialize};

// ── Permission Level ─────────────────────────────────────────────────────────

/// Permission level for a tool or command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionLevel {
    /// Always allowed without prompting.
    Allow,
    /// Requires user confirmation each time.
    Ask,
    /// Always denied, cannot be overridden by user settings.
    Deny,
}

// ── Permission Rule ──────────────────────────────────────────────────────────

/// A rule matching a specific tool or command pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    /// Tool name or pattern (e.g., "Bash", "Write", "Bash(cmd:git *)").
    pub pattern: String,
    /// Permission level.
    pub level: PermissionLevel,
    /// Source of this rule.
    #[serde(default)]
    pub source: RuleSource,
    /// Optional description of why this rule exists.
    #[serde(default)]
    pub description: String,
}

/// Where a permission rule originated from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleSource {
    /// User-level settings.
    #[default]
    User,
    /// Project-level settings.
    Project,
    /// Enterprise managed settings (highest priority).
    Managed,
    /// Session-specific (temporary).
    Session,
}

// ── Permission Config ────────────────────────────────────────────────────────

/// Full permissions configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionsConfig {
    /// Tools always allowed.
    #[serde(default)]
    pub allow: Vec<String>,
    /// Tools requiring confirmation.
    #[serde(default)]
    pub ask: Vec<String>,
    /// Tools always denied.
    #[serde(default)]
    pub deny: Vec<String>,
    /// Default mode for unmatched tools.
    #[serde(default)]
    pub default_mode: Option<String>,
    /// Whether bypass mode is disabled.
    #[serde(default)]
    pub disable_bypass_permissions_mode: Option<String>,
}

// ── Permission Decision ──────────────────────────────────────────────────────

/// Result of a permission check.
#[derive(Debug, Clone)]
pub struct PermissionDecision {
    /// The resolved permission level.
    pub level: PermissionLevel,
    /// Which rule matched.
    pub matched_rule: Option<PermissionRule>,
    /// Suggestion for "always allow" prefix.
    pub allow_prefix: Option<String>,
}

// ── Bash Command Parser ──────────────────────────────────────────────────────

/// Parse compound bash commands into individual subcommands.
pub fn parse_bash_subcommands(command: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;
    let mut chars = command.chars().peekable();

    while let Some(c) = chars.next() {
        if escape_next {
            current.push(c);
            escape_next = false;
            continue;
        }

        match c {
            '\\' if !in_single_quote => {
                escape_next = true;
                current.push(c);
            },
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                current.push(c);
            },
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                current.push(c);
            },
            '&' if !in_single_quote && !in_double_quote => {
                if chars.peek() == Some(&'&') {
                    chars.next();
                    let trimmed = current.trim().to_string();
                    if !trimmed.is_empty() {
                        commands.push(trimmed);
                    }
                    current.clear();
                } else {
                    current.push(c);
                }
            },
            '|' if !in_single_quote && !in_double_quote => {
                if chars.peek() == Some(&'|') {
                    chars.next();
                    let trimmed = current.trim().to_string();
                    if !trimmed.is_empty() {
                        commands.push(trimmed);
                    }
                    current.clear();
                } else {
                    // Pipe: treat both sides as separate commands.
                    let trimmed = current.trim().to_string();
                    if !trimmed.is_empty() {
                        commands.push(trimmed);
                    }
                    current.clear();
                }
            },
            ';' if !in_single_quote && !in_double_quote => {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    commands.push(trimmed);
                }
                current.clear();
            },
            _ => {
                current.push(c);
            },
        }
    }

    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        commands.push(trimmed);
    }

    commands
}

/// Compute a smart "always allow" prefix for a bash command.
pub fn compute_allow_prefix(command: &str) -> String {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return command.to_string();
    }

    // Skip env var prefixes (e.g., "FOO=bar command").
    let cmd_start = parts.iter().position(|p| !p.contains('=')).unwrap_or(0);

    if cmd_start < parts.len() {
        let base_cmd = parts[cmd_start];
        // For well-known commands, include subcommand.
        let known_with_sub = ["git", "npm", "cargo", "docker", "kubectl", "gh", "pip"];
        if known_with_sub.contains(&base_cmd) && cmd_start + 1 < parts.len() {
            format!("{} {}*", base_cmd, parts[cmd_start + 1])
        } else {
            format!("{base_cmd}*")
        }
    } else {
        format!("{}*", parts[0])
    }
}

// ── Permission Manager ───────────────────────────────────────────────────────

/// Central permission manager.
pub struct PermissionManager {
    rules: Vec<PermissionRule>,
    /// Whether bypass mode is active (skip all prompts).
    bypass_mode: bool,
    /// Whether bypass mode can be activated.
    bypass_allowed: bool,
    /// Auto-approved commands for this session.
    session_allows: Vec<String>,
    /// Read-only safe commands (auto-allow list).
    safe_commands: Vec<&'static str>,
}

impl PermissionManager {
    /// Create a new permission manager.
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            bypass_mode: false,
            bypass_allowed: true,
            session_allows: Vec::new(),
            safe_commands: vec![
                "ls",
                "cat",
                "head",
                "tail",
                "wc",
                "grep",
                "find",
                "which",
                "whoami",
                "pwd",
                "echo",
                "date",
                "env",
                "uname",
                "id",
                "hostname",
                "df",
                "du",
                "file",
                "stat",
                "readlink",
                "basename",
                "dirname",
                "sort",
                "uniq",
                "cut",
                "tr",
                "sed",
                "awk",
                "diff",
                "tee",
                "xargs",
                "test",
                "git status",
                "git log",
                "git diff",
                "git branch",
                "git remote",
                "git show",
                "git rev-parse",
                "git ls-files",
                "git describe",
                "cargo check",
                "cargo build",
                "cargo test",
                "cargo clippy",
                "npm test",
                "npm run",
                "npm list",
                "python --version",
                "node --version",
                "rustc --version",
                "lsof",
                "pgrep",
                "tput",
                "ss",
                "fd",
                "fdfind",
                "rg",
                "fmt",
                "comm",
                "cmp",
                "numfmt",
                "expr",
                "printf",
                "seq",
                "tsort",
            ],
        }
    }

    /// Load permissions from a config object.
    pub fn load_config(&mut self, config: &PermissionsConfig, source: RuleSource) {
        for tool in &config.allow {
            self.rules.push(PermissionRule {
                pattern: tool.clone(),
                level: PermissionLevel::Allow,
                source: source.clone(),
                description: String::new(),
            });
        }
        for tool in &config.ask {
            self.rules.push(PermissionRule {
                pattern: tool.clone(),
                level: PermissionLevel::Ask,
                source: source.clone(),
                description: String::new(),
            });
        }
        for tool in &config.deny {
            self.rules.push(PermissionRule {
                pattern: tool.clone(),
                level: PermissionLevel::Deny,
                source: source.clone(),
                description: String::new(),
            });
        }

        if let Some(ref mode) = config.disable_bypass_permissions_mode {
            if mode == "disable" {
                self.bypass_allowed = false;
            }
        }
    }

    /// Check permission for a tool invocation.
    pub fn check(&self, tool_name: &str, tool_input: &serde_json::Value) -> PermissionDecision {
        // Bypass mode skips all checks.
        if self.bypass_mode {
            return PermissionDecision {
                level: PermissionLevel::Allow,
                matched_rule: None,
                allow_prefix: None,
            };
        }

        // Build the full tool descriptor.
        let descriptor = self.build_descriptor(tool_name, tool_input);

        // Check rules in priority order: Managed > Project > User > Session.
        let priority_order = [
            RuleSource::Managed,
            RuleSource::Project,
            RuleSource::User,
            RuleSource::Session,
        ];

        for source in &priority_order {
            for rule in &self.rules {
                if rule.source == *source
                    && self.matches_pattern(&rule.pattern, &descriptor, tool_name)
                {
                    // Managed deny cannot be overridden.
                    if rule.source == RuleSource::Managed && rule.level == PermissionLevel::Deny {
                        return PermissionDecision {
                            level: PermissionLevel::Deny,
                            matched_rule: Some(rule.clone()),
                            allow_prefix: None,
                        };
                    }

                    return PermissionDecision {
                        level: rule.level,
                        matched_rule: Some(rule.clone()),
                        allow_prefix: if tool_name == "Bash" {
                            tool_input
                                .get("command")
                                .and_then(|v| v.as_str())
                                .map(|cmd| compute_allow_prefix(cmd))
                        } else {
                            None
                        },
                    };
                }
            }
        }

        // Check session allows.
        for pattern in &self.session_allows {
            if self.matches_pattern(pattern, &descriptor, tool_name) {
                return PermissionDecision {
                    level: PermissionLevel::Allow,
                    matched_rule: None,
                    allow_prefix: None,
                };
            }
        }

        // Check safe commands for Bash tool.
        if tool_name == "Bash" {
            if let Some(cmd) = tool_input.get("command").and_then(|v| v.as_str()) {
                let sub_cmds = parse_bash_subcommands(cmd);
                let all_safe = sub_cmds
                    .iter()
                    .all(|sub| self.safe_commands.iter().any(|safe| sub.starts_with(safe)));
                if all_safe {
                    return PermissionDecision {
                        level: PermissionLevel::Allow,
                        matched_rule: None,
                        allow_prefix: None,
                    };
                }
            }
        }

        // Default: ask.
        PermissionDecision {
            level: PermissionLevel::Ask,
            matched_rule: None,
            allow_prefix: if tool_name == "Bash" {
                tool_input
                    .get("command")
                    .and_then(|v| v.as_str())
                    .map(|cmd| compute_allow_prefix(cmd))
            } else {
                None
            },
        }
    }

    /// Add a session-level always-allow pattern.
    pub fn add_session_allow(&mut self, pattern: String) {
        self.session_allows.push(pattern);
    }

    /// Enable bypass mode (skip all permission checks).
    pub fn enable_bypass(&mut self) -> bool {
        if self.bypass_allowed {
            self.bypass_mode = true;
            true
        } else {
            false
        }
    }

    /// Build a full descriptor string like "Bash(cmd:git push origin main)".
    fn build_descriptor(&self, tool_name: &str, tool_input: &serde_json::Value) -> String {
        if tool_name == "Bash" {
            if let Some(cmd) = tool_input.get("command").and_then(|v| v.as_str()) {
                return format!("Bash(cmd:{cmd})");
            }
        }
        if tool_name == "Write" || tool_name == "Edit" || tool_name == "MultiEdit" {
            if let Some(path) = tool_input.get("file_path").and_then(|v| v.as_str()) {
                return format!("{tool_name}(path:{path})");
            }
        }
        tool_name.to_string()
    }

    /// Check if a pattern matches a descriptor.
    fn matches_pattern(&self, pattern: &str, descriptor: &str, tool_name: &str) -> bool {
        // Exact tool name match.
        if pattern == tool_name {
            return true;
        }

        // Wildcard matching.
        if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len() - 1];
            if descriptor.starts_with(prefix) || tool_name.starts_with(prefix) {
                return true;
            }
            // Check compound: "Bash(cmd:git *)" vs descriptor.
            if let (Some(pat_start), Some(desc_start)) =
                (pattern.find("(cmd:"), descriptor.find("(cmd:"))
            {
                let pat_cmd = &pattern[pat_start + 5..pattern.len() - 2]; // before *)
                let desc_cmd_end = descriptor.find(')').unwrap_or(descriptor.len());
                let desc_cmd = &descriptor[desc_start + 5..desc_cmd_end];
                return desc_cmd.starts_with(pat_cmd);
            }
        }

        // Full descriptor match.
        pattern == descriptor
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bash_subcommands() {
        assert_eq!(
            parse_bash_subcommands("cd /tmp && git fetch && git push"),
            vec!["cd /tmp", "git fetch", "git push"]
        );

        assert_eq!(
            parse_bash_subcommands("echo 'hello && world'"),
            vec!["echo 'hello && world'"]
        );

        assert_eq!(
            parse_bash_subcommands("cat file | grep pattern"),
            vec!["cat file", "grep pattern"]
        );
    }

    #[test]
    fn test_compute_allow_prefix() {
        assert_eq!(compute_allow_prefix("git push origin main"), "git push*");
        assert_eq!(compute_allow_prefix("npm test"), "npm test*");
        assert_eq!(compute_allow_prefix("ls -la"), "ls*");
        assert_eq!(compute_allow_prefix("FOO=bar cargo build"), "cargo build*");
    }
}
