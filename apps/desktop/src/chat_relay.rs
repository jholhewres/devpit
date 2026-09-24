//! A turn's frames, kept while it runs, so a chat reopened mid-turn picks the
//! answer up where it is.
//!
//! The window that sent a turn is not the only one that may want it: leaving
//! the project unmounts the chat, and a reload drops every listener. Without
//! this the answer went on streaming to nobody and the reopened chat said the
//! app had closed, about a turn that was still running.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use devpit_rpc::{Frame, RpcError};
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::State;
use tokio::sync::oneshot;

/// Frames kept for one turn. Past it they still stream to whoever listens;
/// only a late joiner misses the middle, and the transcript fills it at the end.
const KEPT_FRAMES: usize = 20_000;

#[derive(Default)]
struct Live {
    frames: Vec<Frame>,
    joined: Vec<Channel<Frame>>,
    ended: Vec<oneshot::Sender<()>>,
}

#[derive(Default, Clone)]
pub(crate) struct Relay(Arc<Mutex<HashMap<String, Live>>>);

/// Held for as long as the turn runs; its end is what joiners wait for.
pub(crate) struct Relaying {
    relay: Relay,
    conversation: String,
    owned: bool,
}

impl Drop for Relaying {
    fn drop(&mut self) {
        if !self.owned {
            return;
        }
        let gone = self
            .relay
            .0
            .lock()
            .ok()
            .and_then(|mut all| all.remove(&self.conversation));
        for waiting in gone.map(|live| live.ended).unwrap_or_default() {
            let _ = waiting.send(());
        }
    }
}

impl Relay {
    /// The channel a turn sends through: each frame reaches `first` and anyone
    /// who joins later. A second turn on a busy conversation gets `first`
    /// alone — it is about to be refused, and must not take the running one's place.
    pub(crate) fn open(
        &self,
        conversation: &str,
        first: Channel<Frame>,
    ) -> (Channel<Frame>, Relaying) {
        let owned = self.0.lock().is_ok_and(|mut all| {
            if all.contains_key(conversation) {
                return false;
            }
            all.insert(
                conversation.to_owned(),
                Live {
                    joined: vec![first.clone()],
                    ..Live::default()
                },
            );
            true
        });
        let relaying = Relaying {
            relay: self.clone(),
            conversation: conversation.to_owned(),
            owned,
        };
        if !owned {
            return (first, relaying);
        }
        let relay = self.clone();
        let key = conversation.to_owned();
        let channel = Channel::new(move |body: InvokeResponseBody| {
            if let Ok(frame) = body.deserialize::<Frame>() {
                relay.pass(&key, frame);
            }
            Ok(())
        });
        (channel, relaying)
    }

    fn pass(&self, conversation: &str, frame: Frame) {
        let Ok(mut all) = self.0.lock() else { return };
        let Some(live) = all.get_mut(conversation) else {
            return;
        };
        if live.frames.len() < KEPT_FRAMES {
            live.frames.push(frame.clone());
        }
        for one in &live.joined {
            let _ = one.send(frame.clone());
        }
    }

    /// Joins the running turn: what it has sent so far, then the rest. `None`
    /// when nothing runs in this conversation.
    fn join(&self, conversation: &str, channel: Channel<Frame>) -> Option<oneshot::Receiver<()>> {
        let mut all = self.0.lock().ok()?;
        let live = all.get_mut(conversation)?;
        for frame in &live.frames {
            let _ = channel.send(frame.clone());
        }
        live.joined.push(channel);
        let (said, heard) = oneshot::channel();
        live.ended.push(said);
        Some(heard)
    }
}

/// `chat.rejoin` — the turn running in this conversation, from where it is.
///
/// Resolves when that turn ends: `true` if one was running, `false` at once
/// when none was.
#[tauri::command]
pub async fn chat_rejoin(
    relay: State<'_, Relay>,
    conversation_id: String,
    on_frame: Channel<Frame>,
) -> Result<bool, RpcError> {
    let Some(ended) = relay.join(&conversation_id, on_frame) else {
        return Ok(false);
    };
    let _ = ended.await;
    Ok(true)
}

#[cfg(test)]
#[path = "chat_relay_tests.rs"]
mod tests;
