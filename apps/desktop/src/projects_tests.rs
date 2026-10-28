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

/// #219: a project's status is read only for the one in front; switching
/// to it does not run `git status` in every other checkout.
#[test]
fn the_list_reads_the_status_of_the_project_in_front_alone() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("store");
    let checkout = |name: &str| {
        let root = dir.path().join(name);
        std::fs::create_dir_all(&root).expect("checkout");
        crate::devflow_review::tests::git(&root, &["init", "-q", "-b", "main"]);
        std::fs::write(root.join("new.txt"), "dirty").expect("a file");
        store.add_project(&root, None).expect("project")
    };
    let (front, behind) = (checkout("front"), checkout("behind"));
    let listed = listed_in(&store, Some(&front)).expect("listed").projects;
    let dirty = |id: &str| {
        listed
            .iter()
            .find(|one| one.id == id)
            .and_then(|one| one.worktrees.first())
            .map(|tree| tree.dirty_files)
    };
    assert_eq!(dirty(&front), Some(Some(1)), "the project in front");
    assert_eq!(dirty(&behind), Some(None), "read without its status");
}

/// The project in front is the one opened last, read in the order the store
/// lists them, so a project opened after another is the front.
#[test]
fn the_project_opened_last_is_in_front() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("store");
    let add = |name: &str| {
        let root = dir.path().join(name);
        std::fs::create_dir_all(&root).expect("folder");
        store.add_project(&root, None).expect("project")
    };
    let (older, newer) = (add("older"), add("newer"));
    // Seconds apart, as two opens a person makes are.
    let opened = |id: &str, at: i64| {
        store
            .conn()
            .execute_batch(&format!(
                "UPDATE project SET last_opened_at = {at} WHERE id = '{id}'"
            ))
            .expect("opened");
    };
    opened(&newer, 100);
    opened(&older, 200);
    assert_eq!(front_of(&store), Some(older));
}
