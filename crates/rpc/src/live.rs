//! The agent sessions running on this machine for one account, as the CLI
//! itself lists them — what an orchestrator can message, and what it shows.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LiveSession {
    /// The name another session messages it by.
    pub name: String,
    /// The CLI's own word: `busy` or `idle`.
    pub status: String,
    /// `interactive` or `background`.
    pub kind: String,
    /// Local only: the folder it runs in.
    pub cwd: String,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    /// The card whose checkout it runs in, when it runs in one.
    pub card_id: Option<String>,
    /// Milliseconds since the epoch, when its status last changed.
    pub since: Option<f64>,
    /// Whether it runs in one of devpit's own terminals, where a reply can be
    /// typed for the person.
    pub in_devpit: bool,
}

/// A list, so tomorrow's field has somewhere to go.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LiveSessions {
    pub sessions: Vec<LiveSession>,
}
