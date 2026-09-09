//! The session layout contract.
//!
//! The tree is the Orca answer: nested splits, one process per leaf. The axis
//! is the project, not the worktree. Leaves are terminals today; a later leaf
//! kind does not need a tmux window.
//!
//! The shapes live here and the operations on them live in [`session_tree`],
//! because the two are read for different reasons: this file answers "what
//! crosses to the screen", and that one answers "what a drag does to the tree".
//!
//! [`session_tree`]: crate::session_tree

use serde::{Deserialize, Serialize};
use specta::Type;

/// What a leaf shows. Only `terminal` in this slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PaneKind {
    Terminal,
}

/// Who is running in the leaf. `none` is a plain shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AgentPresence {
    None,
}

/// Orca's names: horizontal is left/right, vertical is top/bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum LayoutNode {
    Leaf {
        id: String,
        /// `session:window` on our private tmux server.
        #[serde(rename = "tmuxTarget")]
        tmux_target: String,
        kind: PaneKind,
        agent: AgentPresence,
        /// The name the person gave this pane, or empty when they have not.
        ///
        /// Empty rather than absent so the screen has one field to read: a
        /// pane titled by an OSC sequence is showing what the program called
        /// itself, and a pane titled here is showing what its owner called it.
        /// The second always wins, and only a rename clears or sets it.
        #[serde(default)]
        title: String,
    },
    Split {
        /// What a dragged boundary is addressed by.
        ///
        /// Defaulted rather than migrated: a tree persisted before splits had
        /// ids loads with this empty, and [`LayoutNode::name_the_splits`]
        /// fills it on the first read. A migration would have to rewrite every
        /// stored tree to add a field nothing had asked for yet.
        #[serde(default)]
        id: String,
        direction: SplitDirection,
        ratio: f64,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

/// Response of `session.layout` / `session.ensure` / `session.split`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SessionLayout {
    pub project_id: String,
    pub focused_id: String,
    pub tree: LayoutNode,
}

/// Response of `pane.resize`.
///
/// The size that ended up applied, which is not always the size asked for: a
/// pty clamps, and a pane that believes it has two hundred columns when it has
/// eighty draws wrongly a long way from the line that caused it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PaneSize {
    pub rows: u16,
    pub cols: u16,
}
