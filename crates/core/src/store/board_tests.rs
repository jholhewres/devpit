//! The board's tests, kept next to it rather than inside it.
//!
//! Half of `board.rs` was its own suite, and a file where the queries and the
//! tests take turns is a file read twice to answer one question.

use super::*;

/// A store with one project, ready for a board.
fn store_with_project(dir: &std::path::Path) -> (Store, String) {
    let store = Store::open(&dir.join("state.db")).expect("open");
    let id = store.add_project(dir, None).expect("add project");
    (store, id)
}

#[test]
fn a_new_project_gets_the_default_board_once() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project) = store_with_project(dir.path());

    store.ensure_board(&project).expect("seed");
    let first: Vec<String> = store
        .columns(&project)
        .expect("columns")
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(first, DEFAULT_COLUMNS.to_vec());

    // The second call is what every start after the first one does.
    store.ensure_board(&project).expect("seed again");
    assert_eq!(store.columns(&project).expect("columns").len(), 6);
}

#[test]
fn columns_are_renamed_reordered_and_deleted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project) = store_with_project(dir.path());
    store.ensure_board(&project).expect("seed");

    let ids: Vec<String> = store
        .columns(&project)
        .expect("columns")
        .into_iter()
        .map(|c| c.id)
        .collect();

    store.rename_column(&ids[0], "backlog").expect("rename");
    let mut reversed = ids.clone();
    reversed.reverse();
    store.reorder_columns(&project, &reversed).expect("reorder");

    let after: Vec<String> = store
        .columns(&project)
        .expect("columns")
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(after.last().expect("last"), "backlog");

    store
        .delete_column(&ids[0])
        .expect("delete an empty column");
    assert_eq!(store.columns(&project).expect("columns").len(), 5);
}

/// Deleting a column with cards in it has to fail, so the interface asks
/// where they go. CASCADE would delete described work on one click.
#[test]
fn a_column_holding_cards_refuses_to_be_deleted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project) = store_with_project(dir.path());
    store.ensure_board(&project).expect("seed");
    let column = store.columns(&project).expect("columns")[0].id.clone();

    store
        .create_card(&project, &column, "a card", "")
        .expect("create card");

    assert!(
        store.delete_column(&column).is_err(),
        "a column with cards was deleted and took them with it"
    );
}

/// A column keeps existing when its step goes away. A lane that runs
/// nothing is a lane doing its job.
#[test]
fn deleting_a_step_leaves_the_column_standing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project) = store_with_project(dir.path());
    store.ensure_board(&project).expect("seed");
    let column = store.columns(&project).expect("columns")[0].id.clone();

    let step = store
        .create_step(
            &project,
            "command",
            "tests",
            r#"{"command":"make test"}"#,
            false,
        )
        .expect("create step");
    store.set_column_step(&column, Some(&step)).expect("attach");

    store
        .conn()
        .execute("DELETE FROM step WHERE id = ?1", [&step])
        .expect("delete step");

    let columns = store.columns(&project).expect("columns");
    assert_eq!(columns.len(), 6, "the column went with the step");
    assert!(columns[0].step_id.is_none(), "the column kept a dead step");
}

#[test]
fn a_card_moves_between_columns_and_keeps_its_body() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project) = store_with_project(dir.path());
    store.ensure_board(&project).expect("seed");
    let columns = store.columns(&project).expect("columns");

    let card = store
        .create_card(&project, &columns[0].id, "ship it", "the body")
        .expect("create");
    store.move_card(&card, &columns[3].id, 0).expect("move");

    let moved = store.card(&card).expect("read").expect("still there");
    assert_eq!(moved.column_id, columns[3].id);
    assert_eq!(moved.body, "the body");
}

#[test]
fn a_card_reports_what_every_run_of_it_cost() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project) = store_with_project(dir.path());
    store.ensure_board(&project).expect("seed");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &column, "a card", "")
        .expect("card");
    let step = store
        .create_step(&project, "agent", "review", "{}", false)
        .expect("step");

    for cost in [0.10_f64, 0.25, 0.05] {
        let run = store.start_run(&card, &step, None).expect("start");
        store
            .finish_run(&run, "ok", Some("done"), Some(cost), Some(1000), None)
            .expect("finish");
    }

    let total = store.card_cost(&card).expect("cost");
    assert!((total - 0.40).abs() < 1e-9, "summed to {total}");
    assert_eq!(store.runs(&card).expect("runs").len(), 3);
}

/// The seed is a starting point, not a contract. A query that matched on
/// a column name would make the names undeletable in practice.
#[test]
fn no_query_matches_on_a_column_name() {
    let source = include_str!("board.rs");
    let queries = source
        .split("#[cfg(test)]")
        .next()
        .expect("the non-test half");
    for name in DEFAULT_COLUMNS {
        assert!(
            !queries.contains(&format!("'{name}'")),
            "a query matches on the column name {name}"
        );
    }
}
