// SPDX-License-Identifier: MIT
//! Property tests for the /loop interval parser.

use proptest::prelude::*;
use std::time::Duration;

fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        return Err("empty".into());
    }
    let mut total_secs: u64 = 0;
    let mut current_num = String::new();

    for ch in s.chars() {
        if ch.is_ascii_digit() {
            current_num.push(ch);
        } else {
            if current_num.is_empty() {
                return Err(format!("No number before unit '{ch}'"));
            }
            let num: u64 = current_num.parse().map_err(|e| format!("{e}"))?;
            current_num.clear();
            match ch {
                's' => total_secs = total_secs.saturating_add(num),
                'm' => total_secs = total_secs.saturating_add(num.saturating_mul(60)),
                'h' => total_secs = total_secs.saturating_add(num.saturating_mul(3600)),
                'd' => total_secs = total_secs.saturating_add(num.saturating_mul(86400)),
                _ => return Err(format!("Unknown unit: {ch}")),
            }
        }
    }

    if total_secs == 0 {
        return Err("Zero duration".into());
    }

    Ok(Duration::from_secs(total_secs))
}

proptest! {
    #[test]
    fn seconds_roundtrip(secs in 1u64..100000) {
        let input = format!("{secs}s");
        let result = parse_duration(&input).unwrap();
        prop_assert_eq!(result, Duration::from_secs(secs));
    }

    #[test]
    fn minutes_roundtrip(mins in 1u64..10000) {
        let input = format!("{mins}m");
        let result = parse_duration(&input).unwrap();
        prop_assert_eq!(result, Duration::from_secs(mins * 60));
    }

    #[test]
    fn composite_is_sum(hours in 0u64..24, mins in 0u64..60, secs in 0u64..60) {
        let total = hours * 3600 + mins * 60 + secs;
        if total == 0 {
            return Ok(());
        }

        let mut input = String::new();
        if hours > 0 { input.push_str(&format!("{hours}h")); }
        if mins > 0 { input.push_str(&format!("{mins}m")); }
        if secs > 0 { input.push_str(&format!("{secs}s")); }

        if input.is_empty() {
            return Ok(());
        }

        let result = parse_duration(&input).unwrap();
        prop_assert_eq!(result, Duration::from_secs(total));
    }

    #[test]
    fn never_panics_on_arbitrary(s in "\\PC{0,50}") {
        let _ = parse_duration(&s);
    }
}
