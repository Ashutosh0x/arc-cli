//! # arc-policy
//!
//! Customizable Policy & Constraints Engine.
//! Evaluates LLM actions and output against a defined set of user rules
//! before they are executed or shown to the user.

pub mod engine;
pub mod rules;

pub use engine::{PolicyEngine, PolicyResult, PolicyViolation};
pub use rules::{PolicyRule, RuleSeverity};
