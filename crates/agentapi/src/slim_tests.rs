use serde_json::{json, Value};

use super::slimmed;

/// The reported case: a Read of a big file posted the whole file back.
#[test]
fn a_read_comes_back_without_the_file_it_read() {
    let file = "x".repeat(5 * 1024 * 1024);
    let body = json!({
        "hook_event_name": "PostToolUse",
        "session_id": "s1",
        "cwd": "/w/app",
        "tool_name": "Read",
        "tool_input": { "file_path": "/w/app/big.log" },
        "tool_response": { "file": { "content": file } },
    })
    .to_string();
    let slim = slimmed(&body).expect("json");
    assert!(slim.len() < 4 * 1024, "{} bytes", slim.len());
    let value: Value = serde_json::from_str(&slim).expect("json");
    assert_eq!(value["tool_name"], "Read");
    assert_eq!(value["tool_input"]["file_path"], "/w/app/big.log");
    assert!(value.get("tool_response").is_none());
}

/// An Agent call's response is what says which subagent it started.
#[test]
fn an_agent_call_keeps_its_response() {
    let body = json!({
        "hook_event_name": "PostToolUse",
        "tool_name": "Agent",
        "tool_response": { "agentId": "a1", "description": "look around" },
    })
    .to_string();
    let value: Value = serde_json::from_str(&slimmed(&body).expect("json")).expect("json");
    assert_eq!(value["tool_response"]["agentId"], "a1");
}

/// A Write keeps its input, cut to what any view of it would show.
#[test]
fn long_strings_and_arrays_are_cut() {
    let body = json!({
        "tool_name": "Write",
        "tool_input": { "content": "é".repeat(40_000), "list": vec![1; 1000] },
    })
    .to_string();
    let value: Value = serde_json::from_str(&slimmed(&body).expect("json")).expect("json");
    let content = value["tool_input"]["content"].as_str().expect("text");
    assert_eq!(
        content.chars().count(),
        16 * 1024 + 1,
        "cut on a character, and marked"
    );
    assert!(content.ends_with('…'));
    assert_eq!(
        value["tool_input"]["list"].as_array().expect("list").len(),
        256
    );
}

#[test]
fn what_is_not_json_is_not_read() {
    assert_eq!(slimmed("not json"), None);
}
