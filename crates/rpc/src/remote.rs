//! What the window asks of a checkout's remote.

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
