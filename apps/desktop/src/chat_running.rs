//! One turn at a time in a conversation, and what a turn lets go of when it
//! ends.
//!
//! Apart from `chat.rs` because it is a rule, not a step of the send: the
//! claim is taken before the process exists and released however the turn
//! leaves — including the `?` at the end, which used to skip the cleanup and
//! leave a pid behind. A later `chat.cancel` then sent SIGTERM to a number the
//! operating system had already given to somebody else.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use devpit_rpc::{ErrorCode, RpcError};

use crate::chat::Talking;
use crate::steering::Steering;

/// The claim a turn holds while it runs.
///
/// Dropping it removes the pid and releases the steering handle, so every exit
/// path — answered, failed, cancelled, or a `?` on the way out — leaves the
/// conversation ready for the next turn. The pid itself is written by the
/// callback the turn already reports its process through.
pub(crate) struct TurnGuard {
    running: Arc<Mutex<HashMap<String, Option<u32>>>>,
    stopping: Arc<Mutex<HashMap<String, Signal>>>,
    steering: Steering,
    conversation_id: String,
}

impl Drop for TurnGuard {
    fn drop(&mut self) {
        if let Ok(mut held) = self.running.lock() {
            held.remove(&self.conversation_id);
            if let Ok(mut stopping) = self.stopping.lock() {
                stopping.remove(&self.conversation_id);
            }
        }
        self.steering.release(&self.conversation_id);
    }
}

impl Talking {
    /// Claims this conversation for a turn, or says it is busy.
    ///
    /// Two turns in one conversation are not two turns: they are two processes
    /// appending to the same transcript, with one pid recorded for both.
    pub(crate) fn begin(
        &self,
        conversation_id: &str,
        steering: &Steering,
    ) -> Result<TurnGuard, RpcError> {
        let mut held = self
            .running
            .lock()
            .map_err(|_| RpcError::internal("the turns in flight could not be read"))?;
        if held.contains_key(conversation_id) {
            return Err(RpcError::new(
                ErrorCode::Conflict,
                "this conversation already has a turn running".to_owned(),
            ));
        }
        held.insert(conversation_id.to_owned(), None);
        drop(held);

        Ok(TurnGuard {
            running: self.running.clone(),
            stopping: self.stopping.clone(),
            steering: steering.clone(),
            conversation_id: conversation_id.to_owned(),
        })
    }
}

/// How a stop ends a turn's process.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Signal {
    Term,
    Kill,
}

impl Talking {
    /// Stops this conversation's turn with `signal`: its process now, or —
    /// claimed, its process not started yet — as soon as it starts. Whether a
    /// turn was there to stop.
    pub(crate) fn stop(&self, conversation: &str, signal: Signal) -> bool {
        let Ok(held) = self.running.lock() else {
            return false;
        };
        match held.get(conversation) {
            None => false,
            Some(Some(pid)) => {
                signalled(*pid, signal);
                true
            }
            Some(None) => {
                if let Ok(mut stopping) = self.stopping.lock() {
                    let asked = stopping.entry(conversation.to_owned()).or_insert(signal);
                    if signal == Signal::Kill {
                        *asked = signal;
                    }
                }
                true
            }
        }
    }

    /// What the turn calls with its process once it starts: kept to be
    /// stopped, and stopped at once when that was asked before.
    pub(crate) fn on_start(&self, conversation: &str) -> impl Fn(u32) + Send + Sync + 'static {
        let (running, stopping) = (self.running.clone(), self.stopping.clone());
        let key = conversation.to_owned();
        move |pid| {
            let Ok(mut held) = running.lock() else {
                return;
            };
            held.insert(key.clone(), Some(pid));
            let asked = stopping.lock().ok().and_then(|mut all| all.remove(&key));
            if let Some(signal) = asked {
                signalled(pid, signal);
            }
        }
    }
}

fn signalled(pid: u32, signal: Signal) {
    let flag = match signal {
        Signal::Term => "-TERM",
        Signal::Kill => "-KILL",
    };
    let _ = std::process::Command::new("kill")
        .arg(flag)
        .arg(pid.to_string())
        .status();
}

#[cfg(test)]
#[path = "chat_running_tests.rs"]
mod tests;
