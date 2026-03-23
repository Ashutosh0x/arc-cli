// SPDX-License-Identifier: MIT
use crate::suites::BenchmarkSuite;
use anyhow::Result;
use std::time::Instant;
use tracing::info;

#[derive(Debug, Clone)]
pub struct EvaluationMetrics {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub pass_rate: f64,
    pub total_latency_ms: u64,
}

pub struct EvaluationResult {
    pub suite_name: String,
    pub metrics: EvaluationMetrics,
}

/// Runs a BenchmarkSuite through the active agent configuration.
pub struct Evaluator {
    suite: BenchmarkSuite,
}

impl Evaluator {
    pub fn new(suite: BenchmarkSuite) -> Self {
        Self { suite }
    }

    pub async fn run(&self) -> Result<EvaluationResult> {
        info!("Starting benchmark suite: {}", self.suite.name);

        let start = Instant::now();
        let mut passed = 0;
        let mut failed = 0;

        for test in &self.suite.tests {
            info!("Running test: {}", test.id);
            // In full implementation, this calls `Agent::execute_task(test.prompt)`
            // and then checks if the output satisfies `test.expected_output_regex`.

            // Mock success for now
            let is_success = true;

            if is_success {
                passed += 1;
            } else {
                failed += 1;
            }
        }

        let total = passed + failed;
        let pass_rate = if total > 0 {
            (passed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        Ok(EvaluationResult {
            suite_name: self.suite.name.clone(),
            metrics: EvaluationMetrics {
                total_tests: total,
                passed_tests: passed,
                failed_tests: failed,
                pass_rate,
                total_latency_ms: start.elapsed().as_millis() as u64,
            },
        })
    }
}
