use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "arc",
    about = "⚡ ARC — Agent for Rapid Coding",
    version
)]
pub struct Cli {
    /// Prompt for one-shot mode. If omitted, starts interactive REPL.
    pub prompt: Option<String>,

    /// Model to use (or "auto" for smart routing)
    #[arg(short, long, default_value = "auto")]
    pub model: String,

    /// Approval mode
    #[arg(long, default_value = "ask")]
    pub mode: ApprovalMode,

    /// Plan mode — analyze and plan without executing changes
    #[arg(long)]
    pub plan: bool,

    /// Run in non-interactive CI mode
    #[arg(long)]
    pub headless: bool,

    /// Output format (text or json)
    #[arg(long, default_value = "text")]
    pub output_format: String,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Start API proxy server
    Serve {
        #[arg(short, long, default_value = "3777")]
        port: u16,
    },
    /// Manage config
    Config,
    /// Interactive provider & authentication setup
    Setup,
    /// Run system diagnostics and connectivity checks
    Doctor,
    /// Display usage analytics: tokens, cost, latency percentiles.
    Stats,
    /// Manage auth credentials
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    /// Start a chat session
    Chat,
    /// View or manage past chat sessions
    History {
        #[arg(short, long)]
        delete: Option<String>,
        #[arg(short, long)]
        resume: Option<String>,
    },
    /// Inspect or clear memory context
    Memory {
        #[command(subcommand)]
        action: MemoryAction,
    },
    /// Manage sessions
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum AuthAction {
    Status,
    Login,
    Logout,
    SetKey {
        provider: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum SessionAction {
    List,
    Resume { id: String },
    Delete { id: String },
}

#[derive(Subcommand, Debug, Clone)]
pub enum MemoryAction {
    /// Show current memory breakdown
    Inspect,
    /// Clear working memory
    Clear,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub enum ApprovalMode {
    Ask,
    Auto,
    Yolo,
    Readonly,
}
