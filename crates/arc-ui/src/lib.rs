pub mod animation;
pub mod diff;
pub mod footer;
pub mod header;
pub mod input;
pub mod layout;
pub mod plan;
pub mod renderer;
pub mod state;
pub mod theme;

use crossterm::terminal;
use state::{Phase, UiState};
use std::io::{self, stdout};

pub struct TerminalUi {
    state: UiState,
    layout: layout::Layout,
}

impl TerminalUi {
    pub fn new() -> io::Result<Self> {
        let (w, h) = terminal::size()?;
        Ok(Self {
            state: UiState::new(w, h),
            layout: layout::Layout::new(),
        })
    }

    /// Run startup sequence
    pub fn boot(&mut self) -> io::Result<()> {
        let mut out = stdout();
        terminal::enable_raw_mode()?;
        animation::play_startup(&mut out)?;
        self.state.transition(Phase::Idle);
        self.layout.paint(&self.state, &mut out)?;
        Ok(())
    }

    /// Enter diff review mode with computed diffs
    pub fn enter_review(&mut self, diffs: Vec<state::DiffBlock>) -> io::Result<ReviewResult> {
        self.state.diff_blocks = diffs;
        self.state.selected_diff = 0;
        self.state.transition(Phase::DiffReview);

        let mut out = stdout();
        self.layout.paint(&self.state, &mut out)?;

        // Interactive loop
        loop {
            if let Some(action) = input::poll_key() {
                let should_exit = input::apply_action(action, &mut self.state);
                self.layout.repaint_diffs(&self.state, &mut out)?;

                if should_exit {
                    break;
                }

                // Check if all diffs resolved
                if self.state.diff_blocks.iter().all(|d| d.accepted.is_some()) {
                    break;
                }
            }

            // Advance spinner
            self.state.spinner_frame = self.state.spinner_frame.wrapping_add(1);
        }

        terminal::disable_raw_mode()?;

        Ok(ReviewResult {
            accepted: self
                .state
                .diff_blocks
                .iter()
                .filter(|d| d.accepted == Some(true))
                .map(|d| d.file_path.clone())
                .collect(),
            rejected: self
                .state
                .diff_blocks
                .iter()
                .filter(|d| d.accepted == Some(false))
                .map(|d| d.file_path.clone())
                .collect(),
        })
    }

    /// Update live metrics during streaming
    pub fn update_metrics(&mut self, tokens: u64, cost: f64, agents: Vec<String>) {
        self.state.metrics.output_tokens = tokens;
        self.state.metrics.total_cost = cost;
        self.state.metrics.active_agents = agents;
    }

    pub fn shutdown(&self) -> io::Result<()> {
        terminal::disable_raw_mode()
    }
}

pub struct ReviewResult {
    pub accepted: Vec<String>,
    pub rejected: Vec<String>,
}
