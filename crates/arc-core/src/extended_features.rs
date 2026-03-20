//! Interactive Question Tool — agent can prompt user mid-execution.
//! Bundled Slash Commands — /simplify, /batch, /explain.
//! ExitWorktree tool — complement to worktree.rs enter.
//! Language/Locale setting — language: "ja" in settings.
//! respectGitignore — honor .gitignore in @-mention file picker.
//! IS_DEMO mode — strip PII from UI.
//! 1M context toggle — CLAUDE_CODE_DISABLE_1M_CONTEXT equivalent.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ─── Interactive Question Tool ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveQuestion {
    pub question: String,
    pub question_type: QuestionType,
    pub options: Vec<String>,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionType {
    FreeText,
    Confirm,
    SingleSelect,
    MultiSelect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionResponse {
    pub answer: String,
    pub answered_at: String,
}

pub struct InteractiveQuestionTool;

impl InteractiveQuestionTool {
    pub fn build(question: &str) -> InteractiveQuestion {
        InteractiveQuestion {
            question: question.to_string(),
            question_type: QuestionType::FreeText,
            options: Vec::new(),
            default: None,
        }
    }

    pub fn confirm(question: &str) -> InteractiveQuestion {
        InteractiveQuestion {
            question: question.to_string(),
            question_type: QuestionType::Confirm,
            options: vec!["yes".into(), "no".into()],
            default: Some("yes".into()),
        }
    }

    pub fn select(question: &str, options: Vec<String>) -> InteractiveQuestion {
        InteractiveQuestion {
            question: question.to_string(),
            question_type: QuestionType::SingleSelect,
            options,
            default: None,
        }
    }
}

// ─── ExitWorktree Tool ───────────────────────────────────────────────

pub struct ExitWorktreeTool;

impl ExitWorktreeTool {
    /// Exit the current worktree session and return to the original working directory.
    pub fn exit(worktree_path: &Path, original_cwd: &Path) -> Result<(), String> {
        if !worktree_path.exists() {
            return Err(format!("Worktree path does not exist: {}", worktree_path.display()));
        }

        // Run git worktree remove
        let status = std::process::Command::new("git")
            .args(["worktree", "remove", &worktree_path.to_string_lossy()])
            .current_dir(original_cwd)
            .status();

        match status {
            Ok(s) if s.success() => {
                tracing::info!("Exited worktree: {}", worktree_path.display());
                Ok(())
            }
            Ok(s) => Err(format!("git worktree remove failed with status: {}", s)),
            Err(e) => Err(format!("Failed to run git: {}", e)),
        }
    }

    /// Force-remove a worktree (even if dirty)
    pub fn force_exit(worktree_path: &Path, original_cwd: &Path) -> Result<(), String> {
        let status = std::process::Command::new("git")
            .args(["worktree", "remove", "--force", &worktree_path.to_string_lossy()])
            .current_dir(original_cwd)
            .status();

        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("git worktree remove --force failed: {}", s)),
            Err(e) => Err(format!("Failed to run git: {}", e)),
        }
    }
}

// ─── Language/Locale Setting ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleConfig {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub output_format: OutputLanguage,
}

