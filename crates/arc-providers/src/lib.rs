#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used)]

pub mod provider;
pub mod anthropic;
pub mod openai;
pub mod gemini;
pub mod ollama;
pub mod router;
pub mod stream;
pub mod security;
pub mod message;
pub mod traits;

pub mod breaker;
pub mod routing;
pub mod mock;
pub mod streaming {
    pub use crate::stream::{StreamingClient, StreamEvent};
}
