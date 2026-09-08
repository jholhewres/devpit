//! What a card's session is doing.

use serde::{Deserialize, Serialize};
use specta::Type;

/// A background agent session, as the board needs to draw it.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    /// What `attach`, `logs` and `stop` all take.
    pub short_id: String,
    pub status: SessionStatus,
}

/// What a card's session is doing.
///
/// `Blocked` is separate from `Busy` because it is the one state where nothing
/// happens until a person comes back — a board that cannot tell them apart
/// shows five cards working when one has been waiting on you for an hour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Busy,
    /// Waiting on you.
    Blocked,
    Done,
    Idle,
    /// The link is on the card but the CLI no longer lists it.
    Gone,
}
