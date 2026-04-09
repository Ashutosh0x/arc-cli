// ARC CLI — UI rendering (ratatui)
// 4 screens: Prompt → AgentView → DiffView → Output
// Real data everywhere — no mocks.
//
// NO EMOJIS — pure ASCII only for max terminal compatibility.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, PromptMode, Screen, MODELS};
use crate::models::{AgentKind, DiffLine, LogLevel, TaskStatus};
use crate::theme::Theme;

// =====================================================================
//  Top-level draw
// =====================================================================

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Fill entire background with dark theme
    frame.render_widget(Clear, area);
    let bg = Block::default().style(Theme::base());
    frame.render_widget(bg, area);

    // Main layout: tab bar + body + footer
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // tab bar
            Constraint::Min(1),   // body
            Constraint::Length(3), // footer
        ])
        .split(area);

    draw_tab_bar(frame, app, outer[0]);

    match app.screen {
        Screen::Prompt => draw_prompt_screen(frame, app, outer[1]),
        Screen::AgentView => draw_agent_screen(frame, app, outer[1]),
        Screen::DiffView => draw_diff_screen(frame, app, outer[1]),
        Screen::Output => draw_output_screen(frame, app, outer[1]),
    }

    draw_footer(frame, app, outer[2]);
}

// =====================================================================
//  Tab bar — shows all 4 screens with active highlight
// =====================================================================

fn draw_tab_bar(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![Span::styled(" ", Style::default().bg(Theme::PANEL))];

    for (i, screen) in Screen::ALL.iter().enumerate() {
        let is_active = *screen == app.screen;

        if i > 0 {
            spans.push(Span::styled(
                " | ",
                Style::default().fg(Theme::DIM).bg(Theme::PANEL),
            ));
        }

        let label = format!(" {} ", screen.label());

        // Add status indicators
        let indicator = match screen {
            Screen::AgentView if app.pipeline_running => " *",
            Screen::AgentView if app.pipeline_failed => " X",
            Screen::AgentView if app.pipeline_complete => " +",
            Screen::DiffView if app.current_diff.is_some() => " +",
            Screen::Output if app.streaming => " *",
            _ => "",
        };

        let full_label = format!("{}{}", label, indicator);

        if is_active {
            spans.push(Span::styled(
                full_label,
                Style::default()
                    .fg(Theme::BG)
                    .bg(Theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                full_label,
                Style::default().fg(Theme::TEXT).bg(Theme::PANEL),
            ));
        }
    }

    // Right-align uptime
    let uptime = format!(
        " {}s ",
        app.uptime_secs()
    );
    // Fill remaining space
    let used: usize = spans.iter().map(|s| s.content.len()).sum();
    let remaining = (area.width as usize).saturating_sub(used + uptime.len());
    spans.push(Span::styled(
        " ".repeat(remaining),
        Style::default().bg(Theme::PANEL),
    ));
    spans.push(Span::styled(
        uptime,
        Style::default().fg(Theme::MUTED).bg(Theme::PANEL),
    ));

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}

// =====================================================================
//  SCREEN 1 — Prompt entry
// =====================================================================

fn draw_prompt_screen(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // ASCII header
            Constraint::Length(2),  // shell prompt
            Constraint::Length(7),  // prompt box
            Constraint::Min(7),    // model selector
        ])
        .split(area);

    draw_ascii_header(frame, chunks[0]);
    draw_shell_prompt(frame, "arc /prompt", chunks[1]);
    draw_prompt_box(frame, app, chunks[2]);
    draw_model_selector(frame, app, chunks[3]);
}

// =====================================================================
//  SCREEN 2 — Agent orchestration (REAL logs + tasks)
// =====================================================================

fn draw_agent_screen(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // shell prompt
            Constraint::Length(5),  // task pipeline status
            Constraint::Min(5),    // agent log viewer
        ])
        .split(area);

    let cmd = if app.pipeline_running {
        "arc /agents --pipeline RUNNING"
    } else if app.pipeline_failed {
        "arc /agents --pipeline FAILED"
    } else if app.pipeline_complete {
        "arc /agents --pipeline COMPLETE"
    } else {
        "arc /agents --waiting"
    };
    draw_shell_prompt(frame, cmd, chunks[0]);

    // ── Task pipeline status ──
    draw_task_pipeline(frame, app, chunks[1]);

    // ── Agent log viewer ──
    draw_agent_logs(frame, app, chunks[2]);
}

