//! A chat, reachable by Remote Control — an orchestrator's or a project's.
//!
//! The chat's own process takes a `remote_control` control request and answers
//! with the session's page on claude.ai (measured on Claude Code 2.1.282), so
//! the conversation on the phone is the one in this chat — same process, same
//! turns. Messages written there wake it like any other session's, and the
//! chat shows them as they come. A project's chat keeps a process of its own
//! only while it is reachable; an orchestrator's always has one.

use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

use devpit_rpc::{RemoteState, RpcError};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

/// How long a connection is waited on before the chat is told it is pending.
const WAITS: Duration = Duration::from_secs(20);

#[derive(Default)]
pub(crate) struct Remotes {
    /// Conversations asked to be reachable, with the name they show and the
    /// page they were given.
    wanted: Mutex<HashMap<String, Wanted>>,
    /// Control requests waiting on their answer, by request id.
    pending: Mutex<HashMap<String, mpsc::Sender<Value>>>,
}

struct Wanted {
    name: String,
    url: Option<String>,
}

/// `chat.remote` — Remote Control for a conversation, on or off.
#[tauri::command]
#[specta::specta]
pub async fn chat_remote(
    app: AppHandle,
    project_id: String,
    conversation_id: String,
    enabled: bool,
) -> Result<RemoteState, RpcError> {
    crate::off_main::blocking(move || {
        let (name, orchestrating) = remote_of(&project_id)?;
        let remotes = app.state::<Remotes>();
        if enabled {
            if let Ok(mut wanted) = remotes.wanted.lock() {
                wanted.insert(
                    conversation_id.clone(),
                    Wanted {
                        name: name.clone(),
                        url: None,
                    },
                );
            }
            // Its process may not be running yet; then it connects when the
            // next message starts it, and the chat says so.
            if let Some(url) = ask(&app, &conversation_id, true, &name) {
                remember(&app, &conversation_id, url);
            }
        } else {
            if let Ok(mut wanted) = remotes.wanted.lock() {
                wanted.remove(&conversation_id);
            }
            let _ = ask(&app, &conversation_id, false, &name);
            // A project's chat kept its process only to be reachable.
            if !orchestrating {
                crate::chat_resident::close(&app, &conversation_id);
            }
        }
        Ok(state_of(&app, &conversation_id))
    })
    .await
}

/// `chat.remoteState` — whether this conversation is reachable, and where.
#[tauri::command]
#[specta::specta]
pub fn chat_remote_state(app: AppHandle, conversation_id: String) -> Result<RemoteState, RpcError> {
    Ok(state_of(&app, &conversation_id))
}

/// Whether this conversation was asked to be reachable.
pub(crate) fn wanted(app: &AppHandle, conversation_id: &str) -> bool {
    app.try_state::<Remotes>()
        .and_then(|remotes| {
            remotes
                .wanted
                .lock()
                .ok()
                .map(|wanted| wanted.contains_key(conversation_id))
        })
        .unwrap_or(false)
}

/// A control answer from a conversation's process, handed to whoever asked.
pub(crate) fn answered(app: &AppHandle, said: Value) {
    let Some(id) = said.get("request_id").and_then(Value::as_str) else {
        return;
    };
    let waiting = app.try_state::<Remotes>().and_then(|remotes| {
        remotes
            .pending
            .lock()
            .ok()
            .and_then(|mut all| all.remove(id))
    });
    if let Some(tell) = waiting {
        let _ = tell.send(said);
    }
}

/// A conversation's process was (re)started: one that was asked to be
/// reachable connects again, off the thread that started it.
pub(crate) fn reconnect(app: &AppHandle, conversation_id: &str) {
    let name = app.try_state::<Remotes>().and_then(|remotes| {
        remotes
            .wanted
            .lock()
            .ok()
            .and_then(|wanted| wanted.get(conversation_id).map(|one| one.name.clone()))
    });
    let Some(name) = name else { return };
    let app = app.clone();
    let conversation = conversation_id.to_owned();
    std::thread::spawn(move || {
        if let Some(url) = ask(&app, &conversation, true, &name) {
            remember(&app, &conversation, url);
        }
    });
}

fn ask(app: &AppHandle, conversation_id: &str, enabled: bool, name: &str) -> Option<String> {
    let remotes = app.try_state::<Remotes>()?;
    let id = format!("remote-{}", ulid::Ulid::generate());
    let (tell, heard) = mpsc::channel();
    remotes.pending.lock().ok()?.insert(id.clone(), tell);
    let request = json!({ "subtype": "remote_control", "enabled": enabled, "name": name });
    if !crate::chat_resident::control(app, conversation_id, &id, request) {
        remotes.pending.lock().ok()?.remove(&id);
        return None;
    }
    let said = heard.recv_timeout(WAITS).ok();
    remotes.pending.lock().ok()?.remove(&id);
    said?
        .get("response")?
        .get("session_url")?
        .as_str()
        .map(str::to_owned)
}

/// Said when a conversation's page arrives: a process that connects with a
/// turn answers after that turn, when the chat has already looked.
pub(crate) const CONNECTED: &str = "chat:remote";

fn remember(app: &AppHandle, conversation_id: &str, url: String) {
    if let Some(remotes) = app.try_state::<Remotes>() {
        if let Ok(mut wanted) = remotes.wanted.lock() {
            if let Some(one) = wanted.get_mut(conversation_id) {
                one.url = Some(url);
            }
        }
    }
    let _ = app.emit(CONNECTED, conversation_id);
}

fn state_of(app: &AppHandle, conversation_id: &str) -> RemoteState {
    let wanted = app.try_state::<Remotes>().and_then(|remotes| {
        remotes
            .wanted
            .lock()
            .ok()
            .and_then(|wanted| wanted.get(conversation_id).map(|one| one.url.clone()))
    });
    RemoteState {
        on: wanted.is_some(),
        url: wanted.flatten(),
    }
}

/// The name the session shows — the project's or the orchestrator's — and
/// whether it is an orchestrator.
fn remote_of(project_id: &str) -> Result<(String, bool), RpcError> {
    let store = crate::projects::store()?;
    let (_, root) = crate::projects::locate(&store, project_id)?;
    let home = devpit_core::Store::root()
        .ok()
        .and_then(|home| home.canonicalize().ok())
        .unwrap_or_default();
    let orchestrating = devpit_core::home::orchestrator_of(&home, &root).is_some();
    let name = store
        .projects()?
        .into_iter()
        .find(|row| row.id == project_id)
        .map(|row| row.name)
        .unwrap_or_default();
    Ok((remote_name(&name), orchestrating))
}

/// `devpit-<name>`, made plain: it is shown on claude.ai and in the app.
pub(crate) fn remote_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let words: String = out.trim_end_matches('-').chars().take(40).collect();
    if words.is_empty() {
        "devpit".to_owned()
    } else {
        format!("devpit-{words}")
    }
}

#[cfg(test)]
mod tests {
    use super::remote_name;

    #[test]
    fn a_remote_session_is_named_after_its_orchestrator_plainly() {
        assert_eq!(remote_name("Client work"), "devpit-client-work");
        assert_eq!(remote_name("x'; reboot #"), "devpit-x-reboot");
        assert_eq!(remote_name("   "), "devpit");
    }
}
