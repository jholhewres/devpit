//! Turning a pty's reads into frames for the screen.
//!
//! Its own thread, apart from the one that reads. Coalescing on the reading
//! thread could only flush once a `read` came back, and a `read` on a quiet
//! pty does not come back: the last few milliseconds of a burst — the prompt
//! a program draws before it goes idle — sat unsent until the program said
//! something else. The screen showed a spinner frozen mid-count while the
//! program behind it had long finished.

use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;

use crate::{Counters, FRAME};

/// Hands `chunks` on as frames: at most one per [`FRAME`], and never holding
/// a byte for longer than one.
///
/// Returns when the reader is gone and everything it read is handed on, or
/// when nobody is listening for frames any more.
pub(crate) fn run(
    chunks: Receiver<Vec<u8>>,
    frames: mpsc::Sender<Vec<u8>>,
    counters: Arc<Counters>,
) {
    let mut pending: Vec<u8> = Vec::new();
    // Long enough ago that the first chunk goes out as it arrives: a keystroke's
    // echo after a pause is the one that must not wait.
    let mut last_flush = Instant::now()
        .checked_sub(FRAME)
        .unwrap_or_else(Instant::now);

    loop {
        let heard = if pending.is_empty() {
            chunks.recv().map_err(|_| RecvTimeoutError::Disconnected)
        } else {
            chunks.recv_timeout(FRAME.saturating_sub(last_flush.elapsed()))
        };
        match heard {
            Ok(chunk) => pending.extend_from_slice(&chunk),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        if !pending.is_empty() && last_flush.elapsed() >= FRAME {
            counters.frames.fetch_add(1, Ordering::Relaxed);
            if frames.blocking_send(std::mem::take(&mut pending)).is_err() {
                return;
            }
            last_flush = Instant::now();
        }
    }

    if !pending.is_empty() {
        counters.frames.fetch_add(1, Ordering::Relaxed);
        let _ = frames.blocking_send(pending);
    }
}

#[cfg(test)]
#[path = "coalesce_tests.rs"]
mod tests;
