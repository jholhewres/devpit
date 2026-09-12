//! Throwing away uncommitted work in a path.
//!
//! Every path here has already been resolved and checked against the project
//! root by the caller — this module only decides *how* git forgets it.

use std::path::Path;

use devpit_rpc::GitStatus;

use crate::status::status;
use crate::{run, GitError};

/// Undoes whatever a path is carrying.
///
/// A path git already has a version of — modified or deleted — is restored
/// from the index and then HEAD, in one `restore`. A path with no earlier
/// version — untracked, or `git add`ed but never committed — has nothing to
/// go back to, so it is removed instead: `clean` for the former, `rm` for the
/// latter, because `clean` skips what is already tracked and `rm` refuses
/// what is not. That removal is real data loss; the caller confirms before
/// this runs.
pub fn discard(root: &Path, paths: &[String]) -> Result<(), GitError> {
    if paths.is_empty() {
        return Ok(());
    }
    let current = status(root)?;

    let mut untracked = Vec::new();
    let mut added = Vec::new();
    let mut tracked = Vec::new();

    for path in paths {
        match current.paths.get(path) {
            Some(GitStatus::Untracked) => untracked.push(path.as_str()),
            Some(GitStatus::Added) => added.push(path.as_str()),
            _ => tracked.push(path.as_str()),
        }
    }

    if !untracked.is_empty() {
        // No `-d`: status is read with `--untracked-files=all`, so it only
        // ever hands back individual files, never a bare directory `clean`
        // would need `-d` to remove.
        let mut argv = vec!["clean", "-f", "-q", "--"];
        argv.extend(untracked);
        run(root, &argv)?;
    }
    if !added.is_empty() {
        let mut argv = vec!["rm", "-f", "-q", "--"];
        argv.extend(added);
        run(root, &argv)?;
    }
    if !tracked.is_empty() {
        let mut argv = vec!["restore", "--staged", "--worktree", "--"];
        argv.extend(tracked);
        run(root, &argv)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;

    #[test]
    fn a_modified_file_goes_back_to_head() {
        let dir = tempfile::tempdir().expect("tempdir");
        fixture::repo(dir.path());
        std::fs::write(dir.path().join("a.txt"), "one\n").expect("write");
        fixture::commit(dir.path(), "first");

        std::fs::write(dir.path().join("a.txt"), "two\n").expect("write");
        discard(dir.path(), &["a.txt".to_owned()]).expect("discard");

        assert_eq!(
            std::fs::read_to_string(dir.path().join("a.txt")).expect("read"),
            "one\n"
        );
    }

    #[test]
    fn an_untracked_file_is_deleted_not_restored() {
        let dir = tempfile::tempdir().expect("tempdir");
        fixture::repo(dir.path());
        std::fs::write(dir.path().join("a.txt"), "one\n").expect("write");
        fixture::commit(dir.path(), "first");

        std::fs::write(dir.path().join("new.txt"), "scratch\n").expect("write");
        discard(dir.path(), &["new.txt".to_owned()]).expect("discard");

        assert!(
            !dir.path().join("new.txt").exists(),
            "the file survived discard"
        );
    }

    #[test]
    fn a_staged_new_file_is_gone_from_both_index_and_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        fixture::repo(dir.path());
        std::fs::write(dir.path().join("a.txt"), "one\n").expect("write");
        fixture::commit(dir.path(), "first");

        std::fs::write(dir.path().join("new.txt"), "scratch\n").expect("write");
        run(dir.path(), &["add", "new.txt"]).expect("stage");

        discard(dir.path(), &["new.txt".to_owned()]).expect("discard");

        assert!(
            !dir.path().join("new.txt").exists(),
            "the file survived discard"
        );
        let left = status(dir.path()).expect("status");
        assert!(
            !left.paths.contains_key("new.txt"),
            "the index still has it"
        );
    }
}
