//! The index: what a commit would take, and taking it.
//!
//! Every path is passed with `--` in front of it, so a file named `-f` is a
//! file and not a flag. Git accepts that separator everywhere, and leaving it
//! out is how a path becomes an argument.

use std::path::Path;

use devpit_rpc::Commit;

use crate::{run, GitError};

/// Puts these paths in the index.
pub fn stage(root: &Path, paths: &[String]) -> Result<(), GitError> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut argv = vec!["add", "--"];
    argv.extend(paths.iter().map(String::as_str));
    run(root, &argv).map(|_| ())
}

/// Takes these paths back out of the index, leaving the file alone.
///
/// `restore --staged` and not `reset`: on a repository with no commits there
/// is no HEAD to reset against, and `reset` fails there while this does not.
pub fn unstage(root: &Path, paths: &[String]) -> Result<(), GitError> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut argv = vec!["restore", "--staged", "--"];
    argv.extend(paths.iter().map(String::as_str));
    run(root, &argv).map(|_| ())
}

/// Commits what is in the index.
///
/// Refuses an empty message here rather than letting git open an editor: this
/// process has no terminal to open one in, and the command would hang.
pub fn commit(root: &Path, message: &str) -> Result<Commit, GitError> {
    if message.trim().is_empty() {
        return Err(GitError::Refused("a commit needs a message".to_owned()));
    }
    run(root, &["commit", "-m", message])?;
    let line = run(root, &["log", "-1", "--format=%h%x00%s%x00%an%x00%at"])?;
    let mut fields = line.trim().split('\0');
    Ok(Commit {
        sha: fields.next().unwrap_or_default().to_owned(),
        subject: fields.next().unwrap_or_default().to_owned(),
        author: fields.next().unwrap_or_default().to_owned(),
        committed_at: fields
            .next()
            .and_then(|seconds| seconds.parse::<f64>().ok())
            .unwrap_or_default(),
    })
}
