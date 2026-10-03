//! What erasing a project deletes, and what it leaves.

use devpit_core::home::projects_dir;

use super::*;

/// A synced folder name `ProjectHome` refuses, here another project's folder:
/// the row still goes, and nothing is deleted on that name's word.
#[test]
fn a_project_whose_stored_folder_is_refused_is_erased_and_deletes_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let store = Store::open(&root.join("state.db")).expect("store");
    let checkout = root.join("checkout");
    std::fs::create_dir_all(&checkout).expect("checkout");
    let id = store.add_project(&checkout, None).expect("project");
    let theirs = projects_dir(root).join("other-abcdef");
    std::fs::create_dir_all(&theirs).expect("another project's folder");
    std::fs::write(theirs.join("keep.txt"), "theirs").expect("a file");
    store
        .conn()
        .execute(
            "UPDATE project SET folder = 'other-abcdef' WHERE id = ?1",
            [id.as_str()],
        )
        .expect("a folder from elsewhere");

    erase(&store, root, &id).expect("erased");

    assert!(
        store.project(&id).expect("read").is_none(),
        "the row stayed"
    );
    let kept = std::fs::read_to_string(theirs.join("keep.txt")).expect("kept");
    assert_eq!(kept, "theirs");
    let told = store.notices(10).expect("notices");
    assert!(
        told.iter().any(|row| row.kind == FOLDER_NOTICE),
        "nobody was told the folder was left"
    );
}

/// A plain folder is a project devpit opens, so its changes are an empty
/// list, not an error the report hears about on every turn.
#[test]
fn a_folder_that_is_not_a_repository_has_no_changes() {
    let dir = tempfile::tempdir().expect("tempdir");

    let changes = changes_in(dir.path()).expect("a plain folder is not an error");

    assert!(changes.is_empty());
}

/// The touch only orders the list: a store that refuses it still opens the
/// project it was asked about.
#[test]
fn a_project_opens_when_its_place_in_the_list_cannot_be_written() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("store");
    let id = store.add_project(dir.path(), None).expect("project");
    store
        .conn()
        .execute_batch(
            "CREATE TRIGGER refused BEFORE UPDATE ON project
             BEGIN SELECT RAISE(FAIL, 'database is locked'); END;",
        )
        .expect("a store that refuses the touch");
    assert!(store.touch_project(&id).is_err(), "the touch went through");

    opened(&store, &id).expect("opened all the same");
}
