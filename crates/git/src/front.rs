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

/// A file as `HEAD` holds it — the left side of the Changes panel's diff.
///
/// `None` when `HEAD` has no such file: an untracked or newly added file, or a
/// repository with no commit yet. That is an empty left side, not a failure,
/// so it is asked for first rather than read out of an error.
pub fn at_head(worktree: &Path, path: &str) -> Result<Option<String>, GitError> {
    // `./` makes the path relative to the worktree, not to the repository
    // root, which is what every other path here is.
    let object = format!("HEAD:./{path}");
    if run(worktree, &["cat-file", "-e", &object]).is_err() {
        return Ok(None);
    }
    run(worktree, &["show", &object]).map(Some)
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
#[path = "front_tests.rs"]
mod tests;
