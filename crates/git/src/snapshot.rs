//! What a checkout looked like at a moment, and what changed since.
//!
//! A turn of an agent edits files through its tools and through anything its
//! shell runs, so the only honest account of what a turn changed is the tree
//! before against the tree after — not the tool calls it happened to report.
//!
//! The snapshot is a tree object written through a throwaway index, so the
//! person's own index, stash and working tree are never touched. Untracked
//! files are included and ignored ones are not, exactly as `git add -A` sees
//! them.

use std::path::{Path, PathBuf};

use crate::GitError;

/// One path a turn changed, with its size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changed {
    pub path: String,
    pub added: u32,
    pub removed: u32,
}

fn git(root: &Path, index: Option<&Path>, args: &[&str]) -> Result<String, GitError> {
    let mut command = devpit_pty::host_env::command("git");
    command
        .arg("-c")
        .arg("core.quotepath=false")
        .arg("-C")
        .arg(root)
        .args(args);
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    let output = command.output().map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => GitError::Missing,
        _ => GitError::Failed {
            command: args.join(" "),
            stderr: err.to_string(),
        },
    })?;
    if !output.status.success() {
        return Err(GitError::Failed {
            command: args.join(" "),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// A throwaway index beside the real one, unique to this call.
fn scratch_index(root: &Path) -> Result<PathBuf, GitError> {
    let git_dir = git(root, None, &["rev-parse", "--absolute-git-dir"])?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_nanos())
        .unwrap_or_default();
    Ok(PathBuf::from(git_dir).join(format!(
        "devpit-snapshot-{}-{nanos}.index",
        std::process::id()
    )))
}

/// The whole working tree as a tree object, or an error when `root` is not a
/// repository.
pub fn snapshot(root: &Path) -> Result<String, GitError> {
    let index = scratch_index(root)?;
    let written = git(root, Some(&index), &["add", "-A"])
        .and_then(|_| git(root, Some(&index), &["write-tree"]));
    // Removed whichever way it went: a stale index here is a file nobody owns.
    let _ = std::fs::remove_file(&index);
    written
}

/// Every path that differs between two snapshots, with line counts.
///
/// Renames are read as a deletion and an addition: the paths come back one per
/// record that way, and a turn that moved a file changed both names.
pub fn changed_between(root: &Path, before: &str, after: &str) -> Result<Vec<Changed>, GitError> {
    if before == after {
        return Ok(Vec::new());
    }
    let raw = git(
        root,
        None,
        &["diff", "--numstat", "--no-renames", "-z", before, after],
    )?;
    Ok(raw
        .split('\0')
        .filter(|record| !record.is_empty())
        .filter_map(|record| {
            let mut fields = record.splitn(3, '\t');
            let added = fields.next()?;
            let removed = fields.next()?;
            let path = fields.next()?;
            // A binary file reads `-\t-`: a real change with no line count.
            Some(Changed {
                path: path.to_owned(),
                added: added.parse().unwrap_or(0),
                removed: removed.parse().unwrap_or(0),
            })
        })
        .collect())
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
