//! Property-based tests for the SSE parser.

use proptest::prelude::*;

fn parse_sse_frame(input: &str) -> Option<(&str, &str)> {
    let frame_end = input.find("\n\n")?;
    let frame = &input[..frame_end];

    let mut event_type = "message";
    let mut data_lines = Vec::new();

    for line in frame.lines() {
        if let Some(v) = line.strip_prefix("event: ") {
            event_type = v;
        } else if let Some(v) = line.strip_prefix("data: ") {
            data_lines.push(v);
        }
    }

    if data_lines.is_empty() {
        return None;
    }

    // Return references into the original string
    Some((event_type, data_lines[0]))
}

proptest! {
    #[test]
    fn never_panics_on_arbitrary_input(s in ".*") {
        let _ = parse_sse_frame(&s);
    }

    #[test]
    fn valid_sse_always_parses(
        event_type in "[a-z_]{1,20}",
        data in "[a-zA-Z0-9 ]{1,100}"
    ) {
        let input = format!("event: {event_type}\ndata: {data}\n\n");
        let result = parse_sse_frame(&input);
        prop_assert!(result.is_some());
        let (parsed_event, parsed_data) = result.unwrap();
        prop_assert_eq!(parsed_event, event_type.as_str());
        prop_assert_eq!(parsed_data, data.as_str());
    }

    #[test]
    fn no_double_newline_returns_none(
        s in "[^\n]{1,100}"
    ) {
        let result = parse_sse_frame(&s);
        prop_assert!(result.is_none());
    }

    #[test]
    fn handles_multiple_data_lines(
        lines in proptest::collection::vec("[a-zA-Z0-9]{1,50}", 1..5)
    ) {
        let mut input = String::new();
        for line in &lines {
            input.push_str(&format!("data: {line}\n"));
        }
        input.push('\n');

        let result = parse_sse_frame(&input);
        prop_assert!(result.is_some());
    }
}
