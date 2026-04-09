// ARC CLI — Core data models
// All types are serializable for state persistence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =====================================================================
//  Task — a unit of work dispatched to an agent
// =====================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub description: String,
    pub status: TaskStatus,
    pub agent: AgentKind,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl Task {
    pub fn new(description: String, agent: AgentKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            description,
            status: TaskStatus::Pending,
            agent,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    pub fn start(&mut self) {
        self.status = TaskStatus::InProgress;
        self.started_at = Some(Utc::now());
    }

    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    pub fn fail(&mut self, reason: String) {
        self.status = TaskStatus::Failed(reason);
        self.completed_at = Some(Utc::now());
    }

    /// Duration in milliseconds, if started.
    pub fn elapsed_ms(&self) -> Option<i64> {
        let start = self.started_at?;
        let end = self.completed_at.unwrap_or_else(Utc::now);
        Some((end - start).num_milliseconds())
    }
}

// =====================================================================
//  Task status — real lifecycle with failure reason
// =====================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "PENDING"),
            TaskStatus::InProgress => write!(f, "IN_PROGRESS"),
            TaskStatus::Completed => write!(f, "COMPLETE"),
            TaskStatus::Failed(r) => write!(f, "FAILED: {}", r),
        }
    }
}

// =====================================================================
//  Agent kinds
// =====================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentKind {
    RepoMap,
    Architect,
    Coder,
}

impl std::fmt::Display for AgentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentKind::RepoMap => write!(f, "RepoMap"),
            AgentKind::Architect => write!(f, "Architect"),
            AgentKind::Coder => write!(f, "Coder"),
        }
    }
}

// =====================================================================
//  Agent log — timestamped structured log entry
// =====================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLog {
    pub timestamp: DateTime<Utc>,
    pub agent: AgentKind,
    pub level: LogLevel,
    pub message: String,
}

impl AgentLog {
    pub fn new(agent: AgentKind, level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            agent,
            level,
            message: message.into(),
        }
    }

    pub fn info(agent: AgentKind, msg: impl Into<String>) -> Self {
        Self::new(agent, LogLevel::Info, msg)
    }

    pub fn warn(agent: AgentKind, msg: impl Into<String>) -> Self {
        Self::new(agent, LogLevel::Warn, msg)
    }

    pub fn error(agent: AgentKind, msg: impl Into<String>) -> Self {
        Self::new(agent, LogLevel::Error, msg)
    }

    pub fn debug(agent: AgentKind, msg: impl Into<String>) -> Self {
        Self::new(agent, LogLevel::Debug, msg)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

// =====================================================================
//  LLM usage stats — token counts, latency, model
// =====================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LLMUsage {
    pub model: String,
    pub provider: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub latency_ms: u64,
}

// =====================================================================
//  Diff result — output from diff engine
// =====================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    pub file_path: String,
    pub old_content: String,
    pub new_content: String,
    pub lines: Vec<DiffLine>,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffLine {
    Added(String),
    Removed(String),
    Unchanged(String),
}

// =====================================================================
//  Orchestrator messages — used by channels
// =====================================================================

#[derive(Debug, Clone)]
pub enum OrchestratorEvent {
    /// A task status changed
    TaskUpdate(Task),
    /// An agent produced a log entry
    Log(AgentLog),
    /// An agent produced a diff
    DiffProduced(DiffResult),
    /// LLM usage stats from an agent
    Usage(LLMUsage),
    /// A token streamed from an LLM
    Token(String),
    /// All agents finished
    PipelineComplete,
    /// Pipeline failed
    PipelineFailed(String),
}
