use super::*;

use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from("/home/someone/.devpit")
}

#[test]
fn a_project_gets_shortcuts_to_its_own_folders() {
    let places = places_for(&root(), Some("prj_1"), |_| true);
    assert_eq!(
        places
            .iter()
            .map(|place| place.path.as_str())
            .collect::<Vec<_>>(),
        ["projects/prj_1", "worktrees/prj_1", "agents"]
    );
}

#[test]
fn a_folder_devpit_has_not_made_yet_is_not_offered() {
    // A project that has never run a card has no worktree folder. A shortcut
    // to it would fail on the click, which teaches the reader that none of
    // the shortcuts can be trusted.
    let places = places_for(&root(), Some("prj_1"), |at| {
        !at.ends_with("worktrees/prj_1")
    });
    assert!(places.iter().all(|place| place.path != "worktrees/prj_1"));
    assert!(places.iter().any(|place| place.path == "projects/prj_1"));
}

#[test]
fn with_no_project_only_the_machines_own_folders_are_offered() {
    let places = places_for(&root(), None, |_| true);
    assert_eq!(
        places
            .iter()
            .map(|place| place.path.as_str())
            .collect::<Vec<_>>(),
        ["agents"]
    );
}

#[test]
fn a_file_is_measured_and_a_directory_is_counted() {
    let home = tempfile::tempdir().expect("a temporary directory");
    std::fs::create_dir(home.path().join("sessions")).expect("a directory");
    std::fs::write(home.path().join("sessions/one.jsonl"), b"{}\n").expect("a file");
    std::fs::write(home.path().join("hooks.json"), b"{\"a\":1}").expect("a file");

    let listed = tree::children(home.path(), "").expect("a listing");
    let entries: Vec<WorkspaceEntry> = listed
        .into_iter()
        .map(|entry| described(home.path(), entry))
        .collect();

    let folder = entries
        .iter()
        .find(|entry| entry.name == "sessions")
        .expect("the folder");
    // A directory reports what is in it, not what it weighs: measuring the
    // weight means walking it, once per folder on screen.
    assert_eq!(folder.count, Some(1));
    assert_eq!(folder.bytes, 0.0);

    let file = entries
        .iter()
        .find(|entry| entry.name == "hooks.json")
        .expect("the file");
    assert_eq!(file.bytes, 7.0);
    assert_eq!(file.count, None);
    assert!(file.modified > 0.0);
}

#[test]
fn a_path_that_climbs_out_of_the_workspace_is_refused() {
    // The workspace holds the state database and the tmux socket, and this
    // process runs terminals: reaching out of it is reaching the machine.
    // Read through the same call `workspace_file` makes, so the refusal being
    // asserted is the one that will actually run.
    let home = tempfile::tempdir().expect("a temporary directory");
    std::fs::create_dir(home.path().join("inside")).expect("a directory");
    std::fs::write(home.path().join("secret"), b"x").expect("a file");

    let refused = crate::files::contents(&home.path().join("inside"), "../secret".to_owned())
        .expect_err("a refusal");
    assert_eq!(refused.code, devpit_rpc::ErrorCode::Forbidden);
}
