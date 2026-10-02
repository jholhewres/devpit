//! A pause: devpit quiet for a while, as the person asked.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Whether devpit is paused, and until when.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Paused {
    pub on: bool,
    /// Seconds since the epoch it ends at; `null` while on means "until
    /// resumed".
    pub until: Option<f64>,
}
