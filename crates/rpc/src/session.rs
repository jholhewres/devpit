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

/// What is running in one pane, for `session.running`.
///
/// The foreground process, asked of the operating system rather than reported
/// by the program itself: an agent CLI opened in a terminal has no reason to
/// tell this app it exists, and the sidebar still has to know.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PaneRunning {
    pub pane_id: String,
    /// The command's own name: `zsh`, `claude`, `codex`, `cargo`.
    ///
    /// Read from the arguments, not from the executable: every agent CLI
    /// written in JavaScript runs as `node`, and a row saying `node` names
    /// nothing anyone recognises.
    pub command: String,
    /// False while the shell itself is in front, which is nothing running.
    pub busy: bool,
    /// Which agent this is, when it is one. `null` for a shell, a build, an
    /// editor — anything the app has no particular name for.
    pub agent: Option<String>,
    /// What to call it on screen: `Claude Code` for an agent, and the
    /// command's own name for everything else.
    pub label: String,
}

/// One agent CLI this build can start, for `agents.known`.
///
/// The same list that recognises a running one. They cannot be two lists: a
/// menu that starts Gemini and a sidebar that then calls the pane `node` is
/// the shape of the bug this replaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct KnownAgent {
    /// Stable, and what the screen keys an icon by.
    pub id: String,
    pub label: String,
    /// What gets typed into the terminal to start it.
    pub launch: String,
    /// Whether it is on this machine's PATH. A menu still lists the others —
    /// saying what could be installed is more use than a short list with no
    /// explanation.
    pub installed: bool,
}
