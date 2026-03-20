pub struct Theme {
    pub border: BorderChars,
    pub colors: Colors,
    pub icons: Icons,
    pub spacing: Spacing,
}

pub struct BorderChars {
    pub tl: &'static str, // ╭
    pub tr: &'static str, // ╮
    pub bl: &'static str, // ╰
    pub br: &'static str, // ╯
    pub h: &'static str,  // ─
    pub v: &'static str,  // │
    pub jl: &'static str, // ├
    pub jr: &'static str, // ┤
}

pub struct Colors {
    pub add: &'static str,    // green
    pub del: &'static str,    // red
    pub header: &'static str, // bold blue
    pub dim: &'static str,    // gray
    pub reset: &'static str,
    pub bold: &'static str,
    pub accent: &'static str,     // cyan
    pub warn: &'static str,       // yellow
    pub file_path: &'static str,  // bold white
    pub line_num: &'static str,   // dim
    pub box_border: &'static str, // dim cyan
    pub status_ok: &'static str,  // green
    pub status_run: &'static str, // yellow
}

pub struct Icons {
    pub collapsed: &'static str, // ▶
    pub expanded: &'static str,  // ▼
    pub added: &'static str,     // +
    pub removed: &'static str,   // -
    pub dot_ok: &'static str,    // ●
    pub dot_run: &'static str,   // ◉
    pub spinner: &'static [&'static str],
    pub check: &'static str, // ✓
    pub cross: &'static str, // ✗
}

pub struct Spacing {
    pub indent: usize,
    pub box_pad: usize,
    pub max_width: usize,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            border: BorderChars {
                tl: "╭",
                tr: "╮",
                bl: "╰",
                br: "╯",
                h: "─",
                v: "│",
                jl: "├",
                jr: "┤",
            },
            colors: Colors {
                add: "\x1b[32m",
                del: "\x1b[31m",
                header: "\x1b[1;34m",
                dim: "\x1b[90m",
                reset: "\x1b[0m",
                bold: "\x1b[1m",
                accent: "\x1b[36m",
                warn: "\x1b[33m",
                file_path: "\x1b[1;37m",
                line_num: "\x1b[90m",
                box_border: "\x1b[90;36m",
                status_ok: "\x1b[32m",
                status_run: "\x1b[33m",
            },
            icons: Icons {
                collapsed: "▶",
                expanded: "▼",
                added: "+",
                removed: "-",
                dot_ok: "●",
                dot_run: "◉",
                spinner: &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
                check: "✓",
                cross: "✗",
            },
            spacing: Spacing {
                indent: 2,
                box_pad: 1,
                max_width: 80,
            },
        }
    }
}
