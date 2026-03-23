// SPDX-License-Identifier: MIT
use anyhow::Result;

pub async fn run() -> Result<()> {
    println!("Checking for updates...");

    // `self_update` uses synchronous networking, so we run it in spawn_blocking
    let update_result = tokio::task::spawn_blocking(|| {
        self_update::backends::github::Update::configure()
            .repo_owner("Ashutosh0x")
            .repo_name("arc-cli")
            .bin_name("arc")
            .show_download_progress(true)
            .current_version(env!("CARGO_PKG_VERSION"))
            .build()?
            .update()
    })
    .await??;

    match update_result {
        self_update::Status::UpToDate(v) => {
            println!("You already have the latest version: v{}", v);
        },
        self_update::Status::Updated(v) => {
            println!("Successfully updated to v{}!", v);
            println!("Please restart the ARC CLI to use the new version.");
        },
    }

    Ok(())
}
