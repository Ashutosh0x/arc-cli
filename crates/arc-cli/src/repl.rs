use anyhow::Result;
use std::io::{self, Write};
use futures::StreamExt;
use reqwest::Client;

use arc_providers::message::{Message, Role, StreamEvent};
use arc_providers::traits::Provider;
use arc_providers::anthropic::AnthropicProvider;
// use arc_providers::traits::ProviderClient;

/// Physical entrypoint connecting the Terminal to the active Agent models.
pub async fn run_repl(api_key: String) -> Result<()> {
    println!(">>> ARC Agentic CLI - Autonomous Loop Booted Native.");
    println!(">>> Connected via HTTP/2 stream to Anthropic Opus.");
    println!(">>> Type /exit or /checkpoint to manage history.");

    let client = Client::builder()
        .http2_prior_knowledge()
        .build()?;
        
    let provider = AnthropicProvider::new(client, api_key);
    
    let mut system_prompt = "You are ARC, a terminal-native autonomous Rust-based agent.".to_string();
    
    // Load project-specific instructions from ARC.md or .arc.md
    for filename in ["ARC.md", ".arc.md", "arc.md"] {
        if let Ok(content) = std::fs::read_to_string(filename) {
            system_prompt.push_str(&format!("\n\n=== Project Instructions & Architecture ({}) ===\n{}", filename, content));
            println!(">>> Loaded project context from {}", filename);
            break;
        }
    }

    let mut session_messages = vec![
        Message {
            role: Role::System,
            content: system_prompt,
        }
    ];

    loop {
        print!("arc> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input == "/exit" {
            break;
        }
        
        if input.is_empty() {
            continue;
        }

        session_messages.push(Message {
            role: Role::User,
            content: input.to_string(),
        });

        let mut stream = provider
            .stream("claude-3-5-sonnet-20241022", &session_messages, &[])
            .await?;

        print!("|ARC|: ");
        io::stdout().flush()?;
        
        let mut full_response = String::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(StreamEvent::TextDelta(text)) => {
                    print!("{}", text);
                    io::stdout().flush()?;
                    full_response.push_str(&text);
                }
                Ok(_) => { /* Ignore other stream events for now */ }
                Err(e) => {
                    eprintln!("\n[Stream Disconnect Error]: {}", e);
                    break;
                }
            }
        }
        println!(); // Terminate flush boundary cleanly 

        session_messages.push(Message {
            role: Role::Assistant,
            content: full_response,
        });
    }

    Ok(())
}