fn default_language() -> String {
    "en".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputLanguage {
    #[default]
    English,
    Japanese,
    Spanish,
    French,
    German,
    Chinese,
    Korean,
    Portuguese,
    Russian,
    Arabic,
    Hindi,
}

impl OutputLanguage {
    pub fn system_prompt_suffix(&self) -> &str {
        match self {
            Self::English => "",
            Self::Japanese => "\nPlease respond in Japanese (日本語).",
            Self::Spanish => "\nPlease respond in Spanish (Español).",
            Self::French => "\nPlease respond in French (Français).",
            Self::German => "\nPlease respond in German (Deutsch).",
            Self::Chinese => "\nPlease respond in Chinese (中文).",
            Self::Korean => "\nPlease respond in Korean (한국어).",
            Self::Portuguese => "\nPlease respond in Portuguese (Português).",
            Self::Russian => "\nPlease respond in Russian (Русский).",
            Self::Arabic => "\nPlease respond in Arabic (العربية).",
            Self::Hindi => "\nPlease respond in Hindi (हिंदी).",
        }
    }
}

// ─── Bundled Slash Commands ──────────────────────────────────────────

pub struct BundledCommands;

impl BundledCommands {
    pub fn list() -> Vec<(&'static str, &'static str)> {
        vec![
            ("/simplify", "Simplify the selected code — make it shorter and clearer"),
            ("/batch", "Run multiple prompts from a file, one per line"),
            ("/explain", "Explain the selected code or concept in detail"),
            ("/review", "Review code for bugs, security issues, and improvements"),
            ("/refactor", "Refactor the selected code with best practices"),
            ("/test", "Generate unit tests for the selected code"),
            ("/doc", "Generate documentation for the selected code"),
            ("/fix", "Fix the error or issue described in the last output"),
        ]
    }

    pub fn get_prompt(command: &str) -> Option<&'static str> {
        match command {
            "/simplify" => Some("Simplify this code. Remove unnecessary complexity, reduce line count, and improve clarity while maintaining the same behavior. Show the simplified version."),
            "/batch" => Some("Execute each line of the following file as a separate prompt, collecting results:"),
            "/explain" => Some("Explain this code in detail. Cover: what it does, how it works, key design decisions, potential issues, and suggestions for improvement."),
            "/review" => Some("Review this code thoroughly. Check for: bugs, security vulnerabilities, performance issues, code style, error handling, and suggest concrete improvements."),
            "/refactor" => Some("Refactor this code following best practices. Improve naming, structure, error handling, and maintainability while preserving behavior."),
            "/test" => Some("Generate comprehensive unit tests for this code. Cover happy paths, edge cases, error cases, and boundary conditions."),
            "/doc" => Some("Generate documentation for this code. Include: module-level docs, function docs with examples, type descriptions, and usage notes."),
            "/fix" => Some("Fix the error described above. Identify the root cause, explain why it happened, and provide the corrected code."),
            _ => None,
        }
    }
}

// ─── respectGitignore ────────────────────────────────────────────────

pub struct GitignoreFilter {
    patterns: Vec<String>,
}

impl GitignoreFilter {
    pub fn load(project_root: &Path) -> Self {
        let gitignore_path = project_root.join(".gitignore");
        let patterns = if gitignore_path.exists() {
            std::fs::read_to_string(&gitignore_path)
                .unwrap_or_default()
                .lines()
                .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
                .map(|l| l.trim().to_string())
                .collect()
        } else {
            Vec::new()
        };
        Self { patterns }
    }

    /// Check if a file path should be ignored based on .gitignore patterns
    pub fn should_ignore(&self, relative_path: &str) -> bool {
        for pattern in &self.patterns {
            if simple_gitignore_match(pattern, relative_path) {
                return true;
            }
        }
        false
    }

    /// Filter a list of files, removing those matching .gitignore
    pub fn filter_files(&self, files: Vec<PathBuf>) -> Vec<PathBuf> {
        files
            .into_iter()
            .filter(|f| !self.should_ignore(&f.to_string_lossy()))
            .collect()
    }
}

fn simple_gitignore_match(pattern: &str, path: &str) -> bool {
    let pattern = pattern.trim_end_matches('/');
    if pattern.contains('/') {
        path.contains(pattern)
    } else {
        // Match against filename or directory component
        let components: Vec<&str> = path.split('/').collect();
        components.iter().any(|c| {
            if pattern.contains('*') {
                glob_match_simple(pattern, c)
            } else {
                *c == pattern
            }
        })
    }
}

fn glob_match_simple(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == text;
    }
    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if let Some(found) = text[pos..].find(part) {
            if i == 0 && found != 0 {
                return false;
            }
            pos += found + part.len();
        } else {
            return false;
        }
    }
    true
}

// ─── JSON Output Mode ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonOutput {
    pub session_id: String,
    pub messages: Vec<JsonMessage>,
    pub metadata: JsonMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonMetadata {
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub duration_ms: u64,
    pub model: String,
    pub provider: String,
}

impl JsonOutput {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

// ─── Session Summary Export ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub started_at: String,
    pub ended_at: String,
    pub total_messages: u32,
    pub user_messages: u32,
    pub assistant_messages: u32,
    pub tool_calls: u32,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
    pub model: String,
    pub files_modified: Vec<String>,
    pub files_created: Vec<String>,
}

impl SessionSummary {
    pub fn export_to_file(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }
}

// ─── IS_DEMO Mode ────────────────────────────────────────────────────

pub struct DemoMode;

impl DemoMode {
    pub fn is_active() -> bool {
        std::env::var("IS_DEMO").map(|v| v == "1" || v.to_lowercase() == "true").unwrap_or(false)
    }

    pub fn redact_email(text: &str) -> String {
        let re = regex::Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").ok();
        match re {
            Some(re) => re.replace_all(text, "[REDACTED]").to_string(),
            None => text.to_string(),
        }
    }

