use devpit_rpc::{Message, Part, Role};

use super::*;

fn said(role: Role, text: &str) -> Message {
    Message {
        id: text.to_owned(),
        turn_id: None,
        role,
        parts: vec![Part::Text {
            text: text.to_owned(),
            parent: None,
        }],
        created_at: 0.0,
        streaming: false,
    }
}

#[test]
fn the_latest_replies_and_prompts_come_back_in_order() {
    let messages = vec![
        said(Role::User, "fix the bug"),
        said(Role::Assistant, "looking"),
        said(Role::User, "and test it"),
        said(Role::Assistant, "done, 3 tests pass"),
    ];
    let told = told(&messages, 1);
    assert_eq!(told["replies"], serde_json::json!(["done, 3 tests pass"]));
    assert_eq!(
        told["prompts"],
        serde_json::json!(["fix the bug", "and test it"])
    );
    assert_eq!(told["messages"], 4);
}

/// A reply can be a whole report; what comes back has a ceiling.
#[test]
fn a_long_reply_is_cut() {
    let long = "x".repeat(REPLY * 2);
    let told = told(&[said(Role::Assistant, &long)], 5);
    let reply = told["replies"][0].as_str().expect("a reply");
    assert_eq!(reply.chars().count(), REPLY + 1);
}
