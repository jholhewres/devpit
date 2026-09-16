//! What a board does to the steps it runs: edited, counted, deleted.
//!
//! Its own file because `board_tests.rs` was at its ceiling, and the steps are
//! a story of their own — the lanes let go of one, the runs hold on to it.

use super::tests::store_with_project;

/// Deleting a step is two writes, and a board that did one of them would draw
/// a lane pointing at a step that is not there.
#[test]
fn deleting_a_step_lets_go_of_the_lanes_that_ran_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project) = store_with_project(dir.path());
    store.ensure_board(&project).expect("seed");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let step = store
        .create_step(&project, "command", "tests", r#"{"command":"make"}"#, false)
        .expect("create step");
    store.set_column_step(&column, Some(&step)).expect("attach");

    assert!(store.delete_step(&step).expect("delete"));

    assert!(store.step(&step).expect("read").is_none());
    let lanes = store.columns(&project).expect("columns");
    assert_eq!(lanes[0].step_id, None, "the lane still points at a step");
    assert!(!store.delete_step(&step).expect("delete"), "deleted twice");
}

#[test]
fn a_step_is_edited_where_it_already_runs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project) = store_with_project(dir.path());
    let step = store
        .create_step(&project, "command", "tests", r#"{"command":"make"}"#, false)
        .expect("create step");

    assert!(store
        .update_step(&step, "suite", r#"{"command":"make test"}"#, true)
        .expect("update"));

    let read = store.step(&step).expect("read").expect("still there");
    assert_eq!(read.name, "suite");
    assert_eq!(read.config, r#"{"command":"make test"}"#);
    assert!(read.irreversible);
    // The kind is not something an edit changes.
    assert_eq!(read.kind, "command");
}

/// Both halves of the answer, because they refuse a deletion differently.
#[test]
fn a_steps_runs_are_counted_as_a_whole_and_as_what_is_still_going() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project) = store_with_project(dir.path());
    store.ensure_board(&project).expect("seed");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &column, "a card", "")
        .expect("card");
    let step = store
        .create_step(&project, "command", "tests", r#"{"command":"make"}"#, false)
        .expect("create step");
    assert_eq!(store.step_runs(&step).expect("counts"), (0, 0));

    let done = store.start_run(&card, &step, None).expect("run");
    store
        .finish_run(&done, "ok", Some("out"), Some(0.0), Some(1), Some(0))
        .expect("finish");
    store.start_run(&card, &step, None).expect("run");

    assert_eq!(store.step_runs(&step).expect("counts"), (2, 1));
}

/// The review's case: A(0), B(1), C(2); A leaves; "the end" as a count is 2,
/// which C already holds. As an index it is the end, and the lane is renumbered.
#[test]
fn a_card_moved_to_the_end_of_a_lane_lands_after_the_last_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project) = store_with_project(dir.path());
    store.ensure_board(&project).expect("seed");
    let lanes = store.columns(&project).expect("columns");
    let (here, there) = (lanes[0].id.clone(), lanes[1].id.clone());
    let a = store.create_card(&project, &here, "A", "").expect("A");
    let b = store.create_card(&project, &here, "B", "").expect("B");
    let c = store.create_card(&project, &here, "C", "").expect("C");
    let moving = store
        .create_card(&project, &there, "moving", "")
        .expect("moving");

    store.move_card(&a, &there, 0).expect("A leaves");
    let count = store.cards_in_column(&here).expect("count");
    store.move_card(&moving, &here, count).expect("to the end");

    let order: Vec<(String, i64)> = store
        .cards(&project)
        .expect("cards")
        .into_iter()
        .filter(|card| card.column_id == here)
        .map(|card| (card.title, card.position))
        .collect();
    assert_eq!(
        order,
        vec![
            ("B".to_owned(), 0),
            ("C".to_owned(), 1),
            ("moving".to_owned(), 2)
        ]
    );
    let _ = (b, c);
}

#[test]
fn a_card_moved_between_two_lands_between_them() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project) = store_with_project(dir.path());
    store.ensure_board(&project).expect("seed");
    let lanes = store.columns(&project).expect("columns");
    let here = lanes[0].id.clone();
    for title in ["A", "B", "C"] {
        store.create_card(&project, &here, title, "").expect("card");
    }
    let moving = store
        .create_card(&project, &lanes[1].id, "moving", "")
        .expect("moving");

    store.move_card(&moving, &here, 1).expect("between A and B");

    let titles: Vec<String> = store
        .cards(&project)
        .expect("cards")
        .into_iter()
        .filter(|card| card.column_id == here)
        .map(|card| card.title)
        .collect();
    assert_eq!(titles, ["A", "moving", "B", "C"]);
}
