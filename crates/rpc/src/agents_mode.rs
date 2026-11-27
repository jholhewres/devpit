//! Agents mode: an orchestrator that plans and hands work to agents of its own.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Agents mode for one orchestrator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentsMode {
    pub on: bool,
}
