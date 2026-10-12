//! Channels off the machine — Telegram, e-mail, push — and which events reach
//! which, when.

use serde::{Deserialize, Serialize};
use specta::Type;

/// What can be told on a channel.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Type,
)]
#[serde(rename_all = "snake_case")]
pub enum ChannelEvent {
    /// A session stopped on a question or a permission.
    SessionWaiting,
    /// A session finished a turn it was working on.
    SessionDone,
    /// A session stopped on an error.
    SessionFailed,
    /// The orchestrator drafted words for a session.
    DraftReady,
    /// A card's date went off.
    Reminder,
    /// A lane's step failed.
    StepFailed,
    /// devpit's own MCP stopped answering and was restarted.
    McpRestarted,
}

/// Which channels one event goes to, by channel id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChannelRoute {
    pub event: ChannelEvent,
    pub channels: Vec<String>,
    /// Minutes a notice waits for the person on one channel before it moves
    /// to the next — push, then Telegram, then e-mail. None sends it to
    /// every channel at once.
    #[serde(default)]
    pub escalate_after: Option<u32>,
}

/// Hours nothing is sent, but a reminder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QuietHours {
    /// Minutes since midnight, local time. `from` after `to` runs overnight.
    pub from: u32,
    pub to: u32,
    /// Days it holds, 0 = Sunday. Empty is every day.
    pub days: Vec<u32>,
    /// The person's offset from UTC in minutes, as the window last saw it:
    /// the process has no time zone database, the window does.
    pub offset_minutes: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChannelRules {
    pub routes: Vec<ChannelRoute>,
    pub quiet: Option<QuietHours>,
    /// Seconds events are gathered into one message per channel.
    pub group_seconds: u32,
}

/// A channel this machine can send on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChannelInfo {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Channels {
    pub connected: Vec<ChannelInfo>,
    pub rules: ChannelRules,
    /// The events devpit can tell were dealt with, so a notice about them can
    /// move on to the next channel while nobody does.
    pub escalates: Vec<ChannelEvent>,
}

/// How much of what devpit says leaves on a channel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Privacy {
    /// Only that something happened or was answered, nothing named.
    #[default]
    Minimal,
    /// The session, card or project named, and an answer's first line.
    Titles,
    /// Names, and the orchestrator's answer whole.
    Full,
}

/// An orchestrator a channel can talk to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChannelOrchestrator {
    pub project: String,
    pub name: String,
}

/// The person's Telegram bot, as the screen shows it: never its token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TelegramStatus {
    /// A chat is linked and gets the notices.
    pub linked: bool,
    pub bot: Option<String>,
    /// The code to send the bot from the chat to link, while one is waiting.
    pub code: Option<String>,
    /// `t.me/<bot>?start=<code>`, which sends it in one tap.
    pub link: Option<String>,
    pub privacy: Privacy,
    /// The orchestrator the chat's messages go to, when one was chosen.
    pub orchestrator: Option<String>,
    /// The orchestrators on this machine, to choose from.
    pub orchestrators: Vec<ChannelOrchestrator>,
}

/// devpit.app as a channel: e-mail and push, set up at devpit.app/account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HubStatus {
    pub on: bool,
    pub titles: bool,
    /// Signed in to devpit.app on this machine, which the hub needs.
    pub signed_in: bool,
}

/// Card dates and reminders as a calendar on devpit.app, for a calendar app
/// to subscribe to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CalendarFeed {
    /// The address with its secret, while there is one.
    pub link: Option<String>,
    /// Whether events name the card and project, or only that something is due.
    pub titles: bool,
    /// Signed in to devpit.app on this machine, which the feed needs.
    pub signed_in: bool,
}
