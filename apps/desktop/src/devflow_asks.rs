//! Devflow changes only the person makes: switching it on in a project, and
//! moving its Decisions to enforce.
//!
//! An agent's request becomes a card under the top bar, in the pattern of
//! `devpit_propose_project`. The click writes devflow.yaml and keeps that the
//! person made it, so a file that says `enabled: true` on its own — a hand
//! edit, a fresh clone — still waits for a click on this machine.
//!
//! Asks are held in memory: an ask is for now. The click is kept.

use std::sync::{Mutex, OnceLock};

use devpit_core::{preference, Store};
use devpit_devflow::preset::Preset;
use devpit_devflow::{folder, proposal, setup, Ask};
use devpit_rpc::{DevflowAsk, DevflowAsks, DevflowChange, ErrorCode, RpcError};
use tauri::{AppHandle, Emitter};

/// The most asks kept waiting; the oldest goes first.
const MOST: usize = 16;

/// The window reads the asks again.
pub(crate) const ASKED: &str = "devflow:asked";
/// A project's Devflow changed: its payload is the project's id.
pub(crate) const CHANGED: &str = "devflow:changed";

#[derive(Default)]
struct Held {
    waiting: Vec<DevflowAsk>,
    /// Dropped by the person, so not raised again by itself until a restart.
    dropped: Vec<(String, DevflowChange)>,
}

fn held() -> &'static Mutex<Held> {
    static HELD: OnceLock<Mutex<Held>> = OnceLock::new();
    HELD.get_or_init(Mutex::default)
}

pub(crate) fn change_of(ask: Ask) -> DevflowChange {
    match ask {
        Ask::Enable => DevflowChange::Enable,
        Ask::Enforce => DevflowChange::Enforce,
    }
}

fn ask_of(change: DevflowChange) -> Ask {
    match change {
        DevflowChange::Enable => Ask::Enable,
        DevflowChange::Enforce => Ask::Enforce,
    }
}

/// Asks the person for `ask` in the project. The same change asked again is
/// the one already waiting. `by_itself` is devpit noticing the file, not an
/// agent asking, and stays quiet once the person dropped it.
pub(crate) fn raise(
    app: Option<&AppHandle>,
    project_id: &str,
    project: &str,
    ask: Ask,
    by_itself: bool,
) {
    let change = change_of(ask);
    {
        let Ok(mut held) = held().lock() else {
            return;
        };
        let key = (project_id.to_owned(), change);
        if held
            .waiting
            .iter()
            .any(|one| one.project_id == project_id && one.change == change)
            || (by_itself && held.dropped.contains(&key))
        {
            return;
        }
        held.dropped.retain(|one| *one != key);
        held.waiting.push(DevflowAsk {
            id: format!("dfa_{}", ulid::Ulid::generate()),
            project_id: project_id.to_owned(),
            project: project.to_owned(),
            change,
            asked_at: now(),
        });
        if held.waiting.len() > MOST {
            held.waiting.remove(0);
        }
    }
    if let Some(app) = app {
        let _ = app.emit(ASKED, ());
    }
}

/// Whether `ask` waits for the person in the project.
pub(crate) fn waiting_in(project_id: &str) -> Vec<Ask> {
    held()
        .lock()
        .map(|held| {
            held.waiting
                .iter()
                .filter(|one| one.project_id == project_id)
                .map(|one| ask_of(one.change))
                .collect()
        })
        .unwrap_or_default()
}

fn key_of(ask: Ask) -> &'static str {
    match ask {
        Ask::Enable => preference::DEVFLOW_ON,
        Ask::Enforce => preference::DEVFLOW_ENFORCED,
    }
}

