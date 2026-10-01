//! A card's date going off as a reminder, as the window draws it.

use serde::{Deserialize, Serialize};
use specta::Type;

/// One card whose date went off, or is still to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub card_id: String,
    pub project_id: String,
    /// The project's name, so a reminder from another project says where.
    pub project: Option<String>,
    pub title: String,
    /// Seconds since the epoch.
    pub due_at: f64,
    /// Whether the date carries a time somebody chose, rather than a day.
    pub timed: bool,
}

/// A list, so tomorrow's field has somewhere to go.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Reminders {
    pub reminders: Vec<Reminder>,
}
