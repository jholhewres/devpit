//! A conversation as a whole, and what the composer sends to add to one.
//!
//! Apart from `chat.rs` because that file is what a message is made of, and
//! this is what holds messages and asks for more — two lifetimes, and the
//! first is the one every driver has to agree on.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::chat::Message;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub project_id: String,
    pub card_id: Option<String>,
    /// That card's title, so the chat can name it.
    pub card_title: Option<String>,
    /// Whether that card is still on a board, which is when the chat can open it.
    pub card_on_board: bool,
    /// The profile — account and driver — this conversation belongs to, for
    /// its whole life.
    ///
    /// Fixed on purpose: the transcript, the shape of a message and the way
    /// cost is counted all belong to one account. Changing it is starting
    /// another conversation, not continuing this one.
    pub profile: String,
    /// The model within that provider, which the composer may change.
    pub model: Option<String>,
    /// The permission mode and effort the last turn ran with, so reopening the
    /// conversation does not quietly fall back to the most careful mode.
    #[serde(default)]
    pub permission: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
    /// The folder its turns run in, which a relative link in an answer is read against.
    #[serde(default)]
    pub cwd: Option<String>,
    /// The CLI's own id for this thread, once it has run a turn. It is what
    /// ties a permission question back to the conversation that raised it.
    pub session_id: Option<String>,
    pub messages: Vec<Message>,
    pub cost_usd: f64,
    /// How full the context was when the last turn ended.
    #[serde(default)]
    pub context: Option<crate::frame::Context>,
    /// The turns a rewind can fork at: those whose place in the CLI's
    /// transcript was kept. Earlier turns ran before it was.
    #[serde(default)]
    pub rewindable: Vec<String>,
    pub created_at: f64,
}

/// What one turn needs to run. One object because the composer sends these
/// together, and because tomorrow's field needs somewhere to live.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Ask {
    pub project_id: String,
    pub conversation_id: String,
    /// Which profile — the account, and the binary it names.
    pub profile_id: String,
    pub model: Option<String>,
    pub prompt: String,
    pub cwd: String,
    /// A ceiling for the whole conversation, not for this turn.
    pub budget_usd: Option<f64>,
    /// What the agent may do without asking, in the CLI's own words.
    /// Absent keeps whatever the conversation already had.
    pub permission: Option<String>,
    /// How hard to think. Absent keeps what the conversation already had.
    pub effort: Option<String>,
}

/// A file the person put in front of the agent.
///
/// The path is relative to the project root, because that is the only form
/// the agent can use and the only form that survives another machine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub name: String,
    pub path: String,
    /// The extension, lowercased, or empty. What the chip draws.
    pub kind: String,
}

/// A session of this project the CLI holds and devpit never saw — one started
/// in a terminal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OutsideSession {
    pub session_id: String,
    /// The CLI's own title, when it wrote one.
    pub title: Option<String>,
    /// The configuration directory of the installation that holds it.
    pub installation: String,
    /// Unix seconds.
    pub last_at: f64,
}
