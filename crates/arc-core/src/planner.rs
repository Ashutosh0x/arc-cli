// SPDX-License-Identifier: MIT
use tokio::sync::Mutex;
use crate::agent::{PlanStep, PlanStepStatus};

/// Handles the decomposition of user tasks into actionable checklist items.
pub struct Planner {
    steps: Mutex<Vec<PlanStep>>,
}

impl Planner {
    pub fn new() -> Self {
        Self {
            steps: Mutex::new(Vec::new()),
        }
    }

    /// Parse markdown list format into internal PlanSteps
    pub async fn set_plan_from_markdown(&self, markdown: &str) {
        let mut steps = Vec::new();
        let mut index = 1;

        for line in markdown.lines() {
            let line = line.trim();
            // Look for `1. `, ` - `, or `* ` prefixes
            if line.starts_with(char::is_numeric) || line.starts_with("- ") || line.starts_with("* ") {
                // simple heuristic to strip prefix
                let description = if let Some((_, text)) = line.split_once(' ') {
                    text.trim().to_string()
                } else {
                    line.to_string()
                };

                steps.push(PlanStep {
                    index,
                    description,
                    status: PlanStepStatus::Pending,
                });
                index += 1;
            }
        }

        let mut lock = self.steps.lock().await;
        *lock = steps;
    }

    pub async fn get_steps(&self) -> Vec<PlanStep> {
        self.steps.lock().await.clone()
    }

    pub async fn update_status(&self, index: usize, status: PlanStepStatus) {
        let mut lock = self.steps.lock().await;
        if let Some(step) = lock.iter_mut().find(|s| s.index == index) {
            step.status = status;
        }
    }
}

impl Default for Planner {
    fn default() -> Self {
        Self::new()
    }
}
