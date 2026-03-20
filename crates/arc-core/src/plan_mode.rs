//! # Plan Mode — Full /plan Command with Accept/Reject/Feedback
//!
//! `/plan [description]` enters read-only analysis mode.
//! Plan persistence across compaction.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Draft,
    PendingReview,
    Accepted,
    Rejected,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub title: String,
    pub description: String,
    pub steps: Vec<PlanStep>,
    pub status: PlanStatus,
    pub feedback: Vec<PlanFeedback>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: usize,
    pub description: String,
    pub status: StepStatus,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanFeedback {
    pub message: String,
    pub timestamp: u64,
}

impl Plan {
    pub fn new(title: &str, description: &str) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id: format!("plan-{}", &uuid_simple()),
            title: title.to_string(),
            description: description.to_string(),
            steps: Vec::new(),
            status: PlanStatus::Draft,
            feedback: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_step(&mut self, desc: &str) {
        self.steps.push(PlanStep {
            id: self.steps.len(),
            description: desc.to_string(),
            status: StepStatus::Pending,
            files: Vec::new(),
        });
    }
    pub fn accept(&mut self) {
        self.status = PlanStatus::Accepted;
        self.touch();
    }
    pub fn reject(&mut self, reason: &str) {
        self.status = PlanStatus::Rejected;
        self.add_feedback(reason);
    }
    pub fn add_feedback(&mut self, msg: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.feedback.push(PlanFeedback {
            message: msg.to_string(),
            timestamp: now,
        });
        self.touch();
    }
    fn touch(&mut self) {
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
}

pub struct PlanManager {
    active_plan: Option<Plan>,
    history: Vec<Plan>,
}

impl PlanManager {
    pub fn new() -> Self {
        Self {
            active_plan: None,
            history: Vec::new(),
        }
    }
    pub fn create_plan(&mut self, title: &str, desc: &str) -> &mut Plan {
        self.active_plan = Some(Plan::new(title, desc));
        self.active_plan.as_mut().expect("just created")
    }
    pub fn active(&self) -> Option<&Plan> {
        self.active_plan.as_ref()
    }
    pub fn active_mut(&mut self) -> Option<&mut Plan> {
        self.active_plan.as_mut()
    }
    pub fn finalize(&mut self) {
        if let Some(plan) = self.active_plan.take() {
            self.history.push(plan);
        }
    }
    pub fn history(&self) -> &[Plan] {
        &self.history
    }
}

impl Default for PlanManager {
    fn default() -> Self {
        Self::new()
    }
}

fn uuid_simple() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    std::time::SystemTime::now().hash(&mut h);
    format!("{:016x}", h.finish())
}
