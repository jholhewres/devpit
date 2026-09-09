use crate::question::question_in;

/// Recorded from the CLI, like every other payload in these tests.
const PRE_TOOL: &str = r#"{"session_id":"abc","cwd":"/tmp/p","hook_event_name":"PreToolUse",
    "tool_name":"Bash","tool_input":{"command":"rm -rf /"},"tool_use_id":"t1",
    "permission_mode":"manual","prompt_id":"p1","transcript_path":"/x.jsonl"}"#;

const STOP: &str = r#"{"session_id":"abc","cwd":"/tmp/p","hook_event_name":"Stop",
    "last_assistant_message":"Done.","stop_hook_active":false}"#;

#[test]
fn a_tool_about_to_run_becomes_a_question() {
    let asked = question_in(PRE_TOOL).expect("a question");
    assert_eq!(asked.id, "t1");
    assert_eq!(asked.session_id, "abc");
    assert_eq!(asked.tool, "Bash");
    assert_eq!(asked.cwd, "/tmp/p");
}

/// Verbatim: parsing it here would be guessing at a shape the provider
/// changes without asking, and the screen only ever shows it.
#[test]
fn the_tools_input_comes_through_whole() {
    let asked = question_in(PRE_TOOL).expect("a question");
    assert!(asked.input.contains("rm -rf /"), "{}", asked.input);
}

/// Only `PreToolUse` can be answered. Holding the connection on the others
/// would stall the turn for a decision nobody can make.
#[test]
fn another_event_is_not_a_question() {
    assert!(question_in(STOP).is_none());
}

#[test]
fn a_payload_with_no_tool_use_id_is_not_a_question() {
    let nameless = r#"{"hook_event_name":"PreToolUse","tool_name":"Bash"}"#;
    assert!(
        question_in(nameless).is_none(),
        "a question with no id has nothing to answer"
    );
}

#[test]
fn something_that_is_not_json_is_not_a_question() {
    assert!(question_in("not json").is_none());
}
