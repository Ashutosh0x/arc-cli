// SPDX-License-Identifier: MIT
//! # Background Bash Tasks — 5GB Cap + Process Cleanup + Ctrl+B Queries
//!
//! Manages long-running background shell tasks with output limits.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const MAX_OUTPUT_BYTES: usize = 5 * 1024 * 1024 * 1024; // 5GB

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Running,
    Completed,
    Failed,
    Killed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundTask {
    pub id: String,
    pub command: String,
    pub status: TaskStatus,
    pub output_bytes: usize,
    pub pid: Option<u32>,
    pub started_at: u64,
    pub completed_at: Option<u64>,
}

pub struct BackgroundTaskManager {
    tasks: HashMap<String, BackgroundTask>,
    next_id: usize,
}

impl BackgroundTaskManager {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn spawn(&mut self, command: &str) -> String {
        let id = format!("bg-{}", self.next_id);
        self.next_id += 1;
        let task = BackgroundTask {
            id: id.clone(),
            command: command.to_string(),
            status: TaskStatus::Running,
            output_bytes: 0,
            pid: None,
            started_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            completed_at: None,
        };
        self.tasks.insert(id.clone(), task);
        id
    }

    pub fn update_output(&mut self, id: &str, bytes: usize) -> bool {
        if let Some(task) = self.tasks.get_mut(id) {
            task.output_bytes += bytes;
            if task.output_bytes > MAX_OUTPUT_BYTES {
                task.status = TaskStatus::Killed;
                return false; // Kill signal.
            }
        }
        true
    }

    pub fn complete(&mut self, id: &str, success: bool) {
        if let Some(task) = self.tasks.get_mut(id) {
            task.status = if success {
                TaskStatus::Completed
            } else {
                TaskStatus::Failed
            };
            task.completed_at = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
        }
    }

    pub fn kill(&mut self, id: &str) {
        if let Some(task) = self.tasks.get_mut(id) {
            task.status = TaskStatus::Killed;
        }
    }

    pub fn kill_all(&mut self) {
        for task in self.tasks.values_mut() {
            if task.status == TaskStatus::Running {
                task.status = TaskStatus::Killed;
            }
        }
    }

    pub fn running(&self) -> Vec<&BackgroundTask> {
        self.tasks
            .values()
            .filter(|t| t.status == TaskStatus::Running)
            .collect()
    }
    pub fn get(&self, id: &str) -> Option<&BackgroundTask> {
        self.tasks.get(id)
    }
    pub fn all(&self) -> Vec<&BackgroundTask> {
        self.tasks.values().collect()
    }
}

impl Default for BackgroundTaskManager {
    fn default() -> Self {
        Self::new()
    }
}
