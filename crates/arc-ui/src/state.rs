use std::time::Instant;

/// Top-level phase of the CLI session
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    Startup,
    Idle,
    Planning,
    Streaming,
    DiffReview,
    Executing,
    Done,
}

/// State of one plan step
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepState {
    Pending,
    InProgress,
    Complete,
    Failed(String),
    Skipped,
}

/// A single planned action
#[derive(Debug, Clone)]
pub struct PlanStep {
    pub label:       String,
    pub description: String,
    pub state:       StepState,
    pub agent:       String,
    pub started_at:  Option<Instant>,
    pub finished_at: Option<Instant>,
}

/// Live cost + token counters
#[derive(Debug, Clone, Default)]
pub struct UsageMetrics {
    pub input_tokens:  u64,
    pub output_tokens: u64,
    pub total_cost:    f64,
    pub active_agents: Vec<String>,
    pub checkpoint_id: Option<String>,
}

/// Entire UI state
#[derive(Debug, Clone)]
pub struct UiState {
    pub phase:           Phase,
    pub header_mode:     HeaderMode,
    pub plan_steps:      Vec<PlanStep>,
    pub diff_blocks:     Vec<DiffBlock>,
    pub selected_diff:   usize,
    pub scroll_offset:   usize,
    pub metrics:         UsageMetrics,
    pub spinner_frame:   usize,
    pub terminal_width:  u16,
    pub terminal_height: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderMode {
    Expanded,
    Compact,
}

/// One file's diff
#[derive(Debug, Clone)]
pub struct DiffBlock {
    pub file_path:  String,
    pub additions:  usize,
    pub deletions:  usize,
    pub hunks:      Vec<Hunk>,
    pub expanded:   bool,
    pub accepted:   Option<bool>,  // None = pending
}

/// One contiguous change region
#[derive(Debug, Clone)]
pub struct Hunk {
    pub old_start: usize,
    pub new_start: usize,
    pub context:   Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub enum DiffLine {
    Context(String),
    Add(String),
    Del(String),
}

impl UiState {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            phase:          Phase::Startup,
            header_mode:    HeaderMode::Expanded,
            plan_steps:     Vec::new(),
            diff_blocks:    Vec::new(),
            selected_diff:  0,
            scroll_offset:  0,
            metrics:        UsageMetrics::default(),
            spinner_frame:  0,
            terminal_width: width,
            terminal_height: height,
        }
    }

    pub fn transition(&mut self, next: Phase) {
        // Auto-collapse header once user starts working
        if self.phase == Phase::Startup && next != Phase::Startup {
            self.header_mode = HeaderMode::Compact;
        }
        self.phase = next;
    }

    pub fn visible_diffs(&self) -> impl Iterator<Item = (usize, &DiffBlock)> {
        self.diff_blocks
            .iter()
            .enumerate()
            .filter(|(_, d)| d.additions + d.deletions > 0)
    }

    pub fn toggle_selected_diff(&mut self) {
        if let Some(diff) = self.diff_blocks.get_mut(self.selected_diff) {
            diff.expanded = !diff.expanded;
        }
    }

    pub fn accept_selected(&mut self) {
        if let Some(d) = self.diff_blocks.get_mut(self.selected_diff) {
            d.accepted = Some(true);
        }
        self.advance_selection();
    }

    pub fn reject_selected(&mut self) {
        if let Some(d) = self.diff_blocks.get_mut(self.selected_diff) {
            d.accepted = Some(false);
        }
        self.advance_selection();
    }

    pub fn accept_all(&mut self) {
        for d in &mut self.diff_blocks {
            if d.accepted.is_none() {
                d.accepted = Some(true);
            }
        }
    }

    pub fn reject_all(&mut self) {
        for d in &mut self.diff_blocks {
            if d.accepted.is_none() {
                d.accepted = Some(false);
            }
        }
    }

    fn advance_selection(&mut self) {
        // Jump to next pending diff
        for i in (self.selected_diff + 1)..self.diff_blocks.len() {
            if self.diff_blocks[i].accepted.is_none()
                && self.diff_blocks[i].additions + self.diff_blocks[i].deletions > 0
            {
                self.selected_diff = i;
                return;
            }
        }
    }
}
