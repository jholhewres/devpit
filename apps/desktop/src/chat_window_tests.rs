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

#[test]
fn a_conversation_id_that_reaches_out_of_its_folder_is_refused() {
    assert!(super::conversation("8ee138cc-6ff4-4421-bf59-80570ca59450").is_ok());
    for bad in ["../other/sessions/x", "/etc/passwd", "a/b", "", "a.b"] {
        assert!(super::conversation(bad).is_err(), "{bad}");
    }
}

#[test]
fn the_history_asked_without_a_page_is_all_of_it_and_with_one_the_last_page() {
    use devpit_agentcli::store::{append, conversation_path};
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().canonicalize().expect("root");
    let store = crate::projects::store().expect("store");
    let project = store.add_project(&root, None).expect("project");
    let sessions = crate::projects::project_home(&project)
        .expect("home")
        .sessions();
    let id = "conv_paged_history";
    for at in 0..5 {
        let text = Part::Text {
            text: format!("said {at}"),
            parent: None,
        };
        append(
            &conversation_path(&sessions, id),
            &message(&format!("m{at}"), vec![text]),
        )
        .expect("append");
    }
    let ids = |read: &devpit_rpc::Conversation| {
        read.messages
            .iter()
            .map(|one| one.id.clone())
            .collect::<Vec<_>>()
    };

    // As every caller before paging asked: the whole conversation, none left.
    let all = crate::chat::chat_history_now(project.clone(), id.to_owned()).expect("all");
    assert_eq!(ids(&all), ["m0", "m1", "m2", "m3", "m4"]);
    assert!(!all.older);

    let last =
        super::chat_page_now(project.clone(), id.to_owned(), Some((2, None))).expect("last page");
    assert_eq!(ids(&last), ["m3", "m4"]);
    assert!(last.older);
    let before =
        super::chat_page_now(project, id.to_owned(), Some((2, Some("m3")))).expect("page before");
    assert_eq!(ids(&before), ["m1", "m2"]);
    assert!(before.older);
}
