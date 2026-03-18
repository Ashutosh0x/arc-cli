//! Tests for the zero-copy SSE parser.

use arc_providers::streaming::parse_sse_chunk;

#[test]
fn single_data_event() {
    let input = b"data: hello world\n\n";
    let events = parse_sse_chunk(input);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data.as_str(), "hello world");
    assert_eq!(events[0].event_type.as_str(), "message");
}

#[test]
fn multiple_events() {
    let input = b"data: first\n\ndata: second\n\ndata: third\n\n";
    let events = parse_sse_chunk(input);
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].data.as_str(), "first");
    assert_eq!(events[1].data.as_str(), "second");
    assert_eq!(events[2].data.as_str(), "third");
}

#[test]
fn multiline_data() {
    let input = b"data: line one\ndata: line two\n\n";
    let events = parse_sse_chunk(input);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data.as_str(), "line one\nline two");
}

#[test]
fn custom_event_type() {
    let input = b"event: delta\ndata: {\"content\":\"hi\"}\n\n";
    let events = parse_sse_chunk(input);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type.as_str(), "delta");
}

#[test]
fn openai_style_stream() {
    let input = br#"data: {"choices":[{"delta":{"content":"Hello"}}]}

data: {"choices":[{"delta":{"content":" World"}}]}

data: [DONE]

"#;
    let events = parse_sse_chunk(input);
    assert_eq!(events.len(), 3);
    assert!(events[0].data.contains("Hello"));
    assert!(events[1].data.contains("World"));
    assert_eq!(events[2].data.as_str(), "[DONE]");
}

#[test]
fn handles_carriage_returns() {
    let input = b"data: hello\r\n\r\n";
    let events = parse_sse_chunk(input);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data.as_str(), "hello");
}

#[test]
fn empty_input_yields_nothing() {
    let events = parse_sse_chunk(b"");
    assert!(events.is_empty());
}

#[test]
fn comments_and_id_ignored() {
    let input = b": this is a comment\nid: 42\ndata: payload\n\n";
    let events = parse_sse_chunk(input);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data.as_str(), "payload");
}

#[test]
fn data_without_trailing_blank_line() {
    // Stream cut off — should still emit the buffered event.
    let input = b"data: partial";
    let events = parse_sse_chunk(input);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data.as_str(), "partial");
}

#[test]
fn anthropic_style_stream() {
    let input = br#"event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"!"}}

event: message_stop
data: {"type":"message_stop"}

"#;
    let events = parse_sse_chunk(input);
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].event_type.as_str(), "content_block_delta");
    assert_eq!(events[2].event_type.as_str(), "message_stop");
}
