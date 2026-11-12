//! A session stopped at the orchestrator's call: on the person's own word,
//! read in the orchestrator's log as `draft_sending` reads it, when another
//! session already works the same card and relieves it, or on the person's
//! click. A stop their words did not cover waits above the orchestrator's
//! composer with every other one it asked for, so one click stops them all.
//!
//! Held in memory, per orchestrator account: a stop asked is for now.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use devpit_agentcli::person::{From, Said};
use devpit_rpc::{LiveSession, RpcError, StopAsked, StopsAsked, StopsConfirmed};
use serde_json::{json, Value};
use tauri::AppHandle;

/// Said to the window when an orchestrator asks for a stop.
pub(crate) const STOP_ASKED: &str = "orchestrator:stop-asked";

/// Whether stopping `name` goes ahead now: on the person's word — the whole
/// message the request, beside other names of `running` to stop several — or
/// a swap, and what it would lose only with `force`. Otherwise why: it waits
/// for the person's click.
pub(crate) fn allowed(
    said: Option<&Said>,
    name: &str,
    running: &[&str],
    swapped: bool,
    losing: Option<&str>,
    force: bool,
) -> Result<(), String> {
    let refused = match said {
        _ if swapped => None,
        Some(said) if matches!(said.from, From::Channel(_)) => Some(format!(
            "the person last wrote from {}, where a stop goes only by its button",
            said.from.label()
        )),
        Some(said) if devpit_agentcli::person::asks_to_stop(&said.text, name, running) => None,
        Some(said) => Some(format!(
            "the person's last message («{}») is not only a request to stop {name}",
            crate::draft_sending::short(&said.text)
        )),
        None => Some(format!(
            "devpit found nothing the person said asking for {name} to stop"
        )),
    };
    if let Some(why) = refused {
        return Err(why);
    }
    match losing {
        Some(losing) if !force => Err(format!("{name}: {losing}")),
        _ => Ok(()),
    }
}

type Asked = HashMap<String, Vec<StopAsked>>;

fn asked() -> &'static Mutex<Asked> {
    static ASKED: OnceLock<Mutex<Asked>> = OnceLock::new();
    ASKED.get_or_init(Mutex::default)
}

/// Keeps the stop of `name` (only `pid`) waiting for the click of `profile`'s
/// person, in place of one asked for it before; answers the orchestrator
/// `why` nothing stopped yet.
pub(crate) fn ask(
    profile: &str,
    name: &str,
    pid: Option<i32>,
    losing: Option<String>,
    why: &str,
) -> Value {
    if let Ok(mut held) = asked().lock() {
        let list = held.entry(profile.to_owned()).or_default();
        list.retain(|one| one.name != name);
        list.push(StopAsked {
            name: name.to_owned(),
            pid,
            losing,
        });
    }
    json!({
        "done": false,
        "asked": format!("stop {name}"),
        "note": format!("Not stopped yet: {why}. It waits above this chat's composer, listed with every other stop you asked for and what each would lose, and one click of the person's stops them all. Tell them so; do not ask them to type it again."),
    })
}

/// The stops waiting for `profile`'s person, oldest first.
pub(crate) fn waiting(profile: &str) -> Vec<StopAsked> {
    asked()
        .lock()
        .ok()
        .and_then(|held| held.get(profile).cloned())
        .unwrap_or_default()
}

/// The stops among `names` that wait, taken: what the person's click
/// confirms. A name the orchestrator did not ask for is not among them.
pub(crate) fn take(profile: &str, names: &[String]) -> Vec<StopAsked> {
    let Ok(mut held) = asked().lock() else {
        return Vec::new();
    };
    let Some(list) = held.get_mut(profile) else {
        return Vec::new();
    };
    let (taken, kept) = std::mem::take(list)
        .into_iter()
        .partition(|one| names.contains(&one.name));
    *list = kept;
    taken
}

/// A session stopped some other way waits no more.
pub(crate) fn forget(profile: &str, name: &str) {
    if let Ok(mut held) = asked().lock() {
        if let Some(list) = held.get_mut(profile) {
            list.retain(|one| one.name != name);
        }
    }
}

/// `orchestrator.stops_asked` — the stops waiting for the person's click.
#[tauri::command]
#[specta::specta]
pub fn orchestrator_stops_asked(profile_id: String) -> StopsAsked {
    StopsAsked {
        stops: waiting(&profile_id),
    }
}

/// `orchestrator.stops_confirm` — the person's click: each of `names` still
/// waiting stops as theirs, with what it loses, which the click was shown.
#[tauri::command]
#[specta::specta]
pub async fn orchestrator_stops_confirm(
    app: AppHandle,
    profile_id: String,
    names: Vec<String>,
) -> Result<StopsConfirmed, RpcError> {
    crate::off_main::blocking(move || {
        let mut done = StopsConfirmed {
            stopped: Vec::new(),
            failed: Vec::new(),
        };
        for one in take(&profile_id, &names) {
            match crate::stopping::stop(&app, &profile_id, &one.name, one.pid, "person", true) {
                Ok(_) => done.stopped.push(one.name),
                Err(err) => done.failed.push(format!("{}: {}", one.name, err.message)),
            }
        }
        Ok(done)
    })
    .await
}

/// `orchestrator.stops_drop` — the person keeps them running.
#[tauri::command]
#[specta::specta]
pub fn orchestrator_stops_drop(profile_id: String) {
    if let Ok(mut held) = asked().lock() {
        held.remove(&profile_id);
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
