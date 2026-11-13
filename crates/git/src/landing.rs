//! Whether a card's branch has landed, and putting a landed one away.
//!
//! Landed means every commit the branch made is in the default branch: what a
//! merge leaves behind. A squash merge rewrites the commits, so its branch
//! reads as not landed and is kept, which is the safe way to be wrong.

use std::path::Path;

use crate::lifecycle::short;
use crate::{run, GitError};

/// The refs the default branch may go by here: origin's own, then the usual
/// names, locally and on origin. Only the ones that exist.
fn defaults(root: &Path) -> Vec<String> {
    let origin = run(
        root,
        &["symbolic-ref", "--quiet", "refs/remotes/origin/HEAD"],
    )
    .ok()
    .map(|said| said.trim().to_owned())
    .filter(|said| !said.is_empty());
    origin
        .into_iter()
        .chain(
            ["refs/heads/main", "refs/heads/master"]
                .into_iter()
                .chain(["refs/remotes/origin/main", "refs/remotes/origin/master"])
                .map(str::to_owned),
        )
        .filter(|name| run(root, &["rev-parse", "--verify", "--quiet", name]).is_ok())
        .collect()
}

/// Whether `branch` made commits since `began` and every one of them is in the
/// default branch. A branch with none of its own has nothing to land.
pub fn landed(root: &Path, branch: &str, began: &str) -> bool {
    if matches!(branch, "main" | "master") {
        return false;
    }
    let tip = format!("refs/heads/{branch}");
    let own = run(root, &["rev-list", "--count", &format!("{began}..{tip}")])
        .ok()
        .and_then(|said| said.trim().parse::<u64>().ok())
        .unwrap_or(0);
    own > 0
        && defaults(root)
            .iter()
            .any(|base| run(root, &["merge-base", "--is-ancestor", &tip, base]).is_ok())
}

/// Deletes a card's branch once it has landed.
///
/// Only a branch named for this card, so nothing a person made by hand, and
/// only after asking again whether it landed: the check and the delete are
/// one step to the caller.
pub fn drop_landed(root: &Path, branch: &str, card_id: &str, began: &str) -> Result<(), GitError> {
    if !branch.ends_with(&format!("-{}", short(card_id))) {
        return Err(GitError::Refused(format!(
            "{branch} is not this card's branch"
        )));
    }
    if !landed(root, branch, began) {
        return Err(GitError::Refused(format!(
            "{branch} has commits the default branch does not"
        )));
    }
    run(root, &["branch", "-D", branch]).map(|_| ())
}

/// Whether git lists `path` as one of `root`'s linked worktrees: never the
/// main checkout, and never a folder git does not know.
pub fn linked_checkout(root: &Path, path: &Path) -> bool {
    let Ok(listed) = run(root, &["worktree", "list", "--porcelain"]) else {
        return false;
    };
    let wanted = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    listed
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        // The first entry is always the main worktree.
        .skip(1)
        .any(|listed| {
            let listed = Path::new(listed.trim());
            std::fs::canonicalize(listed).unwrap_or_else(|_| listed.to_path_buf()) == wanted
        })
}

#[cfg(test)]
#[path = "landing_tests.rs"]
mod tests;
