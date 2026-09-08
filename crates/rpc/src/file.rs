//! A file's contents, as the editor needs them.

use serde::{Deserialize, Serialize};
use specta::Type;

/// What is in a file, or why it is not shown.
///
/// A binary file is refused rather than rendered: a megabyte of bytes drawn as
/// replacement characters is worse than a sentence saying it is not text, and
/// saving it back would corrupt it.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileContents {
    /// Relative to the worktree root, the same path the tree hands out.
    pub path: String,
    pub text: Option<String>,
    /// Set when `text` is absent, and it says which reason.
    pub not_shown: Option<String>,
    pub bytes: f64,
    /// The mtime the read saw, given back on write so a save can refuse to
    /// overwrite a change it never saw.
    pub read_at: f64,
}

/// The answer to a write.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileSaved {
    pub path: String,
    pub bytes: f64,
    pub read_at: f64,
}
