//! A card as the board draws it: the tile.
//!
//! Apart from `board.rs`, which had reached its ceiling holding the whole
//! vocabulary, and because a tile is about to say what its sessions are doing.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::board::Run;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub id: String,
    pub column_id: String,
    pub title: String,
    pub body: String,
    pub position: i32,
    pub worktree_path: Option<String>,
    /// Seconds since the epoch; `f64` for the usual reason. Absent for most
    /// cards, which is why it is an option and not a date nobody chose.
    pub due_at: Option<f64>,
    /// What every run of this card has cost, added up.
    pub cost_usd: f64,
    /// Counted, not carried: the tile shows that there is a conversation, and
    /// the conversation itself is read when the card is opened.
    pub comments: u32,
    pub pinned: u32,
    /// Most recent first.
    pub runs: Vec<Run>,
    /// What the sessions working on this card add up to, the one most worth a
    /// look. `None` when nothing is working on it.
    pub activity: Option<Doing>,
}

/// What a session is doing, as its agent last said.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Doing {
    /// It began and has said nothing since.
    Open,
    Working,
    /// Stopped on a person — the one worth coming back for.
    Waiting,
    Done,
    /// It ended.
    Gone,
}

/// Where a session working on a card lives.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Type,
)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    Pane,
    Run,
    Background,
    Chat,
}

/// One session working on a card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CardSession {
    pub kind: SessionKind,
    /// A leaf for a pane, a session id for a run or a background session, a
    /// conversation for a chat.
    #[serde(rename = "ref")]
    pub reference: String,
    /// `None` for a pane or a run whose agent has said nothing since the app
    /// opened — it is listed, but nobody knows what it is doing.
    pub state: Option<Doing>,
    /// Where a pane is, so the card can go to it.
    pub tab_id: Option<String>,
    pub leaf_id: Option<String>,
}

/// What `card:happening` carries: a card's sessions, and what they add up to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CardHappening {
    pub card_id: String,
    /// The one the tile shows. `None` once nothing is left.
    pub activity: Option<Doing>,
    pub sessions: Vec<CardSession>,
}
