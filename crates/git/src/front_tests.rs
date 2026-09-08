//! Against real repositories, because the question is what git does.

use super::*;
use std::process::Command;

/// A repository with one commit, and git configured enough to make more.
fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let at = dir.path();
    for args in [
        vec!["init", "-q", "-b", "main"],
        vec!["config", "user.email", "t@example.com"],
        vec!["config", "user.name", "Test"],
    ] {
        Command::new("git")
            .args(&args)
            .current_dir(at)
            .output()
            .expect("git");
    }
    commit(at, "start", "first");
    dir
}

fn commit(at: &Path, file: &str, message: &str) {
    std::fs::write(at.join(file), format!("{message}\n")).expect("write");
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(at)
        .output()
        .expect("add");
    Command::new("git")
        .args(["commit", "-q", "-m", message])
        .current_dir(at)
        .output()
        .expect("commit");
}

/// The reason this module exists, in one test.
///
/// Two commits land on the base after the front started. Against `HEAD`
/// they would show up as this front's work; against the ref it began from,
/// they do not.
#[test]
fn the_diff_is_against_where_the_front_began_not_against_head() {
    let repo = repository();
    let root = repo.path();
    let base = head_of(root).expect("head");

    let front = root.join("front");
    Command::new("git")
        .args(["worktree", "add", "-q", "-b", "front", "front"])
        .current_dir(root)
        .output()
        .expect("worktree add");

    // The base moves on without this front.
    commit(root, "elsewhere.txt", "someone else");
    commit(root, "elsewhere-again.txt", "and again");

    // And the front does its own work.
    commit(&front, "mine.txt", "my change");

    let changed = changed_since(&front, &base).expect("diff");
    assert!(
        changed.contains(&"mine.txt".to_owned()),
        "the front's own file is missing: {changed:?}"
    );
    assert!(
        !changed.iter().any(|f| f.starts_with("elsewhere")),
        "work from the base leaked into this front's diff: {changed:?}"
    );
}

#[test]
fn uncommitted_work_counts_as_this_front_s_change() {
    let repo = repository();
    let base = head_of(repo.path()).expect("head");
    std::fs::write(repo.path().join("wip.txt"), "not committed\n").expect("write");
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(repo.path())
        .output()
        .expect("add");

    let changed = changed_since(repo.path(), &base).expect("diff");
    assert!(changed.contains(&"wip.txt".to_owned()), "{changed:?}");
}

#[test]
fn a_tracked_file_diffs_against_head() {
    let repo = repository();
    std::fs::write(repo.path().join("start"), "changed\n").expect("write");

    let diff = diff_file(repo.path(), "start").expect("diff");
    assert!(diff.contains("-first"), "{diff}");
    assert!(diff.contains("+changed"), "{diff}");
}

/// An untracked file has nothing to diff against, and an empty answer
/// would read as a file with no changes in it.
#[test]
fn an_untracked_file_shows_as_added() {
    let repo = repository();
    std::fs::write(repo.path().join("new.txt"), "brand new\n").expect("write");

    let diff = diff_file(repo.path(), "new.txt").expect("diff");
    assert!(diff.contains("+brand new"), "{diff}");
}

/// A file nobody touched has an empty diff, which is the truthful answer.
#[test]
fn an_unchanged_file_diffs_to_nothing() {
    let repo = repository();
    assert!(diff_file(repo.path(), "start")
        .expect("diff")
        .trim()
        .is_empty());
}

#[test]
fn a_clean_checkout_has_nothing_unsaved() {
    let repo = repository();
    assert!(unsaved_in(repo.path()).expect("status").is_empty());
}

#[test]
fn work_nothing_has_saved_is_listed() {
    let repo = repository();
    std::fs::write(repo.path().join("wip.txt"), "unsaved\n").expect("write");
    assert_eq!(unsaved_in(repo.path()).expect("status").len(), 1);
}

/// A directory with unsaved work in it is not the product's to delete.
#[test]
fn a_front_holding_unsaved_work_refuses_to_be_removed() {
    let repo = repository();
    let root = repo.path();
    let front = root.join("front");
    Command::new("git")
        .args(["worktree", "add", "-q", "-b", "front", "front"])
        .current_dir(root)
        .output()
        .expect("worktree add");

    std::fs::write(front.join("wip.txt"), "unsaved\n").expect("write");

    let refused = remove_front(root, &front).expect_err("it should refuse");
    assert!(front.is_dir(), "the checkout was deleted anyway");
    assert!(
        refused.to_string().contains("nothing has saved"),
        "{refused}"
    );
}

#[test]
fn a_clean_front_is_removed() {
    let repo = repository();
    let root = repo.path();
    let front = root.join("front");
    Command::new("git")
        .args(["worktree", "add", "-q", "-b", "front", "front"])
        .current_dir(root)
        .output()
        .expect("worktree add");

    remove_front(root, &front).expect("remove");
    assert!(!front.is_dir(), "the checkout is still there");
}
