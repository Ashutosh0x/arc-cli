//! # arc-remote
//!
//! Enables connecting to a running ARC CLI session from another terminal window,
//! tracking its progress, or sending commands remotely over a local socket or HTTP.

pub mod auth;
pub mod client;
pub mod protocol;
pub mod server;

pub use auth::{auth_middleware, SecretKey};
pub use client::{RemoteClient, RemoteClientConfig};
pub use protocol::{ClientMessage, ServerMessage, SessionStateDto};
pub use server::{RemoteServer, ServerConfig};
