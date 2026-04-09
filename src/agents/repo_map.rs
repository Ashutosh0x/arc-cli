// ARC CLI — RepoMap Agent
// Scans the project directory and builds a file tree for context.

use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

use crate::models::{AgentKind, AgentLog, OrchestratorEvent, Task};

/// Scan a directory recursively and produce a file tree string.
pub fn scan_directory(root: &Path) -> String {
    let mut output = String::new();
    output.push_str(&format!("Project root: {}\n\n", root.display()));
    walk_dir(root, root, 0, &mut output);
    output
}

fn walk_dir(root: &Path, dir: &Path, depth: usize, output: &mut String) {
    let indent = "  ".repeat(depth);

    // Read directory entries, sorted
    let mut entries: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect(),
        Err(_) => return,
    };
    entries.sort();

    for entry in entries {
        let name = entry
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Skip hidden dirs, target/, node_modules/
        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }

        if entry.is_dir() {
            output.push_str(&format!("{}{}/\n", indent, name));
            walk_dir(root, &entry, depth + 1, output);
        } else {
            let size = std::fs::metadata(&entry)
                .map(|m| m.len())
                .unwrap_or(0);
            let size_str = if size < 1024 {
                format!("{}B", size)
            } else if size < 1024 * 1024 {
                format!("{:.1}KB", size as f64 / 1024.0)
            } else {
                format!("{:.1}MB", size as f64 / (1024.0 * 1024.0))
            };
            output.push_str(&format!("{}{}  ({})\n", indent, name, size_str));
        }
    }
}

/// Run the RepoMap agent: scan project and report results.
pub async fn run(
    mut task: Task,
    project_dir: PathBuf,
    event_tx: mpsc::UnboundedSender<OrchestratorEvent>,
) {
    // Mark task as started
    task.start();
    let _ = event_tx.send(OrchestratorEvent::TaskUpdate(task.clone()));
    let _ = event_tx.send(OrchestratorEvent::Log(
        AgentLog::info(AgentKind::RepoMap, format!("Scanning directory: {}", project_dir.display())),
    ));

    // Scan
    let tree = tokio::task::spawn_blocking(move || {
        scan_directory(&project_dir)
    })
    .await
    .unwrap_or_else(|_| "Failed to scan directory".to_string());

    let file_count = tree.lines().count();
    let _ = event_tx.send(OrchestratorEvent::Log(
        AgentLog::info(AgentKind::RepoMap, format!("Scan complete: {} entries found", file_count)),
    ));

    // Stream the tree as tokens so UI can display it
    let _ = event_tx.send(OrchestratorEvent::Token(
        format!("=== Project Structure ===\n{}\n", tree),
    ));

    // Complete
    task.complete();
    let _ = event_tx.send(OrchestratorEvent::TaskUpdate(task));
    let _ = event_tx.send(OrchestratorEvent::Log(
        AgentLog::info(AgentKind::RepoMap, "RepoMap agent finished"),
    ));
}
