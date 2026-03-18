//! # arc-loop
//!
//! Top-level autonomous execution loop that ties together Planner, Session,
//! and Agents into a continuous, resumable feedback cycle.

pub struct AutonomousLoop {
    max_iterations: usize,
}

impl AutonomousLoop {
    pub fn new(max_iterations: usize) -> Self {
        Self { max_iterations }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        tracing::info!("Starting autonomous loop for {} iterations", self.max_iterations);
        Ok(())
    }
}
