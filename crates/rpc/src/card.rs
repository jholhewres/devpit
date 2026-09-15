//! What one card carries when you open it.
//!
//! Apart from `board.rs` because that file is the shape of the *board* — what
//! a lane holds and where a card sits in it — and none of this belongs on a
//! tile. A conversation and a list of files are what you look at after you
//! have decided which card you care about.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::board::{Card, Run};

/// One line of the card's conversation.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: String,
    /// `you`, or an agent id — never a display name. Names change, and the
    /// screen resolves the id to whatever it calls that agent today.
    pub author: String,
    pub body: String,
    /// Seconds since the epoch; `f64` because this crosses into a JavaScript
    /// number. See `Commit::committed_at` for the reason stated once.
    pub created_at: f64,
    /// Present when the line was changed after it was said.
    pub edited_at: Option<f64>,
}

/// A file pinned to a card.
///
/// `Pinned` and not `Attachment` because `chat::Attachment` already means
/// something else — a file handed to an agent for one turn. This one outlives
/// the turn and belongs to the card.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Pinned {
    pub id: String,
    /// Absolute, and local only.
    pub path: String,
    pub label: String,
    /// False when the file has since been moved or deleted. Drawn as missing
    /// rather than dropped: a pin that vanishes takes the memory of it too.
    pub exists: bool,
    /// Bytes, when it could be read.
    pub bytes: Option<f64>,
    pub created_at: f64,
    /// The plugin that pinned it. The screen opens the pin there while that
    /// plugin is on, and as a plain file otherwise.
    pub plugin: Option<String>,
}

/// The checkout this card's work happens in.
///
/// `Checkout` and not `CardWorktree`, which the settings pane already uses for
/// its own row — that one is about disk and orphans, this one about the branch
/// and what is uncommitted in it. Two views of one fact, and the names have to
/// say which is which.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Checkout {
    pub path: String,
    pub branch: Option<String>,
    /// What it started from. The diff is against this, never against HEAD.
    pub base_ref: Option<String>,
    /// Null when git could not be read — not 0, which would claim it is clean.
    pub dirty_files: Option<u32>,
    /// False when the row names a folder that is no longer there.
    pub exists: bool,
}

/// Everything one card is, for the screen that opens it.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CardDetail {
    pub card: Card,
    pub column_name: String,
    /// What the lane this card is in runs, if anything. Here rather than
    /// looked up by the screen: the card pane needs it to say what play does,
    /// and a second read would be a second answer.
    pub column_step: Option<crate::board::Step>,
    pub comments: Vec<Comment>,
    pub pinned: Vec<Pinned>,
    /// Absent until the card has one.
    pub worktree: Option<Checkout>,
    /// Every run, newest first — the tile only carries the latest.
    pub runs: Vec<Run>,
}

/// Something worth telling somebody about.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub id: String,
    pub project_id: Option<String>,
    /// `run`, `agent`, `due`, `irreversible`. A word rather than an enum
    /// because the set grows with whatever learns to notice something, and a
    /// kind this build does not know should draw as a plain row, not fail.
    pub kind: String,
    pub title: String,
    pub detail: Option<String>,
    /// Where clicking it goes. Null for a notice with nowhere to go.
    pub card_id: Option<String>,
    pub created_at: f64,
    pub read_at: Option<f64>,
}

/// Response of every notice command.
///
/// The count comes with the list so the bell and the list can never disagree —
/// two reads a second apart is exactly how a badge ends up saying 3 over an
/// empty panel.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Notices {
    pub notices: Vec<Notice>,
    pub unread: u32,
}

/// What pressing play did, or why it did not.
///
/// Three answers rather than an error for two of them: a lane that runs
/// nothing and a step that wants confirming are both ordinary, and the screen
/// says something different for each.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Played {
    /// The run that started, when one did.
    pub run: Option<crate::board::Run>,
    /// The lane's step has no undo and nobody has said to run it.
    pub needs_confirming: bool,
    /// This lane runs nothing, so there was nothing to play.
    pub lane_runs_nothing: bool,
}

/// Response of `card.delete`.
///
/// A refusal is an answer rather than an error: the screen has to know whether
/// asking again with `force` can change it.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CardDeleted {
    pub deleted: bool,
    pub refused: Option<DeleteRefusal>,
}

/// Why a card was not deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRefusal {
    pub reason: String,
    /// True only for unsaved work, which is the person's to throw away.
    pub forcible: bool,
}
