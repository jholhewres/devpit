//! The project vocabulary, shared by every producer of it.
//!
//! One noun: a project *is* its workspace. There is no second object to create
//! or switch between, and everything here belongs to exactly one project.
//!
//! These types are the contract, so they live here rather than in whichever
//! crate happens to fill them in. `crates/git` reads a repository into them
//! and the store reads rows into them; neither owns a second copy that would
//! drift from this one.

use serde::{Deserialize, Serialize};
use specta::Type;

/// How a path stands with git.
///
/// Deliberately smaller than git's own vocabulary: the screen draws four
/// colours, and a status the screen cannot draw is a status nobody asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum GitStatus {
    Clean,
    Modified,
    Added,
    Deleted,
    Untracked,
}

/// A checkout of the project.
///
/// Real git worktrees, so the fields are git's own facts and nothing this
/// product invented.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Worktree {
    /// Stable across restarts: the absolute path, hashed.
    pub id: String,
    /// `main`, or a detached HEAD's short oid.
    pub branch: String,
    /// Last path segment. The whole path is a tooltip; it never fits a row.
    pub folder: String,
    pub ahead: u32,
    pub behind: u32,
    /// Files with uncommitted work, or `null` when git could not be read.
    ///
    /// Null rather than 0: the two look identical on screen and one of them is
    /// a lie. Every count in this contract that can fail to be read is
    /// optional for the same reason.
    pub dirty_files: Option<u32>,
    /// The checkout the project opens into.
    pub current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub path: String,
    pub status: GitStatus,
    pub added: u32,
    pub removed: u32,
    /// True when the change is in the index — what a commit would take.
    pub staged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    /// Abbreviated, because that is what is read and quoted.
    pub sha: String,
    pub subject: String,
    pub author: String,
    /// Seconds since the epoch.
    ///
    /// `f64` and not `i64`, because this crosses into a JavaScript number and
    /// that is what a JavaScript number is. An `f64` represents every whole
    /// second exactly up to 2^53 of them — 285 million years — so nothing is
    /// lost, and the type says so instead of leaving a 64-bit integer to be
    /// silently truncated by `JSON.parse`.
    ///
    /// Formatting a duration stays the screen's job, in the reader's locale
    /// and against the reader's clock.
    pub committed_at: f64,
}

/// One entry of the file tree.
///
/// Children are `None` for a file and `Some` for a directory — including an
/// empty `Some`, which is how an empty directory differs from a file.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileNode {
    pub name: String,
    /// Relative to the worktree root. What every other command takes back.
    pub path: String,
    pub status: GitStatus,
    pub children: Option<Vec<FileNode>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub body: String,
    /// Seconds since the epoch; see `Commit::committed_at` for why `f64`.
    pub created_at: f64,
}

/// A project as the window needs it.
///
/// The worktrees come with it because the row that names a project is the row
/// that reports its branch and its dirt — fetching those separately would draw
/// the list once and then correct it.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    /// Local only. An absolute path means nothing on another machine.
    pub root_path: String,
    /// Optional heading in the list.
    pub group: Option<String>,
    /// Persistent and mandatory, from the trust workspace: it is what keeps
    /// one client's context from being read as another's.
    pub accent: String,
    pub worktrees: Vec<Worktree>,
    /// Why the repository could not be read, when it could not be. Present and
    /// non-null is the only honest way to draw a project whose git is missing.
    pub unreadable: Option<String>,
}

// ── Responses ────────────────────────────────────────────────────────────
//
// Objects, never bare lists: tomorrow's extra field must not break today's
// consumer, and a bare list has nowhere to put it.

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectList {
    pub projects: Vec<Project>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTree {
    pub nodes: Vec<FileNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectChanges {
    pub changes: Vec<Change>,
    /// Totals come from the server so the screen never adds up a list it may
    /// have truncated.
    pub added: u32,
    pub removed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectHistory {
    pub commits: Vec<Commit>,
    /// Whether git has more commits than this page carries — what the screen
    /// reads to decide whether "load older" still does anything.
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectNotes {
    pub notes: Vec<Note>,
}
