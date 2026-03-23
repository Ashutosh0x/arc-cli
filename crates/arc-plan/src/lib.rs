// SPDX-License-Identifier: MIT
//! # arc-plan — Read-Only Planning Mode
//!
//! Provides a safe, read-only research subagent that analyzes codebases,
//! maps dependencies, and proposes multi-step execution plans without
//! modifying any files.

mod dependency_mapper;
mod plan_model;
mod plan_renderer;
mod planner;
mod read_only_tools;

// Phase 28: Persistent Task Tracker with DAG validation
pub mod tracker;

pub use plan_model::{DependencyGraph, Plan, PlanPhase, PlanStep, StepStatus};
pub use planner::Planner;
pub use read_only_tools::ReadOnlyToolSet;
