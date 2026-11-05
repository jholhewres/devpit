use devpit_rpc::Board;
use serde_json::json;

use super::{
    context_in, first_message, hand_off_reason, handoff_percent, mark, next_card, past_share,
    relays, relieved, replaced_id, still_allowed,
};

fn column(id: &str, position: i32, role: &str) -> serde_json::Value {
    json!({
        "id": id, "name": id, "position": position, "step": null, "onPass": null,
        "autonomy": "manual", "role": role, "roleChosen": false,
    })
}

fn card(id: &str, column: &str, parent: Option<&str>, position: i32) -> serde_json::Value {
    json!({
        "id": id, "columnId": column, "title": id, "body": "", "position": position,
        "worktreePath": null, "dueAt": null, "dueTime": false, "costUsd": 0.0,
        "comments": 0, "pinned": 0, "runs": [], "activity": null, "parentId": parent,
    })
}

fn board(cards: Vec<serde_json::Value>) -> Board {
    let columns = vec![
        column("inbox", 0, "backlog"),
        column("doing", 1, "doing"),
        column("check", 2, "check"),
    ];
    serde_json::from_value(
        json!({ "projectId": "p", "columns": columns, "cards": cards, "steps": [] }),
    )
    .expect("a board")
}

#[test]
fn the_next_card_is_the_epics_first_waiting_card_no_unfinished_card_holds() {
    let mut blocked = card("blocked", "inbox", Some("epic"), 1);
    blocked["blockedBy"] = json!([{ "id": "x", "title": "X", "done": false }]);
    let mut freed = card("freed", "inbox", Some("epic"), 3);
    freed["blockedBy"] = json!([{ "id": "done", "title": "Done", "done": true }]);
    let board = board(vec![
        card("done", "check", Some("epic"), 0),
        blocked,
        card("started", "doing", Some("epic"), 2),
        freed,
        card("later", "inbox", Some("epic"), 4),
        card("elsewhere", "inbox", Some("other"), 0),
    ]);
    assert_eq!(
        next_card(&board, "done").map(|c| c.id.as_str()),
        Some("freed")
    );
}

#[test]
fn a_card_with_no_epic_opens_nothing() {
    let board = board(vec![
        card("alone", "check", None, 0),
        card("waiting", "inbox", None, 1),
    ]);
    assert!(next_card(&board, "alone").is_none());
}

#[test]
fn devpit_starts_no_session_on_its_own_unless_the_person_turns_it_on() {
    assert_eq!(handoff_percent(None), None);
    assert_eq!(handoff_percent(Some(" ")), None);
    assert_eq!(handoff_percent(Some("off")), None);
    assert_eq!(handoff_percent(Some("0")), None);
    assert_eq!(handoff_percent(Some("lots")), None);
    assert_eq!(handoff_percent(Some("70")), Some(70));
    assert!(!relays(None));
    assert!(!relays(Some("off")));
    assert!(relays(Some("on")));
}

#[test]
fn a_session_is_past_its_share_at_three_hundred_thousand_tokens_of_a_million() {
    assert!(!past_share(299_999, 30));
    assert!(past_share(300_000, 30));
}

#[test]
fn the_context_is_the_newest_main_thread_answer_with_its_cache() {
    let tail = [
        json!({ "type": "assistant", "message": { "usage": { "input_tokens": 1, "cache_read_input_tokens": 100_000 } } }),
        json!({ "type": "assistant", "message": { "usage": { "input_tokens": 5, "cache_read_input_tokens": 250_000, "cache_creation_input_tokens": 2_000 } } }),
        json!({ "type": "assistant", "isSidechain": true, "message": { "usage": { "input_tokens": 9 } } }),
        json!({ "type": "user", "message": { "content": "go" } }),
    ]
    .iter()
    .map(|row| row.to_string())
    .collect::<Vec<_>>()
    .join("\n");
    // A line cut by the tail's start is skipped, not misread.
    let tail = format!("{{\"type\":\"assi\n{tail}\n");
    assert_eq!(context_in(&tail), Some(252_005));
    assert_eq!(context_in("{\"type\":\"user\"}"), None);
}

#[test]
fn a_session_past_its_share_may_still_read_its_card_and_leave_the_handoff() {
    assert!(still_allowed("mcp__devpit__devpit_handoff"));
    assert!(still_allowed("devpit_card"));
    assert!(!still_allowed("Bash"));
    assert!(!still_allowed("mcp__devpit__devpit_board"));
    assert!(!still_allowed("mcp__evil__not_devpit_handoff_really"));
    let said = hand_off_reason(312_400, 30);
    assert!(said.contains("312k") && said.contains("30%") && said.contains("devpit_handoff"));
}

#[test]
fn a_handoff_relieves_the_session_told_to_leave_it_once() {
    assert_eq!(relieved("card_relay_never_marked"), None);
    mark("card_relay_marked", "/p/1f3c-session.jsonl");
    let transcript = relieved("card_relay_marked").expect("relieved");
    // The relief stops the session it replaces, by the id it is named after.
    assert_eq!(replaced_id(&transcript), Some("1f3c-session"));
    assert_eq!(relieved("card_relay_marked"), None);
    // Told again after its handoff, the same session opens no second one.
    mark("card_relay_marked", "/p/1f3c-session.jsonl");
    assert_eq!(relieved("card_relay_marked"), None);
}

#[test]
fn the_next_session_is_told_its_card_and_the_one_finished_before_it() {
    let board = board(vec![card("card_next", "inbox", Some("epic"), 0)]);
    let said = first_message(&board.cards[0], Some("card_prev"), true);
    assert!(said.contains("card_next") && said.contains("card_prev"));
    assert!(said.contains("One card per session"));
    // With the next card left to the orchestrator, nothing tells it to stop.
    let said = first_message(&board.cards[0], None, false);
    assert!(!said.contains("One card per session") && !said.contains("end the session"));
}
