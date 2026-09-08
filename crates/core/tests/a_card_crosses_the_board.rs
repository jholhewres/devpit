//! The whole path through the store, in the order a person walks it.
//!
//! The unit tests each prove one query. This proves they add up: a project
//! gets a board, a card is written, a lane is given a step, the card lands on
//! it, and the run that follows stays on the card with what it cost.

use quockpit_core::Store;

#[test]
fn a_card_crosses_the_board_and_the_runs_stay_on_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let project = store.add_project(dir.path(), None).expect("project");

    // A new project opens onto a board.
    store.ensure_board(&project).expect("board");
    let columns = store.columns(&project).expect("columns");
    assert_eq!(columns.len(), 6, "a new project has no board");

    // The lane the card will land on runs the tests.
    let step = store
        .create_step(
            &project,
            "command",
            "the tests",
            r#"{"command":"make test"}"#,
            false,
        )
        .expect("step");
    store
        .set_column_step(&columns[4].id, Some(&step))
        .expect("attach the step");

    // A card is written in the first lane.
    let card = store
        .create_card(&project, &columns[0].id, "Ship the board", "the body")
        .expect("card");
    assert_eq!(store.card_cost(&card).expect("cost"), 0.0);

    // And moved along it.
    store
        .move_card(&card, &columns[4].id, 0)
        .expect("move the card");
    let moved = store.card(&card).expect("read").expect("still there");
    assert_eq!(moved.column_id, columns[4].id);

    // The run that followed is on the card, with what it cost.
    let run = store.start_run(&card, &step).expect("start");
    store
        .finish_run(
            &run,
            "ok",
            Some("all green"),
            Some(0.25),
            Some(1_200),
            Some(0),
        )
        .expect("finish");

    let runs = store.runs(&card).expect("runs");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].state, "ok");
    assert_eq!(runs[0].exit_code, Some(0));
    assert_eq!(store.card_cost(&card).expect("cost"), 0.25);

    // The lane it came from can be renamed under it without anything breaking.
    store
        .rename_column(&columns[0].id, "somewhere else")
        .expect("rename");
    assert_eq!(store.runs(&card).expect("runs").len(), 1);

    // And the lane holding the card refuses to be deleted.
    assert!(
        store.delete_column(&columns[4].id).is_err(),
        "the lane holding a card was deleted and took it along"
    );
}

/// A session belongs to a card, and the card finds it again after a restart.
///
/// This is what makes the target terminal work at all: reopening the app has
/// to know which session belongs to which card, or every restart orphans the
/// work that was running.
#[test]
fn a_session_stays_attached_to_its_card_across_a_restart() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("state.db");

    let card = {
        let store = Store::open(&path).expect("open");
        let project = store.add_project(dir.path(), None).expect("project");
        store.ensure_board(&project).expect("board");
        let column = store.columns(&project).expect("columns")[3].id.clone();
        let card = store
            .create_card(&project, &column, "Fix the parser", "")
            .expect("card");
        store
            .link_session(&card, "a1b2", "uuid-1", Some("/transcript.jsonl"))
            .expect("link");
        card
    };

    let store = Store::open(&path).expect("reopen");
    let link = store
        .session_link(&card)
        .expect("read")
        .expect("still linked");
    assert_eq!(link.short_id, "a1b2");
    assert_eq!(link.transcript_path.as_deref(), Some("/transcript.jsonl"));
}
