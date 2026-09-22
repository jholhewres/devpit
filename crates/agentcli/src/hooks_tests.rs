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

/// A subagent finishing is not the agent finishing.
///
/// Both used to read as `Stopped`, so the sidebar marked a pane done the moment
/// any subagent it started returned — while the agent itself was still at work.
#[test]
fn a_subagent_stopping_does_not_stop_the_agent() {
    let payload = r#"{"hook_event_name":"SubagentStop","session_id":"s1","cwd":"/work"}"#;
    let happening = read(payload).expect("a happening");
    assert_eq!(happening.event, Event::SubagentDone { agent: None });
    let stop = r#"{"hook_event_name":"Stop","session_id":"s1","cwd":"/work","last_assistant_message":"done"}"#;
    assert!(matches!(
        read(stop).expect("stop").event,
        Event::Stopped { .. }
    ));
}

/// Recorded from 2.1.270 with every hook pointed at `cat`, around one `Agent`
/// call. Ids kept, the prompt shortened.
const SUBAGENT_START: &str = r#"{"session_id":"abc","cwd":"/work","hook_event_name":"SubagentStart",
    "agent_id":"a82d987eefe13ba95","agent_type":"general-purpose","prompt_id":"p1",
    "transcript_path":"/x.jsonl"}"#;

const AGENT_LAUNCHED: &str = r#"{"session_id":"abc","cwd":"/work","hook_event_name":"PostToolUse",
    "tool_name":"Agent","tool_input":{"description":"probe child","prompt":"Reply ok."},
    "tool_response":{"isAsync":true,"status":"async_launched","agentId":"a82d987eefe13ba95",
    "description":"probe child","resolvedModel":"claude-haiku-4-5-20251001","prompt":"Reply ok."},
    "tool_use_id":"toolu_1","duration_ms":12,"permission_mode":"default","prompt_id":"p1",
    "transcript_path":"/x.jsonl"}"#;

/// Sent 16 times for this one subagent in the recording.
const SUBAGENT_STOP: &str = r#"{"session_id":"abc","cwd":"/work","hook_event_name":"SubagentStop",
    "agent_id":"a82d987eefe13ba95","agent_type":"general-purpose","agent_transcript_path":"/y.jsonl",
    "last_assistant_message":"ok","stop_hook_active":false,"background_tasks":[{"id":"a82d987eefe13ba95",
    "type":"subagent","status":"running","description":"probe child","agent_type":"general-purpose"}],
    "session_crons":[],"permission_mode":"default","prompt_id":"p1","transcript_path":"/x.jsonl"}"#;

#[test]
fn a_subagent_is_known_by_one_id_from_start_to_stop() {
    assert_eq!(
        read(SUBAGENT_START).expect("start").event,
        Event::SubagentStarted {
            agent: "a82d987eefe13ba95".to_owned(),
            kind: Some("general-purpose".to_owned()),
        }
    );
    assert_eq!(
        read(SUBAGENT_STOP).expect("stop").event,
        Event::SubagentDone {
            agent: Some("a82d987eefe13ba95".to_owned())
        }
    );
}

/// The call returns at launch, so it names the subagent and does not end it.
#[test]
fn a_launched_agent_call_names_its_subagent_and_its_model() {
    assert_eq!(
        read(AGENT_LAUNCHED).expect("launch").event,
        Event::Delegated {
            agent: "a82d987eefe13ba95".to_owned(),
            description: Some("probe child".to_owned()),
            model: Some("claude-haiku-4-5-20251001".to_owned()),
            ended: false,
        }
    );
}

#[test]
fn an_agent_call_that_ran_to_its_end_ends_its_subagent() {
    let finished = AGENT_LAUNCHED.replace("async_launched", "completed");
    assert!(matches!(
        read(&finished).expect("finished").event,
        Event::Delegated { ended: true, .. }
    ));
}

