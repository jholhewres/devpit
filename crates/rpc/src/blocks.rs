//! A terminal's commands, as blocks.

use serde::{Deserialize, Serialize};
use specta::Type;

/// One command a pane ran, or is running.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CommandBlock {
    /// Rises with every block the pane has had while the app ran. `f64`
    /// because it crosses into a JavaScript number.
    pub id: f64,
    pub command: Option<String>,
    /// Where it ran, when the shell said.
    pub cwd: Option<String>,
    /// Unix milliseconds.
    pub started_at: f64,
    pub ended_at: Option<f64>,
    /// Absent while running, and for a command that ended without saying.
    pub code: Option<i32>,
    /// It took the whole screen, so its output is drawing, not lines.
    pub interactive: bool,
    /// Its output was too long, and the start of it was dropped.
    pub truncated: bool,
}

/// A block that started or ended, in the pane it belongs to.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BlockChanged {
    pub pane_id: String,
    pub block: CommandBlock,
}

/// A pane's blocks, and where its shell stands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PaneBlocks {
    /// Oldest first; the one still running, if any, last.
    pub blocks: Vec<CommandBlock>,
    /// The shell has been heard marking its prompts since the app started, so
    /// its commands can be drawn as blocks.
    pub integrated: bool,
    /// It is at its prompt, waiting for a line.
    pub at_prompt: bool,
    /// The folder it last said it was in.
    pub cwd: Option<String>,
}

/// A folder's git state, for a terminal's prompt chips.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderGlance {
    pub branch: String,
    pub files: u32,
    pub added: u32,
    pub removed: u32,
}
