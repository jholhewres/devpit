//! The conversations a project has had, as rows.
//!
//! Apart from `Conversation` because a row is not a transcript: drawing a
//! list of them out of the full type would read every message ever said to
//! put six lines on screen.

use serde::{Deserialize, Serialize};
use specta::Type;

/// One conversation, as a row in a list.
///
/// A summary, not a `Conversation`: that one carries every message, and a
/// list of them would read the whole history to draw a sidebar.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub id: String,
    /// The first thing the person said. Their words, not a summary written
    /// by a model — the row exists to be recognised.
    pub title: String,
    pub profile: String,
    pub model: Option<String>,
    pub cost_usd: f64,
    /// Unix seconds, when it was last spoken in.
    pub last_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Conversations {
    pub conversations: Vec<Thread>,
}
