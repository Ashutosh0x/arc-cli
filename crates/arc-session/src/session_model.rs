use chrono::{DateTime, Utc};
use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a checkpoint.
pub type CheckpointId = Uuid;

/// Complete session state that can be serialized and restored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// Unique session identifier
    pub session_id: Uuid,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last modified
    pub updated_at: DateTime<Utc>,
    /// The project root directory
    pub project_root: String,
    /// Git branch at session start
    pub git_branch: Option<String>,
    /// Git commit hash at session start
    pub git_commit: Option<String>,
    /// Full conversation history
    pub conversation: Vec<ConversationTurn>,
    /// Memory tier snapshots
    pub memory: MemorySnapshot,
    /// Active model/provider
    pub active_model: String,
    /// Active provider
    pub active_provider: String,
    /// Accumulated token usage
    pub total_input_tokens: u64,
    /// Accumulated output tokens
    pub total_output_tokens: u64,
    /// Accumulated cost
    pub total_cost_usd: f64,
    /// System prompt used
    pub system_prompt: String,
    /// Project context (from ARC.md)
    pub project_context: Option<String>,
    /// Files modified during this session (path -> original content for undo)
    pub modified_files: Vec<FileModificationRecord>,
    /// Checkpoint history
    pub checkpoints: Vec<CheckpointMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub id: Uuid,
    pub role: TurnRole,
    pub content: CompactString,
    pub timestamp: DateTime<Utc>,
    pub token_count: u32,
    pub tool_calls: Vec<ToolCallRecord>,
    /// Checkpoint ID created after this turn
    pub checkpoint_id: Option<CheckpointId>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TurnRole {
    User,
    Assistant,
    System,
    ToolResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool_name: CompactString,
    pub input: serde_json::Value,
    pub output: CompactString,
    pub duration_ms: u64,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    /// Working memory (current turn context)
    pub working: Vec<CompactString>,
    /// Short-term memory (recent interactions)
    pub short_term: Vec<CompactString>,
    /// Long-term memory (summarized knowledge)
    pub long_term: Vec<CompactString>,
    /// Total tokens across all tiers
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileModificationRecord {
    pub path: String,
    pub original_content: Option<String>,
    pub modified_at: DateTime<Utc>,
    pub action: FileAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileAction {
    Created,
    Modified,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    pub id: CheckpointId,
    pub turn_index: usize,
    pub created_at: DateTime<Utc>,
    pub description: String,
    pub token_count: u64,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub session_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub project_root: String,
    pub git_branch: Option<String>,
    pub turn_count: usize,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub active_model: String,
    pub last_user_message: Option<String>,
}

impl SessionState {
    pub fn new(
        project_root: String,
        active_model: String,
        active_provider: String,
        system_prompt: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            session_id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            project_root,
            git_branch: detect_git_branch(),
            git_commit: detect_git_commit(),
            conversation: Vec::new(),
            memory: MemorySnapshot {
                working: Vec::new(),
                short_term: Vec::new(),
                long_term: Vec::new(),
                total_tokens: 0,
            },
            active_model,
            active_provider,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost_usd: 0.0,
            system_prompt,
            project_context: None,
            modified_files: Vec::new(),
            checkpoints: Vec::new(),
        }
    }

    pub fn metadata(&self) -> SessionMetadata {
        SessionMetadata {
            session_id: self.session_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            project_root: self.project_root.clone(),
            git_branch: self.git_branch.clone(),
            turn_count: self.conversation.len(),
            total_tokens: self.total_input_tokens + self.total_output_tokens,
            total_cost_usd: self.total_cost_usd,
            active_model: self.active_model.clone(),
            last_user_message: self
                .conversation
                .iter()
                .rev()
                .find(|t| t.role == TurnRole::User)
                .map(|t| t.content.to_string()),
        }
    }
}

fn detect_git_branch() -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
}

fn detect_git_commit() -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
}
