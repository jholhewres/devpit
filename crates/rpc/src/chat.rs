//! A conversation with an agent, and what one message is made of.
//!
//! Parts rather than one string: the screen draws a tool call, a thought and
//! an answer differently, and a flat string cannot tell them apart.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Who said it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    System,
}

/// How a tool call ended, or that it has not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum CallState {
    Running,
    Ok,
    Failed,
}

/// One piece of a message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Part {
    Text {
        text: String,
    },
    /// Reasoning the model showed. Separate because it is not the answer.
    Thinking {
        text: String,
    },
    ToolCall {
        id: String,
        name: String,
        /// Verbatim. Parsing it here would be this crate guessing at a shape
        /// the provider is free to change.
        input: String,
        state: CallState,
    },
    ToolResult {
        /// The call this answers.
        call_id: String,
        output: String,
        is_error: bool,
    },
    /// A line the driver did not recognise. Kept rather than dropped: losing
    /// output is worse than showing it plain.
    Unknown {
        text: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub turn_id: Option<String>,
    pub role: Role,
    pub parts: Vec<Part>,
    /// Unix seconds.
    pub created_at: f64,
    /// True while more of it is still arriving.
    pub streaming: bool,
}

/// What a turn cost and why it stopped.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TurnEnd {
    pub turn_id: String,
    pub cost_usd: Option<f64>,
    pub duration_ms: Option<f64>,
    /// The CLI's own word, kept rather than flattened into "failed".
    pub stop_reason: Option<String>,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub project_id: String,
    pub card_id: Option<String>,
    /// The profile — account and driver — this conversation belongs to, for
    /// its whole life.
    ///
    /// Fixed on purpose: the transcript, the shape of a message and the way
    /// cost is counted all belong to one account. Changing it is starting
    /// another conversation, not continuing this one.
    pub profile: String,
    /// The model within that provider, which the composer may change.
    pub model: Option<String>,
    pub messages: Vec<Message>,
    pub cost_usd: f64,
    pub created_at: f64,
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
    /// The turn is over. Absence of frames is not an ending.
    Ended { end: TurnEnd },
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
    pub budget_usd: Option<f64>,
}
