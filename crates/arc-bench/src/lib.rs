//! # arc-bench
//!
//! Benchmarking and evaluation subsystem (ARC-EVAL).
//! Measures accuracy, cost, latency, and context-retention of ARC agents
//! against standardized coding benchmarks (e.g., HumanEval, SWE-bench mini).

pub mod evaluator;
pub mod suites;

pub use evaluator::{EvaluationMetrics, EvaluationResult, Evaluator};
pub use suites::{BenchmarkSuite, TestCase};
