use std::path::Path;

use crate::head::{head_path, write_head, Head};
use crate::history::{conversations, title_of};
use crate::store::{append, conversation_path};
use devpit_rpc::{Message, Part, Role};

fn said(text: &str, role: Role) -> Message {
    Message {
        id: "m1".to_owned(),
        turn_id: None,
        role,
        parts: vec![Part::Text {
            text: text.to_owned(),
        }],
        created_at: 0.0,
        streaming: false,
    }
}

fn head(profile: &str, cost: f64) -> Head {
    Head {
        profile: profile.to_owned(),
        model: Some("opus".to_owned()),
        card_id: None,
        created_at: 0.0,
        cost_usd: cost,
        budget_usd: None,
        session_id: None,
        permission: None,
    }
}

fn conversation(home: &Path, id: &str, opening: &str, profile: &str, cost: f64) {
    let file = conversation_path(home, "proj", id);
    append(&file, &said(opening, Role::User)).expect("append");
    append(&file, &said("Sure.", Role::Assistant)).expect("append");
    write_head(&head_path(home, "proj", id), &head(profile, cost)).expect("head");
}

#[test]
fn a_conversation_is_named_by_what_the_person_said_first() {
    assert_eq!(title_of("Fix the login redirect"), "Fix the login redirect");
}

#[test]
fn only_the_first_line_becomes_the_title() {
    assert_eq!(
        title_of("Fix the login\n\nand also the logout"),
        "Fix the login"
    );
}

#[test]
fn a_long_opening_is_cut_and_says_it_was() {
    let title = title_of(&"x".repeat(200));
    assert!(title.ends_with('…'));
    assert!(title.chars().count() <= 73, "{}", title.chars().count());
}

#[test]
fn a_conversation_with_nothing_in_it_still_has_a_name() {
    assert_eq!(title_of("   \n  "), "Untitled");
}

#[test]
fn every_conversation_on_disk_is_listed_with_what_it_cost() {
    let home = tempfile::tempdir().expect("tempdir");
    conversation(home.path(), "c1", "Fix the login", "claude", 0.25);

    let found = conversations(home.path(), "proj");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, "c1");
    assert_eq!(found[0].title, "Fix the login");
    assert_eq!(found[0].profile, "claude");
    assert_eq!(found[0].cost_usd, 0.25);
    assert_eq!(found[0].model.as_deref(), Some("opus"));
}

/// The one you spoke in last is the one you are looking for.
#[test]
fn the_most_recent_comes_first() {
    let home = tempfile::tempdir().expect("tempdir");
    conversation(home.path(), "older", "The older one", "claude", 0.0);
    std::thread::sleep(std::time::Duration::from_millis(1100));
    conversation(home.path(), "newer", "The newer one", "claude", 0.0);

    let found = conversations(home.path(), "proj");
    assert_eq!(found[0].id, "newer", "the older one sorted first");
}

/// The assistant speaks in every conversation; the person's own words are
/// what they will recognise in a list.
#[test]
fn the_agents_reply_is_never_the_title() {
    let home = tempfile::tempdir().expect("tempdir");
    let file = conversation_path(home.path(), "proj", "c1");
    append(&file, &said("Working on it…", Role::Assistant)).expect("append");
    append(&file, &said("Fix the login", Role::User)).expect("append");

    assert_eq!(conversations(home.path(), "proj")[0].title, "Fix the login");
}

#[test]
fn a_project_that_has_said_nothing_lists_nothing() {
    let home = tempfile::tempdir().expect("tempdir");
    assert!(conversations(home.path(), "quiet").is_empty());
}

/// A transcript with no head is still a conversation: the head is written on
/// the first turn, and a crash before that must not hide what was said.
#[test]
fn a_transcript_with_no_head_is_still_listed() {
    let home = tempfile::tempdir().expect("tempdir");
    let file = conversation_path(home.path(), "proj", "c1");
    append(&file, &said("Only this", Role::User)).expect("append");

    let found = conversations(home.path(), "proj");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].title, "Only this");
    assert!(found[0].profile.is_empty());
}