fn draw_task_pipeline(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" Pipeline Status ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(if app.pipeline_running {
            Style::default().fg(Theme::WARNING)
        } else if app.pipeline_failed {
            Style::default().fg(Theme::ERROR)
        } else if app.pipeline_complete {
            Style::default().fg(Theme::SUCCESS)
        } else {
            Theme::border()
        })
        .style(Style::default().bg(Theme::PANEL));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.tasks.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "  No tasks dispatched yet. Submit a prompt to start the pipeline.",
            Style::default().fg(Theme::MUTED),
        )));
        frame.render_widget(msg, inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    for task in &app.tasks {
        let (icon, style) = match &task.status {
            TaskStatus::Pending => ("[..]", Style::default().fg(Theme::DIM)),
            TaskStatus::InProgress => {
                let spinner = match (app.tick / 5) % 4 {
                    0 => "[|>]",
                    1 => "[/>]",
                    2 => "[-/]",
                    _ => "[\\>]",
                };
                (spinner, Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD))
            }
            TaskStatus::Completed => ("[OK]", Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)),
            TaskStatus::Failed(_) => ("[XX]", Style::default().fg(Theme::ERROR).add_modifier(Modifier::BOLD)),
        };

        let elapsed = task
            .elapsed_ms()
            .map(|ms| format!(" ({}ms)", ms))
            .unwrap_or_default();

        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", icon), style),
            Span::styled(
                format!("{:<12}", task.agent.to_string()),
                Style::default().fg(Theme::ACCENT),
            ),
            Span::styled(&task.description, Style::default().fg(Theme::TEXT)),
            Span::styled(elapsed, Style::default().fg(Theme::MUTED)),
        ]));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn draw_agent_logs(frame: &mut Frame, app: &App, area: Rect) {
    let log_count = app.agent_logs.len();
    let block = Block::default()
        .title(Span::styled(
            format!(" Agent Logs ({}) ", log_count),
            Theme::title(),
        ))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .style(Style::default().bg(Theme::PANEL));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.agent_logs.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "  Waiting for agent activity...",
            Style::default().fg(Theme::MUTED),
        )));
        frame.render_widget(msg, inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    for log in &app.agent_logs {
        let time = log.timestamp.format("%H:%M:%S%.3f");
        let level_style = match log.level {
            LogLevel::Debug => Style::default().fg(Theme::DIM),
            LogLevel::Info => Style::default().fg(Theme::INFO),
            LogLevel::Warn => Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD),
            LogLevel::Error => Style::default().fg(Theme::ERROR).add_modifier(Modifier::BOLD),
        };

        let agent_color = match log.agent {
            AgentKind::RepoMap => Theme::SUCCESS,
            AgentKind::Architect => Theme::ACCENT,
            AgentKind::Coder => Theme::WARNING,
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", time),
                Style::default().fg(Theme::DIM),
            ),
            Span::styled(
                format!("[{:<5}] ", log.level),
                level_style,
            ),
            Span::styled(
                format!("{:<12} ", log.agent),
                Style::default().fg(agent_color),
            ),
            Span::styled(&log.message, Style::default().fg(Theme::TEXT)),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .scroll((app.agent_log_scroll, 0));
    frame.render_widget(paragraph, inner);
}

// =====================================================================
//  SCREEN 3 — Diff view (REAL diff data)
// =====================================================================

