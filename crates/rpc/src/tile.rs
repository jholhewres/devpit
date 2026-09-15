//! A card as the board draws it: the tile.
//!
//! Apart from `board.rs`, which had reached its ceiling holding the whole
//! vocabulary, and because a tile is about to say what its sessions are doing.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::board::Run;
use crate::session_status::Session;

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
    /// The session working on this card, if one is.
    pub session: Option<Session>,
}