    pub fn redact_org(text: &str) -> String {
        if let Ok(org) = std::env::var("ARC_ORG") {
            text.replace(&org, "[ORG]")
        } else {
            text.to_string()
        }
    }
}

// ─── Context Size Toggle ─────────────────────────────────────────────

pub struct ContextToggle;

impl ContextToggle {
    pub fn is_1m_disabled() -> bool {
        std::env::var("ARC_DISABLE_1M_CONTEXT")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false)
    }

    pub fn max_context_tokens() -> u64 {
        if Self::is_1m_disabled() {
            200_000
        } else {
            1_000_000
        }
    }
}

// ─── tmux/screen Clipboard Compat ────────────────────────────────────

pub struct ClipboardCompat;

impl ClipboardCompat {
    pub fn is_tmux() -> bool {
        std::env::var("TMUX").is_ok()
    }

    pub fn is_screen() -> bool {
        std::env::var("STY").is_ok()
    }

    pub fn is_ssh() -> bool {
        std::env::var("SSH_CLIENT").is_ok() || std::env::var("SSH_TTY").is_ok()
    }

    /// Copy text to clipboard with tmux/screen/SSH awareness
    pub fn copy(text: &str) -> Result<(), String> {
        if Self::is_tmux() {
            Self::copy_tmux(text)
        } else if Self::is_screen() {
            Self::copy_screen(text)
        } else if cfg!(target_os = "windows") {
            Self::copy_windows(text)
        } else if cfg!(target_os = "macos") {
            Self::copy_macos(text)
        } else {
            Self::copy_xclip(text)
        }
    }

    fn copy_tmux(text: &str) -> Result<(), String> {
        use std::io::Write;
        let mut child = std::process::Command::new("tmux")
            .args(["load-buffer", "-"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("tmux copy failed: {}", e))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        }
        child.wait().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn copy_screen(text: &str) -> Result<(), String> {
        let tmp = std::env::temp_dir().join("arc_screen_buf");
        std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
        std::process::Command::new("screen")
            .args(["-X", "readbuf", &tmp.to_string_lossy()])
            .status()
            .map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&tmp);
        Ok(())
    }

    /// Windows: use PowerShell Set-Clipboard for CJK/Unicode safety
    fn copy_windows(text: &str) -> Result<(), String> {
        use std::io::Write;
        let mut child = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", "Set-Clipboard -Value $input"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("PowerShell clipboard failed: {}", e))?;
        if let Some(mut stdin) = child.stdin.take() {
            // Write as UTF-8 — PowerShell Set-Clipboard handles CJK/emoji correctly
            stdin.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        }
        child.wait().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn copy_macos(text: &str) -> Result<(), String> {
        use std::io::Write;
        let mut child = std::process::Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("pbcopy failed: {}", e))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        }
        child.wait().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn copy_xclip(text: &str) -> Result<(), String> {
        use std::io::Write;
        let cmd = if which::which("xclip").is_ok() {
            "xclip"
        } else if which::which("xsel").is_ok() {
            "xsel"
        } else {
            return Err("No clipboard tool found (install xclip or xsel)".into());
        };

        let mut child = std::process::Command::new(cmd)
            .args(if cmd == "xclip" { vec!["-selection", "clipboard"] } else { vec!["--clipboard", "--input"] })
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("{} failed: {}", cmd, e))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        }
        child.wait().map_err(|e| e.to_string())?;
        Ok(())
    }
}

// ─── Todo/Task Tracker ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoTracker {
    pub items: Vec<TodoItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: u32,
    pub description: String,
    pub status: TodoStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Done,
    Skipped,
}

impl TodoTracker {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add(&mut self, description: &str) -> u32 {
        let id = self.items.len() as u32 + 1;
        self.items.push(TodoItem {
            id,
            description: description.to_string(),
            status: TodoStatus::Pending,
            created_at: chrono::Utc::now().to_rfc3339(),
        });
        id
    }

    pub fn complete(&mut self, id: u32) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.status = TodoStatus::Done;
        }
    }

    pub fn progress(&self) -> (usize, usize) {
        let done = self.items.iter().filter(|i| i.status == TodoStatus::Done).count();
        (done, self.items.len())
    }

    pub fn render(&self) -> String {
        let (done, total) = self.progress();
        let mut out = format!("📋 Progress: {}/{}\n", done, total);
        for item in &self.items {
            let icon = match item.status {
                TodoStatus::Pending => "○",
                TodoStatus::InProgress => "◉",
                TodoStatus::Done => "✓",
                TodoStatus::Skipped => "⊘",
            };
            out.push_str(&format!("  {} [{}] {}\n", icon, item.id, item.description));
        }
        out
    }
}
