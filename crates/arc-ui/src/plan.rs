// SPDX-License-Identifier: MIT
use crate::state::{StepState, UiState};
use crate::theme::Theme;
use std::io::{self, Write};

pub fn render_plan(state: &UiState, theme: &Theme, out: &mut impl Write) -> io::Result<()> {
    if state.plan_steps.is_empty() {
        return Ok(());
    }

    let c = &theme.colors;
    let ic = &theme.icons;

    writeln!(out)?;
    writeln!(out, "{}{}  Execution Plan{}", c.bold, c.accent, c.reset)?;
    writeln!(out)?;

    for (i, step) in state.plan_steps.iter().enumerate() {
        let (icon, color) = match &step.state {
            StepState::Pending => ("○", c.dim),
            StepState::InProgress => {
                let _frame = theme.icons.spinner[state.spinner_frame % theme.icons.spinner.len()];
                // Can't return borrowed frame easily, so use dot
                ("◉", c.status_run)
            },
            StepState::Complete => (ic.check, c.status_ok),
            StepState::Failed(_) => (ic.cross, c.del),
            StepState::Skipped => ("⊘", c.dim),
        };

        let agent_tag = format!("{}[{}]{}", c.dim, step.agent, c.reset);

        writeln!(
            out,
            "    {}{}{} {}Step {}: {}{}  {}",
            color,
            icon,
            c.reset,
            c.bold,
            i + 1,
            step.label,
            c.reset,
            agent_tag,
        )?;

        if step.state == StepState::InProgress {
            writeln!(out, "      {}{}{}", c.dim, step.description, c.reset)?;
        }
    }

    writeln!(out)?;
    Ok(())
}
