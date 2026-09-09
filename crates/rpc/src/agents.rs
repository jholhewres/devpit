//! The agents on this machine, as the step picker needs them.

use serde::{Deserialize, Serialize};
use specta::Type;

/// An agent on this machine, as the step picker needs it.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    /// The name in the file's frontmatter — what a step stores.
    pub name: String,
    pub description: String,
    pub model: Option<String>,
    /// Where it came from — `omc`, `claude`, `yours`. Shown beside the name so
    /// two agents that share one can be told apart.
    pub source: String,
}

/// Response of `agents.list`.
///
/// The files that would not load come back too, by name. A step pointing at an
/// agent that is silently missing fails at the moment it runs, which is the
/// worst time to find out it was a typo.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Agents {
    pub agents: Vec<Agent>,
    pub rejected: Vec<RejectedAgent>,
    /// Where they live, so the screen can say where to put another one.
    pub directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RejectedAgent {
    pub file: String,
    pub reason: String,
}
