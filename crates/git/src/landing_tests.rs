use std::path::Path;

use crate::fixture;
use crate::landing::*;
use crate::lifecycle::{branch_for, create};

fn git(at: &Path, args: &[&str]) {
    let out = devpit_pty::host_env::command("git")
        .arg("-C")
        .arg(at)
        .args(args)
        .output()
        .expect("git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A repository on `main` and a card's worktree beside it, one commit in.
fn card_with_a_commit(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf, String, String) {
    let root = dir.join("repo");
    std::fs::create_dir_all(&root).expect("mkdir");
    fixture::repo(&root);
    std::fs::write(root.join("a.txt"), "one\n").expect("write");
    fixture::commit(&root, "first");

    let at = dir.join("wt").join("card_01HZXY9K");
    let branch = branch_for("Fix it", "card_01HZXY9K");
    let made = create(&root, &at, &branch, "HEAD").expect("create");
    std::fs::write(at.join("b.txt"), "two\n").expect("write");
    fixture::commit(&at, "the card's work");
    (root, at, branch, made.base_ref)
}

#[test]
fn a_branch_merged_into_main_has_landed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (root, _at, branch, began) = card_with_a_commit(dir.path());
    assert!(!landed(&root, &branch, &began), "landed before any merge");

    git(&root, &["merge", "-q", "--no-ff", "-m", "merge", &branch]);
    assert!(landed(&root, &branch, &began));
}

#[test]
fn a_branch_with_no_commit_of_its_own_has_not_landed() {
    // Every commit of a fresh branch is already in main; that is not a merge.
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("repo");
    std::fs::create_dir_all(&root).expect("mkdir");
    fixture::repo(&root);
    std::fs::write(root.join("a.txt"), "one\n").expect("write");
    fixture::commit(&root, "first");
    let at = dir.path().join("card_aaaaaaaa");
    let made = create(&root, &at, "devpit/fresh-aaaaaaaa", "HEAD").expect("create");

    assert!(!landed(&root, "devpit/fresh-aaaaaaaa", &made.base_ref));
}

#[test]
fn a_commit_made_after_the_merge_keeps_the_branch_from_having_landed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (root, at, branch, began) = card_with_a_commit(dir.path());
    git(&root, &["merge", "-q", "--no-ff", "-m", "merge", &branch]);
    std::fs::write(at.join("c.txt"), "three\n").expect("write");
    fixture::commit(&at, "more work");

    assert!(!landed(&root, &branch, &began));
}

#[test]
fn the_default_branch_itself_never_lands() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (root, _at, _branch, began) = card_with_a_commit(dir.path());
    std::fs::write(root.join("c.txt"), "three\n").expect("write");
    fixture::commit(&root, "on main");

    assert!(!landed(&root, "main", &began));
}

#[test]
fn a_landed_branch_is_deleted_and_one_not_landed_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (root, at, branch, began) = card_with_a_commit(dir.path());
    crate::remove(&root, &at, false).expect("remove the worktree");

    assert!(drop_landed(&root, &branch, "card_01HZXY9K", &began).is_err());
    assert!(branch_exists(&root, &branch));

    git(&root, &["merge", "-q", "--no-ff", "-m", "merge", &branch]);
    drop_landed(&root, &branch, "card_01HZXY9K", &began).expect("drop");
    assert!(!branch_exists(&root, &branch));
}

#[test]
fn a_branch_not_named_for_the_card_is_never_deleted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (root, at, branch, began) = card_with_a_commit(dir.path());
    crate::remove(&root, &at, false).expect("remove the worktree");
    git(&root, &["merge", "-q", "--no-ff", "-m", "merge", &branch]);

    assert!(drop_landed(&root, &branch, "card_99999999", &began).is_err());
    assert!(branch_exists(&root, &branch));
}

#[test]
fn only_a_linked_worktree_of_the_repository_is_one_of_its_checkouts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (root, at, _branch, _began) = card_with_a_commit(dir.path());
    let stranger = dir.path().join("stranger");
    std::fs::create_dir_all(&stranger).expect("mkdir");

    assert!(linked_checkout(&root, &at));
    assert!(!linked_checkout(&root, &root), "the main checkout");
    assert!(!linked_checkout(&root, &stranger));
}

fn branch_exists(root: &Path, branch: &str) -> bool {
    devpit_pty::host_env::command("git")
        .arg("-C")
        .arg(root)
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .output()
        .expect("git")
        .status
        .success()
}
