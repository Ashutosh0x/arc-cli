use serde::{Deserialize, Serialize};

/// What the Planner agent MUST return
#[derive(Debug, Serialize, Deserialize)]
pub struct PlanOutput {
    pub task_summary: String,
    pub steps: Vec<PlanStep>,
    pub estimated_files: Vec<String>,
    pub confidence: f32,            // 0.0 - 1.0
    pub requires_clarification: bool,
    pub clarification_questions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: usize,
    pub label: String,
    pub description: String,
    pub agent: AgentRole,
    pub depends_on: Vec<usize>,    // step IDs this depends on
    pub estimated_tokens: u64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentRole {
    Planner,
    Architect,
    Coder,
    Reviewer,
    Tester,
}

/// What the Architect agent MUST return
#[derive(Debug, Serialize, Deserialize)]
pub struct ArchitectOutput {
    pub file_specs: Vec<FileSpec>,
    pub new_dependencies: Vec<DependencySpec>,
    pub architectural_notes: String,
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileSpec {
    pub path: String,
    pub action: FileAction,
    pub purpose: String,
    pub changes_description: String,
    pub estimated_diff_size: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FileAction {
    Create,
    Modify,
    Delete,
    Rename { from: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DependencySpec {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
    pub justification: String,
}

/// What the Coder agent MUST return
#[derive(Debug, Serialize, Deserialize)]
pub struct CoderOutput {
    pub file_edits: Vec<FileEdit>,
    pub confidence: f32,
    pub test_suggestions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEdit {
    pub path: String,
    pub edit_type: EditType,
    pub content: String,          // full new file content or diff
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EditType {
    FullRewrite,
    UnifiedDiff,
    SearchReplace { search: String, replace: String },
}

// Validation
pub fn validate_plan(output: &PlanOutput) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if output.steps.is_empty() {
        errors.push("Plan must have at least one step".into());
    }

    if output.confidence < 0.0 || output.confidence > 1.0 {
        errors.push("Confidence must be between 0.0 and 1.0".into());
    }

    // Check dependency graph is a DAG (no cycles)
    for step in &output.steps {
        for dep in &step.depends_on {
            if *dep >= step.id {
                errors.push(format!(
                    "Step {} depends on step {} which hasn't executed yet",
                    step.id, dep
                ));
            }
        }
    }

    if output.requires_clarification && output.clarification_questions.is_empty() {
        errors.push("requires_clarification is true but no questions provided".into());
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

// Escalation
pub fn should_escalate(output: &impl HasConfidence) -> EscalationDecision {
    let c = output.confidence();
    if c >= 0.8 {
        EscalationDecision::AutoProceed
    } else if c >= 0.5 {
        EscalationDecision::AskUser {
            reason: format!("Agent confidence is {:.0}%", c * 100.0),
        }
    } else {
        EscalationDecision::Halt {
            reason: format!("Agent confidence too low ({:.0}%), aborting", c * 100.0),
        }
    }
}

pub enum EscalationDecision {
    AutoProceed,
    AskUser { reason: String },
    Halt { reason: String },
}

pub trait HasConfidence {
    fn confidence(&self) -> f32;
}

impl HasConfidence for PlanOutput {
    fn confidence(&self) -> f32 { self.confidence }
}
impl HasConfidence for ArchitectOutput {
    fn confidence(&self) -> f32 { self.confidence }
}
impl HasConfidence for CoderOutput {
    fn confidence(&self) -> f32 { self.confidence }
}
