use devpit_rpc::{CallState, Part};

use crate::driver::{driver, Claude, Driver, Read};

#[test]
fn a_line_it_cannot_parse_is_kept_as_text() {
    // Losing output is worse than showing it plain: a CLI prints its errors
    // in exactly the lines a strict parser would drop.
    let read = Claude.read("panic: something went wrong");
    assert_eq!(
        read,
        Read::Parts(vec![Part::Unknown {
            text: "panic: something went wrong".to_owned()
        }])
    );
}

#[test]
fn a_blank_line_is_nothing() {
    assert_eq!(Claude.read("   "), Read::Nothing);
}

#[test]
fn assistant_text_becomes_a_text_part() {
    let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"hi"}]}}"#;
    assert_eq!(
        Claude.read(line),
        Read::Parts(vec![Part::Text {
            text: "hi".to_owned()
        }])
    );
}

#[test]
fn thinking_is_not_the_answer() {
    let line =
        r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"hmm"}]}}"#;
    assert_eq!(
        Claude.read(line),
        Read::Parts(vec![Part::Thinking {
            text: "hmm".to_owned()
        }])
    );
}

#[test]
fn a_tool_call_arrives_running() {
    let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"c1","name":"Bash","input":{"cmd":"ls"}}]}}"#;
    let Read::Parts(parts) = Claude.read(line) else {
        panic!("expected parts");
    };
    assert!(matches!(
        &parts[0],
        Part::ToolCall { id, name, state, .. }
            if id == "c1" && name == "Bash" && *state == CallState::Running
    ));
}

#[test]
fn a_tool_result_points_at_its_call() {
    let line = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"c1","content":"ok"}]}}"#;
    assert_eq!(
        Claude.read(line),
        Read::Parts(vec![Part::ToolResult {
            call_id: "c1".to_owned(),
            output: "ok".to_owned(),
            is_error: false,
        }])
    );
}

#[test]
fn the_end_carries_the_clis_own_reason() {
    let line = r#"{"type":"result","subtype":"success","total_cost_usd":0.42,"is_error":false}"#;
    assert_eq!(
        Claude.read(line),
        Read::Ended {
            stop_reason: Some("success".to_owned()),
            cost_usd: Some(0.42),
            is_error: false,
        }
    );
}

#[test]
fn a_driver_that_is_not_installed_is_absent_rather_than_swapped() {
    // Falling back to another provider would change what the conversation is.
    assert!(driver("claude").is_some());
    assert!(driver("codex").is_none());
}
