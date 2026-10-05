use std::io::Write;

use super::*;

fn said(id: &str, request: &str, output: u64) -> String {
    format!(
        r#"{{"type":"assistant","requestId":"{request}","sessionId":"s","cwd":"/w","timestamp":"2026-10-05T10:00:00Z","message":{{"id":"{id}","model":"claude-sonnet-4-5","usage":{{"input_tokens":1000,"output_tokens":{output}}}}}}}"#
    )
}

const ASKED: &str = r#"{"type":"user","message":{"role":"user","content":"hello"}}"#;
const TOOL: &str = r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t","content":"ok"}]}}"#;

fn append(path: &Path, text: &str) {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("open");
    file.write_all(text.as_bytes()).expect("write");
}

#[test]
fn it_sums_what_was_added_since_the_last_read_and_counts_a_message_once() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("s.jsonl");
    append(
        &path,
        &format!(
            "{ASKED}\n{}\n{}\n",
            said("m1", "r1", 500),
            said("m1", "r1", 500)
        ),
    );
    let mut tally = Tally::default();
    tally.advance(&path).expect("read");
    let first = tally.cost_usd;
    assert!(first > 0.0);
    assert_eq!(tally.output, 500, "the same message was counted twice");
    assert_eq!(tally.before_usd, Some(first));

    append(&path, &format!("{TOOL}\n{}\n", said("m2", "r2", 100)));
    tally.advance(&path).expect("read");
    assert_eq!(tally.output, 600);
    assert!(
        tally.last_turn_usd > first,
        "a tool result is not a new turn"
    );
    assert_eq!(
        tally.before_usd,
        Some(first),
        "what came after it is not history"
    );
    assert_eq!(tally.model.as_deref(), Some("claude-sonnet-4-5"));

    append(&path, &format!("{ASKED}\n{}\n", said("m3", "r3", 10)));
    tally.advance(&path).expect("read");
    assert!(
        tally.last_turn_usd < first,
        "the last turn did not start again at a prompt"
    );
}

#[test]
fn a_line_still_being_written_waits_for_its_end() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("s.jsonl");
    let line = said("m1", "r1", 500);
    let (head, tail) = line.split_at(40);
    append(&path, head);
    let mut tally = Tally::default();
    tally.advance(&path).expect("read");
    assert_eq!(tally.output, 0);
    append(&path, &format!("{tail}\n"));
    tally.advance(&path).expect("read");
    assert_eq!(tally.output, 500);
}
