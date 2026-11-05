//! How an answer ended, kept on it so a reopened conversation still says so.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::TurnEnd;

/// When an answer ended and why: the facts, read into words by the screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Ended {
    /// Unix seconds.
    pub at: f64,
    /// The CLI's own word, as [`TurnEnd::stop_reason`].
    pub stop_reason: Option<String>,
    pub is_error: bool,
    /// What the turn failed on, when devpit was told.
    #[serde(default)]
    pub error: Option<String>,
}

impl Ended {
    pub fn of(end: &TurnEnd, at: f64) -> Self {
        Self {
            at,
            stop_reason: end.stop_reason.clone(),
            is_error: end.is_error,
            error: None,
        }
    }
}
