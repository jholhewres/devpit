//! A terminal session's permission question, answered from the island.
//!
//! Learned from Coucou, which answers Claude Code's `PermissionRequest` from
//! the notch: that hook fires only when the CLI is about to ask, and whatever
//! the hook does not decide, the CLI goes on to ask in the terminal. So the
//! question is held only while somebody can answer it somewhere else, and let
//! go — with no decision, never a denial — the moment that stops being true:
//!
//! - devpit's own window is in front: the person is looking at the terminal,
//!   and the terminal asks as it always has.
//! - the island never said it was showing the question (`island_seen`).
//! - nobody answered in time.
//!
//! The hold is shorter than the CLI's own hook budget, so it is always this
//! side that lets go.

use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use devpit_rpc::{IslandQuestion, IslandVerdict, RpcError};
use serde::Deserialize;
use tauri::{Emitter as _, Manager as _};

use crate::island::ISLAND;

/// How long the island has to say it is showing the question.
const SEEN_WITHIN: Duration = Duration::from_millis(1500);
/// How long a shown question waits for an answer: under the CLI's own
/// sixty-second hook budget, so the CLI is never the one to give up.
const ANSWER_WITHIN: Duration = Duration::from_secs(50);

#[derive(Deserialize)]
struct Asked {
    #[serde(default)]
    hook_event_name: String,
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    cwd: String,
    #[serde(default)]
    tool_name: String,
    #[serde(default)]
    tool_input: Option<serde_json::Value>,
    #[serde(default)]
    permission_suggestions: Option<serde_json::Value>,
}

struct Held {
    seen: Sender<()>,
    said: Sender<IslandVerdict>,
}

fn held() -> &'static Mutex<HashMap<String, Held>> {
    static HELD: OnceLock<Mutex<HashMap<String, Held>>> = OnceLock::new();
    HELD.get_or_init(Mutex::default)
}

/// Whether a hook's body is a permission question, without reading the rest.
pub(crate) fn is_permission_request(body: &str) -> bool {
    body.contains("\"PermissionRequest\"")
        && serde_json::from_str::<Asked>(body)
            .is_ok_and(|asked| asked.hook_event_name == "PermissionRequest")
}

/// The reply the CLI reads for a verdict; empty is no decision.
pub(crate) fn reply_for(
    verdict: Option<IslandVerdict>,
    suggestions: Option<&serde_json::Value>,
) -> String {
    let decision = match verdict {
        None | Some(IslandVerdict::InTerminal) => return String::new(),
        Some(IslandVerdict::Allow) => serde_json::json!({ "behavior": "allow" }),
        Some(IslandVerdict::Always) => match suggestions {
            Some(rules) => serde_json::json!({ "behavior": "allow", "updatedPermissions": rules }),
            None => serde_json::json!({ "behavior": "allow" }),
        },
        Some(IslandVerdict::Deny) => {
            serde_json::json!({ "behavior": "deny", "message": "Denied in devpit" })
        }
    };
    serde_json::json!({
        "hookSpecificOutput": { "hookEventName": "PermissionRequest", "decision": decision }
    })
    .to_string()
}

