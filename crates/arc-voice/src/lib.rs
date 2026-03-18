//! # arc-voice
//!
//! Experimental support for Realtime Voice APIs (e.g. Google Gemini Live or OpenAI Realtime).
//! Provides WebSocket streaming traits and WebRTC stubs for bi-directional audio.

pub mod streaming;

pub use streaming::{VoiceClient, VoiceSession, AudioFormat};
