//! Reading a repository, by asking git.
//!
//! # Why the binary and not a library
//!
//! Every fact here is one that git already computes and guarantees a stable
//! machine-readable form for: `--porcelain=v2`, `worktree list --porcelain`,
//! `--numstat`, and `log --format`. Those formats are documented as scripting
//! interfaces and do not change under us.
//!
//! Linking a git implementation instead would buy nothing this product needs
//! and cost a C toolchain on every contributor's machine, or a pure-Rust
//! reimplementation whose disagreements with the real git would surface as
//! bugs nobody can reproduce. The app manages git worktrees and runs
//! terminals — a machine without git cannot run it anyway.
//!
//! The trade is real and named: a process per query. Each of these returns in
//! single-digit milliseconds on a warm repository, and none of them sits in a
//! frame budget.

mod clone;
mod log;
mod status;
mod worktrees;

use std::path::{Path, PathBuf};
use std::process::Command;

pub use clone::{clone, folder_for};
pub use log::history;
pub use status::{changes, status, Status};
pub use worktrees::{worktree_path, worktrees};

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("git is not installed, or not on PATH")]
    Missing,

    #[error("{path} is not a git repository")]
    NotARepository { path: PathBuf },

    #[error("git {command} failed: {stderr}")]
    Failed { command: String, stderr: String },

    #[error("git {command} answered something this build cannot read: {detail}")]
    Unreadable { command: String, detail: String },
}

/// Runs git in `root` and returns stdout.
///
/// `-c core.quotepath=false` is not optional: without it git escapes any byte
/// over 0x7f in a path, so a file named `créditos.ts` comes back as
/// `cr\303\251ditos.ts` and every path comparison downstream misses.
fn run(root: &Path, args: &[&str]) -> Result<String, GitError> {
    let output = Command::new("git")
        .arg("-c")
        .arg("core.quotepath=false")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => GitError::Missing,
            _ => GitError::Failed {
                command: args.join(" "),
                stderr: err.to_string(),
            },
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        // Told apart from a generic failure because the screen says something
        // different: a folder that is not a repository is a thing the person
        // can fix, and a git that crashed is not.
        if stderr.contains("not a git repository") {
            return Err(GitError::NotARepository {
                path: root.to_path_buf(),
            });
        }
        return Err(GitError::Failed {
            command: args.join(" "),
            stderr,
        });
    }

    // Lossy rather than strict: a path that is not UTF-8 must not take the
    // whole listing down with it. The replacement character is visible in the
    // row, which is the honest outcome.
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// A stable id for a checkout: its absolute path, hashed.
///
/// Hashed rather than the path itself because the id crosses into the UI as a
/// React key and into logs, and neither is a place for someone's home
/// directory.
pub(crate) fn identify(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(path.to_string_lossy().as_bytes());
    let hex: String = digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("wt_{hex}")
}

#[cfg(test)]
pub(crate) mod fixture {
    use std::path::Path;
    use std::process::Command;

    /// A real repository in a tempdir.
    ///
    /// Tests run against git itself rather than against recorded strings: the
    /// parsers exist to survive the version of git on the machine, and a
    /// recorded string cannot tell us when that assumption breaks.
    pub fn repo(dir: &Path) {
        for args in [
            vec!["init", "--initial-branch=main", "-q"],
            vec!["config", "user.email", "test@example.invalid"],
            vec!["config", "user.name", "Test"],
            vec!["config", "commit.gpgsign", "false"],
        ] {
            let ok = Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(&args)
                .output()
                .expect("run git")
                .status
                .success();
            assert!(ok, "git {args:?} failed");
        }
    }

    pub fn commit(dir: &Path, message: &str) {
        for args in [vec!["add", "-A"], vec!["commit", "-q", "-m", message]] {
            Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(&args)
                .output()
                .expect("run git");
        }
    }
}
