use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Security sandbox for all tool executions
pub struct ToolSandbox {
    /// Approval mode for this session
    pub approval_mode: ApprovalMode,

    /// Allowed directories for file operations
    allowed_paths: Vec<PathBuf>,

    /// Blocked shell commands
    blocked_commands: HashSet<String>,

    /// Blocked shell patterns (regex)
    blocked_patterns: Vec<regex::Regex>,

    /// Network allowlist (for shell commands that make requests)
    allowed_domains: HashSet<String>,

    /// Maximum file size the agent can write
    max_write_size: usize,

    /// Maximum number of tool calls per session
    max_tool_calls: u32,
    current_tool_calls: u32,

    /// Audit log
    audit_log: Vec<AuditEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalMode {
    /// Every destructive action requires confirmation
    Ask,
    /// Auto-approve safe operations, ask for dangerous ones
    Auto,
    /// Approve everything (DANGEROUS — only for trusted contexts)
    Yolo,
    /// No mutations allowed
    Readonly,
}

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: std::time::SystemTime,
    pub tool: String,
    pub action: String,
    pub approved: bool,
    pub approval_mode: ApprovalMode,
    pub details: String,
}

#[derive(Debug)]
pub enum ToolAction {
    FileRead { path: PathBuf },
    FileWrite { path: PathBuf, size: usize },
    FileDelete { path: PathBuf },
    ShellCommand { command: String, args: Vec<String> },
    NetworkRequest { url: String },
}

#[derive(Debug)]
pub enum SandboxVerdict {
    Allowed,
    NeedsApproval(String),
    Blocked(String),
}

impl ToolSandbox {
    pub fn new(approval_mode: ApprovalMode, project_root: &Path) -> Self {
        let mut blocked_commands = HashSet::new();
        // Commands that should never be run by an AI agent
        for cmd in &[
            "rm -rf /", "mkfs", "dd", "shutdown", "reboot", "halt",
            "passwd", "useradd", "userdel", "chown", "chmod 777",
            "curl | sh", "curl | bash", "wget | sh",
            "eval", "exec",
            // Credential theft
            "cat /etc/shadow", "cat ~/.ssh/id_rsa",
            "cat ~/.aws/credentials",
            // Crypto mining
            "xmrig", "minerd", "cgminer",
        ] {
            blocked_commands.insert(cmd.to_string());
        }

        let blocked_patterns = vec![
            // Reverse shells
            regex::Regex::new(r"(?i)(nc|ncat|netcat)\s+.*\s+-e\s+/bin/(ba)?sh").unwrap(),
            regex::Regex::new(r"(?i)/dev/tcp/").unwrap(),
            regex::Regex::new(r"(?i)bash\s+-i\s+>&\s*/dev/tcp").unwrap(),
            // Piping downloads to shell
            regex::Regex::new(r"(?i)(curl|wget)\s+.*\|\s*(ba)?sh").unwrap(),
            // Encoded command execution
            regex::Regex::new(r"(?i)echo\s+[A-Za-z0-9+/=]+\s*\|\s*base64\s+-d\s*\|\s*(ba)?sh")
                .unwrap(),
            // Disk destruction
            regex::Regex::new(r"(?i)dd\s+if=.*of=/dev/").unwrap(),
            // Recursive force delete from root
            regex::Regex::new(r"rm\s+(-[a-zA-Z]*f[a-zA-Z]*\s+)?/\s").unwrap(),
            // Environment variable exfiltration
            regex::Regex::new(r"(?i)(env|printenv|set)\s*\|\s*(curl|wget|nc)").unwrap(),
            // SSH key exfiltration
            regex::Regex::new(r"cat\s+~/?.ssh/(id_rsa|id_ed25519|authorized_keys)").unwrap(),
        ];

        Self {
            approval_mode,
            allowed_paths: vec![
                project_root.to_path_buf(),
                std::env::temp_dir(),
            ],
            blocked_commands,
            blocked_patterns,
            allowed_domains: HashSet::new(),
            max_write_size: 10 * 1024 * 1024, // 10MB
            max_tool_calls: 100,
            current_tool_calls: 0,
            audit_log: Vec::new(),
        }
    }

