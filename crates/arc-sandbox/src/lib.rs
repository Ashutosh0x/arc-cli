// SPDX-License-Identifier: MIT
#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub use linux::LinuxSandbox as OsSandbox;

#[cfg(target_os = "macos")]
pub use macos::MacosSandbox as OsSandbox;

#[cfg(target_os = "windows")]
pub use windows::WindowsSandbox as OsSandbox;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub mod fallback {
    use anyhow::Result;
    pub struct OsSandbox;
    impl OsSandbox {
        pub fn new() -> Self {
            Self
        }
        pub fn apply(&mut self, _paths: &[std::path::PathBuf]) -> Result<()> {
            tracing::warn!("Sandboxing is strictly not supported on this OS framework");
            Ok(())
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub use fallback::OsSandbox;
