use devpit_rpc::{Message, Part, Role};

use crate::store::{append, conversation_path, read};

fn message(id: &str, text: &str) -> Message {
    Message {
        id: id.to_owned(),
        turn_id: None,
        role: Role::Assistant,
        parts: vec![Part::Text {
            text: text.to_owned(),
            parent: None,
        }],
        created_at: 0.0,
        streaming: false,
        ended: None,
    }
}

#[test]
fn a_conversation_comes_back_in_order() {
    let home = tempfile::tempdir().expect("tempdir");
    let path = conversation_path(home.path(), "c1");
    append(&path, &message("m1", "one")).expect("append");
    append(&path, &message("m2", "two")).expect("append");

    let (messages, skipped) = read(&path);
    assert_eq!(
        messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
        ["m1", "m2"]
    );
    assert_eq!(skipped, 0);
}

#[test]
fn one_bad_line_does_not_take_the_conversation_with_it() {
    let home = tempfile::tempdir().expect("tempdir");
    let path = conversation_path(home.path(), "c1");
    append(&path, &message("m1", "one")).expect("append");
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .and_then(|mut file| std::io::Write::write_all(&mut file, b"{ not json\n"))
        .expect("write");
    append(&path, &message("m3", "three")).expect("append");

    let (messages, skipped) = read(&path);
    assert_eq!(messages.len(), 2);
    assert_eq!(skipped, 1);
}

#[test]
fn a_conversation_that_was_never_written_is_empty_rather_than_an_error() {
    let home = tempfile::tempdir().expect("tempdir");
    let (messages, skipped) = read(&conversation_path(home.path(), "never"));
    assert!(messages.is_empty());
    assert_eq!(skipped, 0);
}

#[test]
fn an_old_transcript_with_whole_outputs_reads_cut_and_is_not_rewritten() {
    // A line as devpit wrote it before outputs were cut for the window: a
    // screenshot's base64 inside a tool result.
    let dir = tempfile::tempdir().expect("dir");
    let path = dir.path().join("c.jsonl");
    let image = serde_json::json!([
        { "type": "image", "source": { "type": "base64", "media_type": "image/png", "data": "A".repeat(200_000) } }
    ])
    .to_string();
    let old = serde_json::json!({
        "id": "m1", "turnId": "t1", "role": "assistant", "createdAt": 1.0, "streaming": false,
        "parts": [
            { "kind": "tool_call", "id": "call_1", "name": "Screenshot", "input": "{}", "state": "ok" },
            { "kind": "tool_result", "call_id": "call_1", "output": image, "is_error": false }
        ]
    });
    std::fs::write(&path, format!("{old}\n")).expect("write");
    let before = std::fs::read(&path).expect("read");

    let (mut messages, skipped) = read(&path);
    assert_eq!(skipped, 0);
    messages.iter_mut().for_each(devpit_rpc::cut::cut_message);
    let devpit_rpc::Part::ToolResult { output, .. } = &messages[0].parts[1] else {
        panic!("the result");
    };
    assert!(output.len() < 200 && output.contains(r#""cut":true"#));

    assert_eq!(
        std::fs::read(&path).expect("read"),
        before,
        "the transcript was rewritten"
    );
}
