use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use crossterm::tty::IsTty;

const FRAMES: &[&str] = &[
    "\x1b[1;34m    A\x1b[0m",
    "\x1b[1;34m    AR\x1b[0m",
    "\x1b[1;34m    ARC\x1b[0m",
    "\x1b[1;34m    ARC \x1b[0m",
    "\x1b[1;34m    ARC C\x1b[0m",
    "\x1b[1;34m    ARC CL\x1b[0m",
    "\x1b[1;34m    ARC CLI\x1b[0m",
];

pub fn play_startup(out: &mut impl Write) -> io::Result<()> {
    // Only animate if terminal is interactive
    if !std::io::stdout().is_tty() {
        return Ok(());
    }

    for frame in FRAMES {
        write!(out, "\r{}", frame)?;
        out.flush()?;
        thread::sleep(Duration::from_millis(60));
    }

    // Hold final frame
    thread::sleep(Duration::from_millis(200));
    writeln!(out)?;

    Ok(())
}
