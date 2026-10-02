//! Voice messages, and how they become words.

use serde::{Deserialize, Serialize};
use specta::Type;

/// How voice messages become words, as the settings show it. The key never
/// comes back: only whether one is kept.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Transcribing {
    /// `local`, `api` or `off`.
    pub engine: String,
    pub model: String,
    /// A language code, or empty for the system's.
    pub language: String,
    pub url: String,
    pub key_set: bool,
    /// The whisper found on this machine, by name, when there is one.
    pub local: Option<String>,
}

/// A voice message, heard: its words, or why there are none.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    pub text: Option<String>,
    pub note: Option<String>,
}
