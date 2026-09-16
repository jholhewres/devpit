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
    steering: Steering,
    conversation_id: String,
}

impl Drop for TurnGuard {
    fn drop(&mut self) {
        if let Ok(mut held) = self.running.lock() {
            held.remove(&self.conversation_id);
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
            steering: steering.clone(),
            conversation_id: conversation_id.to_owned(),
        })
    }
}

#[cfg(test)]
#[path = "chat_running_tests.rs"]
mod tests;
