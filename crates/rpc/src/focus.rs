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

/// What a focus has been holding, a page at a time.
///
/// An object rather than a bare list, and `more` rather than a total: a count
/// would mean reading every row to say a number nobody acts on, and what the
/// screen needs to know is whether to ask again.
// No `PartialEq`: `Notice` does not derive it, and a page of notices is not a
// thing two of which are ever compared.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Waiting {
    pub notices: Vec<crate::card::Notice>,
    /// True when the page filled, so there may be another behind it.
    pub more: bool,
}
