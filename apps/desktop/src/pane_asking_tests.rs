use devpit_rpc::IslandVerdict;
use serde_json::json;

use super::*;

/// Shaped like the payload the CLI sends, the fields this reads.
const ASKED: &str = r#"{"hook_event_name":"PermissionRequest","session_id":"s1","cwd":"/w",
    "tool_name":"Bash","tool_input":{"command":"rm -rf build"},
    "permission_suggestions":[{"type":"addRules","rules":[{"toolName":"Bash","ruleContent":"rm -rf build"}],"behavior":"allow","destination":"localSettings"}]}"#;

#[test]
fn a_permission_question_is_told_apart_from_other_hooks() {
    assert!(is_permission_request(ASKED));
    assert!(!is_permission_request(
        r#"{"hook_event_name":"PreToolUse","session_id":"s1"}"#
    ));
    // Named in a command, not the event: not a question.
    assert!(!is_permission_request(
        r#"{"hook_event_name":"PreToolUse","tool_input":{"command":"echo \"PermissionRequest\""}}"#
    ));
}

/// Never a denial by default: what is not decided here the terminal asks.
#[test]
fn no_answer_is_no_decision() {
    assert_eq!(reply_for(None, None), "");
    assert_eq!(reply_for(Some(IslandVerdict::InTerminal), None), "");
}

#[test]
fn an_answer_is_the_decision_the_cli_reads() {
    let allow: serde_json::Value =
        serde_json::from_str(&reply_for(Some(IslandVerdict::Allow), None)).expect("json");
    assert_eq!(
        allow["hookSpecificOutput"]["hookEventName"],
        "PermissionRequest"
    );
    assert_eq!(allow["hookSpecificOutput"]["decision"]["behavior"], "allow");

    let deny: serde_json::Value =
        serde_json::from_str(&reply_for(Some(IslandVerdict::Deny), None)).expect("json");
    assert_eq!(deny["hookSpecificOutput"]["decision"]["behavior"], "deny");
}

/// "Always" keeps the rule the CLI itself suggested, and nothing devpit
/// made up.
#[test]
fn always_keeps_the_rule_the_cli_suggested() {
    let rules = json!([{ "type": "addRules", "destination": "localSettings" }]);
    let always: serde_json::Value =
        serde_json::from_str(&reply_for(Some(IslandVerdict::Always), Some(&rules))).expect("json");
    assert_eq!(
        always["hookSpecificOutput"]["decision"]["updatedPermissions"],
        rules
    );

    let bare: serde_json::Value =
        serde_json::from_str(&reply_for(Some(IslandVerdict::Always), None)).expect("json");
    assert!(bare["hookSpecificOutput"]["decision"]
        .get("updatedPermissions")
        .is_none());
}

#[test]
fn an_answer_counts_only_once_the_question_was_seen() {
    let (seen, seen_rx) = std::sync::mpsc::channel();
    let (said, said_rx) = std::sync::mpsc::channel();
    said.send(IslandVerdict::Allow).expect("send");
    drop(seen);
    assert_eq!(answered(&seen_rx, &said_rx), None);

    let (seen, seen_rx) = std::sync::mpsc::channel();
    seen.send(()).expect("send");
    assert_eq!(answered(&seen_rx, &said_rx), Some(IslandVerdict::Allow));
}
