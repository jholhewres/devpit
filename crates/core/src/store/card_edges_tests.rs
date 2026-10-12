use super::*;

fn seeded() -> (tempfile::TempDir, Store, String, Vec<String>) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let root = dir.path().join("project");
    std::fs::create_dir_all(&root).expect("create");
    let project = store.add_project(&root, None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let cards = ["a", "b", "c"]
        .iter()
        .map(|title| {
            store
                .create_card(&project, &column, title, "")
                .expect("card")
        })
        .collect();
    (dir, store, project, cards)
}

#[test]
fn a_card_never_waits_on_itself_or_on_a_card_that_waits_on_it() {
    let (_dir, store, project, cards) = seeded();
    let conn = store.conn();
    assert!(wait_on(conn, &cards[1], &cards[0], 1).expect("b on a"));
    assert!(wait_on(conn, &cards[2], &cards[1], 1).expect("c on b"));
    assert!(!wait_on(conn, &cards[0], &cards[0], 1).expect("a on a"));
    // a ← b ← c: a waiting on c closes the circle.
    assert!(!wait_on(conn, &cards[0], &cards[2], 1).expect("a on c"));
    assert_eq!(store.card_edges(&project).expect("edges").blockers.len(), 2);
}

#[test]
fn a_card_never_waits_on_another_projects() {
    let (dir, store, _project, cards) = seeded();
    let root = dir.path().join("other");
    std::fs::create_dir_all(&root).expect("create");
    let other = store.add_project(&root, None).expect("project");
    store.ensure_board(&other).expect("board");
    let column = store.columns(&other).expect("columns")[0].id.clone();
    let theirs = store
        .create_card(&other, &column, "theirs", "")
        .expect("card");
    assert!(!wait_on(store.conn(), &cards[0], &theirs, 1).expect("across"));
}

#[test]
fn an_archived_card_neither_blocks_nor_is_blocked_on_the_board() {
    let (_dir, store, project, cards) = seeded();
    wait_on(store.conn(), &cards[1], &cards[0], 1).expect("b on a");
    store.archive_card(&cards[0]).expect("archive");
    assert!(store
        .card_edges(&project)
        .expect("edges")
        .blockers
        .is_empty());
}
