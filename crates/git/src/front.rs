//! What changed on this line of work, and whether it is safe to put away.
//!
//! The diff is against the ref the front started from, never against `HEAD`.
//! That is the question a card is actually asking — *what changed here* — and
//! `HEAD` answers a different one, which drifts further from it with every
//! commit anyone lands on the base branch.

use std::path::Path;

use crate::{run, GitError};

/// The commit a front started from.
///
/// Recorded when the worktree is made, because it cannot be recovered later:
/// once the base branch moves, nothing on disk remembers where this one began.
pub fn head_of(root: &Path) -> Result<String, GitError> {
    Ok(run(root, &["rev-parse", "HEAD"])?.trim().to_owned())
}

/// The names of the files this front changed, against where it began.
///
/// Includes work not yet committed: a front is judged by what it did, and half
/// of that is usually still in the working tree.
pub fn changed_since(worktree: &Path, base_ref: &str) -> Result<Vec<String>, GitError> {
    let raw = run(worktree, &["diff", "--name-only", base_ref])?;
    Ok(raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

/// The diff of this front, against where it began.
pub fn diff_since(worktree: &Path, base_ref: &str) -> Result<String, GitError> {
    run(worktree, &["diff", base_ref])
}

/// Work in the checkout that no commit holds.
///
/// The question asked before putting a front away: uncommitted work is work
/// that disappears with the directory, and no product should decide on its own
/// that it was not worth keeping.
pub fn unsaved_in(worktree: &Path) -> Result<Vec<String>, GitError> {
    let raw = run(worktree, &["status", "--porcelain"])?;
    Ok(raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

/// Removes a front's checkout, refusing while it holds unsaved work.
///
/// `git worktree remove` refuses that too, and this refuses first so the
/// message names the files rather than repeating git's.
pub fn remove_front(root: &Path, worktree: &Path) -> Result<(), GitError> {
    let unsaved = unsaved_in(worktree)?;
    if !unsaved.is_empty() {
        return Err(GitError::Refused(format!(
            "{} has {} change{} nothing has saved yet",
            worktree.display(),
            unsaved.len(),
            if unsaved.len() == 1 { "" } else { "s" }
        )));
    }
    run(
        root,
        &["worktree", "remove", &worktree.display().to_string()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// A repository with one commit, and git configured enough to make more.
    fn repository() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let at = dir.path();
        for args in [
            vec!["init", "-q", "-b", "main"],
            vec!["config", "user.email", "t@example.com"],
            vec!["config", "user.name", "Test"],
        ] {
            Command::new("git")
                .args(&args)
                .current_dir(at)
                .output()
                .expect("git");
        }
        commit(at, "start", "first");
        dir
    }

    fn commit(at: &Path, file: &str, message: &str) {
        std::fs::write(at.join(file), format!("{message}\n")).expect("write");
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(at)
            .output()
            .expect("add");
        Command::new("git")
            .args(["commit", "-q", "-m", message])
            .current_dir(at)
            .output()
            .expect("commit");
    }

    /// The reason this module exists, in one test.
    ///
    /// Two commits land on the base after the front started. Against `HEAD`
    /// they would show up as this front's work; against the ref it began from,
    /// they do not.
    #[test]
    fn the_diff_is_against_where_the_front_began_not_against_head() {
        let repo = repository();
        let root = repo.path();
        let base = head_of(root).expect("head");

        let front = root.join("front");
        Command::new("git")
            .args(["worktree", "add", "-q", "-b", "front", "front"])
            .current_dir(root)
            .output()
            .expect("worktree add");

        // The base moves on without this front.
        commit(root, "elsewhere.txt", "someone else");
        commit(root, "elsewhere-again.txt", "and again");

        // And the front does its own work.
        commit(&front, "mine.txt", "my change");

        let changed = changed_since(&front, &base).expect("diff");
        assert!(
            changed.contains(&"mine.txt".to_owned()),
            "the front's own file is missing: {changed:?}"
        );
        assert!(
            !changed.iter().any(|f| f.starts_with("elsewhere")),
            "work from the base leaked into this front's diff: {changed:?}"
        );
    }

    #[test]
    fn uncommitted_work_counts_as_this_front_s_change() {
        let repo = repository();
        let base = head_of(repo.path()).expect("head");
        std::fs::write(repo.path().join("wip.txt"), "not committed\n").expect("write");
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(repo.path())
            .output()
            .expect("add");

        let changed = changed_since(repo.path(), &base).expect("diff");
        assert!(changed.contains(&"wip.txt".to_owned()), "{changed:?}");
    }

    #[test]
    fn a_clean_checkout_has_nothing_unsaved() {
        let repo = repository();
        assert!(unsaved_in(repo.path()).expect("status").is_empty());
    }

    #[test]
    fn work_nothing_has_saved_is_listed() {
        let repo = repository();
        std::fs::write(repo.path().join("wip.txt"), "unsaved\n").expect("write");
        assert_eq!(unsaved_in(repo.path()).expect("status").len(), 1);
    }

    /// A directory with unsaved work in it is not the product's to delete.
    #[test]
    fn a_front_holding_unsaved_work_refuses_to_be_removed() {
        let repo = repository();
        let root = repo.path();
        let front = root.join("front");
        Command::new("git")
            .args(["worktree", "add", "-q", "-b", "front", "front"])
            .current_dir(root)
            .output()
            .expect("worktree add");

        std::fs::write(front.join("wip.txt"), "unsaved\n").expect("write");

        let refused = remove_front(root, &front).expect_err("it should refuse");
        assert!(front.is_dir(), "the checkout was deleted anyway");
        assert!(
            refused.to_string().contains("nothing has saved"),
            "{refused}"
        );
    }

    #[test]
    fn a_clean_front_is_removed() {
        let repo = repository();
        let root = repo.path();
        let front = root.join("front");
        Command::new("git")
            .args(["worktree", "add", "-q", "-b", "front", "front"])
            .current_dir(root)
            .output()
            .expect("worktree add");

        remove_front(root, &front).expect("remove");
        assert!(!front.is_dir(), "the checkout is still there");
    }
}