fn projects_with(store: &Store, ask: Ask) -> Vec<String> {
    store
        .preference(key_of(ask))
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Whether the person made `ask` in the project, on this machine.
pub(crate) fn made(store: &Store, project_id: &str, ask: Ask) -> bool {
    projects_with(store, ask).iter().any(|id| id == project_id)
}

/// Keeps, or forgets, that the person made `ask` in the project.
pub(crate) fn remember(
    store: &Store,
    project_id: &str,
    ask: Ask,
    yes: bool,
) -> Result<(), RpcError> {
    // One list for every project, read and written whole: two at once would
    // each write back a list without the other's change.
    static REMEMBERING: Mutex<()> = Mutex::new(());
    let _one_at_a_time = REMEMBERING.lock();
    let mut ids = projects_with(store, ask);
    ids.retain(|id| id != project_id);
    if yes {
        ids.push(project_id.to_owned());
    }
    store.set_preference(
        key_of(ask),
        &serde_json::to_string(&ids).unwrap_or_default(),
    )?;
    Ok(())
}

/// Whether Devflow is offered to any project. On unless turned off.
pub(crate) fn globally_on(store: &Store) -> bool {
    store
        .preference_flag(preference::DEVFLOW)
        .ok()
        .flatten()
        .unwrap_or(true)
}

/// The setup a project without Devflow starts from, as Settings chose it.
pub(crate) fn preset(store: &Store) -> Preset {
    store
        .preference(preference::DEVFLOW_PRESET)
        .ok()
        .flatten()
        .and_then(|word| Preset::of(&word))
        .unwrap_or(Preset::Recommended)
}

/// Where an MCP server looks to learn that what it offers may have changed:
/// the click is not a change to any file it watches.
pub(crate) fn rang(app: Option<&AppHandle>, project_id: &str) {
    if let Ok(root) = Store::root() {
        let _ = std::fs::write(
            root.join(devpit_agentapi::devflow::STAMP),
            format!("{}\n", now()),
        );
    }
    if let Some(app) = app {
        let _ = app.emit(CHANGED, project_id);
    }
}

fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs_f64())
        .unwrap_or_default()
}

pub(crate) fn listed() -> DevflowAsks {
    DevflowAsks {
        asks: held()
            .lock()
            .map(|held| held.waiting.clone())
            .unwrap_or_default(),
    }
}

/// `devflow.asks` — the Devflow changes waiting for the person's click.
#[tauri::command]
#[specta::specta]
pub async fn devflow_asks() -> Result<DevflowAsks, RpcError> {
    Ok(listed())
}

/// `devflow.answer` — the person's click: yes makes the change, no drops it.
#[tauri::command]
#[specta::specta]
pub async fn devflow_answer(
    app: AppHandle,
    id: String,
    yes: bool,
) -> Result<DevflowAsks, RpcError> {
    crate::off_main::blocking(move || {
        let lost = || RpcError::new(ErrorCode::NotFound, "that ask is gone");
        let one = listed()
            .asks
            .into_iter()
            .find(|one| one.id == id)
            .ok_or_else(lost)?;
        // A click that could not be made keeps its card, with the reason.
        if yes {
            made_now(&one.project_id, ask_of(one.change))?;
        }
        {
            let mut held = held().lock().map_err(|_| lost())?;
            held.waiting.retain(|waiting| waiting.id != id);
            if !yes {
                held.dropped.push((one.project_id.clone(), one.change));
            }
        }
        let _ = app.emit(ASKED, ());
        if yes {
            rang(Some(&app), &one.project_id);
        }
        Ok(listed())
    })
    .await
}

/// Writes what the person clicked into the project's devflow.yaml, and keeps
/// that they did.
fn made_now(project_id: &str, ask: Ask) -> Result<(), RpcError> {
    let store = crate::projects::store()?;
    let (_, root) = crate::projects::locate(&store, project_id)?;
    let switched = proposal::switched(&folder::read(&root), ask)
        .map_err(|why| RpcError::new(ErrorCode::Invalid, why.to_string()))?;
    setup::write(&root, &switched.files).map_err(|why| {
        RpcError::new(
            ErrorCode::Invalid,
            format!("devflow.yaml was not written: {why}"),
        )
    })?;
    remember(&store, project_id, ask, true)
}

#[cfg(test)]
#[path = "devflow_asks_tests.rs"]
mod tests;