    /// Check if a tool action is allowed, needs approval, or is blocked
    pub fn check(&mut self, action: &ToolAction) -> SandboxVerdict {
        // Rate limit check
        if self.current_tool_calls >= self.max_tool_calls {
            return SandboxVerdict::Blocked(format!(
                "Tool call limit reached ({}/{})",
                self.current_tool_calls, self.max_tool_calls
            ));
        }

        match action {
            ToolAction::FileRead { path } => {
                if !self.is_path_allowed(path) {
                    return SandboxVerdict::Blocked(format!(
                        "Path outside allowed directories: {}",
                        path.display()
                    ));
                }
                // Check for sensitive files
                if self.is_sensitive_file(path) {
                    return SandboxVerdict::NeedsApproval(format!(
                        "Reading sensitive file: {}",
                        path.display()
                    ));
                }
                SandboxVerdict::Allowed
            }

            ToolAction::FileWrite { path, size } => {
                if self.approval_mode == ApprovalMode::Readonly {
                    return SandboxVerdict::Blocked("Session is read-only".into());
                }
                if !self.is_path_allowed(path) {
                    return SandboxVerdict::Blocked(format!(
                        "Write outside allowed directories: {}",
                        path.display()
                    ));
                }
                if *size > self.max_write_size {
                    return SandboxVerdict::Blocked(format!(
                        "Write exceeds max size ({} > {})",
                        size, self.max_write_size
                    ));
                }
                if self.approval_mode == ApprovalMode::Ask {
                    SandboxVerdict::NeedsApproval(format!(
                        "Write {} bytes to {}",
                        size,
                        path.display()
                    ))
                } else {
                    SandboxVerdict::Allowed
                }
            }

            ToolAction::FileDelete { path } => {
                if self.approval_mode == ApprovalMode::Readonly {
                    return SandboxVerdict::Blocked("Session is read-only".into());
                }
                // Deletions ALWAYS require approval unless YOLO mode
                if self.approval_mode != ApprovalMode::Yolo {
                    SandboxVerdict::NeedsApproval(format!(
                        "Delete file: {}",
                        path.display()
                    ))
                } else {
                    SandboxVerdict::Allowed
                }
            }

            ToolAction::ShellCommand { command, args } => {
                self.check_shell_command(command, args)
            }

            ToolAction::NetworkRequest { url } => {
                if self.approval_mode == ApprovalMode::Readonly {
                    return SandboxVerdict::Blocked("Network requests blocked in readonly mode".into());
                }
                SandboxVerdict::NeedsApproval(format!("Network request to: {url}"))
            }
        }
    }

    /// Record a tool action in the audit log
    pub fn record_audit(&mut self, tool: &str, action: &str, approved: bool, details: &str) {
        self.current_tool_calls += 1;
        self.audit_log.push(AuditEntry {
            timestamp: std::time::SystemTime::now(),
            tool: tool.to_string(),
            action: action.to_string(),
            approved,
            approval_mode: self.approval_mode.clone(),
            details: details.to_string(),
        });
    }

    /// Export audit log for review
    pub fn export_audit_log(&self) -> &[AuditEntry] {
        &self.audit_log
    }

    // ── Private helpers ──────────────────────────────────────

    fn is_path_allowed(&self, path: &Path) -> bool {
        // Resolve to absolute path (prevents ../../../etc/passwd traversal)
        let canonical = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // If file doesn't exist yet, check parent
                match path.parent().and_then(|p| p.canonicalize().ok()) {
                    Some(p) => p,
                    None => return false,
                }
            }
        };

        self.allowed_paths
            .iter()
            .any(|allowed| canonical.starts_with(allowed))
    }

    fn is_sensitive_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();
        let sensitive_patterns = [
            ".env", ".ssh", ".aws", ".gnupg", ".config/gcloud",
            "credentials", "secret", "private_key", "id_rsa",
            "id_ed25519", ".arc/config", "shadow", "passwd",
            ".git-credentials", ".netrc", "keychain",
        ];
        sensitive_patterns
            .iter()
            .any(|p| path_str.contains(p))
    }

    fn check_shell_command(&self, command: &str, args: &[String]) -> SandboxVerdict {
        if self.approval_mode == ApprovalMode::Readonly {
            return SandboxVerdict::Blocked("Shell execution blocked in readonly mode".into());
        }

        let full_command = format!("{} {}", command, args.join(" "));

        // Check explicit blocklist
        for blocked in &self.blocked_commands {
            if full_command.contains(blocked) {
                return SandboxVerdict::Blocked(format!(
                    "Blocked command pattern: {blocked}"
                ));
            }
        }

        // Check regex patterns
        for pattern in &self.blocked_patterns {
            if pattern.is_match(&full_command) {
                return SandboxVerdict::Blocked(format!(
                    "Blocked dangerous command pattern: {}",
                    pattern.as_str()
                ));
            }
        }

        // Safe commands that don't need approval
        let safe_commands = [
            "ls", "cat", "head", "tail", "wc", "grep", "find", "echo",
            "pwd", "date", "whoami", "which", "file", "stat", "tree",
            "cargo check", "cargo build", "cargo test", "cargo clippy",
            "cargo fmt", "git status", "git log", "git diff", "git branch",
            "rustc --version", "node --version", "python --version",
        ];

        if self.approval_mode == ApprovalMode::Auto
            && safe_commands.iter().any(|s| full_command.starts_with(s))
        {
            return SandboxVerdict::Allowed;
        }

        SandboxVerdict::NeedsApproval(format!("Execute: {full_command}"))
    }
}
