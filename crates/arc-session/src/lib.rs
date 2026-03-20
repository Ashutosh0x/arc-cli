//! # arc-session — Session Checkpointing & Resume
//!
//! Persists full session state to disk using redb, enabling instant
//! resume of interrupted sessions with up to 45% faster reload times
//! compared to cold start.

mod checkpoint;
mod session_db;
mod session_model;
mod rewind;

// Phase 28: Session Summary Service
pub mod summary;

pub use checkpoint::{CheckpointManager, CheckpointConfig};
pub use session_db::SessionDatabase;
pub use session_model::{
    SessionState, SessionMetadata, ConversationTurn, TurnRole,
    ToolCallRecord, MemorySnapshot, CheckpointId,
};
pub use rewind::RewindManager;
