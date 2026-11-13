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
    relay: State<'_, crate::chat_relay::Relay>,
    project_id: String,
    conversation_id: String,
    prompt: String,
) -> Result<devpit_rpc::Message, RpcError> {
    let running = talking
        .running
        .lock()
        .is_ok_and(|held| held.contains_key(&conversation_id));
    let steering = steering.inner().clone();
    let relay = relay.inner().clone();
    crate::off_main::blocking(move || {
        steer_now(
            &app,
            &steering,
            &relay,
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
    relay: &crate::chat_relay::Relay,
    running: bool,
    project_id: &str,
    conversation_id: &str,
    prompt: &str,
) -> Result<devpit_rpc::Message, RpcError> {
    let busy = |why: &str| RpcError::new(devpit_rpc::ErrorCode::Conflict, why.to_owned());
    if !running || prompt.trim().is_empty() {
        return Err(busy("no turn is running to take it"));
    }
    // No turn of its own: it is part of the one running.
    let said = devpit_rpc::Message {
        turn_id: None,
        ..crate::chat_turn::messages("", prompt, crate::chat::now() * 1000.0).0
    };
    let uuid = steer_uuid(ulid::Ulid::generate().0);
    let taken = crate::chat_resident::steer(app, conversation_id, prompt, &uuid, &said.id)
        .or_else(|| {
            steering
                .get(conversation_id)
                .map(|control| control.steer(prompt, &uuid, &said.id))
        })
        .unwrap_or(false);
    if !taken {
        return Err(busy("the turn is ending; it goes next"));
    }
    let sessions = crate::projects::project_home(project_id)?.sessions();
    let file = devpit_agentcli::store::conversation_path(&sessions, conversation_id);
    let _ = devpit_agentcli::store::append(&file, &said);
    // Through the turn's relay: a second window, a chat rejoining and the
    // Remote draw it too, not only the composer that sent it.
    relay.told(
        conversation_id,
        devpit_rpc::Frame::Opened {
            message: said.clone(),
        },
    );
    Ok(said)
}

/// A version-4 uuid made of `bits`: the form the CLI echoes a message's id back in.
fn steer_uuid(bits: u128) -> String {
    let bits = (bits & !(0xf << 76)) | (0x4 << 76);
    let bits = (bits & !(0x3 << 62)) | (0x2 << 62);
    let hex = format!("{bits:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
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
    // A turn still starting is stopped as it starts: an interrupt written now
    // reaches a process with nothing to stop. One waiting for a woken turn stops waiting.
    // A conversation whose process stays is interrupted, not ended: ending it
    // would leave it deaf to the sessions it follows.
    let stopped = if state.stop_starting(&conversation_id) {
        crate::chat_resident::stop_waiting(&app, &conversation_id);
        true
    } else {
        crate::chat_resident::interrupt(&app, &conversation_id)
            || stop_turn(&state, &conversation_id)
    };
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
        unheard: Vec::new(),
    }))
}

/// Signals the turn in flight to end. Answers whether there was one to signal:
/// a turn still starting is stopped as its process starts.
pub(crate) fn stop_turn(talking: &crate::chat::Talking, conversation_id: &str) -> bool {
    // SIGTERM, not SIGKILL: the CLI gets to write its own last line.
    talking.stop(conversation_id, crate::chat_running::Signal::Term)
}

/// `chat.force_stop` — ends the turn in flight however it is: what a stop
/// that has not ended it after a while offers. A process that stays is ended
/// too, and the next turn starts another.
#[tauri::command]
#[specta::specta]
pub async fn chat_force_stop(
    app: tauri::AppHandle,
    state: State<'_, crate::chat::Talking>,
    conversation_id: String,
) -> Result<bool, RpcError> {
    let talking = state.inner().clone();
    crate::off_main::blocking(move || {
        // The process that stays, then the turn's own unless that was the one:
        // a supervised turn runs apart from a process the conversation keeps.
        let ended = crate::chat_resident::force(&app, &conversation_id);
        Ok(talking.force(&conversation_id, ended) || ended.is_some())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_steer_is_sent_under_a_version_4_uuid() {
        let uuid = steer_uuid(u128::MAX);
        assert_eq!(uuid, "ffffffff-ffff-4fff-bfff-ffffffffffff");
        assert_eq!(steer_uuid(0), "00000000-0000-4000-8000-000000000000");
    }

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
