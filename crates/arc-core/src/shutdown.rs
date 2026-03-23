// SPDX-License-Identifier: MIT
//! Graceful shutdown coordination via `CancellationToken`.

use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Global shutdown controller.
///
/// Distribute clones of the inner token to every long-running task.
/// When `trigger()` is called, all tasks observe cancellation.
#[derive(Clone)]
pub struct ShutdownController {
    token: CancellationToken,
}

impl ShutdownController {
    /// Create a new controller.
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }

    /// Get a child token to pass into spawned tasks.
    pub fn token(&self) -> CancellationToken {
        self.token.child_token()
    }

    /// Signal all tasks to shut down.
    pub fn trigger(&self) {
        tracing::info!("shutdown triggered");
        self.token.cancel();
    }

    /// Wait for Ctrl-C or SIGTERM, then trigger shutdown.
    ///
    /// Call this in a `tokio::spawn` from main.
    pub async fn wait_for_signal(self) {
        let ctrl_c = tokio::signal::ctrl_c();

        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            let mut sigterm =
                signal(SignalKind::terminate()).expect("failed to register SIGTERM handler");
            tokio::select! {
                _ = ctrl_c => {},
                _ = sigterm.recv() => {},
            }
        }

        #[cfg(not(unix))]
        {
            let _ = ctrl_c.await;
        }

        self.trigger();
    }

    /// Wait for shutdown with a grace period.
    ///
    /// Returns `true` if all tasks completed within the deadline.
    pub async fn graceful_shutdown(&self, grace: Duration) -> bool {
        self.trigger();
        tokio::time::timeout(grace, self.token.cancelled())
            .await
            .is_ok()
    }
}

impl Default for ShutdownController {
    fn default() -> Self {
        Self::new()
    }
}
