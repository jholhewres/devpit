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
