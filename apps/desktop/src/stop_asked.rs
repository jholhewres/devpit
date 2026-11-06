//! A session stopped at the orchestrator's call: on the person's own word,
//! read in the orchestrator's log as `draft_sending` reads it, or when
//! another session already works the same card and relieves it.

use devpit_agentcli::person::{From, Said};
use devpit_rpc::LiveSession;

/// Whether stopping `name` goes ahead.
pub(crate) fn allowed(said: Option<&Said>, name: &str, swapped: bool) -> Result<(), String> {
    if swapped {
        return Ok(());
    }
    match said {
        Some(said) if matches!(said.from, From::Channel(_)) => Err(format!(
            "not stopped: the person last wrote from {}, where a stop goes only by its button",
            said.from.label()
        )),
        Some(said) if devpit_agentcli::person::asks_to_stop(&said.text, name) => Ok(()),
        Some(said) => Err(format!(
            "not stopped: the person's last message («{}») is not a request to stop {name}. Ask them to send only `stop {name}` or `✅ {name}`, with the full name.",
            crate::draft_sending::short(&said.text)
        )),
        None => Err(format!(
            "not stopped: devpit found nothing the person said in this conversation asking for {name} to stop"
        )),
    }
}

/// Whether every session stopped as `name` (only `pid`, when given) has
/// another running session on its card: a swap on the card.
pub(crate) fn swapped(live: &[LiveSession], name: &str, pid: Option<i32>) -> bool {
    let stopped = |one: &LiveSession| one.name == name && pid.is_none_or(|pid| one.pid == pid);
    let mut stopping = live.iter().filter(|one| stopped(one)).peekable();
    stopping.peek().is_some()
        && stopping.all(|one| {
            one.card_id.as_deref().is_some_and(|card| {
                live.iter()
                    .any(|other| !stopped(other) && other.card_id.as_deref() == Some(card))
            })
        })
}

#[cfg(test)]
#[path = "stop_asked_tests.rs"]
mod tests;
