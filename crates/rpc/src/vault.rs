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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SecretList {
    pub secrets: Vec<Secret>,
    /// The file the values are kept in.
    pub file: String,
}
