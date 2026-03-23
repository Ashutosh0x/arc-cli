// SPDX-License-Identifier: MIT
use clap::CommandFactory;
use std::env;
use std::fs;
use std::io::Error;
use std::path::PathBuf;

include!("src/cli.rs");

fn main() -> Result<(), Error> {
    let out_dir = match env::var_os("OUT_DIR") {
        None => return Ok(()),
        Some(out_dir) => out_dir,
    };
    
    let mut cmd = Cli::command();
    
    // Shell auto-completions
    let comps_dir = PathBuf::from(&out_dir).join("completions");
    fs::create_dir_all(&comps_dir)?;
    
    for shell in [
        clap_complete::Shell::Bash,
        clap_complete::Shell::Fish,
        clap_complete::Shell::Zsh,
        clap_complete::Shell::PowerShell,
        clap_complete::Shell::Elvish,
    ] {
        clap_complete::generate_to(shell, &mut cmd, "arc", &comps_dir)?;
    }
    
    // Man pages
    let man_dir = PathBuf::from(&out_dir).join("man");
    fs::create_dir_all(&man_dir)?;
    
    let man = clap_mangen::Man::new(cmd);
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;
    fs::write(man_dir.join("arc.1"), buffer)?;
    
    println!("cargo:warning=Generated completions to {:?}", comps_dir);
    println!("cargo:warning=Generated man pages to {:?}", man_dir);
    
    Ok(())
}
