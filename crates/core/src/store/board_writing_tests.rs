//! The board's writes beside another connection's, as a run makes them.

use super::tests::store_with_project;

/// A move waits for a run's write instead of being refused by it.
///
/// A deferred transaction reads, then finds its snapshot stale when it comes
/// to write, and SQLite answers "database is locked" without waiting.
#[test]
fn a_card_moves_while_another_connection_is_writing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project) = store_with_project(dir.path());
    store.ensure_board(&project).expect("seed");
    let columns = store.columns(&project).expect("columns");
    let card = store
        .create_card(&project, &columns[0].id, "ship it", "")
        .expect("create");
    let other = store
        .create_card(&project, &columns[1].id, "running", "")
        .expect("create");

    let writer = rusqlite::Connection::open(dir.path().join("state.db")).expect("writer");
    writer
        .execute_batch("BEGIN IMMEDIATE")
        .expect("take the write lock");
    writer
        .execute("UPDATE card SET body = 'output' WHERE id = ?1", [&other])
        .expect("write");
    let releasing = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        writer.execute_batch("COMMIT").expect("release");
    });

    store
        .move_card(&card, &columns[3].id, 0)
        .expect("the move waited for the writer");
    releasing.join().expect("writer thread");
    assert_eq!(
        store.card(&card).expect("read").expect("there").column_id,
        columns[3].id
    );
}
