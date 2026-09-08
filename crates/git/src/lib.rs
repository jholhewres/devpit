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
mod front;
mod invoke;
mod log;
mod status;
mod worktrees;

use std::path::PathBuf;

#[cfg(test)]
pub(crate) use invoke::fixture;
pub(crate) use invoke::{identify, run, run_diffing};

pub use clone::{clone, folder_for};
pub use front::{changed_since, diff_file, diff_since, head_of, remove_front, unsaved_in};
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

    /// Refused before git was asked, so the message can name what is in the
    /// way rather than repeat git's.
    #[error("{0}")]
    Refused(String),
}
