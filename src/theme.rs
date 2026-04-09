// ARC CLI Theme — exact RGB colors matching the reference screenshots
use ratatui::style::{Color, Modifier, Style};

/// Premium dark theme matching the ARC CLI terminal screenshots.
pub struct Theme;

impl Theme {
    // ── Background ──────────────────────────────────────────────
    pub const BG: Color = Color::Rgb(10, 14, 20);
    pub const PANEL: Color = Color::Rgb(18, 22, 30);
    pub const PANEL_BORDER: Color = Color::Rgb(50, 55, 70);
    pub const HIGHLIGHT_BG: Color = Color::Rgb(30, 35, 50);

    // ── Primary palette ─────────────────────────────────────────
    pub const PRIMARY: Color = Color::Rgb(0, 200, 255);   // Cyan / brand
    pub const ACCENT: Color = Color::Rgb(130, 170, 255);  // Soft blue accent

    // ── Semantic ────────────────────────────────────────────────
    pub const SUCCESS: Color = Color::Rgb(80, 200, 120);   // Green
    pub const WARNING: Color = Color::Rgb(240, 200, 80);   // Yellow
    pub const ERROR: Color = Color::Rgb(220, 80, 80);      // Red
    pub const INFO: Color = Color::Rgb(100, 160, 255);     // Blue info

    // ── Text ────────────────────────────────────────────────────
    pub const TEXT: Color = Color::Rgb(200, 200, 200);
    pub const TEXT_BRIGHT: Color = Color::Rgb(240, 240, 240);
    pub const MUTED: Color = Color::Rgb(100, 105, 115);
    pub const DIM: Color = Color::Rgb(60, 65, 75);

    // ── Diff ────────────────────────────────────────────────────
    pub const DIFF_ADD_BG: Color = Color::Rgb(20, 80, 20);
    pub const DIFF_ADD_FG: Color = Color::Rgb(180, 255, 180);
    pub const DIFF_DEL_BG: Color = Color::Rgb(80, 20, 20);
    pub const DIFF_DEL_FG: Color = Color::Rgb(255, 180, 180);
    pub const DIFF_HATCH_BG: Color = Color::Rgb(35, 35, 40);

    // ── Syntax highlighting (Rust-like) ─────────────────────────
    pub const SYN_KEYWORD: Color = Color::Rgb(198, 120, 221); // purple
    pub const SYN_TYPE: Color = Color::Rgb(229, 192, 123);    // gold
    pub const SYN_FUNCTION: Color = Color::Rgb(97, 175, 239); // blue
    pub const SYN_STRING: Color = Color::Rgb(152, 195, 121);  // green
    pub const SYN_FIELD: Color = Color::Rgb(224, 108, 117);   // red
    pub const SYN_COMMENT: Color = Color::Rgb(92, 99, 112);   // gray

    // ── Traffic light colors (macOS window chrome) ──────────────
    pub const LIGHT_CLOSE: Color = Color::Rgb(255, 96, 92);
    pub const LIGHT_MINIMIZE: Color = Color::Rgb(255, 189, 46);
    pub const LIGHT_MAXIMIZE: Color = Color::Rgb(39, 201, 63);

    // ── Composed styles ─────────────────────────────────────────
    pub fn base() -> Style {
        Style::default().bg(Self::BG).fg(Self::TEXT)
    }

    pub fn title() -> Style {
        Style::default()
            .fg(Self::PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    pub fn subtitle() -> Style {
        Style::default().fg(Self::MUTED)
    }

    pub fn success() -> Style {
        Style::default()
            .fg(Self::SUCCESS)
            .add_modifier(Modifier::BOLD)
    }

    pub fn warning() -> Style {
        Style::default()
            .fg(Self::WARNING)
            .add_modifier(Modifier::BOLD)
    }

    pub fn error() -> Style {
        Style::default().fg(Self::ERROR)
    }

    pub fn pending() -> Style {
        Style::default().fg(Self::DIM)
    }

    pub fn selected_row() -> Style {
        Style::default()
            .bg(Color::Rgb(30, 35, 55))
            .fg(Self::PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    pub fn unselected_row() -> Style {
        Style::default().fg(Self::TEXT)
    }

    pub fn border() -> Style {
        Style::default().fg(Self::PANEL_BORDER)
    }

    pub fn panel_block() -> Style {
        Style::default().fg(Self::PANEL_BORDER).bg(Self::PANEL)
    }

    pub fn diff_added() -> Style {
        Style::default()
            .fg(Self::DIFF_ADD_FG)
            .bg(Self::DIFF_ADD_BG)
    }

    pub fn diff_removed() -> Style {
        Style::default()
            .fg(Self::DIFF_DEL_FG)
            .bg(Self::DIFF_DEL_BG)
    }

    pub fn diff_hatch() -> Style {
        Style::default()
            .fg(Self::DIM)
            .bg(Self::DIFF_HATCH_BG)
    }

    pub fn key_hint() -> Style {
        Style::default()
            .fg(Self::MUTED)
    }

    pub fn key_label() -> Style {
        Style::default()
            .fg(Self::TEXT_BRIGHT)
            .add_modifier(Modifier::BOLD)
    }

    pub fn stat_value() -> Style {
        Style::default().fg(Self::PRIMARY)
    }
}