fn draw_diff_screen(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // shell prompt
            Constraint::Length(3),  // diff stats
            Constraint::Min(5),    // diff content
        ])
        .split(area);

    draw_shell_prompt(frame, "arc /diff --unified", chunks[0]);

    if let Some(ref diff) = app.current_diff {
        // Stats bar
        let stats_line = Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                format!("{} ", diff.file_path),
                Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("+{} ", diff.additions),
                Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("-{} ", diff.deletions),
                Style::default().fg(Theme::ERROR).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("({} lines total)", diff.lines.len()),
                Style::default().fg(Theme::MUTED),
            ),
        ]);
        let stats = Paragraph::new(vec![Line::default(), stats_line])
            .style(Theme::base());
        frame.render_widget(stats, chunks[1]);

        // Diff content
        let block = Block::default()
            .title(Span::styled(" Unified Diff ", Theme::title()))
            .borders(Borders::ALL)
            .border_style(Theme::border())
            .style(Style::default().bg(Theme::PANEL));

        let inner = block.inner(chunks[2]);
        frame.render_widget(block, chunks[2]);

        let mut lines: Vec<Line> = Vec::new();
        for diff_line in &diff.lines {
            match diff_line {
                DiffLine::Added(content) => {
                    lines.push(Line::from(Span::styled(
                        format!("+{}", content.trim_end_matches('\n')),
                        Theme::diff_added(),
                    )));
                }
                DiffLine::Removed(content) => {
                    lines.push(Line::from(Span::styled(
                        format!("-{}", content.trim_end_matches('\n')),
                        Theme::diff_removed(),
                    )));
                }
                DiffLine::Unchanged(content) => {
                    lines.push(Line::from(Span::styled(
                        format!(" {}", content.trim_end_matches('\n')),
                        Style::default().fg(Theme::TEXT),
                    )));
                }
            }
        }

        let paragraph = Paragraph::new(lines)
            .scroll((app.diff_scroll, 0))
            .wrap(ratatui::widgets::Wrap { trim: false });
        frame.render_widget(paragraph, inner);
    } else {
        // No diff available
        let empty_block = Block::default()
            .title(Span::styled(" Diff View ", Theme::title()))
            .borders(Borders::ALL)
            .border_style(Theme::border())
            .style(Style::default().bg(Theme::PANEL));

        let inner = empty_block.inner(chunks[2]);
        frame.render_widget(empty_block, chunks[2]);

        let msg = Paragraph::new(vec![
            Line::default(),
            Line::from(Span::styled(
                "  No diff available yet.",
                Style::default().fg(Theme::MUTED),
            )),
            Line::default(),
            Line::from(Span::styled(
                "  Submit a prompt and the Coder agent will produce real diffs here.",
                Style::default().fg(Theme::DIM),
            )),
        ]);
        let stats = Paragraph::new(Line::default()).style(Theme::base());
        frame.render_widget(stats, chunks[1]);
        frame.render_widget(msg, inner);
    }
}

// =====================================================================
//  SCREEN 4 — Agent output (streaming response)
// =====================================================================

