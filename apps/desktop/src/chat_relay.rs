//! A turn's frames, kept while it runs, so a chat reopened mid-turn picks the
//! answer up where it is.
//!
//! The window that sent a turn is not the only one that may want it: leaving
//! the project unmounts the chat, and a reload drops every listener. Without
//! this the answer went on streaming to nobody and the reopened chat said the
//! app had closed, about a turn that was still running.
//!
//! An orchestrator's conversation can hold two turns at once: one another
//! session woke, and the person's, waiting for it to finish. A joiner hears
//! them one after the other, in the order they had the process.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use devpit_rpc::{Frame, RpcError};
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::State;
use tokio::sync::oneshot;

/// Frames kept for one turn. Past it they still stream to whoever listens;
/// only a late joiner misses the middle, and the transcript fills it at the end.
const KEPT_FRAMES: usize = 20_000;

struct Turn {
    id: u64,
    /// Its place among the turns that had the process, `None` while it waits.
    begun: Option<u64>,
    frames: Vec<Frame>,
    /// The window that sent it.
    first: Channel<Frame>,
    joined: Vec<Joined>,
}

struct Joined {
    channel: Channel<Frame>,
    /// Told when the turn ends; dropped unsent when the joiner leaves.
    ended: oneshot::Sender<()>,
}

#[derive(Default)]
struct Turns {
    /// One counter for ids and places, so both only grow.
    next: u64,
    by_conversation: HashMap<String, Vec<Turn>>,
}

impl Turns {
    fn turn(&mut self, conversation: &str, id: u64) -> Option<&mut Turn> {
        self.by_conversation
            .get_mut(conversation)?
            .iter_mut()
            .find(|turn| turn.id == id)
    }
}

#[derive(Default, Clone)]
pub(crate) struct Relay(Arc<Mutex<Turns>>);

/// Held for as long as the turn runs; its end is what joiners wait for.
pub(crate) struct Relaying {
    relay: Relay,
    conversation: String,
    id: u64,
}

impl Drop for Relaying {
    fn drop(&mut self) {
        let gone = self.relay.0.lock().ok().and_then(|mut all| {
            let turns = all.by_conversation.get_mut(&self.conversation)?;
            let at = turns.iter().position(|turn| turn.id == self.id)?;
            let turn = turns.remove(at);
            if turns.is_empty() {
                all.by_conversation.remove(&self.conversation);
            }
            Some(turn)
        });
        for joined in gone.map(|turn| turn.joined).unwrap_or_default() {
            let _ = joined.ended.send(());
        }
    }
}

impl Relaying {
    /// The turn has the conversation's process now: a joiner hears it after
    /// the turns that had it before, and before any that take it later.
    pub(crate) fn begin(&self) {
        self.relay.begin(&self.conversation, self.id);
    }

    /// [`Relaying::begin`], for the thread the turn runs on.
    pub(crate) fn beginning(&self) -> impl FnOnce() + Send + 'static {
        let (relay, conversation, id) = (self.relay.clone(), self.conversation.clone(), self.id);
        move || relay.begin(&conversation, id)
    }
}

impl Relay {
    /// The channel a turn sends through: each frame reaches `first` and anyone
    /// who joins later. The turn has begun; see [`Relay::waiting`].
    pub(crate) fn open(
        &self,
        conversation: &str,
        first: Channel<Frame>,
    ) -> (Channel<Frame>, Relaying) {
        let (channel, relaying) = self.waiting(conversation, first);
        relaying.begin();
        (channel, relaying)
    }

    /// As [`Relay::open`], for a turn that may have to wait for another to
    /// finish before it has the process. It is joined only when nothing that
    /// has begun is left, until [`Relaying::begin`] gives it its place.
    pub(crate) fn waiting(
        &self,
        conversation: &str,
        first: Channel<Frame>,
    ) -> (Channel<Frame>, Relaying) {
        let id = self.0.lock().ok().map(|mut all| {
            all.next += 1;
            let id = all.next;
            all.by_conversation
                .entry(conversation.to_owned())
                .or_default()
                .push(Turn {
                    id,
                    begun: None,
                    frames: Vec::new(),
                    first: first.clone(),
                    joined: Vec::new(),
                });
            id
        });
        let relaying = Relaying {
            relay: self.clone(),
            conversation: conversation.to_owned(),
            id: id.unwrap_or_default(),
        };
        let Some(id) = id else {
            return (first, relaying);
        };
        let relay = self.clone();
        let key = conversation.to_owned();
        let channel = Channel::new(move |body: InvokeResponseBody| {
            if let Ok(frame) = body.deserialize::<Frame>() {
                relay.pass(&key, id, frame);
            }
            Ok(())
        });
        (channel, relaying)
    }

    fn begin(&self, conversation: &str, id: u64) {
        let Ok(mut all) = self.0.lock() else {
            return;
        };
        all.next += 1;
        let at = all.next;
        if let Some(turn) = all.turn(conversation, id) {
            turn.begun.get_or_insert(at);
        }
    }

    fn pass(&self, conversation: &str, id: u64, frame: Frame) {
        let Ok(mut all) = self.0.lock() else { return };
        let Some(turn) = all.turn(conversation, id) else {
            return;
        };
        if turn.frames.len() < KEPT_FRAMES {
            turn.frames.push(frame.clone());
        }
        let _ = turn.first.send(frame.clone());
        for one in &turn.joined {
            let _ = one.channel.send(frame.clone());
        }
    }

    /// Joins the turn that has had the process longest — or, when none has,
    /// the first waiting for it: what it has sent so far, then the rest.
    /// `None` when nothing runs in this conversation.
    fn join(&self, conversation: &str, channel: Channel<Frame>) -> Option<oneshot::Receiver<()>> {
        let mut all = self.0.lock().ok()?;
        let turn = all
            .by_conversation
            .get_mut(conversation)?
            .iter_mut()
            .min_by_key(|turn| turn.begun.unwrap_or(u64::MAX))?;
        for frame in &turn.frames {
            let _ = channel.send(frame.clone());
        }
        let (ended, heard) = oneshot::channel();
        turn.joined.push(Joined { channel, ended });
        Some(heard)
    }

    /// Lets go of a joiner whose chat has gone: every remount joined again,
    /// and the ones before went on being sent every frame.
    fn leave(&self, conversation: &str, channel: u32) {
        let Ok(mut all) = self.0.lock() else { return };
        for turn in all
            .by_conversation
            .get_mut(conversation)
            .into_iter()
            .flatten()
        {
            turn.joined.retain(|one| one.channel.id() != channel);
        }
    }
}

/// `chat.rejoin` — the turns running in this conversation, from where they
/// are, one after the other.
///
/// Resolves when the last of them ends, or the chat leaves: `true` if one was
/// running, `false` at once when none was.
#[tauri::command]
pub async fn chat_rejoin(
    relay: State<'_, Relay>,
    conversation_id: String,
    on_frame: Channel<Frame>,
) -> Result<bool, RpcError> {
    let mut any = false;
    while let Some(ended) = relay.join(&conversation_id, on_frame.clone()) {
        any = true;
        // Dropped unsent: the chat left, and hears nothing more.
        if ended.await.is_err() {
            break;
        }
    }
    Ok(any)
}

/// `chat.leave` — stops sending a chat that has gone what it joined.
#[tauri::command]
pub async fn chat_leave(
    relay: State<'_, Relay>,
    conversation_id: String,
    channel: u32,
) -> Result<(), RpcError> {
    relay.leave(&conversation_id, channel);
    Ok(())
}

#[cfg(test)]
#[path = "chat_relay_tests.rs"]
mod tests;
