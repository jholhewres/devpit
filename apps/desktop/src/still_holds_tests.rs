//! Whether a result still holds, against a real repository.

use super::*;

use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("git {args:?}: {err}"));
}

/// A repository with one commit and a clean tree.
fn a_repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    git(dir.path(), &["init", "--initial-branch=main", "-q"]);
    git(
        dir.path(),
        &["config", "user.email", "test@example.invalid"],
    );
    git(dir.path(), &["config", "user.name", "Test"]);
    git(dir.path(), &["config", "commit.gpgsign", "false"]);
    std::fs::write(dir.path().join("one.txt"), "one\n").expect("write");
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-q", "-m", "first"]);
    dir
}

/// A run that just happened, in that repository.
fn a_run_in(dir: &Path) -> Ran {
    Ran {
        command: Some("make test".to_owned()),
        in_directory: Some(dir.display().to_string()),
        head_revision: devpit_git::head_of(dir).ok(),
        saw_changes: devpit_git::standing_at(dir).ok(),
        in_a_worktree: Some(false),
        ..Ran::unknown()
    }
}

#[test]
fn a_result_about_the_code_that_is_there_is_current() {
    let dir = a_repository();
    assert_eq!(still_holds(&a_run_in(dir.path())), Validity::Current);
}

/// An edit nobody committed is still an edit, and the result is now about
/// other code.
///
/// Sabotage: drop `changes` from `where_it_stands` and this reads `Unknown`;
/// drop it from both sides and it reads `Current`, which is the lie.
#[test]
fn an_edit_since_the_run_makes_it_stale() {
    let dir = a_repository();
    let ran = a_run_in(dir.path());

    std::fs::write(dir.path().join("one.txt"), "one\ntwo\n").expect("write");
    assert_eq!(still_holds(&ran), Validity::Stale);
}

#[test]
fn a_commit_since_the_run_makes_it_stale() {
    let dir = a_repository();
    let ran = a_run_in(dir.path());

    std::fs::write(dir.path().join("two.txt"), "two\n").expect("write");
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-q", "-m", "second"]);
    assert_eq!(still_holds(&ran), Validity::Stale);
}

/// Every run from before migration 017 recorded nothing. Nothing is `Unknown`,
/// and a screen showing it says so rather than drawing it as current.
#[test]
fn a_run_that_recorded_nothing_is_unknown() {
    assert_eq!(still_holds(&Ran::unknown()), Validity::Unknown);
}

/// A run whose directory is gone — a worktree removed, a project moved — has
/// nowhere to look, and nowhere to look is not an answer either way.
#[test]
fn a_run_whose_directory_is_gone_is_unknown() {
    let dir = a_repository();
    let ran = a_run_in(dir.path());
    drop(dir);

    assert_eq!(still_holds(&ran), Validity::Unknown);
}

/// A run in a directory that is no repository recorded neither half, so there
/// is nothing to compare and nothing is claimed.
#[test]
fn a_run_outside_a_repository_is_unknown() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ran = a_run_in(dir.path());

    assert_eq!(what_it_saw(&ran), where_it_stands(dir.path()));
    assert_eq!(
        still_holds(&ran),
        Validity::Unknown,
        "two empty fingerprints were read as agreeing about something"
    );
}
