//! A file's contents, as the editor needs them.

use serde::{Deserialize, Serialize};
use specta::Type;

/// What a file is, so the screen knows what to draw rather than guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    Text,
    Markdown,
    Image,
    Pdf,
    /// Nothing this window can draw. The screen says which type and how big.
    Binary,
}

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
    pub kind: FileKind,
    /// The bytes, as a `data:` URL, for an image or a PDF small enough to
    /// carry. Absent for everything else — text goes in `text`.
    pub data_url: Option<String>,
    /// The absolute path, for revealing in the file manager. Never drawn.
    pub full_path: String,
}

/// The answer to a write.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileSaved {
    pub path: String,
    pub bytes: f64,
    pub read_at: f64,
}
