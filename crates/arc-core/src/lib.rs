#![forbid(unsafe_code)]

pub mod config;
pub mod credentials;
pub mod error;
pub mod memory;
pub mod models;
pub mod model_picker;
pub mod security;
pub mod shutdown;
pub mod telemetry;
pub mod setup_wizard;
pub mod auth;

// Re-export the error for convenience.
pub use error::ArcError;
