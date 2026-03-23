// SPDX-License-Identifier: MIT
use crate::state::UiState;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

pub enum Action {
    ToggleExpand,
    Accept,
    Reject,
    AcceptAll,
    RejectAll,
    OpenEditor,
    MoveUp,
    MoveDown,
    ScrollUp,
    ScrollDown,
    Halt,
    Exit,
    ToggleHeader,
    None,
}

pub fn poll_key() -> Option<Action> {
    if event::poll(std::time::Duration::from_millis(50)).ok()? {
        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event::read().ok()?
        {
            return Some(match (code, modifiers) {
                // Navigation
                (KeyCode::Char('j'), KeyModifiers::NONE) => Action::MoveDown,
                (KeyCode::Char('k'), KeyModifiers::NONE) => Action::MoveUp,
                (KeyCode::Down, _) => Action::MoveDown,
                (KeyCode::Up, _) => Action::MoveUp,

                // Diff review
                (KeyCode::Enter, _) => Action::ToggleExpand,
                (KeyCode::Char('y'), KeyModifiers::NONE) => Action::Accept,
                (KeyCode::Char('n'), KeyModifiers::NONE) => Action::Reject,
                (KeyCode::Char('a'), KeyModifiers::NONE) => Action::AcceptAll,
                (KeyCode::Char('d'), KeyModifiers::NONE) => Action::RejectAll,
                (KeyCode::Char('e'), KeyModifiers::NONE) => Action::OpenEditor,
                (KeyCode::Esc, _) => Action::Reject,

                // System
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => Action::Halt,
                (KeyCode::Char('d'), KeyModifiers::CONTROL) => Action::Exit,
                (KeyCode::Char('l'), KeyModifiers::CONTROL) => Action::ToggleHeader,

                _ => Action::None,
            });
        }
    }
    None
}

pub fn apply_action(action: Action, state: &mut UiState) -> bool {
    match action {
        Action::ToggleExpand => state.toggle_selected_diff(),
        Action::Accept => state.accept_selected(),
        Action::Reject => state.reject_selected(),
        Action::AcceptAll => state.accept_all(),
        Action::RejectAll => state.reject_all(),
        Action::MoveDown => {
            let max = state.diff_blocks.len().saturating_sub(1);
            state.selected_diff = (state.selected_diff + 1).min(max);
        },
        Action::MoveUp => {
            state.selected_diff = state.selected_diff.saturating_sub(1);
        },
        Action::ToggleHeader => {
            state.header_mode = match state.header_mode {
                crate::state::HeaderMode::Expanded => crate::state::HeaderMode::Compact,
                crate::state::HeaderMode::Compact => crate::state::HeaderMode::Expanded,
            };
        },
        Action::Exit => return true,
        Action::Halt => return true,
        _ => {},
    }
    false
}
