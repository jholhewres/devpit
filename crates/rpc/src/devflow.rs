//! Devflow changes only the person makes, as the window asks for them.

use serde::{Deserialize, Serialize};
use specta::Type;

/// What the click does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DevflowChange {
    /// Devflow switched on in the project.
    Enable,
    /// Decisions moved from shadow to enforce in the project's Devflow.
    Enforce,
}

/// One change waiting for the person's click.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowAsk {
    pub id: String,
    pub project_id: String,
    pub project: String,
    pub change: DevflowChange,
    /// Seconds since the epoch.
    pub asked_at: f64,
}

/// A list, so tomorrow's field has somewhere to go.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowAsks {
    pub asks: Vec<DevflowAsk>,
}
