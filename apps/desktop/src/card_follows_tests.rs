use devpit_rpc::Board;
use serde_json::json;

use super::{column_with, when_started};

fn column(id: &str, position: i32, role: Option<&str>, step: bool) -> serde_json::Value {
    json!({
        "id": id, "name": id, "position": position,
        "step": step.then(|| json!({ "id": "st_1", "kind": "agent", "name": "Review", "config": "{}", "irreversible": false })),
        "onPass": null, "autonomy": "manual", "role": role, "roleChosen": false,
    })
}

fn card(id: &str, column: &str) -> serde_json::Value {
    json!({
        "id": id, "columnId": column, "title": id, "body": "", "position": 0,
        "worktreePath": null, "dueAt": null, "dueTime": false, "costUsd": 0.0,
        "comments": 0, "pinned": 0, "runs": [], "activity": null,
    })
}

fn board(columns: Vec<serde_json::Value>, cards: Vec<serde_json::Value>) -> Board {
    serde_json::from_value(
        json!({ "projectId": "p", "columns": columns, "cards": cards, "steps": [] }),
    )
    .expect("a board")
}

#[test]
fn a_role_is_found_whatever_the_column_is_called_and_never_one_with_a_step() {
    let board = board(
        vec![
            column("inbox", 0, Some("backlog"), false),
            column("ai-review", 1, Some("doing"), true),
            column("doing", 2, Some("doing"), false),
            column("check", 3, Some("check"), false),
        ],
        vec![],
    );
    assert_eq!(
        column_with(&board, "doing").map(|one| one.id.as_str()),
        Some("doing")
    );
    assert_eq!(
        column_with(&board, "check").map(|one| one.id.as_str()),
        Some("check")
    );
    assert!(column_with(&board, "done").is_none());
}

/// Work starting moves a waiting card, and only a waiting one: a card already
/// in progress, being checked or done stays where it is.
#[test]
fn only_a_waiting_card_moves_when_its_work_starts() {
    let board = board(
        vec![
            column("inbox", 0, Some("backlog"), false),
            column("doing", 1, Some("doing"), false),
            column("check", 2, Some("check"), false),
            column("ideas", 3, None, false),
        ],
        vec![
            card("waiting", "inbox"),
            card("checking", "check"),
            card("loose", "ideas"),
        ],
    );
    assert_eq!(
        when_started(&board, "waiting").map(|one| one.id.as_str()),
        Some("doing")
    );
    assert!(when_started(&board, "checking").is_none());
    assert!(
        when_started(&board, "loose").is_none(),
        "a column with no role is not a waiting one"
    );
    assert!(when_started(&board, "missing").is_none());
}
