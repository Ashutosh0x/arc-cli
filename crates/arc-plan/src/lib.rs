//! # arc-plan — Read-Only Planning Mode
//!
//! Provides a safe, read-only research subagent that analyzes codebases,
//! maps dependencies, and proposes multi-step execution plans without
//! modifying any files.

mod planner;
mod read_only_tools;
mod plan_model;
mod dependency_mapper;
mod plan_renderer;

// Phase 28: Persistent Task Tracker with DAG validation
pub mod tracker;

pub use planner::Planner;
pub use plan_model::{Plan, PlanStep, PlanPhase, StepStatus, DependencyGraph};
pub use read_only_tools::ReadOnlyToolSet;
