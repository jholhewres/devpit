//! What the person chose, and what this build can offer them.
//!
//! One object rather than a command per switch: the first run reads all of it
//! once and writes what changed, and a screen that has to make four round
//! trips to draw itself is a screen that draws itself four times.

use serde::{Deserialize, Serialize};
use specta::Type;

/// The appearance the window uses.
///
/// An enum and not a string, so a build that does not ship a theme cannot be
/// asked for one. `Light` is absent on purpose: shipping half a light theme
/// costs more than not having one, and the screen says "coming soon" rather
/// than offering a switch that lands somewhere unfinished.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Three states, not two. `null` is "never asked", which is what makes the
    /// first run ask instead of assuming a silent yes or a silent no.
    pub telemetry: Option<bool>,
    pub theme: Theme,
    /// The account this install is signed in as, when it is.
    pub account: Option<String>,
    /// Unix seconds, or null while the first run has not been finished.
    pub onboarded_at: Option<f64>,
}
