use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "arc", about = "⚡ ARC — Agent for Rapid Coding", version)]
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

    /// Boot natively into PTT Voice mode (`arc-voice`)
    #[arg(long)]
    pub voice: bool,

    /// Pass an image path natively to a multi-modal agent (`arc-vision`)
    #[arg(long)]
    pub vision: Option<String>,

    /// Disable ANSI color output
    #[arg(long, env = "NO_COLOR")]
    pub no_color: bool,

    /// Quiet mode (errors only)
    #[arg(short, long)]
    pub quiet: bool,

    /// Verbose trace mode
    #[arg(short, long)]
    pub verbose: bool,

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
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Interactive provider & authentication setup
    Setup,
    /// Initialize workspace environment, ARC.md, and hooks.toml
    Init,
    /// Generate shell completion scripts (bash, zsh, fish)
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Autonomous diagnostic feedback loop that runs tests, parses errors, and writes fixes
    Fix {
        #[arg(short, long, default_value = "3")]
        max_iterations: u32,
    },
    /// Run system diagnostics and connectivity checks
    Doctor,
    /// Generate a diagnostic bundle for bug reports
    Diagnostic,
    /// Update ARC CLI to the latest GitHub release version
    Update,
    /// Automated PR Review of local changes against origin/main via LLM Swarm
    Review {
        /// Base branch to diff against (default: origin/main)
        #[arg(short, long, default_value = "origin/main")]
        base: String,
    },
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
    /// Structural code intelligence via knowledge graph (codebase-memory-mcp)
    Graph {
        #[command(subcommand)]
        action: GraphAction,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum GraphAction {
    /// Index the current project into the knowledge graph
    Index {
        /// Path to the repository (defaults to current directory)
        path: Option<String>,
    },
    /// Structural search by name pattern (functions, classes, types)
    Search {
        /// Name pattern to search for (regex supported)
        pattern: String,
        /// Filter by label: Function, Class, Method, Interface, etc.
        #[arg(short, long)]
        label: Option<String>,
        /// Filter by file path pattern
        #[arg(short, long)]
        file_pattern: Option<String>,
    },
    /// Trace call paths — who calls a function and what it calls
    Trace {
        /// Function name to trace
        function: String,
        /// Direction: inbound, outbound, or both
        #[arg(short, long, default_value = "both")]
        direction: String,
        /// Traversal depth (1-5)
        #[arg(long, default_value = "3")]
        depth: u32,
    },
    /// Get full architecture overview (languages, packages, routes, hotspots)
    Architecture,
    /// Map uncommitted git changes to affected symbols with risk scores
    Impact {
        /// Path to the repository (defaults to current directory)
        path: Option<String>,
    },
    /// Execute a Cypher-like graph query
    Query {
        /// Cypher query string
        cypher: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum AuthAction {
    Status,
    Login,
    Logout,
    SetKey { provider: String },
}

#[derive(Subcommand, Debug, Clone)]
pub enum ConfigAction {
    /// Open config file in default editor
    Edit,
    /// Validate current config structure
    Validate,
    /// Reset to defaults (backs up current)
    Reset,
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
