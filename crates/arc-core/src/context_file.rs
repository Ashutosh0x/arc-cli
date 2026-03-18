//! # ARC.md Project Context Auto-Loader
//!
//! Automatically discovers and loads project context files from the
//! repository root and user home directory, injecting them into every
//! LLM session as persistent context.

use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

/// Ordered list of context file names to search for.
const CONTEXT_FILE_CANDIDATES: &[&str] = &[
    "ARC.md",
    ".arc/ARC.md",
    ".arc/context.md",
];

/// Global context file paths (user home directory).
const GLOBAL_CONTEXT_CANDIDATES: &[&str] = &[
    ".arc/ARC.md",
    ".arc/global_context.md",
];

/// Maximum context file size (256 KB) to prevent accidentally loading huge files.
const MAX_CONTEXT_FILE_SIZE: u64 = 256 * 1024;

/// Loaded project context with metadata.
#[derive(Debug, Clone)]
pub struct ProjectContext {
    /// The raw content of the context file.
    pub content: String,
    /// Where the file was loaded from.
    pub source: ContextSource,
    /// The file path that was loaded.
    pub file_path: PathBuf,
    /// Parsed directives from the context file.
    pub directives: Vec<ContextDirective>,
}

#[derive(Debug, Clone)]
pub enum ContextSource {
    /// From the project root directory
    ProjectLocal,
    /// From ~/.arc/
    GlobalUser,
}

#[derive(Debug, Clone)]
pub enum ContextDirective {
    /// Files that should always be included in context
    AlwaysInclude(Vec<String>),
    /// Glob patterns for files the agent should never modify
    NeverModify(Vec<String>),
    /// Preferred coding style rules
    StyleRule(String),
    /// Custom system prompt additions
    SystemPromptAddition(String),
    /// Test commands the agent should use
    TestCommand(String),
    /// Build commands
    BuildCommand(String),
    /// Forbidden patterns (e.g., "never use unwrap()")
    ForbiddenPattern(String),
}

/// The context loader that manages finding and parsing context files.
pub struct ContextLoader {
    project_root: PathBuf,
    home_dir: Option<PathBuf>,
}

impl ContextLoader {
    pub fn new(project_root: PathBuf) -> Self {
        let home_dir = dirs::home_dir();
        Self {
            project_root,
            home_dir,
        }
    }

    /// Load all available context files, merging project-local and global.
    /// Project-local context takes priority over global context.
    pub async fn load_all(&self) -> Result<Vec<ProjectContext>> {
        let mut contexts = Vec::new();

        // 1. Load global context first (lower priority)
        if let Some(ref home) = self.home_dir {
            for candidate in GLOBAL_CONTEXT_CANDIDATES {
                let path = home.join(candidate);
                if let Some(ctx) = self.try_load_file(&path, ContextSource::GlobalUser).await {
                    info!("Loaded global context from: {}", path.display());
                    contexts.push(ctx);
                    break; // Only load the first match
                }
            }
        }

        // 2. Load project-local context (higher priority, loaded second so it's later in the list)
        for candidate in CONTEXT_FILE_CANDIDATES {
            let path = self.project_root.join(candidate);
            if let Some(ctx) = self.try_load_file(&path, ContextSource::ProjectLocal).await {
                info!("Loaded project context from: {}", path.display());
                contexts.push(ctx);
                break; // Only load the first match
            }
        }

        if contexts.is_empty() {
            debug!("No ARC.md context file found");
        }

        Ok(contexts)
    }

    /// Merge all loaded contexts into a single system prompt addition.
    pub async fn load_merged_context(&self) -> Result<String> {
        let contexts = self.load_all().await?;

        if contexts.is_empty() {
            return Ok(String::new());
        }

        let mut merged = String::with_capacity(4096);

        merged.push_str("<project_context>\n");

        for ctx in &contexts {
            let source_label = match ctx.source {
                ContextSource::ProjectLocal => "Project",
                ContextSource::GlobalUser => "Global",
            };

            merged.push_str(&format!(
                "<!-- {} context from {} -->\n",
                source_label,
                ctx.file_path.display()
            ));
            merged.push_str(&ctx.content);
            merged.push_str("\n\n");
        }

        merged.push_str("</project_context>\n");

        Ok(merged)
    }

    /// Extract all directives from loaded contexts.
    pub async fn load_directives(&self) -> Result<Vec<ContextDirective>> {
        let contexts = self.load_all().await?;
        Ok(contexts
            .into_iter()
            .flat_map(|c| c.directives)
            .collect())
    }

    async fn try_load_file(&self, path: &Path, source: ContextSource) -> Option<ProjectContext> {
        // Check if file exists
        let metadata = fs::metadata(path).await.ok()?;

        // Size check
        if metadata.len() > MAX_CONTEXT_FILE_SIZE {
            warn!(
                "Context file {} exceeds maximum size ({} > {} bytes), skipping",
                path.display(),
                metadata.len(),
                MAX_CONTEXT_FILE_SIZE
            );
            return None;
        }

        // Read the file
        let content = fs::read_to_string(path).await.ok()?;

        // Parse directives
        let directives = parse_directives(&content);

        Some(ProjectContext {
            content,
            source,
            file_path: path.to_path_buf(),
            directives,
        })
    }
}

