//! devpit Plus, as this install knows it: from a signed entitlement it checks
//! offline.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PlusStatus {
    /// Whether this build checks Plus at all. Off until billing is switched
    /// on, and then nothing about Plus is drawn.
    pub enabled: bool,
    /// Whether a valid, unexpired entitlement is kept here.
    pub active: bool,
    pub plan: Option<String>,
    /// Seconds since the epoch.
    pub expires_at: Option<f64>,
}
