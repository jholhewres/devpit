use devpit_rpc::{Message, Part, Role};

use super::{found, prompts, OPENING};

fn said(id: &str, role: Role, parts: Vec<Part>) -> Message {
    Message {
        id: id.to_owned(),
        turn_id: None,
        role,
        parts,
        created_at: 0.0,
        streaming: false,
        ended: None,
    }
}

fn text(text: &str) -> Part {
    Part::Text {
        text: text.to_owned(),
        parent: None,
    }
}

#[test]
fn a_word_is_found_in_every_message_that_says_it_ignoring_case() {
    let messages = vec![
        said("m1", Role::User, vec![text("Where is the Parser?")]),
        said("m2", Role::Assistant, vec![text("nothing here")]),
        said(
            "m3",
            Role::Assistant,
            vec![text("parser, parser"), text("PARSER")],
        ),
    ];
    let hits = found(&messages, "  parser ");
    assert_eq!(
        hits.iter()
            .map(|hit| (hit.message_id.as_str(), hit.count))
            .collect::<Vec<_>>(),
        [("m1", 1), ("m3", 3)]
    );
}

#[test]
fn thinking_is_not_searched_and_an_empty_query_finds_nothing() {
    let messages = vec![said(
        "m1",
        Role::Assistant,
        vec![Part::Thinking {
            text: "parser".to_owned(),
            parent: None,
            duration_ms: None,
        }],
    )];
    assert!(found(&messages, "parser").is_empty());
    assert!(found(&[said("m2", Role::User, vec![text("x")])], "   ").is_empty());
}

#[test]
fn the_prompts_are_the_persons_messages_where_they_sit() {
    let messages = vec![
        said("m1", Role::User, vec![text("first\n\n  question")]),
        said("m2", Role::Assistant, vec![text("answer")]),
        said("m3", Role::User, vec![text(&"long ".repeat(100))]),
    ];
    let read = prompts(&messages);
    assert_eq!(read.total, 3);
    assert_eq!(read.prompts[0].text, "first question");
    assert_eq!(read.prompts[1].at, 2);
    assert_eq!(read.prompts[1].text.chars().count(), OPENING);
}

#[test]
fn only_what_the_window_draws_is_counted() {
    let messages = vec![
        said(
            "m1",
            Role::User,
            vec![text("@/home/me/parser.png @/tmp/a.txt\nfix the parser")],
        ),
        said(
            "m2",
            Role::User,
            vec![text("/review\n@/home/me/parser.rs\nthe parser again")],
        ),
        said(
            "m3",
            Role::Assistant,
            vec![
                Part::Text {
                    text: "a subagent read the parser".to_owned(),
                    parent: Some("call_1".to_owned()),
                },
                text("I will check the parser first."),
            ],
        ),
    ];
    assert_eq!(
        found(&messages, "parser")
            .iter()
            .map(|hit| (hit.message_id.as_str(), hit.count))
            .collect::<Vec<_>>(),
        [("m1", 1), ("m2", 1), ("m3", 1)]
    );
    assert_eq!(prompts(&messages).prompts[0].text, "fix the parser");
    assert_eq!(
        prompts(&messages).prompts[1].text,
        "/review the parser again"
    );
}
