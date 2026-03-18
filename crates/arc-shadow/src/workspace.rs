use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ShadowOptions {
    /// If true, use file hardlinks on supported OSs for instant,
    /// zero-storage copies of the workspace.
    pub use_hardlinks: bool,
    /// Files or directories to ignore when copying
    pub exclude_patterns: Vec<String>,
}

impl Default for ShadowOptions {
    fn default() -> Self {
        Self {
            use_hardlinks: true,
            exclude_patterns: vec!["target".to_string(), "node_modules".to_string()],
        }
    }
}

pub struct ShadowWorkspace {
    source_dir: PathBuf,
    shadow_dir: PathBuf,
    options: ShadowOptions,
}

impl ShadowWorkspace {
    pub async fn new(source_dir: PathBuf, options: ShadowOptions) -> Result<Self> {
        let shadow_id = Uuid::new_v4();
        let system_temp = std::env::temp_dir();
        let shadow_dir = system_temp.join(format!("arc-shadow-{}", shadow_id));

        tokio::fs::create_dir_all(&shadow_dir).await?;

        info!("Creating shadow workspace at {}", shadow_dir.display());

        let workspace = Self {
            source_dir,
            shadow_dir,
            options,
        };

        workspace.sync_from_source().await?;

        Ok(workspace)
    }

    /// Retrieve the path to the temporary shadow workspace.
    pub fn shadow_path(&self) -> &Path {
        &self.shadow_dir
    }

    /// Run a command inside the shadow workspace.
    pub async fn run_command(&self, program: &str, args: &[&str]) -> Result<std::process::Output> {
        debug!("Running '{}' in shadow workspace", program);
        
        let output = tokio::process::Command::new(program)
            .args(args)
            .current_dir(&self.shadow_dir)
            .output()
            .await?;

        Ok(output)
    }

    /// Sync the shadow workspace with the source directory via git or manual copy.
    pub async fn sync_from_source(&self) -> Result<()> {
        // Simple manual unoptimized copy logic for demo purposes.
        // In a real implementation, we'd use `ignore::WalkBuilder`
        // and hardlinks if supported.
        
        for entry in ignore::Walk::new(&self.source_dir) {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            if path == self.source_dir {
                continue;
            }

            // Strip prefix to get relative path
            let relative = path.strip_prefix(&self.source_dir).unwrap();
            
            // Skip target/node_modules explicitly
            if self.options.exclude_patterns.iter().any(|pat| relative.to_string_lossy().contains(pat)) {
                continue;
            }

            let target = self.shadow_dir.join(relative);

            if path.is_dir() {
                tokio::fs::create_dir_all(&target).await?;
            } else if path.is_file() {
                if let Some(parent) = target.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                
                #[cfg(unix)]
                {
                    if self.options.use_hardlinks {
                        if let Err(_) = std::fs::hard_link(path, &target) {
                            tokio::fs::copy(path, &target).await?;
                        }
                    } else {
                        tokio::fs::copy(path, &target).await?;
                    }
                }
                
                #[cfg(not(unix))]
                {
                    tokio::fs::copy(path, &target).await?;
                }
            }
        }
        
        Ok(())
    }

    /// Clean up the shadow workspace from disk. Called automatically on Drop,
    /// but exposed explicitly if needed.
    pub async fn destroy(self) -> Result<()> {
        info!("Destroying shadow workspace at {}", self.shadow_dir.display());
        tokio::fs::remove_dir_all(&self.shadow_dir).await?;
        Ok(())
    }
}

impl Drop for ShadowWorkspace {
    fn drop(&mut self) {
        // Attempt synchronous cleanup as fallback
        let _ = std::fs::remove_dir_all(&self.shadow_dir);
    }
}
