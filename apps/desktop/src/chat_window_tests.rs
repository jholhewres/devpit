use devpit_rpc::{Message, Part, Role};

use super::{messages_after, whole_output};

fn result(call: &str, output: &str) -> Part {
    Part::ToolResult {
        call_id: call.to_owned(),
        output: output.to_owned(),
        is_error: false,
        parent: None,
    }
}

fn message(id: &str, parts: Vec<Part>) -> Message {
    Message {
        id: id.to_owned(),
        turn_id: None,
        role: Role::Assistant,
        parts,
        created_at: 0.0,
        streaming: false,
    }
}

#[test]
fn the_whole_output_is_the_one_answering_that_call_in_that_message() {
    let long = "z".repeat(50_000);
    let messages = vec![
        message("m1", vec![result("call_1", "other message")]),
        message(
            "m2",
            vec![result("call_0", "another call"), result("call_1", &long)],
        ),
    ];
    let Some(Part::ToolResult { output, .. }) = whole_output(messages.clone(), "m2", "call_1")
    else {
        panic!("the output");
    };
    assert_eq!(output, long);
    assert_eq!(whole_output(messages.clone(), "m2", "call_9"), None);
    assert_eq!(whole_output(messages, "m9", "call_1"), None);
}

#[test]
fn a_conversation_id_that_reaches_out_of_its_folder_is_refused() {
    assert!(super::conversation("8ee138cc-6ff4-4421-bf59-80570ca59450").is_ok());
    for bad in ["../other/sessions/x", "/etc/passwd", "a/b", "", "a.b"] {
        assert!(super::conversation(bad).is_err(), "{bad}");
    }
}

#[test]
fn what_came_after_a_message_is_read_cut_and_all_of_it_when_the_message_is_gone() {
    let long = "w".repeat(9_000);
    let messages = vec![
        message("m1", vec![]),
        message("m2", vec![]),
        message("m3", vec![result("call_1", &long)]),
    ];
    let ids = |read: &[Message]| read.iter().map(|one| one.id.clone()).collect::<Vec<_>>();
    let after = messages_after(messages.clone(), Some("m2"));
    assert_eq!(ids(&after), ["m3"]);
    let Part::ToolResult { output, .. } = &after[0].parts[0] else {
        panic!("the result");
    };
    assert!(output.len() < long.len(), "cut as the history is");
    assert_eq!(
        ids(&messages_after(messages.clone(), Some("m3"))),
        Vec::<String>::new()
    );
    assert_eq!(
        ids(&messages_after(messages.clone(), Some("gone"))),
        ["m1", "m2", "m3"]
    );
    assert_eq!(ids(&messages_after(messages, None)), ["m1", "m2", "m3"]);
}
