use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GitContext {
    pub current_branch: String,
    pub dirty_files: Vec<String>,
    pub recent_commits: Vec<CommitSummary>,
    pub remotes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CommitSummary {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: String,
}

pub fn gather_git_context(repo_root: &Path) -> Result<GitContext, anyhow::Error> {
    let branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_root)
        .output()?;

    let dirty = Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .current_dir(repo_root)
        .output()?;

    let log = Command::new("git")
        .args(["log", "--oneline", "-20", "--format=%H|%s|%an|%at"])
        .current_dir(repo_root)
        .output()?;

    Ok(GitContext {
        current_branch: String::from_utf8_lossy(&branch.stdout).trim().to_string(),
        dirty_files: String::from_utf8_lossy(&dirty.stdout)
            .lines()
            .map(String::from)
            .collect(),
        recent_commits: parse_log(&String::from_utf8_lossy(&log.stdout)),
        remotes: get_remotes(repo_root).unwrap_or_default(),
    })
}

fn parse_log(raw_log: &str) -> Vec<CommitSummary> {
    raw_log
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() == 4 {
                Some(CommitSummary {
                    hash: parts[0].to_string(),
                    message: parts[1].to_string(),
                    author: parts[2].to_string(),
                    timestamp: parts[3].to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn get_remotes(repo_root: &Path) -> Result<Vec<String>, anyhow::Error> {
    let out = Command::new("git")
        .args(["remote"])
        .current_dir(repo_root)
        .output()?;
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(String::from)
        .collect())
}

pub fn generate_commit_message(modified_files: &[String], session_context: &str) -> String {
    // In actual usage, this goes to an LLM. For local fallbacks:
    format!(
        "feat: AI assisted modifications\n\nContext block: {}\n\nFiles modified:\n{}",
        session_context,
        modified_files
            .iter()
            .map(|f| format!("  - {}", f))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

pub fn ensure_arc_gitignored(repo_root: &Path) -> Result<(), anyhow::Error> {
    let gitignore = repo_root.join(".gitignore");
    let content = std::fs::read_to_string(&gitignore).unwrap_or_default();

    if !content.lines().any(|l| l.trim() == ".arc/") {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&gitignore)?;

        use std::io::Write;
        writeln!(file, "\n# ARC CLI")?;
        writeln!(file, ".arc/")?;
        writeln!(file, ".arc-shadow/")?;
    }

    Ok(())
}

pub fn get_changed_files(repo_root: &Path) -> Result<Vec<String>, anyhow::Error> {
    let mut files = Vec::new();

    // Unstaged changes
    let diff = Command::new("git")
        .args(["diff", "--name-only"])
        .current_dir(repo_root)
        .output()?;
    files.extend(
        String::from_utf8_lossy(&diff.stdout)
            .lines()
            .map(String::from),
    );

    // Staged changes
    let diff_cached = Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .current_dir(repo_root)
        .output()?;
    files.extend(
        String::from_utf8_lossy(&diff_cached.stdout)
            .lines()
            .map(String::from),
    );

    // Untracked files
    let untracked = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .current_dir(repo_root)
        .output()?;
    files.extend(
        String::from_utf8_lossy(&untracked.stdout)
            .lines()
            .map(String::from),
    );

    files.sort();
    files.dedup();

    Ok(files)
}
