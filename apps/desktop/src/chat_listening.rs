//! Who hears a resident conversation's process right now: the person's turn,
//! or a turn another session woke.
//!
//! Apart from `chat_resident.rs` because this is where the races were. The
//! person's turn takes the process only when it is quiet, under the same lock
//! a woken turn starts under, so neither lands in the other; and a Stop while
//! it waits stops the wait, not the woken turn it waits for.

use std::path::Path;
use std::sync::{mpsc, Condvar, Mutex};

use devpit_agentcli::resident::Heard;
use devpit_agentcli::store::append;
use devpit_agentcli::talk::Said;
use devpit_rpc::{Frame, Message, Part, Role, TurnEnd};
use tauri::ipc::Channel;

/// Where what the process says goes right now.
#[derive(Default)]
pub(crate) enum Listening {
    /// Nobody asked: a turn that starts now was woken.
    #[default]
    Nobody,
    /// The person's turn, read by the thread that sent it.
    Person(mpsc::Sender<Heard>),
    /// A turn another session woke, written here as it goes.
    Woken(Woken),
}

/// A woken turn in progress.
pub(crate) struct Woken {
    pub turn_id: String,
    pub answer_id: String,
    pub parts: Vec<Part>,
    pub frames: Channel<Frame>,
    pub _relaying: crate::chat_relay::Relaying,
}

#[derive(Default)]
struct Now {
    listening: Listening,
    /// A person's turn is waiting for the process.
    waiting: bool,
    /// ...and was stopped before it had it.
    stopped: bool,
}

/// One conversation's process, as the turns on it see it.
#[derive(Default)]
pub(crate) struct Ear {
    now: Mutex<Now>,
    /// Told whenever the process goes quiet, or a wait is stopped.
    quiet: Condvar,
}

/// The person's turn takes a quiet process, and never one a woken turn is on.
pub(crate) fn taken(listening: &mut Listening, tell: &mpsc::Sender<Heard>) -> bool {
    if !matches!(listening, Listening::Nobody) {
        return false;
    }
    *listening = Listening::Person(tell.clone());
    true
}

impl Ear {
    /// Waits for a woken turn to finish and takes the process, without letting
    /// go of the lock in between: a turn woken in that gap used to be
    /// overwritten, unsaved, and its words went into the person's answer.
    /// False when the wait was stopped.
    pub(crate) fn claim(&self, tell: &mpsc::Sender<Heard>) -> bool {
        let Ok(mut now) = self.now.lock() else {
            return false;
        };
        now.waiting = true;
        loop {
            if std::mem::take(&mut now.stopped) {
                now.waiting = false;
                return false;
            }
            if taken(&mut now.listening, tell) {
                now.waiting = false;
                return true;
            }
            now = match self.quiet.wait(now) {
                Ok(now) => now,
                Err(_) => return false,
            };
        }
    }

    /// Stops a person's turn still waiting for the process. The woken turn it
    /// waits for goes on: Stop is for the person's own turn. False when none
    /// waits.
    pub(crate) fn stop_waiting(&self) -> bool {
        let Ok(mut now) = self.now.lock() else {
            return false;
        };
        if !now.waiting {
            return false;
        }
        now.stopped = true;
        self.quiet.notify_all();
        true
    }

    /// Hands what the process said to whoever it is for. A part nobody asked
    /// for starts a woken turn, made by `wake`.
    pub(crate) fn hear(
        &self,
        heard: Heard,
        wake: impl FnOnce() -> Woken,
        transcript: &Path,
        head: &Path,
    ) {
        let Ok(mut now) = self.now.lock() else { return };
        match (&mut now.listening, heard) {
            (Listening::Person(tell), Heard::Ended(said)) => {
                let _ = tell.send(Heard::Ended(said));
                now.listening = Listening::Nobody;
            }
            // Gone mid-turn: that turn fails, and the conversation is left
            // quiet so the next one starts a new process instead of waiting
            // for this one for ever.
            (Listening::Person(tell), Heard::Gone) => {
                let _ = tell.send(Heard::Gone);
                now.listening = Listening::Nobody;
            }
            (Listening::Person(tell), heard) => {
                let _ = tell.send(heard);
            }
            (Listening::Nobody, Heard::Part(part)) => {
                let mut woken = wake();
                woken.heard(part);
                now.listening = Listening::Woken(woken);
            }
            (Listening::Woken(woken), Heard::Part(part)) => woken.heard(part),
            (Listening::Woken(_), Heard::Ended(said)) => {
                if let Listening::Woken(woken) = std::mem::take(&mut now.listening) {
                    woken.finish(transcript, head, said);
                }
            }
            (Listening::Woken(_), Heard::Gone) => {
                if let Listening::Woken(woken) = std::mem::take(&mut now.listening) {
                    woken.gone(transcript);
                }
            }
            (Listening::Nobody, _) | (_, Heard::Control(_)) => {}
        }
        if matches!(now.listening, Listening::Nobody) {
            self.quiet.notify_all();
        }
    }
}

impl Woken {
    fn heard(&mut self, part: Part) {
        self.parts.push(part.clone());
        self.frames
            .send(Frame::Part {
                message_id: self.answer_id.clone(),
                part,
            })
            .ok();
    }

    fn written(&self) -> Message {
        Message {
            id: self.answer_id.clone(),
            turn_id: Some(self.turn_id.clone()),
            role: Role::Assistant,
            parts: self.parts.clone(),
            created_at: now(),
            streaming: false,
        }
    }

    fn finish(self, transcript: &Path, head: &Path, said: Said) {
        let _ = append(transcript, &self.written());
        crate::chat_turn::settle(
            head,
            None,
            &self.turn_id,
            &said.end,
            said.session_id,
            said.anchor,
        );
        let end = TurnEnd {
            turn_id: self.turn_id,
            ..said.end
        };
        self.frames.send(Frame::Ended { end }).ok();
    }

    /// The process died mid-answer: what it had said is kept, and the end
    /// names the turn, so the chat closes the answer it was showing.
    pub(crate) fn gone(self, transcript: &Path) {
        let _ = append(transcript, &self.written());
        let end = TurnEnd {
            turn_id: self.turn_id,
            cost_usd: None,
            duration_ms: None,
            stop_reason: Some("interrupted".to_owned()),
            is_error: true,
            context: None,
        };
        self.frames.send(Frame::Ended { end }).ok();
    }
}

pub(crate) fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs_f64())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "chat_listening_tests.rs"]
mod tests;
