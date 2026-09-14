//! Opening the store from more than one thread at the same moment.
//!
//! Not hypothetical: startup does it. The shell probe reads the store on its
//! own thread while the main thread is still setting up, so the very first
//! launch after an update — the one with a migration to apply — is exactly
//! when two connections meet.
//!
//! Written as a real race rather than as two sequential opens, because two
//! sequential opens pass either way: the second one simply reads the version
//! the first wrote. The bug only exists when both read before either writes.

use std::sync::{Arc, Barrier};

use devpit_core::Store;

#[test]
fn many_connections_opening_together_all_succeed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = Arc::new(dir.path().join("state.db"));

    // Enough of them, released together, that the interleaving is not left to
    // luck. Nothing has created the file yet, so every one of them has the
    // whole migration list to apply.
    let racers = 8;
    let gate = Arc::new(Barrier::new(racers));

    let opened: Vec<_> = (0..racers)
        .map(|_| {
            let path = Arc::clone(&path);
            let gate = Arc::clone(&gate);
            std::thread::spawn(move || {
                gate.wait();
                Store::open(&path)
                    .map(|_| ())
                    .map_err(|err| err.to_string())
            })
        })
        .collect();

    let refused: Vec<String> = opened
        .into_iter()
        .filter_map(|one| one.join().expect("thread").err())
        .collect();

    assert!(
        refused.is_empty(),
        "opening the store at the same time failed: {refused:?}"
    );
}

#[test]
fn a_store_opened_twice_still_has_one_of_each_table() {
    // What a second run of the same migration would have produced, asked of
    // the schema rather than of the error message.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("state.db");

    let first = Store::open(&path).expect("open");
    let project = first.add_project(dir.path(), None).expect("project");
    first.ensure_board(&project).expect("board");
    let columns = first.columns(&project).expect("columns").len();
    drop(first);

    let again = Store::open(&path).expect("reopen");
    assert_eq!(
        again.columns(&project).expect("columns").len(),
        columns,
        "reopening the store changed the board"
    );
}