/// Parse special directives from the context file content.
/// Directives are embedded as HTML comments or special markdown sections.
fn parse_directives(content: &str) -> Vec<ContextDirective> {
    let mut directives = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Parse <!-- arc:directive value --> style comments
        if let Some(inner) = trimmed
            .strip_prefix("<!-- arc:")
            .and_then(|s| s.strip_suffix("-->"))
        {
            let inner = inner.trim();
            if let Some(value) = inner.strip_prefix("always_include ") {
                let files: Vec<String> = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
                directives.push(ContextDirective::AlwaysInclude(files));
            } else if let Some(value) = inner.strip_prefix("never_modify ") {
                let patterns: Vec<String> = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
                directives.push(ContextDirective::NeverModify(patterns));
            } else if let Some(value) = inner.strip_prefix("test_command ") {
                directives.push(ContextDirective::TestCommand(value.trim().to_string()));
            } else if let Some(value) = inner.strip_prefix("build_command ") {
                directives.push(ContextDirective::BuildCommand(value.trim().to_string()));
            } else if let Some(value) = inner.strip_prefix("forbid ") {
                directives.push(ContextDirective::ForbiddenPattern(value.trim().to_string()));
            } else if let Some(value) = inner.strip_prefix("style ") {
                directives.push(ContextDirective::StyleRule(value.trim().to_string()));
            }
        }

        // Parse @arc-directive style inline markers
        if let Some(rest) = trimmed.strip_prefix("@arc-forbid ") {
            directives.push(ContextDirective::ForbiddenPattern(rest.to_string()));
        }
        if let Some(rest) = trimmed.strip_prefix("@arc-test ") {
            directives.push(ContextDirective::TestCommand(rest.to_string()));
        }
        if let Some(rest) = trimmed.strip_prefix("@arc-build ") {
            directives.push(ContextDirective::BuildCommand(rest.to_string()));
        }
    }

    directives
}

/// Watch for changes to the context file and reload automatically.
pub struct ContextWatcher {
    loader: ContextLoader,
}

impl ContextWatcher {
    pub fn new(loader: ContextLoader) -> Self {
        Self { loader }
    }

    /// Start watching for context file changes. Sends updated context
    /// through the channel whenever the file changes.
    pub async fn watch(
        &self,
        tx: tokio::sync::watch::Sender<String>,
    ) -> Result<()> {
        use notify::{Event, EventKind, RecursiveMode, Watcher};

        let project_root = self.loader.project_root.clone();
        let (notify_tx, mut notify_rx) = tokio::sync::mpsc::channel::<()>(1);

        // Set up file watcher
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                if matches!(
                    event.kind,
                    EventKind::Modify(_) | EventKind::Create(_)
                ) {
                    let _ = notify_tx.blocking_send(());
                }
            }
        })?;

        // Watch all candidate paths
        for candidate in CONTEXT_FILE_CANDIDATES {
            let path = project_root.join(candidate);
            if let Some(parent) = path.parent() {
                if parent.exists() {
                    let _ = watcher.watch(parent, RecursiveMode::NonRecursive);
                }
            }
        }

        // Reload loop
        while notify_rx.recv().await.is_some() {
            // Debounce: wait 100ms for rapid successive writes
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            match self.loader.load_merged_context().await {
                Ok(context) => {
                    info!("Context file changed, reloaded");
                    let _ = tx.send(context);
                }
                Err(e) => {
                    warn!("Failed to reload context file: {}", e);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_load_project_context() {
        let dir = TempDir::new().unwrap();
        let arc_md = dir.path().join("ARC.md");
        tokio::fs::write(
            &arc_md,
            "# My Project\n\nUse async/await everywhere.\n\n\
             <!-- arc:test_command cargo test -->\n\
             <!-- arc:forbid unwrap() -->\n\
             <!-- arc:never_modify migrations/*, schema.sql -->\n",
        )
        .await
        .unwrap();

        let loader = ContextLoader::new(dir.path().to_path_buf());
        let contexts = loader.load_all().await.unwrap();

        assert_eq!(contexts.len(), 1);
        assert!(contexts[0].content.contains("Use async/await"));

        let directives = &contexts[0].directives;
        assert!(directives.iter().any(|d| matches!(d, ContextDirective::TestCommand(cmd) if cmd == "cargo test")));
        assert!(directives.iter().any(|d| matches!(d, ContextDirective::ForbiddenPattern(p) if p == "unwrap()")));
        assert!(directives.iter().any(|d| matches!(d, ContextDirective::NeverModify(patterns) if patterns.contains(&"migrations/*".to_string()))));
    }

    #[tokio::test]
    async fn test_no_context_file() {
        let dir = TempDir::new().unwrap();
        let loader = ContextLoader::new(dir.path().to_path_buf());
        let contexts = loader.load_all().await.unwrap();
        assert!(contexts.is_empty());
    }

    #[tokio::test]
    async fn test_merged_context() {
        let dir = TempDir::new().unwrap();
        let arc_md = dir.path().join("ARC.md");
        tokio::fs::write(&arc_md, "Project rules here").await.unwrap();

        let loader = ContextLoader::new(dir.path().to_path_buf());
        let merged = loader.load_merged_context().await.unwrap();
        assert!(merged.contains("<project_context>"));
        assert!(merged.contains("Project rules here"));
        assert!(merged.contains("</project_context>"));
    }
}
