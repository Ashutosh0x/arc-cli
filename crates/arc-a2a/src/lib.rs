//! # arc-a2a (Agent-to-Agent Protocol)
//!
//! Enables cross-agent communication. Used when ARC CLI agents need to
//! collaborate with other local or remote tools via a standard protocol.

pub mod protocol;

pub use protocol::{A2AMessage, MessageType, ProtocolClient};
