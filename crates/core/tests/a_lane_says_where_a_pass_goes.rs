//! The flow a lane declares, written and read back through a real database.
//!
//! The rule that acts on it lives in `apps/desktop/src/advancing.rs` and is
//! tested there against plain values. This proves the other half: that what a
//! person chooses is what the rule is later handed, across a migration, a
//! write and a reopen.

use devpit_core::Store;

fn board(dir: &std::path::Path) -> (Store, String, Vec<String>) {
    let store = Store::open(&dir.join("state.db")).expect("open");
    let project = store.add_project(dir, None).expect("project");
    store.ensure_board(&project).expect("board");
    let columns = store
        .columns(&project)
        .expect("columns")
        .into_iter()
        .map(|column| column.id)
        .collect();
    (store, project, columns)
}

#[test]
fn a_new_board_moves_nothing_on_its_own() {
    // The default, and the only thing every existing board has. A migration
    // that quietly made somebody's lanes automatic would be the worst
    // possible way to ship this.
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project, _) = board(dir.path());

    for column in store.columns(&project).expect("columns") {
        assert_eq!(column.autonomy, "manual", "{} was not manual", column.name);
        assert_eq!(
            column.on_pass, None,
            "{} already sends cards on",
            column.name
        );
    }
}

#[test]
fn what_a_lane_is_told_is_what_it_says_afterwards() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project, columns) = board(dir.path());

    store
        .set_column_flow(&columns[0], Some(&columns[1]), "auto")
        .expect("set");

    let read = store.columns(&project).expect("columns");
    let first = read.iter().find(|one| one.id == columns[0]).expect("there");
    assert_eq!(first.on_pass.as_deref(), Some(columns[1].as_str()));
    assert_eq!(first.autonomy, "auto");

    // And nothing else moved.
    let second = read.iter().find(|one| one.id == columns[1]).expect("there");
    assert_eq!(second.autonomy, "manual");
    assert_eq!(second.on_pass, None);
}

#[test]
fn a_flow_survives_closing_the_database() {
    // It decides whether work moves somebody's card while they are not
    // looking, so it had better still be there when they come back.
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, _project, columns) = board(dir.path());
    store
        .set_column_flow(&columns[0], Some(&columns[1]), "ask")
        .expect("set");
    drop(store);

    let store = Store::open(&dir.path().join("state.db")).expect("reopen");
    let again = store
        .columns(
            &store
                .projects()
                .expect("projects")
                .first()
                .expect("one")
                .id
                .clone(),
        )
        .expect("columns");
    let first = again
        .iter()
        .find(|one| one.id == columns[0])
        .expect("there");
    assert_eq!(first.autonomy, "ask");
    assert_eq!(first.on_pass.as_deref(), Some(columns[1].as_str()));
}

#[test]
fn clearing_a_destination_leaves_the_lane_with_nowhere_to_send() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project, columns) = board(dir.path());
    store
        .set_column_flow(&columns[0], Some(&columns[1]), "auto")
        .expect("set");
    store
        .set_column_flow(&columns[0], None, "manual")
        .expect("clear");

    let first = store
        .columns(&project)
        .expect("columns")
        .into_iter()
        .find(|one| one.id == columns[0])
        .expect("there");
    assert_eq!(first.on_pass, None);
    assert_eq!(first.autonomy, "manual");
}

#[test]
fn deleting_the_destination_lane_does_not_take_the_lane_that_pointed_at_it() {
    // `ON DELETE SET NULL`. A lane losing its destination should stop sending
    // cards on — not vanish, and not keep pointing at a column that is gone.
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project, columns) = board(dir.path());
    store
        .set_column_flow(&columns[0], Some(&columns[1]), "auto")
        .expect("set");

    store.delete_column(&columns[1]).expect("delete");

    let left = store.columns(&project).expect("columns");
    let first = left
        .iter()
        .find(|one| one.id == columns[0])
        .expect("still there");
    assert_eq!(
        first.on_pass, None,
        "it still points at a lane that is gone"
    );
    // The autonomy stays, and with nowhere to go it decides nothing — which
    // is what `advancing::decide` does with a lane that has no destination.
    assert_eq!(first.autonomy, "auto");
}

#[test]
fn the_schema_refuses_a_word_the_rule_does_not_know() {
    // The CHECK constraint is the last line: the command parses the word and
    // the database refuses anything that got past it.
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, _project, columns) = board(dir.path());
    assert!(
        store
            .set_column_flow(&columns[0], None, "whenever")
            .is_err(),
        "the database accepted an autonomy nothing understands"
    );
}