fn draw_output_screen(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // shell prompt
            Constraint::Length(3),  // status line
            Constraint::Min(5),    // response content
        ])
        .split(area);

    // Show the user's actual prompt in the shell line
    let cmd = if app.prompt_text.is_empty() {
        "arc /output".to_string()
    } else {
        format!("arc /output --prompt \"{}\"", &app.prompt_text)
    };
    draw_shell_prompt(frame, &cmd, chunks[0]);

    // Status indicator with real stats
    let model = &MODELS[app.selected_model];
    let mode_label = app.mode.label();
    let status_line = if app.streaming {
        Line::from(vec![
            Span::styled(
                format!(" [{}] ", mode_label),
                Style::default()
                    .fg(Theme::BG)
                    .bg(match app.mode {
                        PromptMode::Chat => Theme::PRIMARY,
                        PromptMode::FastCode => Theme::SUCCESS,
                        PromptMode::Agent => Theme::WARNING,
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " [STREAMING] ",
                Style::default()
                    .fg(Theme::BG)
                    .bg(Theme::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" Model: {} ", model.name),
                Style::default().fg(Theme::PRIMARY),
            ),
            Span::styled(
                " -- receiving tokens... ",
                Style::default().fg(Theme::WARNING),
            ),
        ])
    } else if app.response_text.is_empty() {
        Line::from(Span::styled(
            " Waiting for prompt...",
            Style::default().fg(Theme::MUTED),
        ))
    } else {
        let latency = if !app.llm_usage.is_empty() {
            format!("  |  Latency: {}ms", app.total_latency_ms())
        } else {
            "  |  Latency: --".to_string()
        };
        let tokens = if !app.llm_usage.is_empty() {
            format!("  |  Tokens: {}", app.total_tokens())
        } else {
            "  |  Tokens: --".to_string()
        };
        Line::from(vec![
            Span::styled(
                " [COMPLETE] ",
                Style::default()
                    .fg(Theme::BG)
                    .bg(Theme::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" Model: {} ", model.name),
                Style::default().fg(Theme::PRIMARY),
            ),
            Span::styled(
                format!(" -- {} chars{}{}", app.response_text.len(), latency, tokens),
                Style::default().fg(Theme::MUTED),
            ),
        ])
    };
    let status_para = Paragraph::new(vec![Line::default(), status_line])
        .style(Theme::base());
    frame.render_widget(status_para, chunks[1]);

    // Response content box
    let response_block = Block::default()
        .title(Span::styled(
            format!(" Agent Response ({}) ", model.name),
            Theme::title(),
        ))
        .borders(Borders::ALL)
        .border_style(if app.streaming {
            Style::default().fg(Theme::SUCCESS)
        } else {
            Theme::border()
        })
        .style(Style::default().bg(Theme::PANEL));

    let display_text = if app.response_text.is_empty() && !app.streaming {
        "Press Enter/i on the Prompt screen to type a prompt, then Enter to submit.\n\nThe response from the agent pipeline will stream here in real-time.".to_string()
    } else if app.response_text.is_empty() && app.streaming {
        "Connecting to LLM provider...".to_string()
    } else {
        app.response_text.clone()
    };

    let paragraph = Paragraph::new(display_text)
        .block(response_block)
        .wrap(ratatui::widgets::Wrap { trim: false })
        .scroll((app.scroll_offset, 0))
        .style(Style::default().fg(Theme::TEXT_BRIGHT));

    frame.render_widget(paragraph, chunks[2]);
}

// =====================================================================
//  ASCII header (ARC CLI logo)
// =====================================================================

fn draw_ascii_header(frame: &mut Frame, area: Rect) {
    let bold_cyan = Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD);
    let accent = Style::default().fg(Theme::ACCENT);

    let logo_lines = vec![
        Line::from(Span::styled(
            "    \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557} \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557} \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557}     \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2557}     \u{2588}\u{2588}\u{2557}",
            bold_cyan,
        )),
        Line::from(Span::styled(
            "    \u{2588}\u{2588}\u{2554}\u{2550}\u{2550}\u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2554}\u{2550}\u{2550}\u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{255D}    \u{2588}\u{2588}\u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{255D}\u{2588}\u{2588}\u{2551}     \u{2588}\u{2588}\u{2551}",
            bold_cyan,
        )),
        Line::from(Span::styled(
            "    \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2554}\u{255D}\u{2588}\u{2588}\u{2551}         \u{2588}\u{2588}\u{2551}     \u{2588}\u{2588}\u{2551}     \u{2588}\u{2588}\u{2551}",
            accent,
        )),
        Line::from(Span::styled(
            "    \u{2588}\u{2588}\u{2554}\u{2550}\u{2550}\u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2554}\u{2550}\u{2550}\u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2551}         \u{2588}\u{2588}\u{2551}     \u{2588}\u{2588}\u{2551}     \u{2588}\u{2588}\u{2551}",
            accent,
        )),
        Line::from(Span::styled(
            "    \u{2588}\u{2588}\u{2551}  \u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2551}  \u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557}    \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2551}",
            bold_cyan,
        )),
        Line::from(Span::styled(
            "    \u{255A}\u{2550}\u{255D}  \u{255A}\u{2550}\u{255D}\u{255A}\u{2550}\u{255D}  \u{255A}\u{2550}\u{255D} \u{255A}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255D}     \u{255A}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255D}\u{255A}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255D}\u{255A}\u{2550}\u{255D}",
            Style::default().fg(Theme::DIM),
        )),
        Line::default(),
        Line::from(Span::styled(
            "        Agentic ARC CLI  (v2.0)",
            Theme::title(),
        )),
        Line::from(Span::styled(
            "   Powered by Rust * Multi-Agent System",
            Theme::subtitle(),
        )),
    ];

    let header = Paragraph::new(logo_lines).style(Theme::base());
    frame.render_widget(header, area);
}

// =====================================================================
//  Shell prompt line
// =====================================================================

fn draw_shell_prompt(frame: &mut Frame, command: &str, area: Rect) {
    let line = Line::from(vec![
        Span::styled(
            "~/projects/my-rust-app ",
            Style::default().fg(Theme::PRIMARY),
        ),
        Span::styled(
            "(main) ",
            Style::default().fg(Theme::ERROR),
        ),
        Span::styled(
            "$ ",
            Style::default()
                .fg(Theme::SUCCESS)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            command,
            Style::default().fg(Theme::TEXT_BRIGHT),
        ),
    ]);

    let prompt = Paragraph::new(vec![Line::default(), line]).style(Theme::base());
    frame.render_widget(prompt, area);
}

// =====================================================================
//  Prompt input box
// =====================================================================

