//! Steering a chat turn while it runs: the stdin handles, by conversation.
//!
//! Apart from `chat.rs` because that file starts and ends turns, and this one
//! holds what lets a click reach a turn in the middle — today only stopping one
//! background task, measured to work on Claude Code 2.1.270 without ending the
//! turn around it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use devpit_agentcli::control::Control;
use devpit_rpc::RpcError;
use tauri::State;

/// The turns that can be steered, by conversation id.
#[derive(Default, Clone)]
pub struct Steering {
    held: Arc<Mutex<HashMap<String, Control>>>,
}

impl Steering {
    /// A fresh handle for a turn about to start, kept until it ends.
    pub(crate) fn hold(&self, conversation_id: &str) -> Control {
        let control = Control::new();
        if let Ok(mut held) = self.held.lock() {
            held.insert(conversation_id.to_owned(), control.clone());
        }
        control
    }

    pub(crate) fn release(&self, conversation_id: &str) {
        if let Ok(mut held) = self.held.lock() {
            held.remove(conversation_id);
        }
    }

    fn get(&self, conversation_id: &str) -> Option<Control> {
        self.held.lock().ok()?.get(conversation_id).cloned()
    }

    /// Whether a turn's handle is still held. Only the guard's test asks.
    #[cfg(test)]
    pub(crate) fn holds(&self, conversation_id: &str) -> bool {
        self.get(conversation_id).is_some()
    }
}

/// `chat.stop_task` — stops one background task of the turn in flight.
///
/// Answers whether the request reached the turn. The task's ending arrives on
/// the stream like any other, so the screen learns it stopped from the CLI and
/// not from this answer.
#[tauri::command]
#[specta::specta]
pub fn chat_stop_task(
    steering: State<'_, Steering>,
    conversation_id: String,
    task_id: String,
) -> Result<bool, RpcError> {
    Ok(steering
        .get(&conversation_id)
        .is_some_and(|control| control.stop_task(&task_id)))
}

/// `chat.steer` — hands the turn in flight one more message, without waiting
/// for it to end: the CLI takes it in at its next step.
///
/// Answers with the message as the transcript keeps it. Refused when no turn
/// runs, or the turn has already let go of its input: then it waits its turn.
#[tauri::command]
#[specta::specta]
pub async fn chat_steer(
    app: tauri::AppHandle,
    talking: State<'_, crate::chat::Talking>,
    steering: State<'_, Steering>,
    project_id: String,
    conversation_id: String,
    prompt: String,
) -> Result<devpit_rpc::Message, RpcError> {
    let running = talking
        .running
        .lock()
        .is_ok_and(|held| held.contains_key(&conversation_id));
    let steering = steering.inner().clone();
    crate::off_main::blocking(move || {
        steer_now(
            &app,
            &steering,
            running,
            &project_id,
            &conversation_id,
            &prompt,
        )
    })
    .await
}

fn steer_now(
    app: &tauri::AppHandle,
    steering: &Steering,
    running: bool,
    project_id: &str,
    conversation_id: &str,
    prompt: &str,
) -> Result<devpit_rpc::Message, RpcError> {
    let busy = |why: &str| RpcError::new(devpit_rpc::ErrorCode::Conflict, why.to_owned());
    if !running || prompt.trim().is_empty() {
        return Err(busy("no turn is running to take it"));
    }
    let taken = crate::chat_resident::steer(app, conversation_id, prompt)
        .or_else(|| {
            steering
                .get(conversation_id)
                .map(|control| control.say(prompt))
        })
        .unwrap_or(false);
    if !taken {
        return Err(busy("the turn is ending; it goes next"));
    }
    // No turn of its own: it is part of the one running.
    let said = devpit_rpc::Message {
        turn_id: None,
        ..crate::chat_turn::messages("", prompt, crate::chat::now() * 1000.0).0
    };
    let sessions = crate::projects::project_home(project_id)?.sessions();
    let file = devpit_agentcli::store::conversation_path(&sessions, conversation_id);
    let _ = devpit_agentcli::store::append(&file, &said);
    Ok(said)
}

/// `chat.cancel` — stops the turn in flight, keeping what already arrived.
///
/// Answers with the ending it caused, or nothing when no turn was running.
#[tauri::command]
#[specta::specta]
pub fn chat_cancel(
    app: tauri::AppHandle,
    state: State<'_, crate::chat::Talking>,
    conversation_id: String,
) -> Result<Option<devpit_rpc::TurnEnd>, RpcError> {
    // A conversation whose process stays is interrupted, not ended: ending it
    // would leave it deaf to the sessions it follows.
    let stopped = crate::chat_resident::interrupt(&app, &conversation_id)
        || stop_turn(&state, &conversation_id);
    if !stopped {
        return Ok(None);
    }
    Ok(Some(devpit_rpc::TurnEnd {
        turn_id: String::new(),
        cost_usd: None,
        duration_ms: None,
        stop_reason: Some("cancelled".to_owned()),
        is_error: false,
        context: None,
    }))
}

/// Signals the turn in flight to end. Answers whether there was one to signal.
pub(crate) fn stop_turn(talking: &crate::chat::Talking, conversation_id: &str) -> bool {
    let pid = talking
        .running
        .lock()
        .ok()
        // `None` is a turn that has been claimed but has no process yet: there
        // is nothing to signal, and pid 0 would mean the whole process group.
        .and_then(|held| held.get(conversation_id).copied().flatten());
    let Some(pid) = pid else {
        return false;
    };
    // SIGTERM, not SIGKILL: the CLI gets to write its own last line.
    let _ = std::process::Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_conversation_with_no_turn_running_cannot_be_steered() {
        let steering = Steering::default();
        assert!(steering.get("c1").is_none());
    }

    #[test]
    fn a_turn_is_held_while_it_runs_and_released_after() {
        let steering = Steering::default();
        steering.hold("c1");
        assert!(steering.get("c1").is_some());
        steering.release("c1");
        assert!(steering.get("c1").is_none());
    }
}
