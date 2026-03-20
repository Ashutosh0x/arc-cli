//! IDE Detection Service
//!
//! Auto-detects 20+ IDEs from environment variables and process info.
//! Provides IDE-specific context to the LLM for better suggestions.

use std::env;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdeInfo {
    pub name: &'static str,
    pub display_name: &'static str,
}

pub const IDE_VSCODE: IdeInfo = IdeInfo {
    name: "vscode",
    display_name: "VS Code",
};
pub const IDE_CURSOR: IdeInfo = IdeInfo {
    name: "cursor",
    display_name: "Cursor",
};
pub const IDE_ZED: IdeInfo = IdeInfo {
    name: "zed",
    display_name: "Zed",
};
pub const IDE_SUBLIME: IdeInfo = IdeInfo {
    name: "sublimetext",
    display_name: "Sublime Text",
};
pub const IDE_JETBRAINS: IdeInfo = IdeInfo {
    name: "jetbrains",
    display_name: "JetBrains IDE",
};
pub const IDE_INTELLIJ: IdeInfo = IdeInfo {
    name: "intellij",
    display_name: "IntelliJ IDEA",
};
pub const IDE_WEBSTORM: IdeInfo = IdeInfo {
    name: "webstorm",
    display_name: "WebStorm",
};
pub const IDE_PYCHARM: IdeInfo = IdeInfo {
    name: "pycharm",
    display_name: "PyCharm",
};
pub const IDE_GOLAND: IdeInfo = IdeInfo {
    name: "goland",
    display_name: "GoLand",
};
pub const IDE_CLION: IdeInfo = IdeInfo {
    name: "clion",
    display_name: "CLion",
};
pub const IDE_RUSTROVER: IdeInfo = IdeInfo {
    name: "rustrover",
    display_name: "RustRover",
};
pub const IDE_ANDROID_STUDIO: IdeInfo = IdeInfo {
    name: "androidstudio",
    display_name: "Android Studio",
};
pub const IDE_XCODE: IdeInfo = IdeInfo {
    name: "xcode",
    display_name: "Xcode",
};
pub const IDE_REPLIT: IdeInfo = IdeInfo {
    name: "replit",
    display_name: "Replit",
};
pub const IDE_CODESPACES: IdeInfo = IdeInfo {
    name: "codespaces",
    display_name: "GitHub Codespaces",
};
pub const IDE_NEOVIM: IdeInfo = IdeInfo {
    name: "neovim",
    display_name: "Neovim",
};
pub const IDE_VIM: IdeInfo = IdeInfo {
    name: "vim",
    display_name: "Vim",
};
pub const IDE_EMACS: IdeInfo = IdeInfo {
    name: "emacs",
    display_name: "Emacs",
};
pub const IDE_TERMINAL: IdeInfo = IdeInfo {
    name: "terminal",
    display_name: "Terminal",
};
pub const IDE_WARP: IdeInfo = IdeInfo {
    name: "warp",
    display_name: "Warp",
};
pub const IDE_WINDOWS_TERMINAL: IdeInfo = IdeInfo {
    name: "windows_terminal",
    display_name: "Windows Terminal",
};

/// Detect the current IDE from environment variables.
pub fn detect_ide() -> IdeInfo {
    // Cursor
    if env::var("CURSOR_TRACE_ID").is_ok() {
        return IDE_CURSOR;
    }
    // Replit
    if env::var("REPLIT_USER").is_ok() {
        return IDE_REPLIT;
    }
    // GitHub Codespaces
    if env::var("CODESPACES").is_ok() {
        return IDE_CODESPACES;
    }
    // Zed
    if env::var("ZED_SESSION_ID").is_ok() || env::var("TERM_PROGRAM").as_deref() == Ok("Zed") {
        return IDE_ZED;
    }
    // Xcode
    if env::var("XCODE_VERSION_ACTUAL").is_ok() {
        return IDE_XCODE;
    }
    // Sublime Text
    if env::var("TERM_PROGRAM").as_deref() == Ok("sublime") {
        return IDE_SUBLIME;
    }
    // JetBrains family
    if let Ok(te) = env::var("TERMINAL_EMULATOR") {
        if te.to_lowercase().contains("jetbrains") {
            return detect_jetbrains_product();
        }
    }
    // Warp terminal
    if env::var("TERM_PROGRAM").as_deref() == Ok("WarpTerminal") {
        return IDE_WARP;
    }
    // Windows Terminal
    if env::var("WT_SESSION").is_ok() {
        return IDE_WINDOWS_TERMINAL;
    }
    // VS Code
    if env::var("TERM_PROGRAM").as_deref() == Ok("vscode") {
        return IDE_VSCODE;
    }
    // Neovim
    if env::var("NVIM").is_ok() || env::var("NVIM_LISTEN_ADDRESS").is_ok() {
        return IDE_NEOVIM;
    }
    // Vim
    if env::var("VIM").is_ok() || env::var("VIMRUNTIME").is_ok() {
        return IDE_VIM;
    }
    // Emacs
    if env::var("INSIDE_EMACS").is_ok() {
        return IDE_EMACS;
    }

    IDE_TERMINAL
}

fn detect_jetbrains_product() -> IdeInfo {
    // Try to identify specific JetBrains product from env or process
    if let Ok(idea) = env::var("IDEA_INITIAL_DIRECTORY") {
        let lower = idea.to_lowercase();
        if lower.contains("rustrover") {
            return IDE_RUSTROVER;
        }
        if lower.contains("clion") {
            return IDE_CLION;
        }
        if lower.contains("idea") {
            return IDE_INTELLIJ;
        }
        if lower.contains("webstorm") {
            return IDE_WEBSTORM;
        }
        if lower.contains("pycharm") {
            return IDE_PYCHARM;
        }
        if lower.contains("goland") {
            return IDE_GOLAND;
        }
        if lower.contains("studio") {
            return IDE_ANDROID_STUDIO;
        }
    }
    IDE_JETBRAINS
}

/// Get a context string describing the detected IDE for LLM prompts.
pub fn ide_context_for_prompt(ide: &IdeInfo) -> String {
    format!(
        "The user is working in {display}. Tailor suggestions accordingly \
         (e.g., keyboard shortcuts, extension recommendations, terminal integration).",
        display = ide.display_name
    )
}
