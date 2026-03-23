// SPDX-License-Identifier: MIT
//! Selective rewind: revert code changes only, keep conversation context.
//! Or revert conversation only, keep code changes.

use std::path::Path;
use tracing::{info, warn};

use crate::snapshot::{FileContent, FileState, SessionSnapshot};

/// What to rewind.
#[derive(Debug, Clone)]
pub enum RewindScope {
    /// Revert both code and conversation to the snapshot state.
    Full,
    /// Revert only code changes; keep the current conversation.
    CodeOnly,
    /// Revert only conversation; keep the current code changes.
    ConversationOnly,
    /// Revert specific files only.
    SpecificFiles(Vec<String>),
}

/// Result of a selective rewind operation.
#[derive(Debug)]
pub struct RewindResult {
    pub files_reverted: Vec<String>,
    pub files_skipped: Vec<String>,
    pub conversation_reverted: bool,
}

/// Perform a selective rewind to a snapshot.
pub fn selective_rewind(
    snapshot: &SessionSnapshot,
    working_dir: &Path,
    scope: RewindScope,
) -> Result<RewindResult, RewindError> {
    let mut files_reverted = Vec::new();
    let mut files_skipped = Vec::new();
    let conversation_reverted = match scope {
        RewindScope::Full => {
            revert_files(
                &snapshot.file_state,
                working_dir,
                None,
                &mut files_reverted,
                &mut files_skipped,
            )?;
            true
        },
        RewindScope::CodeOnly => {
            revert_files(
                &snapshot.file_state,
                working_dir,
                None,
                &mut files_reverted,
                &mut files_skipped,
            )?;
            false
        },
        RewindScope::ConversationOnly => true,
        RewindScope::SpecificFiles(ref paths) => {
            revert_files(
                &snapshot.file_state,
                working_dir,
                Some(paths),
                &mut files_reverted,
                &mut files_skipped,
            )?;
            false
        },
    };

    info!(
        files_reverted = files_reverted.len(),
        files_skipped = files_skipped.len(),
        conversation_reverted,
        "Selective rewind complete"
    );

    Ok(RewindResult {
        files_reverted,
        files_skipped,
        conversation_reverted,
    })
}

fn revert_files(
    file_state: &FileState,
    working_dir: &Path,
    filter: Option<&Vec<String>>,
    reverted: &mut Vec<String>,
    skipped: &mut Vec<String>,
) -> Result<(), RewindError> {
    for (path, record) in &file_state.files {
        // Apply filter if specified
        if let Some(filter_paths) = filter {
            if !filter_paths.iter().any(|f| path.contains(f)) {
                skipped.push(path.clone());
                continue;
            }
        }

        let abs_path = working_dir.join(path);

        match &record.content {
            FileContent::Inline { data } => {
                // Ensure parent directory exists
                if let Some(parent) = abs_path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| RewindError::Io(abs_path.clone(), e))?;
                }

                std::fs::write(&abs_path, data)
                    .map_err(|e| RewindError::Io(abs_path.clone(), e))?;

                // Restore permissions on Unix
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = std::fs::Permissions::from_mode(record.permissions);
                    std::fs::set_permissions(&abs_path, perms)
                        .map_err(|e| RewindError::Io(abs_path.clone(), e))?;
                }

                reverted.push(path.clone());
            },
            FileContent::BlobRef { blob_id } => {
                // In production, retrieve from content-addressable store
                warn!(
                    path = %path,
                    blob_id = %blob_id,
                    "Blob-referenced file revert not yet implemented"
                );
                skipped.push(path.clone());
            },
        }
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum RewindError {
    #[error("I/O error for {0}: {1}")]
    Io(std::path::PathBuf, std::io::Error),
}
