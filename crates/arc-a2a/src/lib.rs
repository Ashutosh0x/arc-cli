//! # arc-a2a — Agent-to-Agent Protocol for ARC CLI
//!
//! Production-grade implementation of the A2A protocol enabling ARC agents
//! to discover, authenticate, and communicate with remote agents over HTTP.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                        A2A Client                          │
//! │  ┌──────────┐  ┌──────────┐  ┌──────┐  ┌──────────────┐   │
//! │  │Discovery │  │   Auth   │  │Retry │  │  SSE Stream  │   │
//! │  │ + Cache  │  │JWT/HMAC  │  │Backoff│  │  Subscriber  │   │
//! │  └────┬─────┘  └────┬─────┘  └──┬───┘  └──────┬───────┘   │
//! │       └──────────────┴──────────┴──────────────┘           │
//! │                          │ HTTP/2                           │
//! ├──────────────────────────┼──────────────────────────────────┤
//! │                          ▼                                  │
//! │                      A2A Server (Axum)                      │
//! │  ┌────────────┐  ┌────────────┐  ┌──────────────────────┐  │
//! │  │Agent Card  │  │  Message   │  │   Task Registry      │  │
//! │  │ Discovery  │  │  Router    │  │ (State Machine +     │  │
//! │  │ Endpoint   │  │            │  │  Watch Channels)     │  │
//! │  └────────────┘  └────────────┘  └──────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use arc_a2a::client::{A2AClient, ClientConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = A2AClient::new(ClientConfig {
//!     agent_id: "my-arc-agent".into(),
//!     ..Default::default()
//! });
//!
//! // Discover a remote agent
//! let card = client.discover("https://remote-agent.example.com").await?;
//! println!("Found agent: {} with {} skills", card.name, card.skills.len());
//!
//! // Submit a task
//! let task_id = client.submit_task(
//!     "https://remote-agent.example.com",
//!     "code_review",
//!     serde_json::json!({"file": "src/main.rs"}),
//! ).await?;
//! # Ok(())
//! # }
//! ```

pub mod auth;
pub mod client;
pub mod discovery;
pub mod error;
pub mod protocol;
pub mod server;
pub mod streaming;
pub mod task;

// Re-export primary types for ergonomic imports
pub use client::{A2AClient, ClientConfig};
pub use error::{A2AError, A2AResult};
pub use protocol::{A2AMessage, AgentCard, MessageBuilder, MessageType, Skill};
pub use server::start_server;
pub use task::{TaskRegistry, TaskState, TrackedTask};
