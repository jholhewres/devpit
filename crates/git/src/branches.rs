//! The branches this checkout can move between.

use std::path::Path;

use crate::{run, GitError};

/// One branch, as the switcher needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    pub name: String,
    /// True for the one HEAD is on.
    pub current: bool,
    /// The subject of its last commit, so a name nobody remembers still says
    /// what it is.
    pub subject: String,
}

/// The branch a checkout is on, empty when HEAD is detached.
///
/// `--show-current` and not `status`: the name is one line, and a step asking
/// which branch it is on should not pay for a scan of the working tree.
pub fn branch_at(root: &Path) -> Result<String, GitError> {
    Ok(run(root, &["branch", "--show-current"])?.trim().to_owned())
}

/// Every local branch, the current one first.
///
/// Local only: listing remotes would offer to switch to something that has to
/// be fetched first, and this never fetches on its own.
pub fn branches(root: &Path) -> Result<Vec<Branch>, GitError> {
    let raw = run(
        root,
        &[
            "for-each-ref",
            "--format=%(HEAD)%00%(refname:short)%00%(contents:subject)",
            "refs/heads",
        ],
    )?;

    let mut found: Vec<Branch> = raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut fields = line.split('\0');
            Branch {
                current: fields.next().unwrap_or_default().trim() == "*",
                name: fields.next().unwrap_or_default().to_owned(),
                subject: fields.next().unwrap_or_default().to_owned(),
            }
        })
        .filter(|branch| !branch.name.is_empty())
        .collect();

    found.sort_by_key(|branch| !branch.current);
    Ok(found)
}

/// Moves this checkout to another branch.
///
/// git refuses on its own when the move would drop uncommitted work, and its
/// refusal is passed through rather than reworded: it names the files, and a
/// summary here would name fewer.
pub fn switch(root: &Path, name: &str) -> Result<(), GitError> {
    run(root, &["switch", "--", name]).map(|_| ())
}
