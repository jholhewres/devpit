//! The sign-ins a session runs on — the account's own, and what its project
//! needs — read before a start and shown in the footer.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::{Credentials, McpServerHealth};

/// One account of the agent CLI: a configuration directory and its sign-in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AccountSignIn {
    /// The profiles that run against it; `Claude Code` for the one none names.
    pub label: String,
    pub directory: String,
    pub state: Credentials,
    /// When the saved sign-in stops renewing itself, in ms since the epoch.
    pub until: Option<f64>,
    /// The command that signs it in again.
    pub fix: String,
}

/// A sign-in a project's sessions need — an AWS profile, gcloud, gh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NeededSignIn {
    pub what: String,
    pub holds: bool,
    pub fix: String,
}

/// `sign_ins`'s answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SignIns {
    pub accounts: Vec<AccountSignIn>,
    /// What the open project declares, checked now.
    pub needed: Vec<NeededSignIn>,
}

/// A sign-in a start would go without, and the command that fixes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MissingSignIn {
    pub what: String,
    pub command: String,
}

/// `session.launch_agent`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentLaunch {
    /// The line typed, or none when the start was held for a sign-in.
    pub line: Option<String>,
    /// What the start was held for.
    pub missing: Vec<MissingSignIn>,
    /// Servers the project needs that the last check found not answering:
    /// said, and the start went ahead.
    pub failing: Vec<McpServerHealth>,
}
