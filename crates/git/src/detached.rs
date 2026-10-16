//! A commit checked out on its own, beside the person's checkout: what a
//! review without a card reads a pull request or a range in, so nothing the
//! person has checked out moves.

use std::path::Path;

use crate::{run, GitError};

/// A revision an agent named, as git is given it: never an option to git.
pub fn plain_revision(said: &str) -> bool {
    !said.is_empty()
        && said.len() <= 200
        && !said.starts_with('-')
        && !said.chars().any(|ch| ch.is_whitespace() || ch.is_control())
}

/// A full commit id, as devpit hands one to `worktree add`.
fn full_commit(said: &str) -> bool {
    matches!(said.len(), 40 | 64) && said.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// The commit `revision` names here, refused when it names none.
pub fn commit_of(root: &Path, revision: &str) -> Result<String, GitError> {
    if !plain_revision(revision) {
        return Err(GitError::Refused(format!(
            "`{revision}` is not a branch or a commit"
        )));
    }
    let asked = format!("{revision}^{{commit}}");
    run(root, &["rev-parse", "--verify", "--quiet", &asked])
        .map(|sha| sha.trim().to_owned())
        .map_err(|_| GitError::Refused(format!("`{revision}` names no commit here")))
}

/// Where `a` and `b` meet.
pub fn merge_base(root: &Path, a: &str, b: &str) -> Result<String, GitError> {
    let (a, b) = (commit_of(root, a)?, commit_of(root, b)?);
    Ok(run(root, &["merge-base", &a, &b])?.trim().to_owned())
}

/// Brings pull request `number`'s head from `origin` without touching any
/// checkout: it lands in the object store and `FETCH_HEAD` only. Never asks
/// for a credential, which would hang the review.
pub fn fetch_pull(root: &Path, number: u64) -> Result<(), GitError> {
    let reference = format!("refs/pull/{number}/head");
    crate::remote::talk(root, &["fetch", "-q", "--no-tags", "origin", &reference])
}

/// `commit` checked out, detached, in a new worktree at `at` — never the
/// main checkout, never a branch anyone is on.
pub fn add_detached(root: &Path, at: &Path, commit: &str) -> Result<(), GitError> {
    if !full_commit(commit) {
        return Err(GitError::Refused(format!("`{commit}` is not a commit id")));
    }
    if !crate::assignable(root, at) || at.exists() {
        return Err(GitError::Refused(format!(
            "{} is not a new folder",
            at.display()
        )));
    }
    if let Some(parent) = at.parent() {
        std::fs::create_dir_all(parent).map_err(|err| GitError::Refused(err.to_string()))?;
    }
    let target = at.to_string_lossy();
    run(
        root,
        &["worktree", "add", "-q", "--detach", &target, commit],
    )
    .map(|_| ())
}

/// The worktree [`add_detached`] made, gone, and git's record of it with it.
pub fn remove_detached(root: &Path, at: &Path) {
    let target = at.to_string_lossy();
    if run(root, &["worktree", "remove", "--force", &target]).is_err() {
        let _ = std::fs::remove_dir_all(at);
        let _ = run(root, &["worktree", "prune"]);
    }
}

#[cfg(test)]
#[path = "detached_tests.rs"]
mod tests;
