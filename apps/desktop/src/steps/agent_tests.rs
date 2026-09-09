use super::*;

#[test]
fn an_assistant_fragment_yields_its_words() {
    let line = r#"{"type":"assistant","message":{"content":[
        {"type":"text","text":"Looking at the parser"}]}}"#;
    assert_eq!(
        assistant_text(line).as_deref(),
        Some("Looking at the parser")
    );
}

/// Thinking blocks and tool calls are not words to show.
#[test]
fn a_fragment_with_no_text_yields_nothing() {
    let line = r#"{"type":"assistant","message":{"content":[
        {"type":"thinking","thinking":"..."}]}}"#;
    assert_eq!(assistant_text(line), None);
}

/// The machinery is not the work.
#[test]
fn the_other_lines_are_not_relayed() {
    for line in [
        r#"{"type":"system","subtype":"init"}"#,
        r#"{"type":"rate_limit_event"}"#,
        r#"{"type":"result","result":"done"}"#,
        "not json at all",
    ] {
        assert_eq!(assistant_text(line), None, "{line}");
    }
}
