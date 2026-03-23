// SPDX-License-Identifier: MIT
//! Conseca Dynamic Safety Checker
//!
//! LLM-generated security policy engine:
//! 1. Takes the user's prompt + available tool declarations
//! 2. Generates a SecurityPolicy (what tools/args are safe)
//! 3. Enforces that policy on every subsequent tool call

use serde::{Deserialize, Serialize};

// ── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub allowed_tools: Vec<ToolPermission>,
    pub denied_patterns: Vec<String>,
    pub max_file_write_count: Option<u32>,
    pub restricted_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermission {
    pub tool_name: String,
    pub allowed: bool,
    pub allowed_args: Option<Vec<ArgConstraint>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgConstraint {
    pub arg_name: String,
    pub must_match: Option<String>,
    pub must_not_contain: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetyDecision {
    Allow,
    Deny,
    AskUser,
}

#[derive(Debug, Clone)]
pub struct SafetyCheckResult {
    pub decision: SafetyDecision,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub tool_name: String,
    pub args: std::collections::HashMap<String, String>,
}

// ── Conseca Checker ─────────────────────────────────────────────────────────

pub struct ConsecaSafetyChecker {
    current_policy: Option<SecurityPolicy>,
    active_user_prompt: Option<String>,
    enabled: bool,
}

impl ConsecaSafetyChecker {
    pub fn new(enabled: bool) -> Self {
        Self {
            current_policy: None,
            active_user_prompt: None,
            enabled,
        }
    }

    /// Set the security policy for the current user prompt.
    pub fn set_policy(&mut self, prompt: String, policy: SecurityPolicy) {
        self.active_user_prompt = Some(prompt);
        self.current_policy = Some(policy);
    }

    /// Check a tool call against the current policy.
    pub fn check(&self, tool_call: &ToolCall) -> SafetyCheckResult {
        if !self.enabled {
            return SafetyCheckResult {
                decision: SafetyDecision::Allow,
                reason: "Conseca is disabled".to_string(),
            };
        }

        let Some(policy) = &self.current_policy else {
            return SafetyCheckResult {
                decision: SafetyDecision::Allow,
                reason: "No security policy generated yet".to_string(),
            };
        };

        // Check denied patterns first
        for pattern in &policy.denied_patterns {
            let pattern_lower = pattern.to_lowercase();
            for (_key, value) in &tool_call.args {
                if value.to_lowercase().contains(&pattern_lower) {
                    return SafetyCheckResult {
                        decision: SafetyDecision::Deny,
                        reason: format!("Argument contains denied pattern: {pattern}"),
                    };
                }
            }
        }

        // Check restricted paths
        for restricted in &policy.restricted_paths {
            for (_key, value) in &tool_call.args {
                if value.contains(restricted) {
                    return SafetyCheckResult {
                        decision: SafetyDecision::Deny,
                        reason: format!("Access to restricted path: {restricted}"),
                    };
                }
            }
        }

        // Check tool permissions
        for perm in &policy.allowed_tools {
            if perm.tool_name == tool_call.tool_name {
                if !perm.allowed {
                    return SafetyCheckResult {
                        decision: SafetyDecision::Deny,
                        reason: format!("Tool '{}' is not allowed by policy", tool_call.tool_name),
                    };
                }
                // Check arg constraints
                if let Some(constraints) = &perm.allowed_args {
                    for constraint in constraints {
                        if let Some(value) = tool_call.args.get(&constraint.arg_name) {
                            if let Some(must_match) = &constraint.must_match {
                                if !value.contains(must_match) {
                                    return SafetyCheckResult {
                                        decision: SafetyDecision::Deny,
                                        reason: format!(
                                            "Arg '{}' does not match required pattern",
                                            constraint.arg_name
                                        ),
                                    };
                                }
                            }
                            if let Some(blocked) = &constraint.must_not_contain {
                                for b in blocked {
                                    if value.contains(b) {
                                        return SafetyCheckResult {
                                            decision: SafetyDecision::Deny,
                                            reason: format!(
                                                "Arg '{}' contains blocked content: {b}",
                                                constraint.arg_name
                                            ),
                                        };
                                    }
                                }
                            }
                        }
                    }
                }
                return SafetyCheckResult {
                    decision: SafetyDecision::Allow,
                    reason: "Tool allowed by policy".to_string(),
                };
            }
        }

        // Unknown tool → ask user
        SafetyCheckResult {
            decision: SafetyDecision::AskUser,
            reason: format!("Tool '{}' not covered by policy", tool_call.tool_name),
        }
    }

    pub fn current_policy(&self) -> Option<&SecurityPolicy> {
        self.current_policy.as_ref()
    }

    pub fn clear(&mut self) {
        self.current_policy = None;
        self.active_user_prompt = None;
    }
}

/// System prompt to generate a SecurityPolicy from user intent.
pub const CONSECA_POLICY_GENERATION_PROMPT: &str = r#"Given the user's request and the available tools, generate a security policy that constrains what the AI assistant may do.

Output a JSON SecurityPolicy with:
- allowed_tools: list of {tool_name, allowed: bool, allowed_args: optional constraints}
- denied_patterns: list of strings that must not appear in any argument (e.g., "rm -rf /", "sudo")
- restricted_paths: list of paths the agent must not access (e.g., "/etc/passwd", "~/.ssh")
- max_file_write_count: optional max number of files the agent may create/modify"#;
