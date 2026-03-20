//! # arc-session — Session Checkpointing & Resume
//!
//! Persists full session state to disk using redb, enabling instant
//! resume of interrupted sessions with up to 45% faster reload times
//! compared to cold start.

mod checkpoint;
mod rewind;
mod session_db;
mod session_model;

// Phase 28: Session Summary Service
pub mod summary;

// Phase 30: Session Fork/Branch
pub mod fork;

pub use checkpoint::{CheckpointConfig, CheckpointManager};
pub use rewind::RewindManager;
pub use session_db::SessionDatabase;
pub use session_model::{
    CheckpointId, ConversationTurn, MemorySnapshot, SessionMetadata, SessionState, ToolCallRecord,
    TurnRole,
};
