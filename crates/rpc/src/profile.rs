//! A profile is a command and the driver that reads it.
//!
//! The same CLI signed into two accounts is two commands on the PATH —
//! `claude` for one, `claudin` for the other. Which account a conversation
//! used is a fact about it, so the profile is recorded and never changes.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    /// What the person calls this account.
    pub label: String,
    /// The binary to run. Two accounts differ here and nowhere else.
    pub command: String,
    /// Which driver reads its output.
    pub driver: String,
    /// Absent means the command is not on the PATH right now.
    pub path: Option<String>,
    /// The models the composer may pick from. They belong to the driver, and
    /// are carried here so one call answers the whole selector.
    #[serde(default)]
    pub models: Vec<String>,
    /// How hard the agent may be asked to think. Empty when the CLI has no
    /// such control, and the composer then draws no chip at all.
    #[serde(default)]
    pub efforts: Vec<String>,
    #[serde(default)]
    pub effort_default: Option<String>,
}

impl Profile {
    pub fn installed(&self) -> bool {
        self.path.is_some()
    }
}
