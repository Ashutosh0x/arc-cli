//! Task lifecycle state machine with compile-time transition validation.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::watch;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::error::{A2AError, A2AResult};

/// All possible states a task can be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// Task has been received but not yet started
    Submitted,
    /// Task is actively being worked on
    Working,
    /// Task needs input from the requester before continuing
    InputRequired,
    /// Task finished successfully
    Completed,
    /// Task failed with an error
    Failed,
    /// Task was canceled by either party
    Canceled,
}

impl TaskState {
    /// Returns whether this state is terminal (no further transitions possible).
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Canceled)
    }

    /// Validates whether transitioning from `self` to `next` is allowed.
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            // Normal flow
            (Self::Submitted, Self::Working)
                | (Self::Working, Self::Completed)
                | (Self::Working, Self::Failed)
                | (Self::Working, Self::InputRequired)
                | (Self::InputRequired, Self::Working)
                // Cancellation from any non-terminal state
                | (Self::Submitted, Self::Canceled)
                | (Self::Working, Self::Canceled)
                | (Self::InputRequired, Self::Canceled)
        )
    }
}

/// A tracked task with its full lifecycle metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedTask {
    pub task_id: Uuid,
    pub skill_id: String,
    pub state: TaskState,
    pub requester_id: String,
    pub input: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    pub progress: f64,
    pub status_message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Thread-safe task registry managing all active and recent tasks.
pub struct TaskRegistry {
    tasks: DashMap<Uuid, TrackedTask>,
    /// Watchers for state changes — clients subscribe to get real-time updates.
    watchers: DashMap<Uuid, Arc<watch::Sender<TaskState>>>,
    /// Maximum completed tasks to retain in memory
    max_history: usize,
}

impl TaskRegistry {
    pub fn new(max_history: usize) -> Self {
        Self {
            tasks: DashMap::new(),
            watchers: DashMap::new(),
            max_history,
        }
    }

    /// Register a new task. Returns a watch receiver for state change notifications.
    pub fn register(
        &self,
        task_id: Uuid,
        skill_id: String,
        requester_id: String,
        input: serde_json::Value,
    ) -> watch::Receiver<TaskState> {
        let task = TrackedTask {
            task_id,
            skill_id,
            state: TaskState::Submitted,
            requester_id,
            input,
            output: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            progress: 0.0,
            status_message: "Submitted".into(),
        error: None,
        };

        let (tx, rx) = watch::channel(TaskState::Submitted);
        self.tasks.insert(task_id, task);
        self.watchers.insert(task_id, Arc::new(tx));

        info!(task_id = %task_id, "Task registered");
        self.gc_if_needed();
        rx
    }

    /// Transition a task to a new state. Validates the transition is legal.
    pub fn transition(
        &self,
        task_id: Uuid,
        new_state: TaskState,
    ) -> A2AResult<()> {
        let mut task = self
            .tasks
            .get_mut(&task_id)
            .ok_or(A2AError::TaskNotFound {
                task_id: task_id.to_string(),
            })?;

        if !task.state.can_transition_to(new_state) {
            return Err(A2AError::InvalidTransition {
                from: task.state,
                to: new_state,
            });
        }

        let old = task.state;
        task.state = new_state;

        match new_state {
            TaskState::Working => task.started_at = Some(Utc::now()),
            TaskState::Completed | TaskState::Failed | TaskState::Canceled => {
                task.completed_at = Some(Utc::now());
            }
            _ => {}
        }

        debug!(task_id = %task_id, from = ?old, to = ?new_state, "Task state transition");

        // Notify watchers
        if let Some(tx) = self.watchers.get(&task_id) {
            let _ = tx.send(new_state);
        }

        Ok(())
    }

    /// Update progress on a working task.
    pub fn update_progress(
        &self,
        task_id: Uuid,
        progress: f64,
        message: String,
        partial_output: Option<serde_json::Value>,
    ) -> A2AResult<()> {
        let mut task = self
            .tasks
            .get_mut(&task_id)
            .ok_or(A2AError::TaskNotFound {
                task_id: task_id.to_string(),
            })?;

        if task.state != TaskState::Working {
            warn!(
                task_id = %task_id,
                state = ?task.state,
                "Progress update on non-working task, ignoring"
            );
            return Ok(());
        }

        task.progress = progress.clamp(0.0, 1.0);
        task.status_message = message;
        if let Some(partial) = partial_output {
            task.output = Some(partial);
        }

        Ok(())
    }

    /// Complete a task with output.
    pub fn complete(&self, task_id: Uuid, output: serde_json::Value) -> A2AResult<()> {
        {
            let mut task = self
                .tasks
                .get_mut(&task_id)
                .ok_or(A2AError::TaskNotFound {
                    task_id: task_id.to_string(),
                })?;
            task.output = Some(output);
            task.progress = 1.0;
            task.status_message = "Completed".into();
        }
        self.transition(task_id, TaskState::Completed)
    }

    /// Fail a task with an error message.
    pub fn fail(&self, task_id: Uuid, error: String) -> A2AResult<()> {
        {
            let mut task = self
                .tasks
                .get_mut(&task_id)
                .ok_or(A2AError::TaskNotFound {
                    task_id: task_id.to_string(),
                })?;
            task.error = Some(error);
            task.status_message = "Failed".into();
        }
        self.transition(task_id, TaskState::Failed)
    }

    /// Get a snapshot of a task.
    pub fn get(&self, task_id: &Uuid) -> Option<TrackedTask> {
        self.tasks.get(task_id).map(|t| t.clone())
    }

    /// Subscribe to state changes for a task.
    pub fn subscribe(&self, task_id: &Uuid) -> Option<watch::Receiver<TaskState>> {
        self.watchers.get(task_id).map(|tx| tx.subscribe())
    }

    /// List all active (non-terminal) tasks.
    pub fn active_tasks(&self) -> Vec<TrackedTask> {
        self.tasks
            .iter()
            .filter(|t| !t.state.is_terminal())
            .map(|t| t.clone())
            .collect()
    }

    /// Count active tasks.
    pub fn active_count(&self) -> u32 {
        self.tasks
            .iter()
            .filter(|t| !t.state.is_terminal())
            .count() as u32
    }

    /// Garbage-collect completed tasks if over history limit.
    fn gc_if_needed(&self) {
        let terminal_count = self
            .tasks
            .iter()
            .filter(|t| t.state.is_terminal())
            .count();

        if terminal_count > self.max_history {
            let mut terminal: Vec<_> = self
                .tasks
                .iter()
                .filter(|t| t.state.is_terminal())
                .map(|t| (t.task_id, t.completed_at.unwrap_or(t.created_at)))
                .collect();

            terminal.sort_by_key(|(_, ts)| *ts);

            let to_remove = terminal_count - self.max_history;
            for (id, _) in terminal.into_iter().take(to_remove) {
                self.tasks.remove(&id);
                self.watchers.remove(&id);
            }

            debug!(removed = to_remove, "GC'd completed tasks from registry");
        }
    }
}
