// SPDX-License-Identifier: MIT
//! # /security-review — Merge-Base Security Audit Command
//!
//! Audits the current branch diff against merge-base for security issues.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub severity: Severity,
    pub file: String,
    pub line: Option<usize>,
    pub rule: String,
    pub description: String,
    pub suggestion: String,
}

/// Security patterns to scan for in diffs.
pub const SECURITY_PATTERNS: &[(&str, &str, Severity)] = &[
    (
        r"child_process\.exec\s*\(",
        "Command injection via exec()",
        Severity::Critical,
    ),
    (
        r"os\.system\s*\(",
        "Command injection via os.system()",
        Severity::Critical,
    ),
    (
        r"eval\s*\(",
        "Code injection via eval()",
        Severity::Critical,
    ),
    (
        r"innerHTML\s*=",
        "Potential XSS via innerHTML",
        Severity::High,
    ),
    (
        r"document\.write\s*\(",
        "Potential XSS via document.write",
        Severity::High,
    ),
    (
        r"sql.*format!|format!.*(?i)(select|insert|update|delete)",
        "Potential SQL injection",
        Severity::High,
    ),
    (
        r#"(?i)(password|secret|token|api_key)\s*=\s*["'][^"']+["']"#,
        "Hardcoded credential",
        Severity::High,
    ),
    (
        r"(?i)allowlist|whitelist|blacklist",
        "Non-inclusive terminology",
        Severity::Low,
    ),
    (
        r"(?i)todo.*security|fixme.*security|hack.*security",
        "Security TODO/FIXME",
        Severity::Medium,
    ),
];

/// Get the merge-base diff for security scanning.
pub fn get_merge_base_diff(base: Option<&str>) -> Result<String, String> {
    let base_ref = base.unwrap_or("main");
    // Find merge-base.
    let mb = std::process::Command::new("git")
        .args(["merge-base", base_ref, "HEAD"])
        .output()
        .map_err(|e| e.to_string())?;
    if !mb.status.success() {
        return Err("Failed to find merge-base".into());
    }
    let merge_base = String::from_utf8_lossy(&mb.stdout).trim().to_string();

    // Get diff.
    let diff = std::process::Command::new("git")
        .args(["diff", &merge_base, "HEAD"])
        .output()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&diff.stdout).to_string())
}

/// Scan a diff for security issues using pattern matching.
pub fn scan_diff(diff: &str) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();
    let mut current_file = String::new();
    let mut line_num: usize = 0;

    for line in diff.lines() {
        if line.starts_with("+++ b/") {
            current_file = line[6..].to_string();
            line_num = 0;
        } else if line.starts_with("@@ ") {
            // Parse hunk header for line number.
            if let Some(plus) = line.find('+') {
                let rest = &line[plus + 1..];
                if let Some(comma) = rest.find(',') {
                    line_num = rest[..comma].parse().unwrap_or(0);
                } else if let Some(space) = rest.find(' ') {
                    line_num = rest[..space].parse().unwrap_or(0);
                }
            }
        } else if line.starts_with('+') && !line.starts_with("+++") {
            // Added line — scan against patterns.
            let added = &line[1..];
            for (pattern, desc, sev) in SECURITY_PATTERNS {
                if let Ok(re) = regex::Regex::new(pattern) {
                    if re.is_match(added) {
                        findings.push(SecurityFinding {
                            severity: *sev,
                            file: current_file.clone(),
                            line: Some(line_num),
                            rule: pattern.to_string(),
                            description: desc.to_string(),
                            suggestion: format!("Review this pattern in {current_file}:{line_num}"),
                        });
                    }
                }
            }
            line_num += 1;
        } else if !line.starts_with('-') {
            line_num += 1;
        }
    }
    findings
}

/// Format findings for terminal output.
pub fn format_findings(findings: &[SecurityFinding]) -> String {
    if findings.is_empty() {
        return "✅ No security issues found.".into();
    }
    let mut out = format!("⚠️  {} security finding(s):\n\n", findings.len());
    for f in findings {
        let sev = match f.severity {
            Severity::Critical => "🔴 CRITICAL",
            Severity::High => "🟠 HIGH",
            Severity::Medium => "🟡 MEDIUM",
            Severity::Low => "🟢 LOW",
            Severity::Info => "ℹ️  INFO",
        };
        out.push_str(&format!(
            "  {sev}: {}\n    File: {}:{}\n    Fix: {}\n\n",
            f.description,
            f.file,
            f.line.unwrap_or(0),
            f.suggestion
        ));
    }
    out
}
