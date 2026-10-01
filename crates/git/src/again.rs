//! A card's worktree made again, after one was removed and its branch kept.

use std::path::Path;

use crate::invoke::run;
use crate::lifecycle::{assignable, short, Made};
use crate::GitError;

/// The branch an earlier checkout of this card left behind, found by the
/// card's own suffix: the title, and the name with it, may have changed.
pub fn branch_left(root: &Path, card_id: &str) -> Option<String> {
    let suffix = format!("-{}", short(card_id));
    run(
        root,
        &[
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/heads/devpit/",
        ],
    )
    .ok()?
    .lines()
    .map(str::trim)
    .find(|name| name.ends_with(&suffix))
    .map(str::to_owned)
}

/// A worktree made again on a branch that is still there, diffing against
/// where the work first began when that is known.
pub fn reattach(
    root: &Path,
    at: &Path,
    branch: &str,
    began: Option<&str>,
) -> Result<Made, GitError> {
    if !assignable(root, at) || at.exists() {
        return Err(GitError::Refused(format!(
            "{} cannot hold this card's worktree",
            at.display()
        )));
    }
    let base_ref = match began {
        Some(began) => began.to_owned(),
        None => run(root, &["merge-base", "HEAD", branch])?
            .trim()
            .to_owned(),
    };
    if let Some(parent) = at.parent() {
        std::fs::create_dir_all(parent).map_err(|err| GitError::Refused(err.to_string()))?;
    }
    run(
        root,
        &["worktree", "add", "-q", &at.to_string_lossy(), branch],
    )?;
    Ok(Made {
        path: at.to_path_buf(),
        branch: branch.to_owned(),
        base_ref,
    })
}
