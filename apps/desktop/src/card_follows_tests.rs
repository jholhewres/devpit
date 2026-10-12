use devpit_core::NewCard;
use devpit_devflow::Ask;
use devpit_rpc::Board;
use serde_json::json;

use super::{column_with, start, unblocked, when_started};

#[test]
fn a_card_blocked_by_one_not_done_is_refused_only_with_devflow_on() {
    let mut blocked = card("second", "inbox");
    blocked["blockedBy"] = json!([
        { "id": "first", "title": "Index the bodies", "done": false },
        { "id": "zero", "title": "Set up", "done": true }
    ]);
    let mut freed = card("third", "inbox");
    freed["blockedBy"] = json!([{ "id": "zero", "title": "Set up", "done": true }]);
    let board = board(
        vec![
            column("inbox", 0, Some("backlog"), false),
            column("doing", 1, Some("doing"), false),
        ],
        vec![blocked, freed],
    );
    let refused = unblocked(&board, "second", true).expect_err("blocked");
    assert_eq!(
        refused,
        "blocked by \"Index the bodies\" (first): finish it first, or ask the person to change the plan"
    );
    // A blocker that is done holds nothing back.
    assert_eq!(unblocked(&board, "third", true), Ok(()));
    // Without Devflow the board behaves as it did.
    assert_eq!(unblocked(&board, "second", false), Ok(()));
}

/// Through `start_card` as an agent asks it, against the store: where the
/// gate lets a card through, the move that follows needs the window.
#[test]
fn starting_a_blocked_card_is_refused_where_devflow_is_on() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = crate::projects::store().expect("store");
    let project = store.add_project(dir.path(), None).expect("project");
    store.ensure_board(&project).expect("board");
    let first = store.columns(&project).expect("columns")[0].id.clone();
    let epic = store
        .create_card(&project, &first, "[devflow] Brief: search", "")
        .expect("epic");
    store.keep_plan(&epic, "[{},{}]", false).expect("plan");
    let cards = [
        NewCard {
            title: "[devflow] index",
            body: "",
            waits_on: &[],
        },
        NewCard {
            title: "[devflow] box",
            body: "",
            waits_on: &[0],
        },
    ];
    let ids = store
        .create_plan(&epic, &first, &cards, |_| String::new())
        .expect("create")
        .expect("made");
    let board = crate::board::board_get_now(project.clone()).expect("board");
    let went_on = |said: Result<serde_json::Value, String>| {
        said.expect_err("no window").contains("running app")
    };

    assert!(went_on(start(None, &board, &ids[1])), "Devflow is off here");

    std::fs::create_dir_all(dir.path().join(".devpit/devflow")).expect("folder");
    std::fs::write(
        dir.path().join(".devpit/devflow/devflow.yaml"),
        "version: 1\nenabled: true\n",
    )
    .expect("yaml");
    crate::devflow_asks::remember(&store, &project, Ask::Enable, true).expect("clicked");
    let refused = start(None, &board, &ids[1]).expect_err("blocked");
    assert!(
        refused.starts_with("blocked by \"[devflow] index\""),
        "{refused}"
    );
    assert!(went_on(start(None, &board, &ids[0])), "nothing blocks it");
}

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
