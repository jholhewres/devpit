//! Whether git ignores a file, and keeping one out of git without touching the
//! repository's own `.gitignore`.

use std::io::Write as _;
use std::path::Path;

use crate::invoke::run;
use crate::GitError;

/// Whether git in `root` ignores `relative`. Outside a repository nothing is
/// tracked, so nothing needs ignoring.
pub fn ignored(root: &Path, relative: &str) -> Result<bool, GitError> {
    match run(root, &["check-ignore", "-q", "--", relative]) {
        Ok(_) => Ok(true),
        Err(GitError::NotARepository { .. }) => Ok(true),
        // `check-ignore` answers "not ignored" with exit 1 and nothing said.
        Err(GitError::Failed { stderr, .. }) if stderr.is_empty() => Ok(false),
        Err(err) => Err(err),
    }
}

/// Keeps `relative` out of git in this clone only: a line in its
/// `info/exclude`, never in a versioned `.gitignore`.
pub fn exclude(root: &Path, relative: &str) -> Result<(), GitError> {
    let said = run(root, &["rev-parse", "--git-path", "info/exclude"])?;
    let file = root.join(said.trim());
    let failed = |err: std::io::Error| GitError::Failed {
        command: "info/exclude".to_owned(),
        stderr: err.to_string(),
    };
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(failed)?;
    }
    let mut out = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)
        .map_err(failed)?;
    writeln!(out, "/{relative}").map_err(failed)
}

#[cfg(test)]
#[path = "ignoring_tests.rs"]
mod tests;
