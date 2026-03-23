// SPDX-License-Identifier: MIT
//! Multi-step progress indicator for compound LLM operations.
//!
//! Shows a checklist-style output:
//!
//! ```text
//!   ✓ Reading file src/main.rs
//!   ✓ Analyzing code structure
//!   ⠹ Generating changes…  12s
//!   ○ Writing to disk
//!   ○ Formatting output
//! ```

use std::io::{self, Write};
use std::time::Instant;

use crossterm::{
    cursor::{Hide, MoveToColumn, MoveUp, Show},
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{Clear, ClearType},
};

/// Status of a single step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    /// Not yet started.
    Pending,
    /// Currently running (shows spinner).
    Running,
    /// Completed successfully.
    Done,
    /// Skipped.
    Skipped,
    /// Failed.
    Failed,
}

/// A single step in a multi-step progress.
#[derive(Debug, Clone)]
pub struct Step {
    pub label: String,
    pub status: StepStatus,
    pub detail: Option<String>,
    started_at: Option<Instant>,
}

impl Step {
    fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            status: StepStatus::Pending,
            detail: None,
            started_at: None,
        }
    }
}

/// A multi-step progress display.
///
/// ```rust,no_run
/// use arc_tui::progress::MultiStepProgress;
///
/// # async fn example() {
/// let mut progress = MultiStepProgress::new(vec![
///     "Reading files",
///     "Analyzing code",
///     "Generating changes",
///     "Writing to disk",
/// ]);
///
/// progress.start(0);     // marks step 0 as running
/// // ... do work ...
/// progress.complete(0);  // marks step 0 as done
///
/// progress.start(1);
/// // ... do work ...
/// progress.complete(1);
/// # }
/// ```
pub struct MultiStepProgress {
    steps: Vec<Step>,
    rendered_once: bool,
    spinner_frames: &'static [&'static str],
    frame_idx: usize,
}

impl MultiStepProgress {
    /// Create a new progress indicator with the given step labels.
    pub fn new(labels: Vec<&str>) -> Self {
        Self {
            steps: labels.into_iter().map(Step::new).collect(),
            rendered_once: false,
            spinner_frames: &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            frame_idx: 0,
        }
    }

    /// Mark a step as running.
    pub fn start(&mut self, index: usize) {
        if let Some(step) = self.steps.get_mut(index) {
            step.status = StepStatus::Running;
            step.started_at = Some(Instant::now());
        }
        self.render();
    }

    /// Mark a step as completed.
    pub fn complete(&mut self, index: usize) {
        if let Some(step) = self.steps.get_mut(index) {
            step.status = StepStatus::Done;
        }
        self.render();
    }

    /// Mark a step as failed.
    pub fn fail(&mut self, index: usize) {
        if let Some(step) = self.steps.get_mut(index) {
            step.status = StepStatus::Failed;
        }
        self.render();
    }

    /// Mark a step as skipped.
    pub fn skip(&mut self, index: usize) {
        if let Some(step) = self.steps.get_mut(index) {
            step.status = StepStatus::Skipped;
        }
        self.render();
    }

    /// Set a detail string for a step (e.g. filename).
    pub fn set_detail(&mut self, index: usize, detail: impl Into<String>) {
        if let Some(step) = self.steps.get_mut(index) {
            step.detail = Some(detail.into());
        }
        self.render();
    }

    /// Advance the spinner frame and re-render (call on a timer).
    pub fn tick(&mut self) {
        self.frame_idx = self.frame_idx.wrapping_add(1);
        self.render();
    }

    /// Render all steps to stderr.
    fn render(&mut self) {
        let mut stderr = io::stderr();
        let num_lines = self.steps.len();

        // Move cursor up to overwrite previous render.
        if self.rendered_once {
            let _ = execute!(stderr, MoveUp(num_lines as u16));
        }

        let _ = execute!(stderr, Hide);

        for step in &self.steps {
            let _ = execute!(stderr, MoveToColumn(0), Clear(ClearType::CurrentLine));

            match step.status {
                StepStatus::Pending => {
                    let _ = execute!(
                        stderr,
                        SetForegroundColor(Color::DarkGrey),
                        Print("  ○ "),
                        Print(&step.label),
                        ResetColor,
                    );
                },
                StepStatus::Running => {
                    let frame = self.spinner_frames[self.frame_idx % self.spinner_frames.len()];
                    let elapsed = step.started_at.map(|s| s.elapsed()).unwrap_or_default();

                    let _ = execute!(
                        stderr,
                        SetForegroundColor(Color::Cyan),
                        Print("  "),
                        Print(frame),
                        Print(" "),
                        SetAttribute(Attribute::Bold),
                        Print(&step.label),
                        SetAttribute(Attribute::Reset),
                        Print("…"),
                    );

                    // Elapsed time.
                    if elapsed.as_secs() > 0 {
                        let _ = execute!(
                            stderr,
                            SetForegroundColor(Color::DarkGrey),
                            Print(format!("  {}s", elapsed.as_secs())),
                            ResetColor,
                        );
                    }

                    // Detail.
                    if let Some(ref detail) = step.detail {
                        let _ = execute!(
                            stderr,
                            SetForegroundColor(Color::DarkGrey),
                            Print(format!("  ({detail})")),
                            ResetColor,
                        );
                    }
                },
                StepStatus::Done => {
                    let _ = execute!(
                        stderr,
                        SetForegroundColor(Color::Green),
                        Print("  ✓ "),
                        ResetColor,
                        Print(&step.label),
                    );
                },
                StepStatus::Skipped => {
                    let _ = execute!(
                        stderr,
                        SetForegroundColor(Color::Yellow),
                        Print("  ⊘ "),
                        ResetColor,
                        SetForegroundColor(Color::DarkGrey),
                        Print(&step.label),
                        Print(" (skipped)"),
                        ResetColor,
                    );
                },
                StepStatus::Failed => {
                    let _ = execute!(
                        stderr,
                        SetForegroundColor(Color::Red),
                        Print("  ✗ "),
                        ResetColor,
                        Print(&step.label),
                    );
                    if let Some(ref detail) = step.detail {
                        let _ = execute!(
                            stderr,
                            SetForegroundColor(Color::Red),
                            Print(format!("  — {detail}")),
                            ResetColor,
                        );
                    }
                },
            }

            let _ = execute!(stderr, Print("\n"));
        }

        let _ = execute!(stderr, Show);
        let _ = stderr.flush();
        self.rendered_once = true;
    }

    /// Final render showing summary.
    pub fn finish_all(&mut self) {
        self.render();

        let mut stderr = io::stderr();

        let done = self
            .steps
            .iter()
            .filter(|s| s.status == StepStatus::Done)
            .count();
        let failed = self
            .steps
            .iter()
            .filter(|s| s.status == StepStatus::Failed)
            .count();

        if failed == 0 {
            let _ = execute!(
                stderr,
                SetForegroundColor(Color::Green),
                Print(format!("\n  All {done} steps completed ✓\n\n")),
                ResetColor,
            );
        } else {
            let _ = execute!(
                stderr,
                SetForegroundColor(Color::Red),
                Print(format!("\n  {failed} step(s) failed, {done} succeeded\n\n")),
                ResetColor,
            );
        }
    }
}

impl Drop for MultiStepProgress {
    fn drop(&mut self) {
        let mut stderr = io::stderr();
        let _ = execute!(stderr, Show);
    }
}
