//! The focus that is on.
//!
//! Two fields, because a focus is interface state and one rule rather than an
//! object of the domain: which project is being worked on, and the second it
//! began. How long it has been on is read from `since` and the clock, never
//! counted in the window — a counter in memory is a focus that forgets itself
//! when the app restarts.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HeadsDown {
    pub project_id: String,
    /// Seconds since the epoch. `f64` because the contract forbids 64-bit
    /// integers, and every other timestamp that crosses it is one too.
    pub since: f64,
}
