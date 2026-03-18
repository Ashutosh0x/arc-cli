#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub use linux::LinuxSandbox as OsSandbox;

#[cfg(target_os = "macos")]
pub use macos::MacosSandbox as OsSandbox;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub mod fallback {
    use anyhow::Result;
    pub struct OsSandbox;
    impl OsSandbox {
        pub fn new() -> Self { Self }
        pub fn apply(&self, _paths: &[std::path::PathBuf]) -> Result<()> {
            tracing::warn!("Sandboxing is not supported on this OS");
            Ok(())
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub use fallback::OsSandbox;
