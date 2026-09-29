//! What a project's fresh worktree needs before it works: the person's own
//! local setup, kept in the devpit workspace and never committed.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::EnvVar;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeSetup {
    /// Files copied from the main checkout, `*` within a name: `.env*`.
    pub copy: Vec<String>,
    /// Files linked back to the main checkout rather than copied.
    pub link: Vec<String>,
    /// Commands run once in the new worktree, in order.
    pub run: Vec<String>,
    /// What every setup command is given, as variables.
    pub share: Vec<EnvVar>,
}
