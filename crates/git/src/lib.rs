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

mod basing;
pub mod branches;
#[cfg(test)]
mod branches_tests;
mod clone;
mod discard;
mod front;
mod glance;
pub mod index;
#[cfg(test)]
mod index_tests;
mod invoke;
pub mod lifecycle;
#[cfg(test)]
mod lifecycle_tests;
mod log;
mod search;
mod snapshot;
mod sourcing;
pub mod standing;
mod status;
mod weblink;
mod worktrees;

use std::path::PathBuf;

#[cfg(test)]
pub(crate) use invoke::fixture;
pub(crate) use invoke::{identify, run, run_diffing};

pub use basing::{allowed as base_allowed, chosen as base_chosen, worktree_at, Refused};
pub use branches::{branch_at, branches, switch, Branch};
pub use clone::{clone, first_commit, folder_for, init, unborn};
pub use discard::discard;
pub use front::{at_head, changed_since, diff_since, head_of, remove_front, unsaved_in};
pub use glance::{glance, Glance};
pub use index::{commit, stage, unstage};
pub use lifecycle::{
    assignable, branch_for, create, disk_usage, remove, uncommitted, worktree_home, Loss, Made,
};
pub use log::{history, show};
pub use search::{grep, GrepHit, GrepOutcome, SearchFlags};
pub use snapshot::{changed_between, snapshot, Changed};
pub use sourcing::{hidden_as, hidden_in, origin_of, shown, word_of};
pub use standing::standing_at;
pub use status::{changes, status, Status};
pub use weblink::{commit_link, web_url, CommitLink};
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
