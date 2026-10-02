use serde_json::{json, Value};

use super::{agent_name, parsed, payload};

#[test]
fn an_agent_is_named_plainly_and_never_as_claude() {
    assert!(agent_name("aider"));
    assert!(agent_name("my-ci-2"));
    for wrong in [
        "",
        "Aider",
        "has space",
        "x".repeat(25).as_str(),
        "claude",
        "ünï",
    ] {
        assert!(!agent_name(wrong), "{wrong}");
    }
}

#[test]
fn a_call_is_read_and_a_permission_question_refused() {
    let call = parsed(
        &[
            ("agent", "aider"),
            ("tool", "Edit"),
            ("target", "src/app.rs"),
        ],
        &["PreToolUse"],
    )
    .expect("call");
    assert_eq!(call.agent, "aider");
    assert_eq!(call.event, "PreToolUse");
    assert!(parsed(&[("agent", "aider")], &["PermissionRequest"]).is_err());
    assert!(parsed(&[("agent", "aider")], &["Teleport"]).is_err());
    assert!(parsed(&[], &["Stop"]).is_err(), "no agent");
    assert!(parsed(&[("agent", "claude")], &["Stop"]).is_err());
}

/// What is sent is a Claude Code payload, which devpit already reads.
#[test]
fn what_is_sent_reads_as_a_claude_code_hook() {
    let call = parsed(
        &[
            ("agent", "aider"),
            ("session", "aider-1"),
            ("tool", "Edit"),
            ("target", "src/app.rs"),
        ],
        &["PreToolUse"],
    )
    .expect("call");
    let body = payload(&call, &Value::Null, "/w/app");
    let happening = devpit_agentcli_shape(&body);
    assert_eq!(happening["hook_event_name"], "PreToolUse");
    assert_eq!(happening["session_id"], "aider-1");
    assert_eq!(happening["cwd"], "/w/app");
    assert_eq!(happening["tool_name"], "Edit");
    assert_eq!(happening["tool_input"]["file_path"], "src/app.rs");
    assert_eq!(happening["devpit_agent"], "aider");

    let stop = parsed(
        &[
            ("agent", "aider"),
            ("session", "aider-1"),
            ("said", "Tests pass"),
        ],
        &["Stop"],
    )
    .expect("call");
    assert_eq!(
        devpit_agentcli_shape(&payload(&stop, &Value::Null, "/w"))["last_assistant_message"],
        "Tests pass"
    );
}

/// JSON piped in is kept, and the command line wins over it.
#[test]
fn piped_json_is_the_base_and_the_flags_win() {
    let call = parsed(&[("agent", "aider"), ("tool", "Bash")], &["PreToolUse"]).expect("call");
    let piped = json!({ "session_id": "from-pipe", "tool_name": "Ignored", "tool_input": { "command": "npm test" } });
    let body = devpit_agentcli_shape(&payload(&call, &piped, "/w"));
    assert_eq!(body["session_id"], "from-pipe");
    assert_eq!(body["tool_name"], "Bash");
    assert_eq!(body["tool_input"]["command"], "npm test");
}

fn devpit_agentcli_shape(body: &str) -> Value {
    serde_json::from_str(body).expect("json")
}
