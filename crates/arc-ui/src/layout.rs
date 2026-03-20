use std::io::{self, Stdout, Write};

use crate::header;
use crate::renderer;
use crate::state::UiState;
use crate::theme::Theme;

pub struct Layout {
    theme: Theme,
    buffer: Vec<u8>,
}

impl Layout {
    pub fn new() -> Self {
        Self {
            theme: Theme::default(),
            buffer: Vec::with_capacity(8192),
        }
    }

    /// Full repaint — called on state change
    pub fn paint(&mut self, state: &UiState, out: &mut Stdout) -> io::Result<()> {
        self.buffer.clear();

        // 1. Header
        header::render_header(state, &self.theme, &mut self.buffer)?;

        // 2. Plan steps (if any)
        crate::plan::render_plan(state, &self.theme, &mut self.buffer)?;

        // 3. Diff review (if any)
        renderer::render_diff_list(state, &self.theme, &mut self.buffer)?;

        // 4. Footer
        crate::footer::render_footer(state, &self.theme, &mut self.buffer)?;

        // Flush single write (eliminates flicker)
        out.write_all(&self.buffer)?;
        out.flush()?;

        Ok(())
    }

    /// Incremental repaint — only diff section
    pub fn repaint_diffs(&mut self, state: &UiState, out: &mut Stdout) -> io::Result<()> {
        self.buffer.clear();
        renderer::render_diff_list(state, &self.theme, &mut self.buffer)?;
        crate::footer::render_footer(state, &self.theme, &mut self.buffer)?;
        out.write_all(&self.buffer)?;
        out.flush()?;
        Ok(())
    }
}
