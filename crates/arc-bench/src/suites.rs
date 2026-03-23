// SPDX-License-Identifier: MIT
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: String,
    pub prompt: String,
    pub expected_output_regex: String,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSuite {
    pub name: String,
    pub description: String,
    pub tests: Vec<TestCase>,
}

impl BenchmarkSuite {
    pub fn dummy_suite() -> Self {
        Self {
            name: "Basic Sanity Checks".to_string(),
            description: "A few simple Rust generation tasks".to_string(),
            tests: vec![TestCase {
                id: "test1".to_string(),
                prompt: "Write a Rust function that adds two numbers.".to_string(),
                expected_output_regex: "fn add\\(a: i32, b: i32\\) -> i32".to_string(),
                timeout_seconds: 30,
            }],
        }
    }
}
