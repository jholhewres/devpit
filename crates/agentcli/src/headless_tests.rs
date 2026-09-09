//! The stream shape, tested beside the code that reads it.

use super::*;
use crate::schema::validates;

const SCHEMA: &str = r#"{"type":"object",
        "properties":{"verdict":{"type":"string"},"findings":{"type":"array"}},
        "required":["verdict","findings"]}"#;

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
        Line::Other => panic!("the result line decoded as something else"),
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
        assert!(matches!(
            serde_json::from_str::<Line>(line),
            Ok(Line::Other)
        ));
    }
}

/// The whole path, against the installed CLI.
///
/// Opt-in rather than skip-if-missing, and the difference matters: this
/// one spends money. A test that quietly bills someone on every `make
/// test` is a test that gets deleted. Run it with
/// `DEVPIT_LIVE_TURN=1 cargo test -p devpit-agentcli`.
#[test]
fn a_real_turn_answers_the_schema_and_reports_its_cost() {
    if std::env::var_os("DEVPIT_LIVE_TURN").is_none() {
        eprintln!("skipped: set DEVPIT_LIVE_TURN=1 to spend money on this one");
        return;
    }

    let mut fragments = 0;
    let outcome = run_turn(
        &Turn {
            env: &[],
            prompt: "Reply with verdict \"approved\" and an empty findings array.",
            cwd: Path::new("/tmp"),
            agents: None,
            schema: Some(SCHEMA),
            budget_usd: Some(0.50),
            model: Some("claude-haiku-4-5-20251001"),
            settings: None,
        },
        |_| fragments += 1,
    )
    .expect("the turn ran");

    assert!(!outcome.is_error, "{outcome:?}");
    assert_eq!(validates(&outcome.result, SCHEMA), Ok(()));
    assert!(outcome.cost_usd > 0.0, "a turn that cost nothing");
    assert!(fragments > 0, "nothing streamed while it worked");
}
