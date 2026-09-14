use super::*;
use crate::fixture;

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    fixture::repo(dir.path());
    std::fs::write(dir.path().join("kept.txt"), "one\ntwo\n").expect("write");
    std::fs::write(dir.path().join("gone.txt"), "bye\n").expect("write");
    std::fs::write(dir.path().join(".gitignore"), "target/\n").expect("write");
    fixture::commit(dir.path(), "start");
    dir
}

fn paths(changed: &[Changed]) -> Vec<&str> {
    changed.iter().map(|one| one.path.as_str()).collect()
}

#[test]
fn a_turn_reports_edits_new_files_and_deletions() {
    let dir = repo();
    let before = snapshot(dir.path()).expect("before");

    std::fs::write(dir.path().join("kept.txt"), "one\nTWO\nthree\n").expect("edit");
    std::fs::write(dir.path().join("new.rs"), "fn main() {}\n").expect("create");
    std::fs::remove_file(dir.path().join("gone.txt")).expect("delete");

    let after = snapshot(dir.path()).expect("after");
    let changed = changed_between(dir.path(), &before, &after).expect("diff");

    assert_eq!(paths(&changed), ["gone.txt", "kept.txt", "new.rs"]);
    let kept = changed
        .iter()
        .find(|one| one.path == "kept.txt")
        .expect("kept");
    assert_eq!((kept.added, kept.removed), (2, 1));
    let new = changed
        .iter()
        .find(|one| one.path == "new.rs")
        .expect("new");
    // Untracked when the turn made it, and still counted: `git diff HEAD` would
    // have said nothing about it.
    assert_eq!((new.added, new.removed), (1, 0));
}

#[test]
fn what_was_already_changed_before_the_turn_is_not_the_turns() {
    let dir = repo();
    std::fs::write(dir.path().join("kept.txt"), "edited by a person\n").expect("dirty");
    let before = snapshot(dir.path()).expect("before");
    std::fs::write(dir.path().join("new.rs"), "x\n").expect("create");
    let after = snapshot(dir.path()).expect("after");

    let changed = changed_between(dir.path(), &before, &after).expect("diff");
    assert_eq!(paths(&changed), ["new.rs"]);
}

#[test]
fn ignored_files_are_not_changes() {
    let dir = repo();
    let before = snapshot(dir.path()).expect("before");
    std::fs::create_dir_all(dir.path().join("target")).expect("mkdir");
    std::fs::write(dir.path().join("target/out.bin"), "build\n").expect("write");
    let after = snapshot(dir.path()).expect("after");
    assert!(changed_between(dir.path(), &before, &after)
        .expect("diff")
        .is_empty());
}

#[test]
fn the_persons_own_index_is_left_alone() {
    let dir = repo();
    std::fs::write(dir.path().join("kept.txt"), "unstaged\n").expect("dirty");
    snapshot(dir.path()).expect("snapshot");
    // Had the snapshot used the real index, this edit would now be staged.
    let staged = std::process::Command::new("git")
        .arg("-C")
        .arg(dir.path())
        .args(["diff", "--cached", "--name-only"])
        .output()
        .expect("git");
    assert!(String::from_utf8_lossy(&staged.stdout).trim().is_empty());
    // And no scratch index is left behind.
    let leftovers = std::fs::read_dir(dir.path().join(".git"))
        .expect("git dir")
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("devpit-snapshot-")
        })
        .count();
    assert_eq!(leftovers, 0);
}

#[test]
fn a_folder_that_is_not_a_repository_is_an_error_not_an_empty_answer() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(snapshot(dir.path()).is_err());
}