/// Other tools answer with strings; reading `Agent`'s shape must not drop them.
#[test]
fn a_tool_that_answers_with_a_string_is_still_used() {
    let payload = r#"{"session_id":"abc","hook_event_name":"PostToolUse","tool_name":"Bash",
        "tool_response":"hi"}"#;
    assert_eq!(
        read(payload).expect("used").event,
        Event::Used {
            tool: "Bash".to_owned()
        }
    );
}

/// Every payload names its transcript; its folder is the installation.
#[test]
fn a_hook_says_where_the_sessions_transcript_is() {
    assert_eq!(
        read(PRE_TOOL).expect("read").transcript_path.as_deref(),
        Some("/x.jsonl")
    );
    assert_eq!(
        read(r#"{"session_id":"abc","hook_event_name":"Stop"}"#)
            .expect("read")
            .transcript_path,
        None
    );
}

/// Recorded from an interactive 2.1.270 started in tmux and left with `/exit`.
const SESSION_START: &str = r#"{"session_id":"94e3457c-dcfa-4580-951a-dc9110a9580e","cwd":"/work",
    "hook_event_name":"SessionStart","source":"startup","model":"haiku","scratchpad_dir":"/s",
    "transcript_path":"/home/me/.claude/projects/-work/94e3457c-dcfa-4580-951a-dc9110a9580e.jsonl"}"#;

const SESSION_END: &str = r#"{"session_id":"94e3457c-dcfa-4580-951a-dc9110a9580e","cwd":"/work",
    "hook_event_name":"SessionEnd","reason":"prompt_input_exit","prompt_id":"p1","scratchpad_dir":"/s",
    "transcript_path":"/home/me/.claude/projects/-work/94e3457c-dcfa-4580-951a-dc9110a9580e.jsonl"}"#;

#[test]
fn a_session_is_heard_starting_and_ending_with_its_reason() {
    let started = read(SESSION_START).expect("start");
    assert_eq!(started.event, Event::SessionStarted);
    assert!(started.transcript_path.is_some());
    assert_eq!(
        read(SESSION_END).expect("end").event,
        Event::SessionEnded {
            reason: Some("prompt_input_exit".to_owned())
        }
    );
}

/// Only a notification that asks something is the agent waiting. `idle_prompt`
/// arrives a minute after a turn that already ended, and rang the bell as
/// though the agent were blocked.
#[test]
fn only_a_notification_that_asks_is_waiting() {
    let notified = |kind: &str| {
        read(&format!(
            r#"{{"session_id":"abc","hook_event_name":"Notification","message":"m","notification_type":"{kind}"}}"#
        ))
    };
    for asking in ["permission_prompt", "elicitation_dialog"] {
        assert_eq!(notified(asking).expect(asking).event, Event::Waiting);
    }
    for telling in ["idle_prompt", "auth_success"] {
        assert_eq!(notified(telling), None, "{telling}");
    }
}

/// A turn that answers in text alone sends nothing before its `Stop` but this.
#[test]
fn a_prompt_begins_a_turn() {
    let payload = r#"{"session_id":"abc","hook_event_name":"UserPromptSubmit","prompt":"hi","source":"user"}"#;
    assert_eq!(read(payload).expect("read").event, Event::Prompted);
}

/// No `Stop` follows a turn an error cut short.
#[test]
fn a_turn_that_failed_says_why() {
    let payload = r#"{"session_id":"abc","hook_event_name":"StopFailure","error":"rate_limit"}"#;
    assert_eq!(
        read(payload).expect("read").event,
        Event::Failed {
            error: Some("rate_limit".to_owned())
        }
    );
}

/// A compaction restarts the same session, often mid-turn: it is not a start.
#[test]
fn a_compaction_is_not_a_session_starting() {
    let started = |source: &str| {
        read(&format!(
            r#"{{"session_id":"abc","hook_event_name":"SessionStart","source":"{source}"}}"#
        ))
    };
    assert_eq!(started("compact"), None);
    for source in ["startup", "resume", "clear"] {
        assert_eq!(started(source).expect(source).event, Event::SessionStarted);
    }
}
