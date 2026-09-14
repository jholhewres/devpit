use super::*;

#[test]
fn a_persons_words_and_an_answers_text_are_found() {
    assert_eq!(
        said_on(
            r#"{"type":"user","message":{"role":"user","content":"why does the lexer drop tabs"}}"#
        ),
        Some(Said {
            role: "user",
            text: "why does the lexer drop tabs".to_owned()
        })
    );
    let answer = r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"hmm"},{"type":"text","text":"It trims them."},{"type":"tool_use","name":"Read","input":{}}]}}"#;
    assert_eq!(
        said_on(answer),
        Some(Said {
            role: "assistant",
            text: "It trims them.".to_owned()
        })
    );
}

#[test]
fn tool_output_thinking_attachments_and_system_lines_are_not() {
    for line in [
        r#"{"type":"user","message":{"content":[{"type":"tool_result","content":"3 files"}]}}"#,
        r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"secret plan"}]}}"#,
        r#"{"type":"attachment","attachment":{"content":"huge"}}"#,
        r#"{"type":"system","subtype":"init","model":"haiku"}"#,
        r#"{"type":"ai-title","aiTitle":"Fix the parser"}"#,
        "not json at all",
    ] {
        assert_eq!(said_on(line), None, "{line}");
    }
}

/// A line past the ceiling is skipped without being held, and the line after
/// it is still read.
#[test]
fn a_line_past_the_ceiling_does_not_hide_the_next_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("aaa.jsonl");
    let huge = format!(
        r#"{{"type":"user","message":{{"content":"{}"}}}}"#,
        "x".repeat(MOST_LINE_BYTES + 1)
    );
    std::fs::write(
        &path,
        format!(
            "{huge}\n{}\n",
            r#"{"type":"user","message":{"content":"kept after it"}}"#
        ),
    )
    .expect("transcript");
    let said = said_in(&path);
    assert_eq!(said.len(), 1);
    assert_eq!(said[0].text, "kept after it");
}

#[test]
fn a_missing_transcript_says_nothing() {
    assert!(said_in(Path::new("/no/such/transcript.jsonl")).is_empty());
}
