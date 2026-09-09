//! The contract between the core and the interface.
//!
//! These structs are the source; TypeScript is generated from them as a build
//! step. A type hand-written on both sides is a type that will diverge.
//!
//! This crate does not know about Tauri (checked by `cargo xtask check`). It
//! declares the shape; `apps/desktop` adapts that shape to a transport, and
//! tomorrow an HTTP server and a CLI do the same over the same types.

pub mod agents;
pub mod board;
pub mod chat;
pub mod error;
pub mod file;
pub mod front;
pub mod profile;
pub mod project;
pub mod session;
pub mod session_status;
pub mod session_tree;
pub mod settings;

pub use agents::{Agent, Agents, RejectedAgent};
pub use board::{Board, Card, CardChanged, Column, ColumnDeleted, Run, RunState, Step, StepKind};
pub use chat::{Ask, Attachment, CallState, Conversation, Frame, Message, Part, Role, TurnEnd};
pub use error::{ErrorCode, RpcError};
pub use file::{FileContents, FileKind, FileSaved};
pub use front::Front;
pub use profile::Profile;
pub use project::{
    Change, Commit, FileNode, GitStatus, Note, Project, ProjectChanges, ProjectHistory,
    ProjectList, ProjectNotes, ProjectTree, Worktree,
};
pub use session::{AgentPresence, LayoutNode, PaneKind, PaneSize, SessionLayout, SplitDirection};
pub use session_status::{Session, SessionStatus};
pub use settings::{Settings, Theme};

use serde::{Deserialize, Serialize};
use specta::Type;

/// Response of `app.info`.
///
/// An object, never a bare list — and that holds for every response in the
/// contract. Tomorrow's extra field must not break today's consumer, and a
/// bare list has nowhere to put it.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub platform: String,
    /// Where this build keeps state. Exposed on purpose: it is the first
    /// question of anyone taking a backup or filing a bug.
    pub state_path: String,
}

/// Response of `app.health`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppHealth {
    pub ok: bool,
    /// The binary on disk is no longer the one running.
    ///
    /// A backend still serving a replaced build makes every symptom read as
    /// "the fix did not work". In the contract from the start so the screen
    /// can say so instead of leaving it to be guessed.
    pub stale: bool,
    pub state_path: String,
}

/// What this build can do. Governs the UI.
///
/// Memory, cloud and vault are optional dependencies: without them the
/// matching panel disappears and the rest works. The screen asks here instead
/// of trying and handling failure — try-and-fail flashes a broken panel first.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub memory: bool,
    pub cloud: bool,
    pub vault: bool,
    pub tmux: bool,
}
