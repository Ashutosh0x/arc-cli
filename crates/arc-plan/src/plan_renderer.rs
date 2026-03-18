use crate::plan_model::*;
use std::fmt;

/// Renders a Plan as a beautiful terminal-friendly string.
pub struct PlanRenderer;

impl PlanRenderer {
    pub fn render(plan: &Plan) -> String {
        let mut out = String::with_capacity(4096);

        out.push_str(&format!("===========================================\n"));
        out.push_str(&format!("  ARC PLAN: {}  \n", truncate(&plan.title, 28)));
        out.push_str(&format!("===========================================\n\n"));

        out.push_str(&format!("{}\n\n", plan.description));

        // Summary
        out.push_str(&format!(
            "{} phases, {} steps | ~{} tokens | ~${:.4}\n",
            plan.phases.len(),
            plan.total_steps(),
            plan.estimated_tokens,
            plan.estimated_cost_usd
        ));

        out.push_str(&format!(
            "Risk: {:?}\n\n",
            plan.risk_assessment.overall_risk
        ));

        // Files affected
        if !plan.files_to_modify.is_empty() {
            out.push_str("Files to modify:\n");
            for file in &plan.files_to_modify {
                out.push_str(&format!(
                    "   {} (~{} lines) — {}\n",
                    file.path, file.estimated_lines_changed, file.description
                ));
            }
            out.push('\n');
        }

        if !plan.files_to_create.is_empty() {
            out.push_str("Files to create:\n");
            for file in &plan.files_to_create {
                out.push_str(&format!("   {file}\n"));
            }
            out.push('\n');
        }

        if !plan.files_to_delete.is_empty() {
            out.push_str("Files to delete:\n");
            for file in &plan.files_to_delete {
                out.push_str(&format!("   {file}\n"));
            }
            out.push('\n');
        }

        // Phases
        for (i, phase) in plan.phases.iter().enumerate() {
            let parallel_badge = if phase.can_parallelize { " (parallel)" } else { "" };
            out.push_str(&format!(
                "--- Phase {}: {}{} ---\n",
                i + 1,
                phase.name,
                parallel_badge
            ));

            for (j, step) in phase.steps.iter().enumerate() {
                let status_icon = match step.status {
                    StepStatus::Pending => "[ ]",
                    StepStatus::InProgress => "[~]",
                    StepStatus::Completed => "[x]",
                    StepStatus::Skipped => "[-]",
                    StepStatus::Failed => "[!]",
                };

                let risk_icon = match step.risk_level {
                    RiskLevel::Low => "(Low)",
                    RiskLevel::Medium => "(Med)",
                    RiskLevel::High => "(High)",
                    RiskLevel::Critical => "(Crit)",
                };

                out.push_str(&format!(
                    "  {status_icon} {}.{} {risk_icon} {}\n",
                    i + 1,
                    j + 1,
                    step.description
                ));

                if let Some(ref path) = step.file_path {
                    out.push_str(&format!("       {}\n", path));
                }

                if !step.rationale.is_empty() {
                    out.push_str(&format!("       {}\n", step.rationale));
                }
            }
            out.push('\n');
        }

        // Risk details
        if !plan.risk_assessment.breaking_changes.is_empty() {
            out.push_str("Breaking changes:\n");
            for change in &plan.risk_assessment.breaking_changes {
                out.push_str(&format!("   - {change}\n"));
            }
            out.push('\n');
        }

        if !plan.risk_assessment.security_concerns.is_empty() {
            out.push_str("Security concerns:\n");
            for concern in &plan.risk_assessment.security_concerns {
                out.push_str(&format!("   - {concern}\n"));
            }
        }

        out
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        format!("{:width$}", s, width = max)
    } else {
        format!("{}...", &s[..max - 3])
    }
}
