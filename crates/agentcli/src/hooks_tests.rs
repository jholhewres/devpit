//! Payloads recorded from the CLI, not written from documentation.

use super::*;

/// Captured by pointing a PreToolUse hook at `cat`.
const PRE_TOOL: &str = r#"{"session_id":"abc","cwd":"/tmp/p","hook_event_name":"PreToolUse",
    "tool_name":"Bash","tool_input":{"command":"echo hi"},"tool_use_id":"t1",
    "permission_mode":"bypassPermissions","prompt_id":"p1","transcript_path":"/x.jsonl"}"#;

/// The same, for Stop — which carries what the agent last said.
const STOP: &str = r#"{"session_id":"abc","cwd":"/tmp/p","hook_event_name":"Stop",
    "last_assistant_message":"Done.","stop_hook_active":false,"prompt_id":"p1",
    "permission_mode":"bypassPermissions","transcript_path":"/x.jsonl",
    "background_tasks":[],"session_crons":[]}"#;

#[test]
fn a_tool_about_to_run_is_named() {
    let happening = read(PRE_TOOL).expect("read");
    assert_eq!(happening.session_id, "abc");
    assert_eq!(happening.cwd, "/tmp/p");
    assert_eq!(
        happening.event,
        Event::Using {
            tool: "Bash".to_owned()
        }
    );
}

#[test]
fn a_turn_that_ended_carries_the_last_thing_it_said() {
    assert_eq!(
        read(STOP).expect("read").event,
        Event::Stopped {
            said: Some("Done.".to_owned())
        }
    );
}

/// The state that matters most: nothing moves until a person comes back.
#[test]
fn a_notification_is_the_agent_waiting_on_someone() {
    let payload = r#"{"session_id":"abc","hook_event_name":"Notification","message":"needs you"}"#;
    assert_eq!(read(payload).expect("read").event, Event::Waiting);
}

/// The CLI is free to send more events. A receiver that fails on an unknown
/// name is a receiver that breaks on an upgrade.
#[test]
fn an_event_this_build_has_no_use_for_is_ignored() {
    for payload in [
        r#"{"session_id":"abc","hook_event_name":"PreCompact"}"#,
        r#"{"session_id":"abc","hook_event_name":"SomethingNew"}"#,
        r#"{"session_id":"abc","hook_event_name":"PreToolUse"}"#,
        "not json",
    ] {
        assert_eq!(read(payload), None, "{payload}");
    }
}

/// Without a session id there is no card to tell.
#[test]
fn a_payload_with_no_session_is_dropped() {
    assert_eq!(read(r#"{"hook_event_name":"Stop"}"#), None);
}