/// Holds one question while the island shows it; answers with what the CLI
/// should print.
pub(crate) fn hold(app: &tauri::AppHandle, body: &str, pane: &str) -> String {
    let Ok(asked) = serde_json::from_str::<Asked>(body) else {
        return String::new();
    };
    let in_front = app
        .get_webview_window("main")
        .and_then(|main| main.is_focused().ok())
        .unwrap_or(false);
    // Paused, nobody is looking at the island: the terminal asks at once.
    if in_front
        || crate::pausing::paused()
        || app.get_webview_window(ISLAND).is_none()
        || asked.session_id.is_empty()
    {
        return String::new();
    }

    let id = format!("ask_{}", ulid::Ulid::generate());
    let (seen, seen_rx) = channel();
    let (said, said_rx) = channel();
    if let Ok(mut all) = held().lock() {
        all.insert(id.clone(), Held { seen, said });
    }
    let question = IslandQuestion {
        id: id.clone(),
        session_id: asked.session_id.clone(),
        pane_id: pane.to_owned(),
        project: project_of(pane, &asked.cwd),
        tool: asked.tool_name.clone(),
        input: asked
            .tool_input
            .map(|input| input.to_string())
            .unwrap_or_default(),
        keepable: asked
            .permission_suggestions
            .as_ref()
            .and_then(|rules| rules.as_array())
            .is_some_and(|rules| !rules.is_empty()),
    };
    let _ = app.emit_to(
        tauri::EventTarget::webview(ISLAND),
        "island:asked",
        &question,
    );
    crate::island_notify::asked(app, &question.tool, &question.input);

    let verdict = answered(&seen_rx, &said_rx);
    if let Ok(mut all) = held().lock() {
        all.remove(&id);
    }
    let _ = app.emit_to(tauri::EventTarget::webview(ISLAND), "island:settled", &id);
    reply_for(verdict, asked.permission_suggestions.as_ref())
}

/// The project a question comes from: its card's, else the one holding its folder.
fn project_of(pane: &str, cwd: &str) -> Option<String> {
    let store = devpit_core::Store::open_default().ok()?;
    let id = crate::card_route::card_of_leaf(&store, pane)
        .map(|route| route.project_id)
        .or_else(|| {
            let projects = store.projects().ok()?;
            crate::island_feed::project_under(
                projects
                    .iter()
                    .map(|one| (one.id.as_str(), one.root_path.as_str())),
                cwd,
            )
        })?;
    store
        .project(&id)
        .ok()
        .flatten()
        .map(|project| project.name)
}

/// devpit came to the front: every question held for the island goes back
/// to its terminal, where the person now is — and where the orchestrator's
/// panel reads it — rather than waiting out its hold on the island alone.
pub(crate) fn release_all() {
    if let Ok(all) = held().lock() {
        for one in all.values() {
            let _ = one.seen.send(());
            let _ = one.said.send(IslandVerdict::InTerminal);
        }
    }
}

fn answered(seen: &Receiver<()>, said: &Receiver<IslandVerdict>) -> Option<IslandVerdict> {
    seen.recv_timeout(SEEN_WITHIN).ok()?;
    said.recv_timeout(ANSWER_WITHIN).ok()
}

/// `island.seen` — the island is showing a held question.
#[tauri::command]
#[specta::specta]
pub async fn island_seen(id: String) -> Result<(), RpcError> {
    if let Some(one) = held()
        .lock()
        .ok()
        .and_then(|all| all.get(&id).map(|one| one.seen.clone()))
    {
        let _ = one.send(());
    }
    Ok(())
}

/// `island.decide` — what the person said to a held question.
#[tauri::command]
#[specta::specta]
pub async fn island_decide(id: String, verdict: IslandVerdict) -> Result<(), RpcError> {
    decide(&id, verdict).map_err(|why| RpcError::new(devpit_rpc::ErrorCode::NotFound, why))
}

/// What the person said to a held question, from the island or a remote
/// device: it counts as seen, so the hold does not give up on it.
pub(crate) fn decide(id: &str, verdict: IslandVerdict) -> Result<(), String> {
    let (seen, said) = held()
        .lock()
        .ok()
        .and_then(|all| all.get(id).map(|one| (one.seen.clone(), one.said.clone())))
        .ok_or_else(|| {
            "that question is no longer waiting — the terminal is asking it now".to_owned()
        })?;
    let _ = seen.send(());
    let _ = said.send(verdict);
    Ok(())
}

/// `island.questions` — the shape `island:asked` carries, for the contract.
#[tauri::command]
#[specta::specta]
pub fn island_questions() -> Vec<IslandQuestion> {
    Vec::new()
}

#[cfg(test)]
#[path = "pane_asking_tests.rs"]
mod tests;
