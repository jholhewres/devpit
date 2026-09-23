//! What a delete, a move and a write may never reach: the project itself,
//! what a symlink points at, and `.git`.

use crate::paths::*;
use crate::tree::TreeError;

/// A folder of real files outside the project, and a link to it inside.
#[cfg(unix)]
fn linked_folder() -> (tempfile::TempDir, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = tempfile::tempdir().expect("tempdir");
    std::fs::write(target.path().join("keep.txt"), "x").expect("write");
    std::os::unix::fs::symlink(target.path(), dir.path().join("link")).expect("symlink");
    (dir, target)
}

/// The bug this guards: `is_dir` follows the link, and `remove_dir_all` on
/// the resolved path emptied the folder it pointed at.
#[cfg(unix)]
#[test]
fn remove_on_a_symlink_takes_only_the_link() {
    let (dir, target) = linked_folder();

    remove(dir.path(), "link").expect("remove");
    assert!(dir.path().join("link").symlink_metadata().is_err());
    assert!(
        target.path().join("keep.txt").exists(),
        "the target was emptied"
    );
}

#[cfg(unix)]
#[test]
fn move_to_on_a_symlink_moves_the_link_not_its_target() {
    let (dir, target) = linked_folder();

    move_to(dir.path(), "link", "renamed").expect("move");
    let moved = dir
        .path()
        .join("renamed")
        .symlink_metadata()
        .expect("moved");
    assert!(
        moved.file_type().is_symlink(),
        "the target moved, not the link"
    );
    assert!(target.path().join("keep.txt").exists());
}

/// A link to nothing is still a row that can be deleted.
#[cfg(unix)]
#[test]
fn remove_takes_a_broken_symlink() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::os::unix::fs::symlink(dir.path().join("gone"), dir.path().join("link")).expect("link");

    remove(dir.path(), "link").expect("remove");
    assert!(dir.path().join("link").symlink_metadata().is_err());
}

/// Every spelling of the root resolves to it, and `path.delete` on it was
/// `remove_dir_all` on the whole project.
#[test]
fn remove_and_move_refuse_the_root_however_it_is_spelled() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("sub")).expect("create");
    std::fs::write(dir.path().join("keep.txt"), "x").expect("write");

    for root in ["", ".", "./", "sub/..", "sub/../"] {
        let removed = remove(dir.path(), root);
        assert!(
            matches!(removed, Err(TreeError::Outside { .. })),
            "remove({root:?}): {removed:?}"
        );
        let moved = move_to(dir.path(), root, "sub/elsewhere");
        assert!(
            matches!(moved, Err(TreeError::Outside { .. })),
            "move_to({root:?}): {moved:?}"
        );
    }
    assert!(dir.path().join("keep.txt").exists());
}

/// A project with a hook in `.git`, and a plain file beside it.
fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join(".git/hooks")).expect("create");
    std::fs::write(dir.path().join(".git/hooks/pre-commit"), "#!/bin/sh\n").expect("write");
    std::fs::write(dir.path().join("a.txt"), "x").expect("write");
    dir
}

fn assert_outside<T: std::fmt::Debug>(what: &str, answer: Result<T, TreeError>) {
    assert!(
        matches!(answer, Err(TreeError::Outside { .. })),
        "{what} reached .git: {answer:?}"
    );
}

#[test]
fn nothing_deletes_moves_or_writes_inside_dot_git() {
    let dir = repository();
    let root = dir.path();

    assert_outside("remove", remove(root, ".git/hooks/pre-commit"));
    assert_outside("remove", remove(root, ".git"));
    assert_outside("move from", move_to(root, ".git/hooks/pre-commit", "b"));
    assert_outside("move to", move_to(root, "a.txt", ".git/hooks/pre-commit"));
    assert_outside("write", resolve_writable(root, ".git/hooks/pre-commit"));
    assert!(root.join(".git/hooks/pre-commit").exists());
    assert!(resolve_writable(root, "a.txt").is_ok());
}

/// The name check alone reads `hooks/pre-commit` and sees no `.git`; only the
/// path after symlinks shows where it lands.
#[cfg(unix)]
#[test]
fn a_symlinked_folder_into_dot_git_is_refused_too() {
    let dir = repository();
    let root = dir.path();
    std::os::unix::fs::symlink(root.join(".git/hooks"), root.join("hooks")).expect("symlink");

    assert_outside("write", resolve_writable(root, "hooks/pre-commit"));
    assert_outside("remove", remove(root, "hooks/pre-commit"));
    assert_outside("move to", move_to(root, "a.txt", "hooks/pre-push"));
    assert_outside("create", create(root, "hooks/pre-push", false));
    assert!(root.join(".git/hooks/pre-commit").exists());
}

/// `.GIT` is `.git` on macOS: the name alone refuses it on every system.
#[test]
fn dot_git_in_another_case_is_refused() {
    let dir = repository();
    let root = dir.path();

    assert_outside("remove", remove(root, ".GIT"));
    assert_outside("move from", move_to(root, ".Git", "x"));
    assert_outside("create", create(root, ".GIT/hooks/x", false));
    assert!(root.join(".git/hooks/pre-commit").exists());
}

/// The ordinary cases still work: a nested rename, and a real folder deleted
/// with what is in it.
#[test]
fn a_nested_move_and_a_real_folder_delete_still_work() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    std::fs::create_dir_all(root.join("a/c")).expect("create");
    std::fs::write(root.join("a/b.txt"), "x").expect("write");

    move_to(root, "a/b.txt", "a/c/b.txt").expect("move");
    assert!(root.join("a/c/b.txt").exists());
    remove(root, "a").expect("remove");
    assert!(!root.join("a").exists());
}

/// A trailing slash on a link still takes only the link.
#[cfg(unix)]
#[test]
fn a_link_written_with_a_trailing_slash_is_still_the_link() {
    let (dir, target) = linked_folder();

    remove(dir.path(), "link/").expect("remove");
    assert!(target.path().join("keep.txt").exists());
}
