//! An orchestrator's chat, reachable by Remote Control.
//!
//! The chat's own process takes a `remote_control` control request and answers
//! with the session's page on claude.ai (measured on Claude Code 2.1.282), so
//! the conversation on the phone is the one in this chat — same process, same
//! turns. Messages written there wake it like any other session's, and the
//! chat shows them as they come.

use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

use devpit_rpc::{ErrorCode, RemoteState, RpcError};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

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

/// `chat.remote` — Remote Control for an orchestrator's conversation, on or off.
#[tauri::command]
#[specta::specta]
pub async fn chat_remote(
    app: AppHandle,
    project_id: String,
    conversation_id: String,
    enabled: bool,
) -> Result<RemoteState, RpcError> {
    crate::off_main::blocking(move || {
        let name = orchestrator_name(&project_id)?;
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

fn remember(app: &AppHandle, conversation_id: &str, url: String) {
    if let Some(remotes) = app.try_state::<Remotes>() {
        if let Ok(mut wanted) = remotes.wanted.lock() {
            if let Some(one) = wanted.get_mut(conversation_id) {
                one.url = Some(url);
            }
        }
    }
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

/// The orchestrator's name, as the session shows it; an error for a project
/// that is not one.
fn orchestrator_name(project_id: &str) -> Result<String, RpcError> {
    let store = crate::projects::store()?;
    let (_, root) = crate::projects::locate(&store, project_id)?;
    let home = devpit_core::Store::root()
        .ok()
        .and_then(|home| home.canonicalize().ok())
        .unwrap_or_default();
    if devpit_core::home::orchestrator_of(&home, &root).is_none() {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            "only an orchestrator's chat is reached remotely",
        ));
    }
    let name = store
        .projects()?
        .into_iter()
        .find(|row| row.id == project_id)
        .map(|row| row.name)
        .unwrap_or_default();
    Ok(remote_name(&name))
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
