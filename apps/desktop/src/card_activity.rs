//! What a card's sessions are doing, as the app hears it.
//!
//! Heard, never stored: an agent reports its state through its hooks, and a
//! row saying `working` would still say it after the app had closed and the
//! agent had finished.

use devpit_agentcli::Event;

/// What a session is doing, in the plan's words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Doing {
    /// It began and has said nothing since.
    Open,
    Working,
    /// Stopped on a person — the one worth coming back for.
    Waiting,
    Done,
    /// It ended.
    Gone,
}

/// What a hook says about the session it fired in; `None` when it says nothing.
///
/// `/clear` ends one session and starts the next in the same agent, so it is
/// not an end.
pub(crate) fn state_of_event(event: &Event) -> Option<Doing> {
    match event {
        Event::SessionStarted => Some(Doing::Open),
        Event::Using { .. }
        | Event::Used { .. }
        | Event::SubagentStarted { .. }
        | Event::Delegated { .. }
        | Event::SubagentDone { .. } => Some(Doing::Working),
        Event::Waiting => Some(Doing::Waiting),
        Event::Stopped { .. } => Some(Doing::Done),
        Event::SessionEnded { reason } if reason.as_deref() == Some("clear") => None,
        Event::SessionEnded { .. } => Some(Doing::Gone),
    }
}

/// The word a pane's own event carries.
///
/// Panes have only ever said these three; a session beginning or ending
/// reaches the window as its own event.
pub(crate) fn pane_word(doing: Option<Doing>) -> Option<&'static str> {
    match doing? {
        Doing::Working => Some("working"),
        Doing::Waiting => Some("waiting"),
        Doing::Done => Some("done"),
        Doing::Open | Doing::Gone => None,
    }
}

#[cfg(test)]
#[path = "card_activity_tests.rs"]
mod tests;
