//! An orchestrator's Schedules: the reminders it reaches and the routines the
//! person set, in one list, and Assistant mode.

use serde::{Deserialize, Serialize};
use specta::Type;

/// What a row is: a card's date, or a routine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleKind {
    Reminder,
    Routine,
}

/// Where a row stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleState {
    /// Still to go off.
    Waiting,
    /// Went off, and waits on the person.
    Due,
    /// Dealt with.
    Done,
    /// A routine the person paused.
    Paused,
}

/// What a routine tells when it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum RoutineKind {
    /// The comeback briefing: sessions, merged pull requests, decisions waiting.
    Morning,
    /// Open pull requests whose checks fail.
    Ci,
    /// Open pull requests.
    Prs,
    /// Cards waiting in a lane whose role is check.
    Review,
}

/// A routine as the person set it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Routine {
    pub kind: RoutineKind,
    /// Minutes since midnight, local time.
    pub minute: u32,
    /// Days it runs, 0 = Sunday. Empty is every day.
    pub days: Vec<u32>,
    /// The zone the time is in, as the window named it.
    pub zone: String,
    /// That zone's offset from UTC in minutes, as the window last saw it:
    /// the process has no time zone database, the window does.
    pub offset_minutes: i32,
}

/// One row of Schedules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
    /// The routine's id, or the reminder's card id.
    pub id: String,
    pub kind: ScheduleKind,
    pub title: String,
    /// When it goes off next, or went off; seconds since the epoch.
    pub at: Option<f64>,
    pub project_id: Option<String>,
    pub project: Option<String>,
    /// A reminder's card, `#42`.
    pub card_number: Option<u32>,
    /// A routine's settings; none for a reminder, which goes off once.
    pub routine: Option<Routine>,
    pub state: ScheduleState,
    /// When a routine last ran.
    pub last_run: Option<f64>,
}

/// A reminder the orchestrator proposed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProposedReminder {
    pub title: String,
    pub at: f64,
    pub project_id: String,
    pub project: Option<String>,
}

/// A schedule the orchestrator proposed: nothing is scheduled until the
/// person adds it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleProposal {
    pub id: String,
    pub routine: Option<Routine>,
    pub reminder: Option<ProposedReminder>,
    /// Why, in the orchestrator's words.
    pub why: String,
}

/// A list, so tomorrow's field has somewhere to go.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Schedules {
    pub schedules: Vec<Schedule>,
    pub proposed: Vec<ScheduleProposal>,
}

/// Assistant mode for one orchestrator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AssistantMode {
    pub on: bool,
}

/// How an orchestrator's heartbeat looks while its Assistant mode is on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Heartbeat {
    /// Minutes between two looks.
    pub minutes: u32,
    /// A session stopped on a question longer than this needs the person.
    pub waiting_minutes: u32,
    /// CI failing on an open pull request needs the person.
    pub ci: bool,
    /// A session stopped on a question needs the person.
    pub waiting: bool,
    /// A card in progress waiting on another needs the person.
    pub blocked: bool,
}

/// What happened while the person was away, in a few lines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Briefing {
    /// Seconds since the epoch.
    pub since: f64,
    pub lines: Vec<String>,
}
