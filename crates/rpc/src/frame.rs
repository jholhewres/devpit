//! What the chat stream carries.
//!
//! Apart from the conversation because it is a different lifetime: a
//! conversation is what was said, a frame is one moment of it arriving.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::chat::{CallState, Message, Part, TurnEnd};

/// How full the model's context was at the end of a turn, in tokens: what
/// the last request carried, against the window it had. What a person needs
/// to see before the CLI compacts — or refuses — on its own.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    pub used: u32,
    pub window: u32,
}

/// What the stream carries, one frame at a time.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Frame {
    /// A message opened; parts follow.
    Opened { message: Message },
    /// More of the message that is open.
    Part { message_id: String, part: Part },
    /// A tool call moved on.
    CallState { call_id: String, state: CallState },
    /// The CLI named its session. A new conversation learns it here, while
    /// the turn runs, rather than from the head written after it.
    Session { session_id: String },
    /// The turn is over. Absence of frames is not an ending.
    Ended { end: TurnEnd },
}
