// SPDX-License-Identifier: MIT
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use similar::{ChangeTag, TextDiff};

pub struct DiffViewer {
    old_text: String,
    new_text: String,
    title: String,
}

impl DiffViewer {
    pub fn new(old_text: String, new_text: String, title: String) -> Self {
        Self {
            old_text,
            new_text,
            title,
        }
    }

    pub fn to_widget(&self) -> Paragraph<'_> {
        let diff = TextDiff::from_lines(&self.old_text, &self.new_text);
        let mut lines = Vec::new();

        for change in diff.iter_all_changes() {
            let (prefix, style) = match change.tag() {
                ChangeTag::Delete => ("- ", Style::default().fg(Color::Red)),
                ChangeTag::Insert => ("+ ", Style::default().fg(Color::Green)),
                ChangeTag::Equal => ("  ", Style::default().fg(Color::DarkGray)),
            };

            let content = change.value().trim_end_matches(&['\n', '\r'][..]);
            lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(content.to_string(), style),
            ]));
        }

        Paragraph::new(lines).block(
            Block::default()
                .title(format!(" {} Diff ", self.title))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
    }
}
