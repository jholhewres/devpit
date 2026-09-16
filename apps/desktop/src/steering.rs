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

/// `chat.cancel` — stops the turn in flight, keeping what already arrived.
///
/// Answers with the ending it caused, or nothing when no turn was running.
#[tauri::command]
#[specta::specta]
pub fn chat_cancel(
    state: State<'_, crate::chat::Talking>,
    conversation_id: String,
) -> Result<Option<devpit_rpc::TurnEnd>, RpcError> {
    let pid = state
        .running
        .lock()
        .ok()
        // `None` is a turn that has been claimed but has no process yet: there
        // is nothing to signal, and pid 0 would mean the whole process group.
        .and_then(|held| held.get(&conversation_id).copied().flatten());
    let Some(pid) = pid else {
        return Ok(None);
    };
    // SIGTERM, not SIGKILL: the CLI gets to write its own last line.
    let _ = std::process::Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status();
    Ok(Some(devpit_rpc::TurnEnd {
        turn_id: String::new(),
        cost_usd: None,
        duration_ms: None,
        stop_reason: Some("cancelled".to_owned()),
        is_error: false,
    }))
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
