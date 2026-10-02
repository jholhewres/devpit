//! The island: a glance at every agent session, in a window of its own above
//! everything else on the screen.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::Doing;

/// What a tool is about to touch, read from its input, for the preview.
///
/// A closed set: a tool whose input is none of these is shown by name alone,
/// rather than guessed at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Touch {
    /// A replacement in a file: what was there, and what goes in its place.
    Edit {
        path: String,
        before: String,
        after: String,
    },
    /// A whole file written.
    Write { path: String, after: String },
    /// A file read, from a line, for so many lines, when the tool said.
    Read {
        path: String,
        offset: Option<u32>,
        limit: Option<u32>,
    },
    /// A shell command.
    Run { command: String },
    /// Questions put to the person, each with its choices: what the session
    /// waits on until they pick.
    Ask { questions: Vec<IslandAsked> },
    /// A plan put to the person to approve before the session goes on.
    Plan { plan: String },
}

/// One question a session asks the person, as its tool put it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IslandAsked {
    /// The short label the CLI shows above it, when it has one.
    pub header: Option<String>,
    pub question: String,
    pub options: Vec<String>,
    /// Several of the choices may be picked.
    pub multi: bool,
}

/// One step of a session's turn: a tool, and what it was run on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IslandStep {
    pub tool: String,
    /// `invoice.ts`, `npm test`: what the step reads as, after the tool.
    pub target: Option<String>,
    pub done: bool,
    /// It ran and the tool reported an error.
    pub failed: bool,
    pub touch: Option<Touch>,
}

/// An agent session, as the island draws it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IslandSession {
    pub session_id: String,
    /// The devpit terminal it runs in, when it runs in one.
    pub pane_id: Option<String>,
    pub project_id: Option<String>,
    pub project: Option<String>,
    /// The project's colour, as `#rrggbb`, when one was chosen.
    pub color: Option<String>,
    pub card_id: Option<String>,
    pub card: Option<String>,
    /// The folder it works in — its card's checkout, else its project — so
    /// a path can be shown from there rather than from `/`.
    pub root: Option<String>,
    pub state: Doing,
    /// The latest steps, oldest first.
    pub steps: Vec<IslandStep>,
    /// The last thing the agent said, cut short.
    pub said: Option<String>,
    /// Milliseconds since the epoch, when it last changed.
    pub at: f64,
}

/// Every session the island knows of. An object, so tomorrow's field has
/// somewhere to go.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IslandNow {
    pub sessions: Vec<IslandSession>,
}

/// One change, as `island:session` carries it: the session as it now is, or
/// its id when it has gone.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "was")]
pub enum IslandChange {
    Changed { session: Box<IslandSession> },
    Gone { session_id: String },
}

/// The visible island, in the window's logical pixels: the only part of the
/// window that takes the mouse.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IslandRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// A terminal session about to ask for permission, held for the island to
/// answer while nobody is at devpit's window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IslandQuestion {
    pub id: String,
    pub session_id: String,
    pub pane_id: String,
    /// The project it runs in, by name, when devpit knows it.
    pub project: Option<String>,
    pub tool: String,
    /// The tool's input, as the JSON the CLI sent.
    pub input: String,
    /// Whether the CLI offered a rule to keep, so "always" means something.
    pub keepable: bool,
}

/// What the person said to a held question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum IslandVerdict {
    Allow,
    /// Allow, and keep the rule the CLI suggested.
    Always,
    Deny,
    /// Let the terminal ask, the way it would have without devpit.
    InTerminal,
}

/// A session's branch as its code host sees it: its pull request, and how
/// the checks on it are doing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IslandChecks {
    pub branch: String,
    pub pull: Option<IslandPull>,
    /// `passing` | `failing` | `running`, or nothing when nothing has run.
    pub checks: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IslandPull {
    pub number: u32,
    /// `OPEN` | `MERGED` | `CLOSED`, as the host says it.
    pub state: String,
    pub title: String,
    pub url: String,
}
