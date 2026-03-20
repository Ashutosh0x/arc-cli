//! # PR Review Toolkit — 6 Specialized Agents + Feature-Dev Workflow
//!
//! Agents: comment-analyzer, pr-test-analyzer, silent-failure-hunter,
//! type-design-analyzer, code-reviewer, code-simplifier.
//! Feature-Dev: 7-phase structured development workflow.

use serde::{Deserialize, Serialize};

// ── PR Review Agents ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewAgentKind {
    CommentAnalyzer,
    PrTestAnalyzer,
    SilentFailureHunter,
    TypeDesignAnalyzer,
    CodeReviewer,
    CodeSimplifier,
}

impl ReviewAgentKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::CommentAnalyzer => "comment-analyzer",
            Self::PrTestAnalyzer => "pr-test-analyzer",
            Self::SilentFailureHunter => "silent-failure-hunter",
            Self::TypeDesignAnalyzer => "type-design-analyzer",
            Self::CodeReviewer => "code-reviewer",
            Self::CodeSimplifier => "code-simplifier",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::CommentAnalyzer => "Analyzes PR comments for actionability, context, and sentiment",
            Self::PrTestAnalyzer => "Evaluates test coverage, identifies untested paths, suggests missing tests",
            Self::SilentFailureHunter => "Finds inadequate error handling: swallowed errors, empty catches, missing validations",
            Self::TypeDesignAnalyzer => "Reviews type definitions for correctness, consistency, and design patterns",
            Self::CodeReviewer => "General code review: style, performance, idioms, potential bugs",
            Self::CodeSimplifier => "Identifies overly complex code and suggests simplifications",
        }
    }

    pub fn all() -> &'static [ReviewAgentKind] {
        &[Self::CommentAnalyzer, Self::PrTestAnalyzer, Self::SilentFailureHunter,
          Self::TypeDesignAnalyzer, Self::CodeReviewer, Self::CodeSimplifier]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingSeverity { Critical, High, Medium, Low }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewFinding {
    pub agent: ReviewAgentKind,
    pub severity: FindingSeverity,
    pub file: String,
    pub line: Option<usize>,
    pub title: String,
    pub description: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReviewReport {
    pub findings: Vec<ReviewFinding>,
    pub summary: String,
    pub agents_run: Vec<String>,
}

impl ReviewReport {
    pub fn add_finding(&mut self, finding: ReviewFinding) { self.findings.push(finding); }

    pub fn critical_count(&self) -> usize { self.findings.iter().filter(|f| f.severity == FindingSeverity::Critical).count() }
    pub fn high_count(&self) -> usize { self.findings.iter().filter(|f| f.severity == FindingSeverity::High).count() }

    pub fn format_summary(&self) -> String {
        let c = self.critical_count();
        let h = self.high_count();
        let m = self.findings.iter().filter(|f| f.severity == FindingSeverity::Medium).count();
        let l = self.findings.iter().filter(|f| f.severity == FindingSeverity::Low).count();
        format!("PR Review: {} findings (🔴 {c} critical, 🟠 {h} high, 🟡 {m} medium, 🟢 {l} low)", self.findings.len())
    }
}

// ── Feature-Dev Workflow ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevPhase {
    Discovery,
    CodebaseExploration,
    ClarifyingQuestions,
    ArchitectureDesign,
    Implementation,
    QualityReview,
    Summary,
}

impl DevPhase {
    pub fn next(&self) -> Option<DevPhase> {
        match self {
            Self::Discovery => Some(Self::CodebaseExploration),
            Self::CodebaseExploration => Some(Self::ClarifyingQuestions),
            Self::ClarifyingQuestions => Some(Self::ArchitectureDesign),
            Self::ArchitectureDesign => Some(Self::Implementation),
            Self::Implementation => Some(Self::QualityReview),
            Self::QualityReview => Some(Self::Summary),
            Self::Summary => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Discovery => "1. Discovery",
            Self::CodebaseExploration => "2. Codebase Exploration",
            Self::ClarifyingQuestions => "3. Clarifying Questions",
            Self::ArchitectureDesign => "4. Architecture Design",
            Self::Implementation => "5. Implementation",
            Self::QualityReview => "6. Quality Review",
            Self::Summary => "7. Summary",
        }
    }

    pub fn all() -> &'static [DevPhase] {
        &[Self::Discovery, Self::CodebaseExploration, Self::ClarifyingQuestions,
          Self::ArchitectureDesign, Self::Implementation, Self::QualityReview, Self::Summary]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureDevWorkflow {
    pub feature_name: String,
    pub current_phase: DevPhase,
    pub phase_outputs: Vec<(DevPhase, String)>,
    pub started_at: u64,
}

impl FeatureDevWorkflow {
    pub fn new(feature_name: &str) -> Self {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        Self { feature_name: feature_name.to_string(), current_phase: DevPhase::Discovery, phase_outputs: Vec::new(), started_at: now }
    }

    pub fn complete_phase(&mut self, output: &str) -> Option<DevPhase> {
        self.phase_outputs.push((self.current_phase, output.to_string()));
        if let Some(next) = self.current_phase.next() {
            self.current_phase = next;
            Some(next)
        } else { None }
    }

    pub fn progress(&self) -> String {
        let completed = self.phase_outputs.len();
        let total = DevPhase::all().len();
        let bar: String = DevPhase::all().iter().map(|p| if self.phase_outputs.iter().any(|(pp, _)| pp == p) { "●" } else if *p == self.current_phase { "◐" } else { "○" }).collect::<Vec<_>>().join("");
        format!("[{bar}] {completed}/{total} — {}", self.current_phase.label())
    }
}
