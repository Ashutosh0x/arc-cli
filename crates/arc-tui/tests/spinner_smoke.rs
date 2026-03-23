// SPDX-License-Identifier: MIT
//! Smoke tests — verify the spinner compiles, starts, and stops
//! without panicking.  No visual assertion (terminal output).

use arc_tui::spinner::{Phase, Spinner, SpinnerStyle, StreamingSpinner, with_spinner};

#[tokio::test]
async fn spinner_start_stop() {
    let handle = Spinner::new()
        .style(SpinnerStyle::Braille)
        .message("test")
        .hide_elapsed()
        .start();

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    handle.finish("ok").await;
}

#[tokio::test]
async fn spinner_fail_path() {
    let handle = Spinner::new().start();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    handle.fail("something broke").await;
}

#[tokio::test]
async fn spinner_phase_transitions() {
    let handle = Spinner::new().manual_phases().start();

    for phase in [
        Phase::Connecting,
        Phase::Thinking,
        Phase::Analyzing,
        Phase::Generating,
        Phase::Writing,
        Phase::Reviewing,
    ] {
        handle.set_phase(phase);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    handle.set_message("Custom message");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    handle.stop().await;
}

#[tokio::test]
async fn spinner_detail_updates() {
    let handle = Spinner::new().start();
    handle.set_detail("reading src/main.rs");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    handle.clear_detail();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    handle.stop().await;
}

#[tokio::test]
async fn with_spinner_success() {
    let result = with_spinner("testing", || async {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        Ok::<i32, color_eyre::Report>(42)
    })
    .await;

    assert_eq!(result.unwrap(), 42);
}

#[tokio::test]
async fn with_spinner_failure() {
    let result = with_spinner("testing failure", || async {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        Err::<i32, color_eyre::Report>(color_eyre::eyre::eyre!("boom"))
    })
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn streaming_spinner_token_counting() {
    let spinner = StreamingSpinner::start();

    for _ in 0..25 {
        spinner.on_token();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    spinner.finish().await;
}

#[tokio::test]
async fn all_spinner_styles_work() {
    for style in [
        SpinnerStyle::Braille,
        SpinnerStyle::BrailleOrbit,
        SpinnerStyle::Circle,
        SpinnerStyle::BouncingBar,
        SpinnerStyle::Diamond,
        SpinnerStyle::Minimal,
        SpinnerStyle::Arc,
    ] {
        let handle = Spinner::new().style(style).hide_elapsed().start();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        handle.stop().await;
    }
}
