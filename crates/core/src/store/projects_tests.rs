//! What the project rows have to keep straight.
//!
//! Beside `projects.rs` rather than inside it, for the size ratchet: the file
//! is the rules, and this is the evidence that they hold.

use super::*;

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    (dir, store)
}

#[test]
fn forgetting_keeps_the_row_and_erasing_takes_the_board_with_it() {
    // The two halves of the same dialog. Forget has to be reversible in
    // principle — the cards are still there — and erase has to actually
    // take them, or the box that says "this cannot be undone" is wrong in
    // the one direction that matters.
    let (dir, store) = store();
    let root = dir.path().join("project");
    std::fs::create_dir_all(&root).expect("create");
    let id = store.add_project(&root, None).expect("add");
    store.ensure_board(&id).expect("board");
    let column = store.columns(&id).expect("columns")[0].id.clone();
    store.create_card(&id, &column, "a card", "").expect("card");

    assert!(store.forget_project(&id).expect("forget"));
    assert_eq!(store.cards(&id).expect("cards").len(), 1);
    assert!(store.project(&id).expect("read").is_some());

    assert!(store.erase_project(&id).expect("erase"));
    assert!(store.project(&id).expect("read").is_none());
    assert!(store.cards(&id).expect("cards").is_empty());
}

#[test]
fn renaming_changes_the_name_and_not_the_folder() {
    let (dir, store) = store();
    let root = dir.path().join("api-v2-final");
    std::fs::create_dir_all(&root).expect("create");
    let id = store.add_project(&root, None).expect("add");

    assert!(store.rename_project(&id, "API").expect("rename"));
    let row = store.project(&id).expect("read").expect("still there");
    assert_eq!(row.name, "API");
    assert_eq!(row.root_path, root.to_string_lossy());
}

#[test]
fn a_project_remembers_its_remote_and_when_it_was_opened() {
    // Both were written on the way in and read by nothing, so the list
    // was ordered by a column it could not show.
    let (dir, store) = store();
    let root = dir.path().join("project");
    std::fs::create_dir_all(&root).expect("create");
    let id = store
        .add_project(&root, Some("git@github.com:owner/name.git"))
        .expect("add");

    let row = store.project(&id).expect("read").expect("there");
    assert_eq!(row.origin.as_deref(), Some("git@github.com:owner/name.git"));
    assert!(row.last_opened_at.is_some(), "adding did not set the clock");
}

#[test]
fn adding_the_same_folder_twice_opens_it_rather_than_failing() {
    let (dir, store) = store();
    let root = dir.path().join("project");
    std::fs::create_dir_all(&root).expect("create");

    let first = store.add_project(&root, None).expect("add");
    let second = store.add_project(&root, None).expect("add again");
    assert_eq!(first, second, "the second add created a duplicate");
    assert_eq!(store.projects().expect("list").len(), 1);
}

#[test]
fn the_two_shapes_of_one_origin_hash_the_same() {
    // The reason this exists: the same repository cloned over ssh on one
    // machine and https on another has to converge on one project.
    assert_eq!(
        origin_hash("git@github.com:jholhewres/devpit.git"),
        origin_hash("https://github.com/jholhewres/devpit")
    );
}

#[test]
fn a_project_carries_its_workspace_colour() {
    let (dir, store) = store();
    let root = dir.path().join("project");
    std::fs::create_dir_all(&root).expect("create");
    store.add_project(&root, None).expect("add");

    let listed = store.projects().expect("list");
    assert_eq!(listed[0].accent, DEFAULT_ACCENT);
    assert_eq!(listed[0].name, "project");
}
