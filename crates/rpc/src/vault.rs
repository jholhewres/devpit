//! A project's secrets, as the window sees them: names and dates, never values.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Secret {
    /// The variable's name, which is all an agent ever sees of it.
    pub name: String,
    /// When it was last set, in seconds since the epoch.
    pub set_at: f64,
    /// Every how many days it is to be rotated, when the person asked.
    #[serde(default)]
    pub rotate_days: Option<u32>,
    /// When it is next due to be rotated, in seconds since the epoch.
    #[serde(default)]
    pub rotate_at: Option<f64>,
    /// The card on the project's board whose date reminds of it.
    #[serde(default)]
    pub reminder_card: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SecretList {
    pub secrets: Vec<Secret>,
    /// The file the values are kept in.
    pub file: String,
}
