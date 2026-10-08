use std::path::{Path, PathBuf};

use super::files;

#[test]
fn every_account_is_trusted_once_each() {
    let home = Path::new("/home/a");
    let found = files(
        home,
        None,
        [
            None,
            Some("/home/a/.claude-work".to_owned()),
            Some("/home/a/.claude-work".to_owned()),
        ]
        .into_iter(),
    );
    assert_eq!(
        found,
        [
            PathBuf::from("/home/a/.claude.json"),
            PathBuf::from("/home/a/.claude-work/.claude.json"),
        ]
    );
}

#[test]
fn a_profile_without_its_own_folder_uses_the_one_it_inherits() {
    let home = Path::new("/home/a");
    let found = files(home, Some("/home/a/.claude-b"), [None].into_iter());
    assert_eq!(found, [PathBuf::from("/home/a/.claude-b/.claude.json")]);
}

#[test]
fn an_account_outside_the_home_is_left_alone() {
    // A test's home under target/ with the real CLAUDE_CONFIG_DIR inherited.
    let home = Path::new("/repo/target/test-home");
    let found = files(home, Some("/home/a/.claude-b"), [None].into_iter());
    assert!(found.is_empty());
}
