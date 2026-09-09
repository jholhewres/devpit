use std::path::Path;

use crate::fixture;
use crate::index::*;
use crate::{changes, GitError};

fn repo(at: &Path) {
    fixture::repo(at);
    std::fs::write(at.join("a.txt"), "one\n").expect("write");
    fixture::commit(at, "first");
}

fn staged_paths(root: &Path) -> Vec<String> {
    changes(root)
        .expect("changes")
        .into_iter()
        .filter(|change| change.staged)
        .map(|change| change.path)
        .collect()
}

#[test]
fn staging_a_file_puts_it_in_what_a_commit_would_take() {
    let dir = tempfile::tempdir().expect("tempdir");
    repo(dir.path());
    std::fs::write(dir.path().join("b.txt"), "two\n").expect("write");

    assert!(staged_paths(dir.path()).is_empty());
    stage(dir.path(), &["b.txt".to_owned()]).expect("stage");
    assert_eq!(staged_paths(dir.path()), vec!["b.txt".to_owned()]);
}

#[test]
fn unstaging_takes_it_back_out_and_leaves_the_file_alone() {
    let dir = tempfile::tempdir().expect("tempdir");
    repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "one\ntwo\n").expect("write");

    stage(dir.path(), &["a.txt".to_owned()]).expect("stage");
    unstage(dir.path(), &["a.txt".to_owned()]).expect("unstage");

    assert!(staged_paths(dir.path()).is_empty());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.txt")).expect("read"),
        "one\ntwo\n",
        "unstaging changed the file on disk"
    );
}

#[test]
fn committing_takes_what_is_staged_and_reports_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    repo(dir.path());
    std::fs::write(dir.path().join("b.txt"), "two\n").expect("write");
    stage(dir.path(), &["b.txt".to_owned()]).expect("stage");

    let made = commit(dir.path(), "add b").expect("commit");
    assert_eq!(made.subject, "add b");
    assert!(!made.sha.is_empty());
    assert!(
        staged_paths(dir.path()).is_empty(),
        "the index was not cleared"
    );
}

/// This process has no terminal for git to open an editor in, so an empty
/// message would hang rather than fail.
#[test]
fn an_empty_message_is_refused_before_git_is_asked() {
    let dir = tempfile::tempdir().expect("tempdir");
    repo(dir.path());
    assert!(matches!(
        commit(dir.path(), "   "),
        Err(GitError::Refused(_))
    ));
}

/// A path is passed after `--`, so a file named like a flag is a file.
#[test]
fn a_file_named_like_a_flag_is_staged_as_a_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    repo(dir.path());
    std::fs::write(dir.path().join("-f"), "odd\n").expect("write");

    stage(dir.path(), &["-f".to_owned()]).expect("stage");
    assert_eq!(staged_paths(dir.path()), vec!["-f".to_owned()]);
}
