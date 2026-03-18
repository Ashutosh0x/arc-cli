//! `arc doctor` — system health check.

use std::path::Path;
use std::time::Duration;

use anyhow::Result;

/// Run the `arc doctor` diagnostic.
pub async fn run(data_dir: &Path, config_path: &Path) -> Result<()> {
    println!();
    println!("  ARC Doctor — System Health Check");
    println!("  ════════════════════════════════════════");
    println!();

    let mut pass = 0u32;
    let mut fail = 0u32;
    let mut warn = 0u32;

    // ── 1. Config file ──────────────────────────────────────────────
    check(
        "Configuration file",
        &config_path.display().to_string(),
        config_path.exists(),
        &mut pass,
        &mut fail,
    );

    // ── 2. Data directory writable ──────────────────────────────────
    let writable = is_dir_writable(data_dir);
    check(
        "Data directory writable",
        &data_dir.display().to_string(),
        writable,
        &mut pass,
        &mut fail,
    );

    // ── 3. Telemetry database ───────────────────────────────────────
    let telem_db = data_dir.join("telemetry.redb");
    check(
        "Telemetry database",
        &telem_db.display().to_string(),
        telem_db.exists(),
        &mut pass,
        &mut fail,
    );

    // ── 4. OS Keyring ───────────────────────────────────────────────
    let keyring_ok = check_keyring().await;
    check(
        "OS Keyring access",
        if keyring_ok { "accessible" } else { "FAILED" },
        keyring_ok,
        &mut pass,
        &mut fail,
    );

    // ── 5. Network connectivity ─────────────────────────────────────
    let endpoints = [
        ("Anthropic API", "https://api.anthropic.com"),
        ("OpenAI API", "https://api.openai.com"),
        ("Google AI", "https://generativelanguage.googleapis.com"),
        ("Ollama (local)", "http://127.0.0.1:11434"),
    ];

    for (name, url) in &endpoints {
        let reachable = check_endpoint(url).await;
        let status = if reachable { "reachable" } else { "unreachable" };

        if name.contains("Ollama") && !reachable {
            // Ollama is optional, downgrade to warning.
            println!("  ⚠  {name:<28} {status}");
            warn += 1;
        } else {
            check(name, status, reachable, &mut pass, &mut fail);
        }
    }

    // ── 6. Git available ────────────────────────────────────────────
    let git_ok = which_exists("git");
    check(
        "Git",
        if git_ok { "found in PATH" } else { "NOT FOUND" },
        git_ok,
        &mut pass,
        &mut fail,
    );

    // ── 7. Rust toolchain ───────────────────────────────────────────
    let rustc_ok = which_exists("rustc");
    if !rustc_ok {
        println!("  ⚠  {:<28} not in PATH (optional)", "Rust toolchain");
        warn += 1;
    } else {
        check("Rust toolchain", "found", true, &mut pass, &mut fail);
    }

    // ── Summary ─────────────────────────────────────────────────────
    println!();
    println!("  ────────────────────────────────────────");
    println!(
        "  ✅ {pass} passed    ❌ {fail} failed    ⚠ {warn} warnings"
    );

    if fail == 0 {
        println!("  🎉 ARC is healthy!");
    } else {
        println!("  🔧 Some checks failed — see above for details.");
    }
    println!();

    Ok(())
}

fn check(name: &str, detail: &str, ok: bool, pass: &mut u32, fail: &mut u32) {
    let icon = if ok { "✅" } else { "❌" };
    println!("  {icon}  {name:<28} {detail}");
    if ok {
        *pass += 1;
    } else {
        *fail += 1;
    }
}

fn is_dir_writable(dir: &Path) -> bool {
    if !dir.exists() {
        return std::fs::create_dir_all(dir).is_ok();
    }
    let probe = dir.join(".arc_doctor_probe");
    let ok = std::fs::write(&probe, b"ok").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

async fn check_keyring() -> bool {
    tokio::task::spawn_blocking(|| {
        let entry: keyring::Entry = keyring::Entry::new("arc-cli-doctor", "probe")
            .map_err(|e| anyhow::anyhow!("keyring entry error: {e}"))
            .unwrap();
        match entry.get_password() {
            Ok(_) | Err(keyring::Error::NoEntry) => true,
            Err(_) => false,
        }
    })
    .await
    .unwrap_or(false)
}

async fn check_endpoint(url: &str) -> bool {
    let client: reqwest::Client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    client
        .head(url)
        .send()
        .await
        .map(|r: reqwest::Response| r.status().as_u16() < 500)
        .unwrap_or(false)
}

fn which_exists(cmd: &str) -> bool {
    std::process::Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
