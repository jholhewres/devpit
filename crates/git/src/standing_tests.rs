//! Where a tree stands, against real repositories.

use super::*;

use crate::invoke::fixture;

fn a_repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    fixture::repo(dir.path());
    std::fs::write(dir.path().join("one.txt"), "one\n").expect("write");
    fixture::commit(dir.path(), "first");
    dir
}

/// Asked twice of a tree nobody touched, it answers the same thing. Without
/// this the screen would call every result stale the moment it looked twice.
#[test]
fn a_tree_nobody_touched_stands_where_it_stood() {
    let dir = a_repository();
    assert_eq!(
        standing_at(dir.path()).expect("read"),
        standing_at(dir.path()).expect("read")
    );
}

/// An edit that is never committed still changes what a person is looking at,
/// which is the whole reason the revision alone cannot answer this.
///
/// Sabotage: hash only the commit and this fails.
#[test]
fn an_uncommitted_edit_moves_the_tree() {
    let dir = a_repository();
    let before = standing_at(dir.path()).expect("read");

    std::fs::write(dir.path().join("one.txt"), "one\ntwo\n").expect("write");
    assert_ne!(standing_at(dir.path()).expect("read"), before);
}

/// A file nobody has told git about is still a file that can change an answer.
#[test]
fn an_untracked_file_moves_the_tree() {
    let dir = a_repository();
    let before = standing_at(dir.path()).expect("read");

    std::fs::write(dir.path().join("new.txt"), "new\n").expect("write");
    assert_ne!(standing_at(dir.path()).expect("read"), before);
}

/// Putting the tree back puts the answer back: what is hashed is the state,
/// not the history of getting there.
#[test]
fn undoing_an_edit_puts_the_tree_back() {
    let dir = a_repository();
    let clean = standing_at(dir.path()).expect("read");

    std::fs::write(dir.path().join("one.txt"), "one\ntwo\n").expect("write");
    std::fs::write(dir.path().join("one.txt"), "one\n").expect("write");
    assert_eq!(standing_at(dir.path()).expect("read"), clean);
}

/// A committed change leaves the tree clean, so the standing goes back to
/// what a clean tree says — and the *revision*, which this does not carry, is
/// what moved. Both are needed, which is why `Fingerprint` has two fields.
#[test]
fn committing_leaves_the_tree_standing_clean_again() {
    let dir = a_repository();
    let clean = standing_at(dir.path()).expect("read");

    std::fs::write(dir.path().join("two.txt"), "two\n").expect("write");
    fixture::commit(dir.path(), "second");
    assert_eq!(standing_at(dir.path()).expect("read"), clean);
}

/// A directory that is no repository answers an error, not a string that
/// would compare equal to another non-repository's.
#[test]
fn somewhere_that_is_no_repository_says_so() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(standing_at(dir.path()).is_err());
}
