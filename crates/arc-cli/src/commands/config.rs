use anyhow::Result;
use arc_core::config::ArcConfig;
use std::fs;

pub async fn run(action: crate::cli::ConfigAction) -> Result<()> {
    match action {
        crate::cli::ConfigAction::Edit => {
            let path = ArcConfig::path()?;
            if !path.exists() {
                ArcConfig::default().save()?;
            }
            // Uses the `open` crate to launch whatever default text editor the OS has bound to .toml
            open::that(&path)?;
            println!("Opened config file in default editor: {}", path.display());
        },
        crate::cli::ConfigAction::Validate => {
            let path = ArcConfig::path()?;
            if !path.exists() {
                println!(
                    "No config file found at {}. Defaults will be used.",
                    path.display()
                );
                return Ok(());
            }

            let content = fs::read_to_string(&path)?;
            match toml::from_str::<ArcConfig>(&content) {
                Ok(_) => {
                    println!("✅ Configuration is perfectly valid.");
                },
                Err(e) => {
                    println!("❌ Configuration validation failed:");
                    eprintln!("   {}", e);
                    std::process::exit(1);
                },
            }
        },
        crate::cli::ConfigAction::Reset => {
            let path = ArcConfig::path()?;
            if path.exists() {
                let backup_path = path.with_extension("bak.toml");
                fs::copy(&path, &backup_path)?;
                println!("Backed up existing config to {}", backup_path.display());
            }

            ArcConfig::default().save()?;
            println!("✅ Configuration reset to un-opinionated defaults.");
        },
    }
    Ok(())
}
