//! What a review reads: the commits from where the work began to `HEAD`, and,
//! apart, what is not committed yet.
//!
//! The Changes panel diffs against the working tree, which is right for a
//! person looking at their own work. A review needs the two apart: a
//! submodule pointer moved but not committed read as if a commit had moved
//! it, and a reviewer judged code that was never going to land.

use std::path::Path;

use crate::invoke::run;
use crate::GitError;

/// The files and the diff a review of `base..head` reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reviewed {
    pub files: Vec<String>,
    pub diff: String,
    /// The commits in `base..head`.
    pub commits: u32,
}

/// The heading the work not yet committed is put under, after the commits.
pub const UNCOMMITTED: &str = "# Not committed yet: the working tree against HEAD";

/// The commits `base..head`, a submodule's as its log, and then what the
/// tracked files hold that `HEAD` does not.
pub fn review_diff(root: &Path, base: &str, head: &str) -> Result<Reviewed, GitError> {
    let range = format!("{base}..{head}");
    let mut files = names(&run(root, &["diff", "--name-only", &range])?);
    let mut diff = run(root, &["diff", "--submodule=log", &range])?;
    let loose = run(root, &["diff", "--submodule=log", "HEAD"])?;
    if !loose.trim().is_empty() {
        for name in names(&run(root, &["diff", "--name-only", "HEAD"])?) {
            if !files.contains(&name) {
                files.push(name);
            }
        }
        diff.push_str(&format!("\n{UNCOMMITTED}\n\n{loose}"));
    }
    let commits = run(root, &["rev-list", "--count", &range])?
        .trim()
        .parse()
        .unwrap_or(0);
    Ok(Reviewed {
        files,
        diff,
        commits,
    })
}

/// Whether code changed from `from` to `to`, leaving `.devpit/` aside: what
/// Devflow writes about a card is not the card's code.
pub fn code_changed_between(root: &Path, from: &str, to: &str) -> Result<bool, GitError> {
    let raw = run(
        root,
        &[
            "diff",
            "--name-only",
            from,
            to,
            "--",
            ".",
            ":(exclude).devpit",
        ],
    )?;
    Ok(!names(&raw).is_empty())
}

fn names(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
#[path = "reviewed_tests.rs"]
mod tests;
