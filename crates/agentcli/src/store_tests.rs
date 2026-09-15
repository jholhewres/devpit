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
