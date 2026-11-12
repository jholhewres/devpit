//! One connection to the daemon: what it has attached, and the queue its
//! frames go out on.
//!
//! The queue is bounded and sending never blocks a pane: a client that stops
//! reading is dropped, not waited on, so one stuck window cannot freeze the
//! terminals another one is watching.

use std::os::unix::net::UnixStream;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};

use crate::frame::{self, Frame};

/// Frames queued for one client before it counts as gone. The flow control
/// keeps output well under this; it is the backstop for a client that froze.
const QUEUED: usize = 4096;

pub struct Client {
    out: SyncSender<Frame>,
}

impl Client {
    /// A client writing to `stream` on a thread of its own.
    pub fn over(stream: UnixStream) -> std::sync::Arc<Client> {
        let (out, queue) = sync_channel(QUEUED);
        std::thread::spawn(move || drain(stream, queue));
        std::sync::Arc::new(Client { out })
    }

    /// A client whose frames land in `queue`, for tests.
    #[cfg(test)]
    pub(crate) fn queued() -> (std::sync::Arc<Client>, Receiver<Frame>) {
        let (out, queue) = sync_channel(QUEUED);
        (std::sync::Arc::new(Client { out }), queue)
    }

    /// Queues `frame`. False once the client is gone or has stopped reading.
    pub fn send(&self, frame: &Frame) -> bool {
        match self.out.try_send(frame.clone()) {
            Ok(()) => true,
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => false,
        }
    }
}

fn drain(mut stream: UnixStream, queue: Receiver<Frame>) {
    for one in queue {
        if frame::write(&mut stream, &one).is_err() {
            return;
        }
    }
}
