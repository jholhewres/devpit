use devpit_core::CommentRow;

use super::{briefed, latest, note};

fn comment(body: &str, at: i64) -> CommentRow {
    CommentRow {
        id: format!("c{at}"),
        author: "agent".to_owned(),
        body: body.to_owned(),
        created_at: at,
        edited_at: None,
    }
}

#[test]
fn a_note_says_its_parts_and_leaves_out_the_empty_ones() {
    let said = note(
        "Added the migration.",
        "",
        "make test",
        "Touches prod data.",
    )
    .expect("note");
    assert!(said.starts_with("**Handoff**"));
    assert!(said.contains("**Done:**\nAdded the migration."));
    assert!(!said.contains("**Left:**"));
    assert!(said.contains("**Risks and decisions:**\nTouches prod data."));
    assert!(note("  ", "x", "", "").is_err());
}

#[test]
fn a_secret_in_a_note_does_not_reach_the_card() {
    let said = note("Ran it with KEY=sk-abcdefghijklmnopqrstuvwx", "", "", "").expect("note");
    assert!(!said.contains("sk-abc"));
    assert!(said.contains("KEY=[secret removed]"));
}

#[test]
fn the_next_session_is_briefed_with_the_latest_note() {
    let comments = [
        comment("**Handoff** — old", 1),
        comment("a plain comment", 2),
        comment("**Handoff** — new", 3),
        comment("another comment", 4),
    ];
    let note = latest(&comments);
    assert_eq!(note, Some("**Handoff** — new"));
    let brief = briefed("Finish the card.", note);
    assert!(brief.starts_with("Finish the card.\n\nWhere the last session left this card"));
    assert!(brief.ends_with("**Handoff** — new"));
    assert_eq!(briefed("Go.", latest(&comments[1..2])), "Go.");
}
