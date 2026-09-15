//! The stream shape, tested against what the CLI writes.

use super::*;

/// The stream is what the product reads, so the recorded shape is the
/// test: `result` carries the cost, and it is the last line.
#[test]
fn the_result_line_carries_what_a_card_shows() {
    let line = r#"{"type":"result","subtype":"success","is_error":false,
            "result":"PONG","total_cost_usd":0.101315,"duration_ms":2303,"num_turns":1,
            "stop_reason":"end_turn"}"#;
    match serde_json::from_str::<Line>(line).expect("decode") {
        Line::Result {
            result,
            total_cost_usd,
            duration_ms,
            num_turns,
            is_error,
            stop_reason,
        } => {
            assert_eq!(result, "PONG");
            assert!((total_cost_usd - 0.101315).abs() < 1e-9);
            assert_eq!(duration_ms, 2303);
            assert_eq!(num_turns, 1);
            assert!(!is_error);
            assert_eq!(stop_reason.as_deref(), Some("end_turn"));
        }
        _ => panic!("the result line decoded as something else"),
    }
}

/// Everything that is not the result is a fragment to show, not an error.
#[test]
fn the_other_lines_are_fragments_not_failures() {
    for line in [
        r#"{"type":"system","subtype":"init","session_id":"x"}"#,
        r#"{"type":"assistant","message":{"content":[]}}"#,
        r#"{"type":"rate_limit_event"}"#,
    ] {
        let mut fragments = 0;
        assert_eq!(
            read_stream([line.to_owned()].into_iter(), |_| fragments += 1),
            None
        );
        assert_eq!(fragments, 1, "{line}");
    }
}

/// The session a turn ran is read off its `init` line, which is the CLI's own
/// word for it even when it was asked for another.
#[test]
fn a_turn_reports_the_session_its_init_line_named() {
    let fixture =
        include_str!("../tests/fixtures/claude-2.1.270-subagent-edit-tasks-background.jsonl");
    let mut fragments = 0;
    let outcome = read_stream(fixture.lines().map(str::to_owned), |_| fragments += 1)
        .expect("the fixture ends with a result");
    assert_eq!(
        outcome.session_id.as_deref(),
        Some("9c684740-743b-4394-8b4c-c11dd543c135")
    );
    assert!(
        fragments > 0,
        "the init line still reaches the card's stream"
    );
}
