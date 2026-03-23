// SPDX-License-Identifier: MIT
use crate::rules::{PolicyRule, RuleSeverity};
use std::path::Path;

pub struct PolicyViolation {
    pub rule_name: String,
    pub severity: RuleSeverity,
    pub message: String,
}

pub struct PolicyResult {
    pub is_allowed: bool,
    pub violations: Vec<PolicyViolation>,
}

/// The PolicyEngine evaluates proposed agent actions against corporate
/// or user-defined constraints.
pub struct PolicyEngine {
    rules: Vec<PolicyRule>,
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn load_default_rules(&mut self) {
        self.rules.push(PolicyRule::NoForcePush);
        self.rules.push(PolicyRule::NoEnvFileRead);
        self.rules.push(PolicyRule::RequireTestsForCorePaths);
    }

    /// Add a custom rule (e.g., from an ARC.toml file).
    pub fn add_rule(&mut self, rule: PolicyRule) {
        self.rules.push(rule);
    }

    /// Evaluate a bash command proposed by the LLM.
    pub fn evaluate_command(&self, command: &str) -> PolicyResult {
        let mut violations = Vec::new();

        for rule in &self.rules {
            if let Some(violation) = rule.check_command(command) {
                violations.push(violation);
            }
        }

        let is_allowed = !violations.iter().any(|v| v.severity == RuleSeverity::Deny);

        PolicyResult {
            is_allowed,
            violations,
        }
    }

    /// Evaluate a file path the LLM wants to read or modify.
    pub fn evaluate_file_access(&self, file_path: &Path, is_write: bool) -> PolicyResult {
        let mut violations = Vec::new();

        for rule in &self.rules {
            if let Some(violation) = rule.check_file_access(file_path, is_write) {
                violations.push(violation);
            }
        }

        let is_allowed = !violations.iter().any(|v| v.severity == RuleSeverity::Deny);

        PolicyResult {
            is_allowed,
            violations,
        }
    }
}
