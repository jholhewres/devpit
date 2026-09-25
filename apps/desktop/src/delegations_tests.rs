use super::{threads, woken_label};

const LOG: &str = r#"{"type":"assistant","timestamp":"2026-09-25T18:11:55Z","message":{"content":[{"type":"tool_use","name":"SendMessage","input":{"to":"gatorclaw-82","summary":"Build and deploy dev","message":"Update master and deploy."}}]}}
{"type":"user","isMeta":true,"timestamp":"2026-09-25T18:13:45Z","origin":{"kind":"peer","name":"gatorclaw-82","body":"Build ready.\nNot deployed."},"message":{"content":"Another Claude session sent a message"}}"#;

#[test]
fn each_session_written_to_or_heard_from_has_its_history() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("a.jsonl"), LOG).expect("log");
    let found = threads(dir.path());
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "gatorclaw-82");
    let kinds: Vec<&str> = found[0]
        .events
        .iter()
        .map(|one| one.kind.as_str())
        .collect();
    assert_eq!(kinds, ["sent", "heard"]);
    assert_eq!(
        found[0].events[0].summary.as_deref(),
        Some("Build and deploy dev")
    );
    assert_eq!(found[0].last_at, "2026-09-25T18:13:45Z");
}

#[test]
fn a_woken_turn_says_who_woke_it_and_quotes_them() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("a.jsonl"), LOG).expect("log");
    let said = woken_label(dir.path());
    assert!(said.starts_with("↪ *`gatorclaw-82` wrote:*"), "{said}");
    assert!(said.contains("> Build ready.\n> Not deployed."), "{said}");
    // Nothing in the log: every way it could have been.
    let empty = tempfile::tempdir().expect("tempdir");
    assert!(woken_label(empty.path()).contains("Not from this chat"));
}
