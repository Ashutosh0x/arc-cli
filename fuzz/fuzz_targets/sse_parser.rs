// SPDX-License-Identifier: MIT
#![no_main]
use libfuzzer_sys::fuzz_target;

fn parse_sse_frame(buf: &[u8]) -> Option<usize> {
    // Find \n\n
    let mut pos = 0;
    while pos + 1 < buf.len() {
        if buf[pos] == b'\n' && buf[pos + 1] == b'\n' {
            // Attempt to parse as UTF-8
            let frame = std::str::from_utf8(&buf[..pos]).ok()?;
            let mut has_data = false;
            for line in frame.lines() {
                if line.starts_with("data: ") || line.starts_with("event: ") {
                    has_data = true;
                }
            }
            if has_data {
                return Some(pos + 2);
            }
            return None;
        }
        pos += 1;
    }
    None
}

fuzz_target!(|data: &[u8]| {
    let _ = parse_sse_frame(data);
});
