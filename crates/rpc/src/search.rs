//! `project.search` — content matches, grouped by file.
//!
//! Grouped here rather than left as a flat list of lines: a flat list would
//! make the screen re-group what the backend already knew when it walked
//! `git grep`'s output file by file.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchLine {
    pub line: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchFile {
    pub path: String,
    pub lines: Vec<SearchLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchHits {
    pub files: Vec<SearchFile>,
    /// How many lines matched, before the ceiling cut `files` short. The true
    /// size, even on a call where `files` cannot carry all of it. Named
    /// `matched` rather than `shown`: `shown` is what a screen renders after
    /// its own cap, the way `SearchResults.tsx` already uses the word for the
    /// slice of `hits` it draws — this is the number before either cap.
    pub matched: u32,
    /// True once `matched` is more than what `files` actually holds.
    pub truncated: bool,
    // No `skipped` field: `git grep --untracked` never reaching a gitignored
    // file is a permanent property of this command, not something that
    // varies call to call, so there is nothing here for a boolean to track.
    // The screen states it unconditionally instead — see `ContentResults`'
    // empty state in `SearchResults.tsx`.
}
