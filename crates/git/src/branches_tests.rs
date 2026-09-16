use std::path::Path;

use crate::branches::*;
use crate::fixture;

fn repo(at: &Path) {
    fixture::repo(at);
    std::fs::write(at.join("a.txt"), "one\n").expect("write");
    fixture::commit(at, "first");
}

#[test]
fn the_branch_you_are_on_comes_first_and_is_marked() {
    let dir = tempfile::tempdir().expect("tempdir");
    repo(dir.path());
    crate::run(dir.path(), &["branch", "later"]).expect("branch");

    let found = branches(dir.path()).expect("branches");
    assert_eq!(found.len(), 2);
    assert!(found[0].current, "the current branch is not first");
    assert_eq!(found[0].name, "main");
    assert!(!found[1].current);
}

#[test]
fn a_branch_carries_what_its_last_commit_said() {
    let dir = tempfile::tempdir().expect("tempdir");
    repo(dir.path());
    assert_eq!(branches(dir.path()).expect("branches")[0].subject, "first");
}

#[test]
fn switching_moves_the_checkout() {
    let dir = tempfile::tempdir().expect("tempdir");
    repo(dir.path());
    crate::run(dir.path(), &["branch", "later"]).expect("branch");

    switch(dir.path(), "later").expect("switch");
    let found = branches(dir.path()).expect("branches");
    assert_eq!(found[0].name, "later");
    assert!(found[0].current);
}

/// git refuses on its own, and its refusal names the files. Rewording it here
/// would name fewer.
#[test]
fn switching_away_from_work_that_would_be_lost_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    repo(dir.path());
    crate::run(dir.path(), &["branch", "later"]).expect("branch");
    crate::run(dir.path(), &["switch", "later"]).expect("switch");
    std::fs::write(dir.path().join("a.txt"), "changed on later\n").expect("write");
    fixture::commit(dir.path(), "on later");
    crate::run(dir.path(), &["switch", "main"]).expect("switch back");
    std::fs::write(dir.path().join("a.txt"), "uncommitted\n").expect("write");

    assert!(
        switch(dir.path(), "later").is_err(),
        "the switch took uncommitted work with it"
    );
}

/// What a step is told it is working on. A detached HEAD has no branch name,
/// and saying so beats naming the commit as though it were one.
#[test]
fn the_branch_a_checkout_is_on_is_read_by_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    repo(dir.path());
    assert_eq!(crate::branch_at(dir.path()).expect("branch"), "main");

    crate::run(dir.path(), &["checkout", "--detach"]).expect("detach");
    assert_eq!(crate::branch_at(dir.path()).expect("branch"), "");
}
