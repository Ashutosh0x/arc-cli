use crate::state::{HeaderMode, Phase, UiState};
use crate::theme::Theme;
use std::io::{self, Write};

const LOGO_ART: &str = r#"
     █████╗ ██████╗  ██████╗
    ██╔══██╗██╔══██╗██╔════╝
    ███████║██████╔╝██║
    ██╔══██║██╔══██╗██║
    ██║  ██║██║  ██║╚██████╗
    ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝
"#;

pub fn render_header(state: &UiState, theme: &Theme, out: &mut impl Write) -> io::Result<()> {
    match state.header_mode {
        HeaderMode::Expanded => render_expanded(state, theme, out),
        HeaderMode::Compact  => render_compact(state, theme, out),
    }
}

fn render_expanded(state: &UiState, t: &Theme, out: &mut impl Write) -> io::Result<()> {
    let w = state.terminal_width as usize;
    let c = &t.colors;
    let b = &t.border;

    // Top border
    writeln!(out, "{}{}{}{}{}", c.box_border, b.tl, b.h.repeat(w.saturating_sub(2)), b.tr, c.reset)?;

    // Logo lines
    for line in LOGO_ART.trim().lines() {
        let padded = format!("{:^width$}", line, width = w.saturating_sub(4));
        writeln!(out, "{}{} {}{} {}{}", c.box_border, b.v, c.header, padded, c.box_border, b.v)?;
    }

    // Subtitle
    let sub = "Agentic CLI  •  Rust  •  Multi-Agent Orchestration";
    let sub_padded = format!("{:^width$}", sub, width = w.saturating_sub(4));
    writeln!(out, "{}{} {}{} {}{}",
        c.box_border, b.v, c.dim, sub_padded, c.box_border, b.v)?;

    // Version
    let ver = format!("v{}", env!("CARGO_PKG_VERSION"));
    let ver_padded = format!("{:^width$}", ver, width = w.saturating_sub(4));
    writeln!(out, "{}{} {}{} {}{}",
        c.box_border, b.v, c.dim, ver_padded, c.box_border, b.v)?;

    // Bottom border
    writeln!(out, "{}{}{}{}{}", c.box_border, b.bl, b.h.repeat(w.saturating_sub(2)), b.br, c.reset)?;

    writeln!(out)?;
    Ok(())
}

fn render_compact(state: &UiState, t: &Theme, out: &mut impl Write) -> io::Result<()> {
    let c = &t.colors;
    let ic = &t.icons;
    let m = &state.metrics;

    let phase_indicator = match state.phase {
        Phase::Idle       => format!("{}{} READY{}",    c.status_ok,  ic.dot_ok,  c.reset),
        Phase::Planning   => format!("{}{} PLANNING{}", c.status_run, ic.dot_run, c.reset),
        Phase::Streaming  => format!("{}{} STREAMING{}",c.status_run, ic.dot_run, c.reset),
        Phase::DiffReview => format!("{}{} REVIEW{}",   c.warn,       ic.dot_run, c.reset),
        Phase::Executing  => format!("{}{} EXECUTING{}",c.status_run, ic.dot_run, c.reset),
        Phase::Done       => format!("{}{} DONE{}",     c.status_ok,  ic.check,   c.reset),
        Phase::Startup    => format!("{}{} INIT{}",     c.dim,        ic.dot_run, c.reset),
    };

    let agents_str = if m.active_agents.is_empty() {
        String::new()
    } else {
        format!(" {}[{}]{}", c.dim, m.active_agents.join(" | "), c.reset)
    };

    let cost_str = if m.total_cost > 0.0 {
        format!(" {}[{}k tok | ${:.4}]{}",
            c.dim,
            (m.input_tokens + m.output_tokens) / 1000,
            m.total_cost,
            c.reset
        )
    } else {
        String::new()
    };

    writeln!(out, "{}ARC CLI{} {}{}{}",
        c.header, c.reset,
        phase_indicator,
        agents_str,
        cost_str
    )?;

    // Thin separator
    let w = state.terminal_width as usize;
    writeln!(out, "{}{}{}", c.dim, "─".repeat(w), c.reset)?;

    Ok(())
}
