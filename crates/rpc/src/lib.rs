//! The contract between the core and the interface.
//!
//! These structs are the source; TypeScript is generated from them as a build
//! step. A type hand-written on both sides is a type that will diverge.
//!
//! This crate does not know about Tauri (checked by `cargo xtask check`). It
//! declares the shape; `apps/desktop` adapts that shape to a transport, and
//! tomorrow an HTTP server and a CLI do the same over the same types.

pub mod account;
pub mod agents;
pub mod board;
pub mod card;
pub mod chat;
pub mod checkpoint;
pub mod conversation;
pub mod error;
pub mod file;
pub mod focus;
mod frame;
pub mod front;
pub mod plugin_data;
pub mod plugins;
pub mod profile;
pub mod project;
pub mod review;
pub mod runs;
pub mod search;
pub mod session;
pub mod session_tree;
pub mod settings;
pub mod spend;
mod threads;
pub mod tile;
pub mod update;
pub mod usage;

pub use account::{Account, Membership, SignIn, SignInState};
pub use agents::{Agent, Agents, RejectedAgent};
pub use board::{Board, CardChanged, Column, ColumnDeleted, Run, RunState, Step, StepKind};
pub use card::{
    ArchivedCard, ArchivedCards, CardDeleted, CardDetail, CardTerminal, Checkout, Comment,
    DeleteRefusal, Notice, Notices, Pinned, Played,
};
pub use chat::{CallState, ChangedFile, Message, Part, Role, SessionInit, TurnEnd};
pub use checkpoint::{
    validity, verdict, Checked, Fingerprint, Report, Validity, Verdict, WhatRan, Whose, WouldRun,
};
pub use conversation::{Ask, Attachment, Conversation, OutsideSession};
pub use error::{ErrorCode, RpcError};
pub use file::{FileContents, FileKind, FileSaved};
pub use focus::{HeadsDown, Waiting};
pub use frame::Frame;
pub use front::Front;
pub use plugin_data::{
    PluginFile, PluginFileRemoved, PluginFileSaved, PluginFileText, PluginFiles,
};
pub use plugins::{
    catalogue, validate, validate_catalogue, DataSpec, Permission, PluginError, PluginList,
    PluginManifest, PluginState, PluginUninstalled, Surface,
};
pub use profile::{Declared, EnvVar, Profile, Reach};
pub use project::{
    Change, Commit, FileNode, GitStatus, Project, ProjectChanges, ProjectHistory, ProjectList,
    ProjectTree, Worktree, WorktreeOrigin,
};
pub use review::{
    blocking, reviewed, standing, Finding, Found, Review, Severity, Standing, REVIEW_EVIDENCE,
};
pub use runs::{ProjectRun, RunCursor, RunsPage, RunsQuery};
pub use search::{SearchFile, SearchHits, SearchLine};
pub use session::{
    AgentPresence, KnownAgent, LayoutNode, PaneKind, PaneRunning, PaneSize, SessionLayout,
    SplitDirection,
};
pub use settings::{Settings, Theme};
pub use spend::{
    PlanLimits, PlanWindow, SpendCard, SpendDay, SpendHistory, SpendInstallation, SpendRow,
    SpendSession, SpendShare, TokenCounts,
};
pub use threads::{Conversations, Thread};
pub use tile::{Card, CardHappening, CardSession, Doing, SessionKind};
pub use update::{InstallKind, UpdateBlocking, UpdateStatus, UpdateWork};
pub use usage::{PaneCost, Usage};

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
    /// Whether this is a devpit being worked on rather than the installed one.
    /// A debug build keeps a home of its own, and the window says so on its
    /// face: two devpits open side by side look identical otherwise, and the
    /// one you are testing in is not the one holding your real work.
    pub dev: bool,
}