fn draw_prompt_box(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.editing {
        Style::default().fg(Theme::PRIMARY)
    } else {
        Theme::border()
    };

    let block = Block::default()
        .title(Span::styled(
            " Enter Detailed Agent Prompt Below ",
            Theme::title(),
        ))
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(Theme::PANEL));

    let display_text = if app.prompt_text.is_empty() && !app.editing {
        Line::from(Span::styled(
            "Describe what you want the agent to build...",
            Style::default().fg(Theme::MUTED),
        ))
    } else {
        let mut text = app.prompt_text.clone();
        if app.editing {
            text.push('_');  // visible cursor
        }
        Line::from(Span::styled(text, Style::default().fg(Theme::TEXT_BRIGHT)))
    };

    let paragraph = Paragraph::new(display_text).block(block);
    frame.render_widget(paragraph, area);
}

// =====================================================================
//  Model selector (with health indicators)
// =====================================================================

fn draw_model_selector(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " Select LLM Provider ",
            Theme::title(),
        ))
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .style(Style::default().bg(Theme::PANEL));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    for (i, model) in MODELS.iter().enumerate() {
        let selected = i == app.selected_model;
        let radio = if selected { "(*)" } else { "( )" };

        let row_style = if selected {
            Theme::selected_row()
        } else {
            Theme::unselected_row()
        };

        let tag_style = match model.tag {
            "Local"   => Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD),
            "Fast"    => Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD),
            "OSS"     => Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD),
            "Premium" => Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD),
            _         => Style::default().fg(Theme::TEXT),
        };

        // Health indicator for known providers
        let health = if model.tag == "Local" {
            match app.ollama_healthy {
                Some(true) => Span::styled(" [UP]", Style::default().fg(Theme::SUCCESS)),
                Some(false) => Span::styled(" [DOWN]", Style::default().fg(Theme::ERROR)),
                None => Span::styled(" [?]", Style::default().fg(Theme::DIM)),
            }
        } else if model.tag == "Premium" || model.tag == "Fast" {
            match app.openai_healthy {
                Some(true) => Span::styled(" [UP]", Style::default().fg(Theme::SUCCESS)),
                Some(false) => Span::styled(" [NO KEY]", Style::default().fg(Theme::ERROR)),
                None => Span::styled(" [?]", Style::default().fg(Theme::DIM)),
            }
        } else {
            Span::styled("", Style::default())
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", radio), row_style),
            Span::styled(format!("{:<22}", model.name), row_style),
            Span::styled(format!("[{}]", model.tag), tag_style),
            Span::styled(format!("  {}", model.provider), Style::default().fg(Theme::MUTED)),
            health,
        ]));
    }

    let list = Paragraph::new(lines);
    frame.render_widget(list, inner);
}

