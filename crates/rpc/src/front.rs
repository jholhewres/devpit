//! What a line of work changed, and what it would cost to put it away.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Response of `card.diff` — what this front changed, against where it began.
///
/// `baseRef` travels with it so the screen can say what the diff is against.
/// A diff with no stated base is a diff nobody can check.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Front {
    pub worktree_path: Option<String>,
    pub base_ref: Option<String>,
    pub files: Vec<String>,
    pub diff: String,
    /// Work no commit holds. Named rather than counted, because the question
    /// it answers is "what would I lose".
    pub unsaved: Vec<String>,
}

/// Response of `session.changes` — where a session's folder stands, read for
/// the window that opens it over the orchestrator's chat.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SessionChanges {
    /// The branch, or a short commit when `HEAD` is detached.
    pub branch: String,
    /// The folder, by its last segment.
    pub folder: String,
    /// In a checkout of its own rather than the project's folder.
    pub worktree: bool,
    pub ahead: u32,
    pub behind: u32,
    /// What its commits are counted from: the card's base, or the branch's
    /// upstream. Absent when there is neither, and then there are none.
    pub base: Option<String>,
    pub changes: Vec<crate::Change>,
    /// The commits on top of `base`, newest first.
    pub commits: Vec<crate::Commit>,
    /// The diff of what is not committed, or for a card of everything since
    /// its base: what the files below it show.
    pub diff: String,
    /// The diff was longer than a window reads, and was cut.
    pub cut: bool,
}

/// Another repository a card's sessions wrote in: what `Changes` cannot see,
/// because it reads only the card's own checkout.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Elsewhere {
    /// The repository's folder, whole. Local only.
    pub root: String,
    /// The project's name when devpit has it, the folder's otherwise.
    pub name: String,
    pub project_id: Option<String>,
    pub files: Vec<ElsewhereFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ElsewhereFile {
    /// Relative to the repository.
    pub path: String,
    /// Still differs from its last commit there.
    pub uncommitted: bool,
}
