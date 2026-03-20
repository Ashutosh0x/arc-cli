//! Snapshot: captures both conversation state AND file state atomically.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// A complete snapshot of the session state at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub id: Uuid,
    pub session_id: Uuid,
    pub parent_snapshot_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub turn_number: u32,
    pub label: Option<String>,

    /// Conversation state.
    pub conversation: ConversationState,

    /// File system state (tracked files only).
    pub file_state: FileState,

    /// Agent state (which agents are active, their context).
    pub agent_state: AgentState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationState {
    /// All messages in the conversation up to this point.
    pub messages: Vec<ConversationMessage>,
    /// Total tokens used.
    pub total_tokens: u64,
    /// Current system prompt hash.
    pub system_prompt_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub token_count: u32,
    /// Tool calls associated with this message.
    #[serde(default)]
    pub tool_calls: Vec<ToolCallRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub input_summary: String,
    pub output_summary: String,
    pub files_modified: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    /// Map of relative file path → file hash + content.
    pub files: HashMap<String, FileRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    /// SHA-256 hash of the file content.
    pub hash: String,
    /// The file content (for small files) or a reference to stored content.
    pub content: FileContent,
    /// File permissions (Unix mode).
    pub permissions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FileContent {
    /// Full content stored inline (for files < 1MB).
    #[serde(rename = "inline")]
    Inline { data: String },
    /// Content stored as a blob reference (for large files).
    #[serde(rename = "blob_ref")]
    BlobRef { blob_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    /// Currently active agents.
    pub active_agents: Vec<AgentRecord>,
    /// Pending tool calls.
    pub pending_tool_calls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRecord {
    pub agent_name: String,
    pub task: String,
    pub status: String,
}

impl SessionSnapshot {
    /// Create a snapshot from the current state.
    pub fn capture(
        session_id: Uuid,
        parent_id: Option<Uuid>,
        turn_number: u32,
        label: Option<String>,
        messages: Vec<ConversationMessage>,
        tracked_files: &[PathBuf],
        working_dir: &Path,
    ) -> Result<Self, SnapshotError> {
        // Capture file states
        let mut files = HashMap::new();
        for file_path in tracked_files {
            let abs_path = if file_path.is_relative() {
                working_dir.join(file_path)
            } else {
                file_path.clone()
            };

            if abs_path.exists() {
                let content = std::fs::read(&abs_path)
                    .map_err(|e| SnapshotError::FileRead(abs_path.clone(), e))?;

                let hash = hex::encode(Sha256::digest(&content));

                let relative = file_path
                    .strip_prefix(working_dir)
                    .unwrap_or(file_path)
                    .display()
                    .to_string();

                let file_content = if content.len() < 1_048_576 {
                    // < 1MB: store inline
                    FileContent::Inline {
                        data: String::from_utf8_lossy(&content).to_string(),
                    }
                } else {
                    // Large file: store as blob reference
                    let blob_id = hash.clone();
                    // In production, write the blob to a content-addressable store
                    FileContent::BlobRef { blob_id }
                };

                #[cfg(unix)]
                let permissions = {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::metadata(&abs_path)
                        .map(|m| m.permissions().mode())
                        .unwrap_or(0o644)
                };

                #[cfg(not(unix))]
                let permissions = 0o644;

                files.insert(
                    relative,
                    FileRecord {
                        hash,
                        content: file_content,
                        permissions,
                    },
                );
            }
        }

        let total_tokens: u64 = messages.iter().map(|m| m.token_count as u64).sum();
        let system_prompt_hash = String::new(); // Populated by caller

        Ok(Self {
            id: Uuid::new_v4(),
            session_id,
            parent_snapshot_id: parent_id,
            created_at: Utc::now(),
            turn_number,
            label,
            conversation: ConversationState {
                messages,
                total_tokens,
                system_prompt_hash,
            },
            file_state: FileState { files },
            agent_state: AgentState {
                active_agents: vec![],
                pending_tool_calls: vec![],
            },
        })
    }

    /// Compute the diff between two snapshots.
    pub fn diff_files(&self, other: &Self) -> Vec<FileDiff> {
        let mut diffs = Vec::new();

        // Files in self but not in other (or changed)
        for (path, record) in &self.file_state.files {
            match other.file_state.files.get(path) {
                None => {
                    diffs.push(FileDiff::Added {
                        path: path.clone(),
                        hash: record.hash.clone(),
                    });
                },
                Some(other_record) if other_record.hash != record.hash => {
                    diffs.push(FileDiff::Modified {
                        path: path.clone(),
                        old_hash: other_record.hash.clone(),
                        new_hash: record.hash.clone(),
                    });
                },
                _ => {},
            }
        }

        // Files in other but not in self
        for path in other.file_state.files.keys() {
            if !self.file_state.files.contains_key(path) {
                diffs.push(FileDiff::Deleted { path: path.clone() });
            }
        }

        diffs
    }
}

#[derive(Debug, Clone)]
pub enum FileDiff {
    Added {
        path: String,
        hash: String,
    },
    Modified {
        path: String,
        old_hash: String,
        new_hash: String,
    },
    Deleted {
        path: String,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("Failed to read file {0}: {1}")]
    FileRead(PathBuf, std::io::Error),

    #[error("Failed to serialize snapshot: {0}")]
    Serialize(String),

    #[error("Failed to deserialize snapshot: {0}")]
    Deserialize(String),
}
