//! A turn's streamed text, sent at most once a window.
//!
//! The CLI writes an answer a few characters at a time, and every frame a
//! window is sent is a script it evaluates. Joined over a window, an answer
//! costs a few frames a second instead of a hundred, and what is not text
//! lets the text before it out first, so nothing changes its order.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use devpit_rpc::{Frame, Part};
use tauri::ipc::{Channel, InvokeResponseBody};

/// The longest a piece of text waits for the text after it.
pub(crate) const WINDOW: Duration = Duration::from_millis(50);

/// The most text one frame carries: past it, what is held goes at once and a
/// longer piece goes in parts, so a burst never makes a frame over
/// [`crate::chat_relay::FRAME_BUDGET`].
pub(crate) const MOST_HELD: usize = 4 * 1024;

/// Text held back to go out with the text after it.
#[derive(Default)]
pub(crate) struct Pace {
    held: Option<(Frame, Instant)>,
}

impl Pace {
    /// What goes out now that `frame` arrived at `now`.
    pub(crate) fn took(&mut self, frame: Frame, now: Instant) -> Vec<Frame> {
        let mut out = Vec::new();
        for frame in parts(frame) {
            out.extend(self.one(frame, now));
        }
        out
    }

    fn one(&mut self, frame: Frame, now: Instant) -> Vec<Frame> {
        if let Some((held, since)) = &mut self.held {
            if size(held) + size(&frame) <= MOST_HELD && joined(held, &frame) {
                let waited = now.duration_since(*since) >= WINDOW;
                return if waited {
                    self.rest().into_iter().collect()
                } else {
                    Vec::new()
                };
            }
        }
        let mut out: Vec<Frame> = self.rest().into_iter().collect();
        if text(&frame) {
            self.held = Some((frame, now));
        } else {
            out.push(frame);
        }
        out
    }

    /// The text held, once it has waited a window.
    pub(crate) fn due(&mut self, now: Instant) -> Option<Frame> {
        let (_, since) = self.held.as_ref()?;
        if now.duration_since(*since) < WINDOW {
            return None;
        }
        self.rest()
    }

    /// Whatever is held, now.
    pub(crate) fn rest(&mut self) -> Option<Frame> {
        self.held.take().map(|(frame, _)| frame)
    }
}

/// The bytes of text a frame carries.
fn size(frame: &Frame) -> usize {
    match frame {
        Frame::Part {
            part: Part::Text { text, .. },
            ..
        } => text.len(),
        _ => 0,
    }
}

/// `frame`, or its text in pieces of at most [`MOST_HELD`] bytes, cut on
/// character boundaries.
fn parts(frame: Frame) -> Vec<Frame> {
    let Frame::Part {
        message_id,
        part: Part::Text { text, parent },
    } = frame
    else {
        return vec![frame];
    };
    let mut out = Vec::new();
    let mut rest = text.as_str();
    loop {
        let mut at = rest.len().min(MOST_HELD);
        while !rest.is_char_boundary(at) {
            at -= 1;
        }
        let (piece, after) = rest.split_at(at);
        out.push(Frame::Part {
            message_id: message_id.clone(),
            part: Part::Text {
                text: piece.to_owned(),
                parent: parent.clone(),
            },
        });
        if after.is_empty() {
            return out;
        }
        rest = after;
    }
}

fn text(frame: &Frame) -> bool {
    matches!(
        frame,
        Frame::Part {
            part: Part::Text { .. },
            ..
        }
    )
}

/// Adds `frame`'s text onto `held` when both are one speaker's text in one message.
fn joined(held: &mut Frame, frame: &Frame) -> bool {
    let (
        Frame::Part {
            message_id: at,
            part: Part::Text { text, parent },
        },
        Frame::Part {
            message_id,
            part:
                Part::Text {
                    text: more,
                    parent: theirs,
                },
        },
    ) = (held, frame)
    else {
        return false;
    };
    if at != message_id || parent != theirs {
        return false;
    }
    text.push_str(more);
    true
}

struct Pacing {
    pace: Mutex<Pace>,
    out: Channel<Frame>,
}

impl Pacing {
    /// Sends under the lock, so what the tick lets out and what a send lets
    /// out never cross.
    fn send(&self, step: impl FnOnce(&mut Pace) -> Vec<Frame>) {
        let Ok(mut pace) = self.pace.lock() else {
            return;
        };
        for frame in step(&mut pace) {
            let _ = self.out.send(frame);
        }
    }
}

impl Drop for Pacing {
    fn drop(&mut self) {
        if let Some(frame) = self.pace.get_mut().ok().and_then(Pace::rest) {
            let _ = self.out.send(frame);
        }
    }
}

/// The channel a turn sends through, its text paced into `out`.
pub(crate) fn paced(out: Channel<Frame>) -> Channel<Frame> {
    let pacing = Arc::new(Pacing {
        pace: Mutex::new(Pace::default()),
        out,
    });
    // Lets out text nothing came after; ends with the last send of the turn.
    let ticking = Arc::downgrade(&pacing);
    let _ = std::thread::Builder::new()
        .name("chat-pace".to_owned())
        .spawn(move || {
            while let Some(pacing) = ticking.upgrade() {
                pacing.send(|pace| pace.due(Instant::now()).into_iter().collect());
                drop(pacing);
                std::thread::sleep(WINDOW / 2);
            }
        });
    Channel::new(move |body: InvokeResponseBody| {
        if let Ok(frame) = body.deserialize::<Frame>() {
            pacing.send(|pace| pace.took(frame, Instant::now()));
        }
        Ok(())
    })
}

#[cfg(test)]
#[path = "chat_pace_tests.rs"]
mod tests;
