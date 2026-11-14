//! What the Changes panel asks of a checkout and its remote.

use serde::{Deserialize, Serialize};
use specta::Type;

/// What the Changes panel asks of a branch's remote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum RemoteAct {
    Fetch,
    Pull,
    Push,
    /// Pull, then push.
    Sync,
}

/// A first commit message, for the person to read and edit before committing.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CommitDraft {
    pub message: String,
}
