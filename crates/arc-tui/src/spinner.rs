//! Professional terminal spinner with phase-aware status messages.
//!
//! Features:
//! - Multiple animation styles (braille, dots, arc, bouncing bar).
//! - Phase-cycling messages ("Thinking…", "Analyzing code…", etc.).
//! - Elapsed-time display.
//! - Async-compatible: wraps any `Future` transparently.
//! - Clean terminal restore on drop (cursor, alternate screen).

#![forbid(unsafe_code)]

use std::fmt;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::{
    cursor::{Hide, MoveToColumn, Show},
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use tokio::sync::watch;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Animation Frame Sets
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Available spinner animation styles.
#[derive(Debug, Clone, Copy, Default)]
pub enum SpinnerStyle {
    /// ⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏  — smooth braille rotation.
    #[default]
    Braille,
    /// ⣾ ⣽ ⣻ ⢿ ⡿ ⣟ ⣯ ⣷  — braille dots orbit.
    BrailleOrbit,
    /// ◐ ◓ ◑ ◒  — quarter-circle rotation.
    Circle,
    /// ▰▱▱▱▱ → ▰▰▱▱▱ → …  — bouncing progress bar.
    BouncingBar,
    /// ◇ ◈ ◆ ◈  — pulsing diamond.
    Diamond,
    /// ⠁ ⠂ ⠄ ⡀ ⢀ ⠠ ⠐ ⠈  — minimal dot.
    Minimal,
    /// ARC-branded: ⟳ with custom frames.
    Arc,
}

impl SpinnerStyle {
    /// Returns the animation frames for this style.
    fn frames(self) -> &'static [&'static str] {
        match self {
            Self::Braille => &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            Self::BrailleOrbit => &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"],
            Self::Circle => &["◐", "◓", "◑", "◒"],
            Self::BouncingBar => &[
                "▰▱▱▱▱▱▱",
                "▰▰▱▱▱▱▱",
                "▰▰▰▱▱▱▱",
                "▱▰▰▰▱▱▱",
                "▱▱▰▰▰▱▱",
                "▱▱▱▰▰▰▱",
                "▱▱▱▱▰▰▰",
                "▱▱▱▱▱▰▰",
                "▱▱▱▱▱▱▰",
                "▱▱▱▱▱▱▱",
                "▱▱▱▱▱▱▰",
                "▱▱▱▱▱▰▰",
                "▱▱▱▱▰▰▰",
                "▱▱▱▰▰▰▱",
                "▱▱▰▰▰▱▱",
                "▱▰▰▰▱▱▱",
            ],
            Self::Diamond => &["◇", "◈", "◆", "◈"],
            Self::Minimal => &["⠁", "⠂", "⠄", "⡀", "⢀", "⠠", "⠐", "⠈"],
            Self::Arc => &["⟨ ⟩", "⟨·⟩", "⟨∙⟩", "⟨●⟩", "⟨∙⟩", "⟨·⟩"],
        }
    }

    /// Milliseconds between frames.
    fn interval_ms(self) -> u64 {
        match self {
            Self::Braille | Self::BrailleOrbit | Self::Minimal => 80,
            Self::Circle | Self::Diamond => 120,
            Self::BouncingBar => 100,
            Self::Arc => 150,
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  LLM Task Phases
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Semantic phases an LLM task goes through.
///
/// The spinner automatically cycles through relevant phases based on
/// elapsed time, or you can set them manually via [`SpinnerHandle::set_phase`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Initial connection / prompt submission.
    Connecting,
    /// LLM is reasoning about the request.
    Thinking,
    /// Reading / analysing referenced files.
    Analyzing,
    /// Generating code or text.
    Generating,
    /// Writing changes to disk.
    Writing,
    /// Applying edits / formatting.
    Applying,
    /// Reviewing output for correctness.
    Reviewing,
    /// Custom message (for plugins / tools).
    Custom,
}

impl Phase {
    /// Human-readable label with an appropriate emoji.
    fn label(self) -> &'static str {
        match self {
            Self::Connecting => "Connecting",
            Self::Thinking => "Thinking",
            Self::Analyzing => "Analyzing code",
            Self::Generating => "Generating",
            Self::Writing => "Writing files",
            Self::Applying => "Applying changes",
            Self::Reviewing => "Reviewing",
            Self::Custom => "",
        }
    }

    /// Accent colour for this phase.
    fn color(self) -> Color {
        match self {
            Self::Connecting => Color::DarkCyan,
            Self::Thinking => Color::Magenta,
            Self::Analyzing => Color::Cyan,
            Self::Generating => Color::Green,
            Self::Writing => Color::Yellow,
            Self::Applying => Color::Blue,
            Self::Reviewing => Color::DarkGreen,
            Self::Custom => Color::White,
        }
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Default phase timeline: auto-transitions based on elapsed seconds.
const AUTO_PHASES: &[(u64, Phase)] = &[
    (0, Phase::Connecting),
    (1, Phase::Thinking),
    (4, Phase::Analyzing),
    (8, Phase::Generating),
    (20, Phase::Writing),
    (35, Phase::Reviewing),
];

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Spinner Handle (user-facing controller)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Handle returned by [`Spinner::start`].
///
/// Use this to update the spinner status from the calling task, then
/// call [`finish`](SpinnerHandle::finish) (or just drop it) to stop.
#[derive(Clone)]
pub struct SpinnerHandle {
    /// Channel to push status text updates.
    status_tx: watch::Sender<SpinnerState>,
    /// Signals the render loop to stop.
    stop: Arc<AtomicBool>,
    /// Join handle to await clean shutdown.
    join: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

#[derive(Clone)]
struct SpinnerState {
    phase: Phase,
    custom_message: Option<String>,
    detail: Option<String>,
}

impl SpinnerHandle {
    /// Update the current phase (overrides auto-progression).
    pub fn set_phase(&self, phase: Phase) {
        self.status_tx.send_modify(|s| {
            s.phase = phase;
            s.custom_message = None;
        });
    }

    /// Set a fully custom status message.
    pub fn set_message(&self, msg: impl Into<String>) {
        self.status_tx.send_modify(|s| {
            s.phase = Phase::Custom;
            s.custom_message = Some(msg.into());
        });
    }

    /// Set a secondary detail line (e.g., filename being edited).
    pub fn set_detail(&self, detail: impl Into<String>) {
        self.status_tx.send_modify(|s| {
            s.detail = Some(detail.into());
        });
    }

    /// Clear the detail line.
    pub fn clear_detail(&self) {
        self.status_tx.send_modify(|s| {
            s.detail = None;
        });
    }

    /// Stop the spinner with a success message.
    pub async fn finish(self, message: &str) {
        self.stop.store(true, Ordering::SeqCst);

        // Await the render task.
        if let Some(handle) = self.join.lock().await.take() {
            let _ = handle.await;
        }

        // Print final line.
        let mut stderr = io::stderr();
        let _ = execute!(
            stderr,
            MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(Color::Green),
            Print("  ✓ "),
            ResetColor,
            Print(message),
            Print("\n"),
            Show,
        );
    }

    /// Stop the spinner with a failure message.
    pub async fn fail(self, message: &str) {
        self.stop.store(true, Ordering::SeqCst);

        if let Some(handle) = self.join.lock().await.take() {
            let _ = handle.await;
        }

        let mut stderr = io::stderr();
        let _ = execute!(
            stderr,
            MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(Color::Red),
            Print("  ✗ "),
            ResetColor,
            Print(message),
            Print("\n"),
            Show,
        );
    }

    /// Stop the spinner silently (no final message).
    pub async fn stop(self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.join.lock().await.take() {
            let _ = handle.await;
        }
        let mut stderr = io::stderr();
        let _ = execute!(
            stderr,
            MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Show,
        );
    }
}

/// Ensure the spinner cleans up on drop (panic safety).
impl Drop for SpinnerHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Best-effort cursor restore on drop.
        let mut stderr = io::stderr();
        let _ = execute!(stderr, Show, MoveToColumn(0), Clear(ClearType::CurrentLine));
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Spinner (builder + render loop)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Configurable terminal spinner.
///
/// ```rust,no_run
/// use arc_tui::spinner::{Spinner, SpinnerStyle};
///
/// # async fn example() {
/// let handle = Spinner::new()
///     .style(SpinnerStyle::Braille)
///     .message("Thinking")
///     .start();
///
/// // … do async work …
///
/// handle.finish("Done!").await;
/// # }
/// ```
pub struct Spinner {
    style: SpinnerStyle,
    initial_message: Option<String>,
    auto_phase: bool,
    show_elapsed: bool,
}

impl Spinner {
    /// Create a new spinner with default settings.
    pub fn new() -> Self {
        Self {
            style: SpinnerStyle::default(),
            initial_message: None,
            auto_phase: true,
            show_elapsed: true,
        }
    }

    /// Set the animation style.
    pub fn style(mut self, style: SpinnerStyle) -> Self {
        self.style = style;
        self
    }

    /// Set an initial status message (overrides auto-phase for the first phase).
    pub fn message(mut self, msg: impl Into<String>) -> Self {
        self.initial_message = Some(msg.into());
        self
    }

    /// Disable automatic phase progression (you control phases via the handle).
    pub fn manual_phases(mut self) -> Self {
        self.auto_phase = false;
        self
    }

    /// Hide the elapsed-time counter.
    pub fn hide_elapsed(mut self) -> Self {
        self.show_elapsed = false;
        self
    }

    /// Start the spinner on a background tokio task.
    ///
    /// Returns a [`SpinnerHandle`] to control it.
    pub fn start(self) -> SpinnerHandle {
        let stop = Arc::new(AtomicBool::new(false));

        let initial_state = SpinnerState {
            phase: Phase::Connecting,
            custom_message: self.initial_message.clone(),
            detail: None,
        };

        let (status_tx, status_rx) = watch::channel(initial_state);

        let join = tokio::spawn(Self::render_loop(
            self.style,
            self.auto_phase,
            self.show_elapsed,
            stop.clone(),
            status_rx,
        ));

        SpinnerHandle {
            status_tx,
            stop,
            join: Arc::new(tokio::sync::Mutex::new(Some(join))),
        }
    }

    /// The main render loop running on a background task.
    async fn render_loop(
        style: SpinnerStyle,
        auto_phase: bool,
        show_elapsed: bool,
        stop: Arc<AtomicBool>,
        status_rx: watch::Receiver<SpinnerState>,
    ) {
        let frames = style.frames();
        let interval = Duration::from_millis(style.interval_ms());
        let start = Instant::now();
        let mut frame_idx: usize = 0;

        let mut stderr = io::stderr();

        // Hide cursor while spinning.
        let _ = execute!(stderr, Hide);

        while !stop.load(Ordering::SeqCst) {
            let elapsed = start.elapsed();
            let elapsed_secs = elapsed.as_secs();

            // Determine current state.
            let state = status_rx.borrow().clone();

            // Auto-phase progression.
            let (phase, message) = if state.phase == Phase::Custom || !auto_phase {
                (
                    state.phase,
                    state
                        .custom_message
                        .clone()
                        .unwrap_or_else(|| state.phase.label().to_owned()),
                )
            } else {
                let auto = AUTO_PHASES
                    .iter()
                    .rev()
                    .find(|(threshold, _)| elapsed_secs >= *threshold)
                    .map(|(_, p)| *p)
                    .unwrap_or(Phase::Thinking);

                // Use custom message if set for non-custom phase, else auto label.
                let msg = state
                    .custom_message
                    .clone()
                    .unwrap_or_else(|| auto.label().to_owned());
                (auto, msg)
            };

            let frame = frames[frame_idx % frames.len()];
            let color = phase.color();

            // Build the line.
            let elapsed_str = if show_elapsed {
                format_elapsed(elapsed)
            } else {
                String::new()
            };

            // Render.
            let _ = execute!(
                stderr,
                MoveToColumn(0),
                Clear(ClearType::CurrentLine),
                SetForegroundColor(color),
                Print("  "),
                Print(frame),
                Print(" "),
                SetAttribute(Attribute::Bold),
                Print(&message),
                SetAttribute(Attribute::Reset),
            );

            // Elapsed time (dimmed).
            if show_elapsed {
                let _ = execute!(
                    stderr,
                    SetForegroundColor(Color::DarkGrey),
                    Print("  "),
                    Print(&elapsed_str),
                    ResetColor,
                );
            }

            // Detail line (if any).
            if let Some(ref detail) = state.detail {
                let _ = execute!(
                    stderr,
                    SetForegroundColor(Color::DarkGrey),
                    Print("  → "),
                    SetAttribute(Attribute::Italic),
                    Print(detail),
                    SetAttribute(Attribute::Reset),
                    ResetColor,
                );
            }

            let _ = stderr.flush();

            frame_idx = frame_idx.wrapping_add(1);
            tokio::time::sleep(interval).await;
        }

        // Clean up line.
        let _ = execute!(
            stderr,
            MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Show,
        );
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Convenience Wrappers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Run an async task with an automatic spinner.
///
/// The spinner is shown while the future executes and automatically
/// finishes with ✓ or ✗ depending on the `Result`.
///
/// ```rust,no_run
/// use arc_tui::spinner::with_spinner;
///
/// # async fn example() -> color_eyre::Result<()> {
/// let result = with_spinner("Generating code", || async {
///     // your async LLM call here
///     Ok::<String, color_eyre::Report>("fn main() {}".to_owned())
/// })
/// .await?;
/// # Ok(())
/// # }
/// ```
pub async fn with_spinner<F, Fut, T, E>(message: &str, task: F) -> Result<T, E>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: fmt::Display,
{
    let handle = Spinner::new().message(message).start();

    let result = task().await;

    match &result {
        Ok(_) => {
            handle.finish(&format!("{message} — done")).await;
        }
        Err(e) => {
            handle
                .fail(&format!("{message} — failed: {e}"))
                .await;
        }
    }

    result
}

/// Run an async task with a spinner and full phase control.
///
/// Returns both the task result and the spinner handle for custom finish messages.
pub async fn with_spinner_handle<F, Fut, T>(
    message: &str,
    task: F,
) -> (SpinnerHandle, T)
where
    F: FnOnce(SpinnerHandle) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let handle = Spinner::new().message(message).start();
    let task_handle = handle.clone();
    let result = task(task_handle).await;
    (handle, result)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Helpers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Format a `Duration` as a human-friendly elapsed string.
///
/// - Under 60 s → `"12s"`
/// - Under 1 h → `"2m 34s"`
/// - Over 1 h → `"1h 05m"`
fn format_elapsed(d: Duration) -> String {
    let total_secs = d.as_secs();

    if total_secs < 60 {
        format!("{total_secs}s")
    } else if total_secs < 3600 {
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{mins}m {secs:02}s")
    } else {
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        format!("{hours}h {mins:02}m")
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Streaming-Aware Spinner
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// A spinner variant that transitions to a streaming indicator when
/// the first token arrives, then stops when streaming completes.
pub struct StreamingSpinner {
    inner: SpinnerHandle,
    token_count: Arc<std::sync::atomic::AtomicU64>,
    start: Instant,
}

impl StreamingSpinner {
    /// Start a new streaming-aware spinner.
    pub fn start() -> Self {
        let handle = Spinner::new()
            .style(SpinnerStyle::BrailleOrbit)
            .message("Thinking")
            .start();

        Self {
            inner: handle,
            token_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            start: Instant::now(),
        }
    }

    /// Call this each time a new token arrives from the LLM stream.
    ///
    /// On the first token, the spinner transitions from "Thinking" to
    /// "Streaming". Subsequent calls update the token counter.
    pub fn on_token(&self) {
        let count = self.token_count.fetch_add(1, Ordering::Relaxed) + 1;

        if count == 1 {
            self.inner.set_phase(Phase::Generating);
            self.inner.set_message("Streaming");
        }

        // Update detail with token count + throughput every 10 tokens.
        if count % 10 == 0 {
            let elapsed = self.start.elapsed().as_secs_f64();
            let tps = if elapsed > 0.0 {
                count as f64 / elapsed
            } else {
                0.0
            };
            self.inner.set_detail(format!(
                "{count} tokens  ({tps:.1} tok/s)"
            ));
        }
    }

    /// Call when the LLM stream is complete.
    pub async fn finish(self) {
        let count = self.token_count.load(Ordering::Relaxed);
        let elapsed = self.start.elapsed();
        let tps = if elapsed.as_secs_f64() > 0.0 {
            count as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        self.inner
            .finish(&format!(
                "Generated {count} tokens in {} ({tps:.1} tok/s)",
                format_elapsed(elapsed),
            ))
            .await;
    }

    /// Call when the LLM stream fails.
    pub async fn fail(self, error: &str) {
        self.inner.fail(error).await;
    }

    /// Access the underlying handle for custom phase control.
    pub fn handle(&self) -> &SpinnerHandle {
        &self.inner
    }
}