// =====================================================================
//  Footer (keyboard shortcuts + live stats)
// =====================================================================

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Theme::DIM))
        .style(Style::default().bg(Theme::PANEL));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    // Row 1: keybindings (context-aware)
    let keys = match app.screen {
        Screen::Prompt if app.editing => {
            Line::from(vec![
                Span::styled(" EDITING ", Style::default().fg(Theme::BG).bg(Theme::PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled("  [", Theme::key_hint()),
                Span::styled("Enter", Theme::key_label()),
                Span::styled("] Submit  ", Theme::key_hint()),
                Span::styled("[", Theme::key_hint()),
                Span::styled("Esc", Theme::key_label()),
                Span::styled("] Stop  ", Theme::key_hint()),
                Span::styled("[", Theme::key_hint()),
                Span::styled("Bksp", Theme::key_label()),
                Span::styled("] Delete", Theme::key_hint()),
            ])
        }
        Screen::Prompt => {
            Line::from(vec![
                Span::styled(" [", Theme::key_hint()),
                Span::styled("Enter/i", Theme::key_label()),
                Span::styled("] Edit  ", Theme::key_hint()),
                Span::styled("[", Theme::key_hint()),
                Span::styled("j/k", Theme::key_label()),
                Span::styled("] Model  ", Theme::key_hint()),
                Span::styled("[", Theme::key_hint()),
                Span::styled("Tab", Theme::key_label()),
                Span::styled("] Next Screen  ", Theme::key_hint()),
                Span::styled("[", Theme::key_hint()),
                Span::styled("q", Theme::key_label()),
                Span::styled("] Quit", Theme::key_hint()),
            ])
        }
        Screen::AgentView => {
            Line::from(vec![
                Span::styled(" [", Theme::key_hint()),
                Span::styled("j/k", Theme::key_label()),
                Span::styled("] Scroll Logs  ", Theme::key_hint()),
                Span::styled("[", Theme::key_hint()),
                Span::styled("Tab", Theme::key_label()),
                Span::styled("] Next Screen  ", Theme::key_hint()),
                Span::styled("[", Theme::key_hint()),
                Span::styled("Shift+Tab", Theme::key_label()),
                Span::styled("] Prev  ", Theme::key_hint()),
                Span::styled("[", Theme::key_hint()),
                Span::styled("q", Theme::key_label()),
                Span::styled("] Quit", Theme::key_hint()),
            ])
        }
        Screen::DiffView => {
            Line::from(vec![
                Span::styled(" [", Theme::key_hint()),
                Span::styled("j/k", Theme::key_label()),
                Span::styled("] Scroll Diff  ", Theme::key_hint()),
                Span::styled("[", Theme::key_hint()),
                Span::styled("Tab", Theme::key_label()),
                Span::styled("] Next Screen  ", Theme::key_hint()),
                Span::styled("[", Theme::key_hint()),
                Span::styled("Shift+Tab", Theme::key_label()),
                Span::styled("] Prev  ", Theme::key_hint()),
                Span::styled("[", Theme::key_hint()),
                Span::styled("q", Theme::key_label()),
                Span::styled("] Quit", Theme::key_hint()),
            ])
        }
        Screen::Output => {
            Line::from(vec![
                Span::styled(" [", Theme::key_hint()),
                Span::styled("j/k", Theme::key_label()),
                Span::styled("] Scroll  ", Theme::key_hint()),
                Span::styled("[", Theme::key_hint()),
                Span::styled("Tab", Theme::key_label()),
                Span::styled("] Next Screen  ", Theme::key_hint()),
                Span::styled("[", Theme::key_hint()),
                Span::styled("Shift+Tab", Theme::key_label()),
                Span::styled("] Prev  ", Theme::key_hint()),
                Span::styled("[", Theme::key_hint()),
                Span::styled("q", Theme::key_label()),
                Span::styled("] Quit", Theme::key_hint()),
            ])
        }
    };
    frame.render_widget(Paragraph::new(keys), chunks[0]);

    // Row 2: live stats
    let model = &MODELS[app.selected_model];
    let word_count = app.response_text.split_whitespace().count();
    let tasks_done = app.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
    let tasks_total = app.tasks.len();

    let stats = Line::from(vec![
        Span::styled(
            format!(" Mode: {} ", app.mode.label()),
            Style::default()
                .fg(match app.mode {
                    PromptMode::Chat => Theme::PRIMARY,
                    PromptMode::FastCode => Theme::SUCCESS,
                    PromptMode::Agent => Theme::WARNING,
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  |  ", Style::default().fg(Theme::DIM)),
        Span::styled(
            format!("Model: {} ", model.name),
            Theme::stat_value(),
        ),
        Span::styled("  |  ", Style::default().fg(Theme::DIM)),
        Span::styled("Tasks: ", Style::default().fg(Theme::MUTED)),
        Span::styled(
            format!("{}/{}", tasks_done, tasks_total),
            if tasks_done == tasks_total && tasks_total > 0 {
                Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)
            } else {
                Theme::stat_value()
            },
        ),
        Span::styled("  |  ", Style::default().fg(Theme::DIM)),
        Span::styled("Words: ", Style::default().fg(Theme::MUTED)),
        Span::styled(
            format!("{}", word_count),
            Theme::stat_value(),
        ),
        Span::styled("  |  ", Style::default().fg(Theme::DIM)),
        Span::styled("Tokens: ", Style::default().fg(Theme::MUTED)),
        Span::styled(
            if app.llm_usage.is_empty() { "--".to_string() } else { format!("{}", app.total_tokens()) },
            Style::default()
                .fg(if app.llm_usage.is_empty() { Theme::DIM } else { Theme::SUCCESS })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  |  ", Style::default().fg(Theme::DIM)),
        Span::styled("Logs: ", Style::default().fg(Theme::MUTED)),
        Span::styled(
            format!("{}", app.agent_logs.len()),
            Theme::stat_value(),
        ),
    ]);
    frame.render_widget(Paragraph::new(stats), chunks[1]);
}
