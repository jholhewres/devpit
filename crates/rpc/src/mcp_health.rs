//! How an account's MCP servers answer, as its CLI measures them.

use serde::{Deserialize, Serialize};
use specta::Type;

/// One server, as `mcp list` reported it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct McpServerHealth {
    pub name: String,
    /// The command it runs, or the address it is reached at.
    pub target: String,
    pub state: McpState,
    /// The CLI's reason, when it gave one.
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum McpState {
    Connected,
    Failed,
    /// Reachable, and waiting on the person to sign in to it.
    NeedsAuth,
}

/// `mcp.health`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct McpHealth {
    /// The account it was measured as.
    pub profile_id: String,
    pub servers: Vec<McpServerHealth>,
}

/// How devpit's own MCP answers: the loopback door its tools post to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentHealth {
    /// Whether the last check was answered.
    pub answering: bool,
    /// How long the last answer took.
    pub latency_ms: Option<f64>,
    /// When it was last checked, in ms since the epoch.
    pub checked_at: Option<f64>,
    /// Checks in a row that went unanswered.
    pub failures: u32,
    /// Restarts since the app started, by hand or by itself.
    pub restarts: u32,
    pub last_restart_at: Option<f64>,
    /// Why the last check failed, or why the last restart happened.
    pub detail: Option<String>,
}
