//! What the chat stream carries.
//!
//! Apart from the conversation because it is a different lifetime: a
//! conversation is what was said, a frame is one moment of it arriving.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::chat::{CallState, Message, Part, TurnEnd};

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
    /// The turn is over. Absence of frames is not an ending.
    Ended { end: TurnEnd },
}
