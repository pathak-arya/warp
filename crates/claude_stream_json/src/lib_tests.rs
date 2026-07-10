use super::*;

/// The real captured stream from `claude --print --output-format stream-json
/// --verbose` — the ground-truth contract for the parser.
const FIXTURE_SIMPLE: &str = include_str!("../tests/fixture_simple.jsonl");

fn parse_all(stream: &str) -> Vec<AgentStreamEvent> {
    stream.lines().flat_map(parse_line).collect()
}

#[test]
fn parses_real_fixture_into_expected_events() {
    let events = parse_all(FIXTURE_SIMPLE);

    // Session init is surfaced.
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AgentStreamEvent::SessionStarted { .. })),
        "expected a SessionStarted event, got {events:?}"
    );

    // The assistant text carrying display math is extracted verbatim,
    // delimiters intact, so Warp's markdown parser can typeset it.
    let assistant_text: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            AgentStreamEvent::AssistantText { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(assistant_text, vec!["hi\n\n$$a^2 + b^2 = c^2$$"]);

    // The final result marker is present and non-error.
    assert!(
        events.iter().any(|e| matches!(
            e,
            AgentStreamEvent::Result { is_error: false, .. }
        )),
        "expected a successful Result event, got {events:?}"
    );
}

#[test]
fn empty_thinking_block_is_dropped() {
    // The fixture contains an empty `thinking` content block; it must not
    // become a blank reasoning section.
    let events = parse_all(FIXTURE_SIMPLE);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, AgentStreamEvent::AssistantThinking { text } if text.is_empty())),
        "empty thinking blocks should be dropped"
    );
}

#[test]
fn blank_and_whitespace_lines_yield_nothing() {
    assert!(parse_line("").is_empty());
    assert!(parse_line("   \t ").is_empty());
}

#[test]
fn invalid_json_is_surfaced_as_unparseable() {
    let events = parse_line("this is not json");
    assert_eq!(
        events,
        vec![AgentStreamEvent::Unparseable {
            line: "this is not json".to_string()
        }]
    );
}

#[test]
fn parses_assistant_text_block() {
    let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"The identity $e^{i\\pi}+1=0$ is Euler's."}]}}"#;
    assert_eq!(
        parse_line(line),
        vec![AgentStreamEvent::AssistantText {
            text: r"The identity $e^{i\pi}+1=0$ is Euler's.".to_string()
        }]
    );
}

#[test]
fn parses_thinking_block() {
    let line = r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"Let me reason."}]}}"#;
    assert_eq!(
        parse_line(line),
        vec![AgentStreamEvent::AssistantThinking {
            text: "Let me reason.".to_string()
        }]
    );
}

#[test]
fn parses_tool_use_and_result() {
    let use_line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"tu_1","name":"Bash","input":{"command":"ls"}}]}}"#;
    match parse_line(use_line).as_slice() {
        [AgentStreamEvent::ToolUse { id, name, input }] => {
            assert_eq!(id, "tu_1");
            assert_eq!(name, "Bash");
            assert_eq!(input["command"], "ls");
        }
        other => panic!("expected ToolUse, got {other:?}"),
    }

    // Tool results arrive on a `user` message, with content as an array of items.
    let result_line = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tu_1","content":[{"type":"text","text":"file.txt"}],"is_error":false}]}}"#;
    assert_eq!(
        parse_line(result_line),
        vec![AgentStreamEvent::ToolResult {
            tool_use_id: "tu_1".to_string(),
            content: "file.txt".to_string(),
            is_error: false,
        }]
    );
}

#[test]
fn tool_result_accepts_plain_string_content() {
    let line = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tu_2","content":"plain output","is_error":true}]}}"#;
    assert_eq!(
        parse_line(line),
        vec![AgentStreamEvent::ToolResult {
            tool_use_id: "tu_2".to_string(),
            content: "plain output".to_string(),
            is_error: true,
        }]
    );
}

#[test]
fn unknown_event_types_are_ignored() {
    // rate_limit_event appears in the real fixture and must be dropped cleanly.
    let line = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"rejected"}}"#;
    assert!(parse_line(line).is_empty());
}

#[test]
fn multi_content_message_yields_multiple_events() {
    let line = r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"hmm"},{"type":"text","text":"answer"},{"type":"tool_use","id":"t","name":"Read","input":{}}]}}"#;
    let events = parse_line(line);
    assert_eq!(events.len(), 3);
    assert!(matches!(events[0], AgentStreamEvent::AssistantThinking { .. }));
    assert!(matches!(events[1], AgentStreamEvent::AssistantText { .. }));
    assert!(matches!(events[2], AgentStreamEvent::ToolUse { .. }));
}
