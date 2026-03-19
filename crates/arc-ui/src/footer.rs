use crate::state::{Phase, UiState};
use crate::theme::Theme;
use std::io::{self, Write};

pub fn render_footer(state: &UiState, theme: &Theme, out: &mut impl Write) -> io::Result<()> {
    let c = &theme.colors;
    let w = state.terminal_width as usize;

    writeln!(out, "{}{}{}", c.dim, "─".repeat(w), c.reset)?;

    match state.phase {
        Phase::DiffReview => {
            let pending = state.diff_blocks.iter().filter(|d| d.accepted.is_none()).count();
            writeln!(out,
                " {}[Enter]{} Expand  {}[y]{} Accept  {}[n]{} Reject  {}[a]{} All  {}[d]{} Deny All  {}[e]{} Editor  {}│ {} pending{}",
                c.accent, c.reset,
                c.add, c.reset,
                c.del, c.reset,
                c.add, c.reset,
                c.del, c.reset,
                c.warn, c.reset,
                c.dim, pending, c.reset,
            )?;
        }
        Phase::Streaming => {
            writeln!(out,
                " {}[Ctrl+C]{} Halt stream  {}│  Streaming...{}",
                c.warn, c.reset, c.dim, c.reset,
            )?;
        }
        _ => {
            writeln!(out,
                " {}[/plan]{} Plan  {}[/doctor]{} Check  {}[/checkpoint]{} Save  {}[Ctrl+D]{} Exit{}",
                c.accent, c.reset,
                c.accent, c.reset,
                c.accent, c.reset,
                c.warn, c.reset,
                c.reset,
            )?;
        }
    }

    Ok(())
}
