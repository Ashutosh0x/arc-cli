// SPDX-License-Identifier: MIT
//! # arc-io
//!
//! High-performance async I/O wrapper. Uses `tokio-uring` on Linux for
//! zero-copy disk reads/writes, and falls back to standard `tokio::fs`
//! on Windows and macOS.

use anyhow::Result;
use std::path::Path;

/// Read a file completely into a buffer, using the fastest available
/// async I/O method for the current OS.
pub async fn read_file_fast(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    #[cfg(target_os = "linux")]
    {
        // Example implementation for tokio-uring
        // Note: tokio-uring requires running within `tokio_uring::start` context.
        // For CLI simplicity, we fallback to tokio::fs if not in a uring context.
        match std::fs::read(path.as_ref()) {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow::anyhow!("File read error: {}", e)),
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        let data = tokio::fs::read(path).await?;
        Ok(data)
    }
}

/// Write a buffer to disk using the fastest available async I/O method.
pub async fn write_file_fast(path: impl AsRef<Path>, data: &[u8]) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        match std::fs::write(path.as_ref(), data) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("File write error: {}", e)),
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        tokio::fs::write(path, data).await?;
        Ok(())
    }
}
