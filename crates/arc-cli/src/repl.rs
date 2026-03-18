//! The main interactive chat REPL.
use std::io::{self, Write};
use arc_tui::spinner::{Phase, StreamingSpinner};
use anyhow::Result;

pub async fn run_repl() -> Result<()> {
    loop {
        let input = read_user_input()?;

        if input.trim().is_empty() {
            continue;
        }

        if input.trim() == "/quit" || input.trim() == "/exit" {
            break;
        }

        let spinner = StreamingSpinner::start();

        if looks_like_file_task(&input) {
            spinner.handle().set_phase(Phase::Analyzing);
            spinner.handle().set_detail("Reading referenced files");
        }

        match simulate_llm_stream(&spinner).await {
            Ok(()) => {
                println!();
                spinner.finish().await;
            }
            Err(e) => {
                spinner.fail(&format!("Error: {e}")).await;
            }
        }
    }

    Ok(())
}

fn looks_like_file_task(input: &str) -> bool {
    let lower = input.to_lowercase();
    lower.contains("write")
        || lower.contains("edit")
        || lower.contains("change")
        || lower.contains("modify")
        || lower.contains("create")
        || lower.contains("fix")
        || lower.contains("refactor")
        || lower.contains("update")
}

async fn simulate_llm_stream(spinner: &StreamingSpinner) -> Result<()> {
    for i in 0..80 {
        spinner.on_token();
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        if i % 20 == 0 {
            print!("word ");
        }
    }
    Ok(())
}

fn read_user_input() -> Result<String> {
    print!("\n  arc › ");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line)
}
