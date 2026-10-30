use devpit_rpc::{Message, Part, Role};

use super::whole_output;

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
