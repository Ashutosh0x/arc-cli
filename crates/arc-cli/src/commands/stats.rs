//! `arc stats` — display usage analytics from the telemetry store.

use std::path::Path;

use anyhow::Result;

use arc_core::telemetry::TelemetryStore;

/// Run the `arc stats` subcommand.
pub async fn run(data_dir: &Path) -> Result<()> {
    let store = TelemetryStore::open(data_dir)?;

    let all = store.aggregate_all()?;
    let week = store.aggregate_last_days(7)?;

    if all.total_requests == 0 {
        println!("  No telemetry data recorded yet.");
        println!("  Start chatting with `arc` to begin tracking.");
        return Ok(());
    }

    println!();
    print_header("ARC Usage Statistics");
    println!();

    // ── Global summary ──────────────────────────────────────────────
    print_section("Lifetime Totals");
    print_kv("Total Requests", &fmt_num(all.total_requests));
    print_kv("Input Tokens", &fmt_num(all.total_input_tokens));
    print_kv("Output Tokens", &fmt_num(all.total_output_tokens));
    print_kv("Estimated Cost", &format!("${:.4}", all.total_cost_usd));
    print_kv("Data Span", &format!("{:.1} days", all.span_days()));
    println!();

    // ── Last 7 days ─────────────────────────────────────────────────
    if week.total_requests > 0 {
        print_section("Last 7 Days");
        print_kv("Requests", &fmt_num(week.total_requests));
        print_kv("Input Tokens", &fmt_num(week.total_input_tokens));
        print_kv("Output Tokens", &fmt_num(week.total_output_tokens));
        print_kv("Cost", &format!("${:.4}", week.total_cost_usd));
        println!();
    }

    // ── Per-provider table ──────────────────────────────────────────
    print_section("Per-Provider Breakdown");
    println!(
        "  {:<14} {:>8} {:>10} {:>10} {:>9} {:>7} {:>7} {:>7} {:>6}",
        "PROVIDER", "REQS", "IN_TOK", "OUT_TOK", "COST", "p50", "p95", "p99", "ERR%"
    );
    println!("  {}", "─".repeat(92));

    let mut providers: Vec<_> = all.providers.values().collect();
    providers.sort_by(|a, b| b.total_requests.cmp(&a.total_requests));

    for s in &providers {
        println!(
            "  {:<14} {:>8} {:>10} {:>10} {:>9} {:>7} {:>7} {:>7} {:>5.1}%",
            s.provider,
            fmt_num(s.total_requests),
            fmt_num(s.total_input_tokens),
            fmt_num(s.total_output_tokens),
            format!("${:.4}", s.total_cost_usd),
            s.p50().map_or("—".into(), |v| format!("{v}ms")),
            s.p95().map_or("—".into(), |v| format!("{v}ms")),
            s.p99().map_or("—".into(), |v| format!("{v}ms")),
            s.error_rate_pct(),
        );
    }

    println!();
    print_kv(
        "Database",
        &format!("{} ({} records)", store.path().display(), store.record_count()?),
    );
    println!();

    Ok(())
}

// ── Formatting helpers ──────────────────────────────────────────────

fn print_header(title: &str) {
    println!("  ╔{}╗", "═".repeat(60));
    println!("  ║ {title:^58} ║");
    println!("  ╚{}╝", "═".repeat(60));
}

fn print_section(title: &str) {
    println!("  ┌── {title} ──");
}

fn print_kv(key: &str, value: &str) {
    println!("  │ {key:<22} {value}");
}

/// Format a number with thousands separators.
fn fmt_num(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fmt_num() {
        assert_eq!(fmt_num(0), "0");
        assert_eq!(fmt_num(999), "999");
        assert_eq!(fmt_num(1_000), "1,000");
        assert_eq!(fmt_num(1_234_567), "1,234,567");
    }
}
