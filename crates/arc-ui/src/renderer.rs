// SPDX-License-Identifier: MIT
use crate::state::{DiffBlock, DiffLine, Hunk, UiState};
use crate::theme::Theme;
use std::io::{self, Write};

pub fn render_diff_list(state: &UiState, theme: &Theme, out: &mut impl Write) -> io::Result<()> {
    let meaningful: Vec<(usize, &DiffBlock)> = state
        .diff_blocks
        .iter()
        .enumerate()
        .filter(|(_, d)| d.additions + d.deletions > 0)
        .collect();

    if meaningful.is_empty() {
        return Ok(());
    }

    let c = &theme.colors;
    let _b = &theme.border;

    writeln!(out)?;
    writeln!(
        out,
        "{}{}  Proposed Changes ({} files){}",
        c.bold,
        c.accent,
        meaningful.len(),
        c.reset
    )?;
    writeln!(out)?;

    for (idx, diff) in &meaningful {
        let is_selected = *idx == state.selected_diff;
        let cursor = if is_selected { "›" } else { " " };

        if diff.expanded {
            render_expanded_block(diff, is_selected, cursor, theme, out)?;
        } else {
            render_collapsed_block(diff, is_selected, cursor, theme, out)?;
        }
    }

    Ok(())
}

fn render_collapsed_block(
    diff: &DiffBlock,
    selected: bool,
    cursor: &str,
    theme: &Theme,
    out: &mut impl Write,
) -> io::Result<()> {
    let c = &theme.colors;
    let ic = &theme.icons;

    let status_badge = match diff.accepted {
        Some(true) => format!(" {}{} accepted{}", c.add, ic.check, c.reset),
        Some(false) => format!(" {}{} rejected{}", c.del, ic.cross, c.reset),
        None => String::new(),
    };

    let sel_color = if selected { c.accent } else { c.dim };

    writeln!(
        out,
        "{}{} {} {}{}{} {}(+{} -{}){} {}",
        sel_color,
        cursor,
        ic.collapsed,
        c.file_path,
        diff.file_path,
        c.reset,
        c.dim,
        diff.additions,
        diff.deletions,
        c.reset,
        status_badge,
    )?;

    Ok(())
}

fn render_expanded_block(
    diff: &DiffBlock,
    selected: bool,
    _cursor: &str,
    theme: &Theme,
    out: &mut impl Write,
) -> io::Result<()> {
    let c = &theme.colors;
    let ic = &theme.icons;
    let b = &theme.border;
    let w = 72usize; // inner box width

    let sel_color = if selected { c.accent } else { c.box_border };

    // File header
    let status_badge = match diff.accepted {
        Some(true) => format!(" {} accepted", ic.check),
        Some(false) => format!(" {} rejected", ic.cross),
        None => String::new(),
    };

    let title = format!(
        " {} {} (+{} -{}){}",
        ic.expanded, diff.file_path, diff.additions, diff.deletions, status_badge
    );

    // Top border
    writeln!(
        out,
        "  {}{}{}{}{}{}",
        sel_color,
        b.tl,
        b.h,
        title,
        b.h.repeat(w.saturating_sub(title.len() + 1).max(0)),
        b.tr
    )?;
    write!(out, "{}", c.reset)?;

    // Hunks
    for (hi, hunk) in diff.hunks.iter().enumerate() {
        // Hunk header
        writeln!(
            out,
            "  {}{} {}@@ -{},{} +{},{} @@{}",
            sel_color,
            b.v,
            c.dim,
            hunk.old_start,
            count_del(hunk),
            hunk.new_start,
            count_add(hunk),
            c.reset,
        )?;

        // Lines
        for line in &hunk.context {
            match line {
                DiffLine::Context(text) => {
                    writeln!(
                        out,
                        "  {}{}{}   {}{}",
                        sel_color,
                        b.v,
                        c.reset,
                        c.dim,
                        truncate(text, w - 6)
                    )?;
                },
                DiffLine::Del(text) => {
                    writeln!(
                        out,
                        "  {}{}{} {}{} {}{}",
                        sel_color,
                        b.v,
                        c.reset,
                        c.del,
                        ic.removed,
                        truncate(text, w - 6),
                        c.reset
                    )?;
                },
                DiffLine::Add(text) => {
                    writeln!(
                        out,
                        "  {}{}{} {}{} {}{}",
                        sel_color,
                        b.v,
                        c.reset,
                        c.add,
                        ic.added,
                        truncate(text, w - 6),
                        c.reset
                    )?;
                },
            }
        }

        // Hunk separator (if not last)
        if hi < diff.hunks.len() - 1 {
            writeln!(
                out,
                "  {}{}{}{}{}{}",
                sel_color,
                b.jl,
                b.h.repeat(w),
                b.jr,
                c.reset,
                ""
            )?;
        }
    }

    // Bottom border
    writeln!(
        out,
        "  {}{}{}{}{}",
        sel_color,
        b.bl,
        b.h.repeat(w + 1),
        b.br,
        c.reset
    )?;
    writeln!(out)?;

    Ok(())
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}

fn count_add(h: &Hunk) -> usize {
    h.context
        .iter()
        .filter(|l| matches!(l, DiffLine::Add(_)))
        .count()
}

fn count_del(h: &Hunk) -> usize {
    h.context
        .iter()
        .filter(|l| matches!(l, DiffLine::Del(_)))
        .count()
}
