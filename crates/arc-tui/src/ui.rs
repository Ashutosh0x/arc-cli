// SPDX-License-Identifier: MIT
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;

pub fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(f.size());

    // History
    let history_text = app.history.join("\n");
    let history_widget = Paragraph::new(history_text)
        .block(Block::default().title(" Chat Output ").borders(Borders::ALL));
    f.render_widget(history_widget, chunks[0]);

    // Input
    let input_widget = Paragraph::new(app.input_text.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().title(" Input (Esc to exit) ").borders(Borders::ALL));
    f.render_widget(input_widget, chunks[1]);
}
