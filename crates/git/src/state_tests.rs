use crate::fixture;

use super::*;

#[test]
fn a_repository_says_its_branch_its_changes_its_commits_and_its_tag() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fixture::repo(root);
    std::fs::write(root.join("a.txt"), "one\n").expect("write");
    fixture::commit(root, "first");
    run(root, &["tag", "v1.0.0"]).expect("tag");
    std::fs::write(root.join("a.txt"), "two\n").expect("write");
    fixture::commit(root, "second");
    std::fs::write(root.join("b.txt"), "loose\n").expect("write");

    let state = repo_state(root, &["main".to_owned(), "nope".to_owned()], false).expect("state");
    assert_eq!(state.branch, "main");
    assert_eq!(state.dirty, 1);
    assert_eq!(
        state.commits.first().map(|(_, subject)| subject.as_str()),
        Some("second")
    );
    assert_eq!(state.tag.as_deref(), Some("v1.0.0"));
    assert_eq!(state.since_tag, Some(1));
    assert_eq!(state.upstream, None);
    assert!(state.branches[0].local);
    assert!(!state.branches[1].local && !state.branches[1].remote);
}

/// A name from an agent goes to git as an argument; one that reads as an
/// option is never passed.
#[test]
fn a_branch_name_that_could_be_an_option_is_not_asked_about() {
    assert!(plain_branch("bugfix/LED-54308_retry"));
    assert!(!plain_branch("--upload-pack=touch /tmp/x"));
    assert!(!plain_branch("main..evil"));
    assert!(!plain_branch("has space"));
}
