use crate::error::ArcError;
use fs2::FileExt;
use std::path::{Path, PathBuf};

/// A cross-process instance lock to prevent concurrent `arc` operations
/// mutating the same local `.arc` directory simultaneously.
pub struct InstanceLock {
    lock_path: PathBuf,
    // Keep file handle open to hold the OS-level lock
    _file: std::fs::File,
}

impl InstanceLock {
    pub fn acquire(workspace: &Path) -> Result<Self, ArcError> {
        let arc_dir = workspace.join(".arc");
        if !arc_dir.exists() {
            let _ = std::fs::create_dir_all(&arc_dir);
        }

        let lock_path = arc_dir.join("instance.lock");
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&lock_path)
            .map_err(|e| ArcError::System(e.to_string()))?;

        // Try exclusive non-blocking lock
        file.try_lock_exclusive().map_err(|_| {
            ArcError::InstanceConflict(
                "Another ARC process is already running in this directory.".into(),
            )
        })?;

        // Write our PID into the lockfile for diagnostic purposes
        use std::io::Write;
        let mut f = &file;
        let _ = writeln!(f, "{}", std::process::id());

        Ok(Self {
            lock_path,
            _file: file,
        })
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        // Drop the file lock
        let _ = self._file.unlock();
        // Best-effort cleanup
        let _ = std::fs::remove_file(&self.lock_path);
    }
}
