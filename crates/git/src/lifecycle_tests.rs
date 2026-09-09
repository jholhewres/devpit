use std::path::Path;

use crate::fixture;
use crate::lifecycle::*;
use crate::{worktrees, GitError};

fn repo(at: &Path) {
    fixture::repo(at);
    std::fs::write(at.join("a.txt"), "one\n").expect("write");
    fixture::commit(at, "first");
}

#[test]
fn a_branch_is_named_after_the_card_and_prefixed() {
    assert_eq!(
        branch_for("Fix the login redirect", "card_01HZXY9K"),
        "devpit/fix-the-login-redirect-01hzxy9k"
    );
}

#[test]
fn a_title_with_nothing_nameable_in_it_still_gets_a_branch() {
    assert_eq!(
        branch_for("!!! ???", "card_abcdefgh"),
        "devpit/card-abcdefgh"
    );
}

#[test]
fn two_cards_with_the_same_title_get_different_branches() {
    let one = branch_for("Refactor", "card_aaaaaaaa");
    let two = branch_for("Refactor", "card_bbbbbbbb");
    assert_ne!(one, two);
}

#[test]
fn a_worktree_lives_outside_the_repository() {
    let home = Path::new("/home/someone/.devpit");
    let path = worktree_home(home, "proj", "card");
    assert_eq!(path, home.join("worktrees").join("proj").join("card"));
}

#[test]
fn the_main_checkout_is_never_a_cards_worktree() {
    let dir = tempfile::tempdir().expect("tempdir");
    repo(dir.path());
    assert!(!assignable(dir.path(), dir.path()));

    let refused = create(dir.path(), dir.path(), "devpit/x-1", "HEAD");
    assert!(
        matches!(refused, Err(GitError::Refused(_))),
        "the main checkout was handed to a card"
    );
}

#[test]
fn creating_records_the_sha_and_not_the_branch_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let main = dir.path().join("main");
    std::fs::create_dir_all(&main).expect("create");
    repo(&main);

    let made = create(&main, &dir.path().join("wt"), "devpit/one-1", "main").expect("create");
    assert_eq!(made.base_ref.len(), 40, "base_ref is not a full sha");
    assert!(made.path.is_dir());

    // The base moves; the card's base does not follow it.
    std::fs::write(main.join("b.txt"), "two\n").expect("write");
    fixture::commit(&main, "second");
    let head = crate::head_of(&main).expect("head");
    assert_ne!(made.base_ref, head, "the card's base moved with the branch");
}

#[test]
fn a_dirty_worktree_is_not_removed_without_being_told_what_is_lost() {
    let dir = tempfile::tempdir().expect("tempdir");
    let main = dir.path().join("main");
    std::fs::create_dir_all(&main).expect("create");
    repo(&main);

    let made = create(&main, &dir.path().join("wt"), "devpit/one-1", "main").expect("create");
    std::fs::write(made.path.join("a.txt"), "one\ntwo\n").expect("write");

    let refused = remove(&main, &made.path, false);
    let Err(GitError::Refused(why)) = refused else {
        panic!("a dirty worktree was removed without asking");
    };
    assert!(
        why.contains("1 file"),
        "the refusal does not say what is lost: {why}"
    );
    assert!(made.path.is_dir(), "the folder went anyway");
}

#[test]
fn removing_keeps_the_branch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let main = dir.path().join("main");
    std::fs::create_dir_all(&main).expect("create");
    repo(&main);

    let made = create(&main, &dir.path().join("wt"), "devpit/one-1", "main").expect("create");
    std::fs::write(made.path.join("b.txt"), "two\n").expect("write");
    fixture::commit(&made.path, "work");

    remove(&main, &made.path, false).expect("remove");
    assert!(!made.path.exists(), "the folder is still there");

    let branches = crate::run(&main, &["branch", "--list", "devpit/one-1"]).expect("branch");
    assert!(
        branches.contains("devpit/one-1"),
        "removing the worktree took the branch with it"
    );
    assert_eq!(
        worktrees(&main).expect("worktrees").len(),
        1,
        "git still lists the removed worktree"
    );
}

#[test]
fn what_is_uncommitted_is_counted_in_files_and_lines() {
    let dir = tempfile::tempdir().expect("tempdir");
    repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "one\ntwo\nthree\n").expect("write");

    let loss = uncommitted(dir.path()).expect("uncommitted");
    assert_eq!(loss.files, 1);
    assert_eq!(loss.lines, 2, "two lines were added to a.txt");
    assert!(!loss.is_empty());
}

#[test]
fn a_folder_no_card_claims_is_an_orphan() {
    let home = tempfile::tempdir().expect("tempdir");
    let project = home.path().join("worktrees").join("proj");
    std::fs::create_dir_all(project.join("card_kept")).expect("create");
    std::fs::create_dir_all(project.join("card_gone")).expect("create");

    let found = orphans(home.path(), "proj", &["card_kept".to_owned()]);
    assert_eq!(found, vec![project.join("card_gone")]);
}

#[test]
fn disk_usage_counts_what_is_in_the_folder() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("deep")).expect("create");
    std::fs::write(dir.path().join("a"), "12345").expect("write");
    std::fs::write(dir.path().join("deep/b"), "123").expect("write");
    assert_eq!(disk_usage(dir.path()), 8);
}
