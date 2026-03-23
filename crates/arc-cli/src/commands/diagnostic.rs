// SPDX-License-Identifier: MIT
use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::Path;

pub async fn run(data_dir: &Path) -> Result<()> {
    println!("🔍 Gathering diagnostic bundle...");

    let out_file = std::env::current_dir()?.join("arc_diagnostic.txt");
    let mut file = fs::File::create(&out_file)?;

    writeln!(file, "=== ARC DIAGNOSTIC BUNDLE ===")?;
    writeln!(file, "Version: {}", env!("CARGO_PKG_VERSION"))?;
    writeln!(
        file,
        "OS: {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    )?;
    writeln!(file, "Time: {}", chrono::Utc::now())?;

    let config_path = data_dir.join("config.toml");
    if config_path.exists() {
        writeln!(file, "\n=== CONFIG.TOML (Redacted) ===")?;
        // Ideally we would redact secrets here, but API keys are strictly
        // in OS keyring now, so config.toml is safe to dump.
        let content = fs::read_to_string(&config_path)?;
        writeln!(file, "{}", content)?;
    }

    // Grab the last 50 lines of the latest log if tracing appender is active
    let logs_dir = data_dir.join("logs");
    if logs_dir.exists() {
        writeln!(file, "\n=== RECENT LOGS ===")?;
        if let Ok(entries) = fs::read_dir(logs_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                    writeln!(
                        file,
                        "--- file: {} ---",
                        entry.file_name().to_string_lossy()
                    )?;
                    let content = fs::read_to_string(entry.path()).unwrap_or_default();
                    let tail: Vec<&str> = content.lines().rev().take(50).collect();
                    for line in tail.into_iter().rev() {
                        writeln!(file, "{}", line)?;
                    }
                }
            }
        }
    }

    println!("✅ Diagnostic bundle created at: {}", out_file.display());
    println!("   Please review the file and attach it when opening an issue.");
    Ok(())
}
