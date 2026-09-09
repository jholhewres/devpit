//! The thread that reads a pty, and what it makes of what it reads.
//!
//! Blocking, and deliberately: reading a pty fd blocks, and blocking inside
//! the async runtime starves every other task on that worker.

use std::io::Read;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use crate::{Counters, RingBuffer, BETWEEN_READS, FRAME};

/// What a failed read means for the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterRead {
    /// The pty had nothing to say. Wait a moment and read again.
    Again,
    /// The process is gone.
    Ended,
    /// Something else went wrong, and it is worth naming on the way out.
    Broken,
}

/// Reads a failed `read` the way the kernel meant it.
///
/// Treating every `Err` as the end closes a terminal that is still working:
/// a signal arriving mid-syscall is `Interrupted`, and a pty with nothing
/// buffered is `WouldBlock`. Neither is a process that died.
///
/// `EIO` is the one that needs the second argument. A pty master answers it
/// for two different reasons — once in the gap between the fork and the child
/// attaching its slave, and once for good when the child is gone — and the
/// only thing that tells them apart is whether the child has actually exited.
/// Guessing wrong the first way closes every session that loses that race at
/// startup, which reads as a terminal that sometimes just shuts.
pub fn after_read(error: &std::io::Error, child_has_exited: bool) -> AfterRead {
    use std::io::ErrorKind;
    match error.kind() {
        ErrorKind::Interrupted | ErrorKind::WouldBlock | ErrorKind::TimedOut => AfterRead::Again,
        // By number rather than by `ErrorKind`: std maps this to `Uncategorized`,
        // which is not a name a match arm can use.
        _ if error.raw_os_error() == Some(EIO) => {
            if child_has_exited {
                AfterRead::Ended
            } else {
                AfterRead::Again
            }
        }
        _ => AfterRead::Broken,
    }
}

/// `EIO`. The same value on every platform this builds for, and named here so
/// the match above reads as the errno it is rather than as a bare 5.
const EIO: i32 = 5;

/// Starts the thread that drains `reader` into frames, scrollback and
/// whatever the terminal said about itself along the way.
pub(crate) fn start(
    mut reader: Box<dyn Read + Send>,
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
    counters: Arc<Counters>,
    ring: Arc<Mutex<RingBuffer>>,
    tx: mpsc::Sender<Vec<u8>>,
) {
    // A blocking thread, not a tokio task: reading a pty fd blocks, and
    // blocking inside the runtime starves every other task on that worker.
    std::thread::spawn(move || {
        let mut buf = [0u8; 64 * 1024];
        let mut pending: Vec<u8> = Vec::with_capacity(64 * 1024);
        let mut last_flush = std::time::Instant::now();

        loop {
            let mut had_nothing = false;
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = &buf[..n];
                    counters.bytes.fetch_add(n as u64, Ordering::Relaxed);
                    if let Ok(mut ring) = ring.lock() {
                        ring.write(chunk);
                    }
                    pending.extend_from_slice(chunk);
                }
                Err(error) => {
                    let exited = matches!(child.try_wait(), Ok(Some(_)));
                    match after_read(&error, exited) {
                        AfterRead::Again => had_nothing = true,
                        AfterRead::Ended => break,
                        AfterRead::Broken => {
                            eprintln!("the pty stopped being readable: {error}");
                            break;
                        }
                    }
                }
            }

            // Coalesce: hand over a frame's worth at a time rather than every
            // read. Under a flood this turns tens of thousands of tiny
            // messages into about sixty per second.
            //
            // Reached on a retry too, not only after a successful read: bytes
            // that arrived just before a transient error would otherwise sit
            // in `pending` for as long as the pty stayed quiet.
            if last_flush.elapsed() >= FRAME && !pending.is_empty() {
                counters.frames.fetch_add(1, Ordering::Relaxed);
                if tx.blocking_send(std::mem::take(&mut pending)).is_err() {
                    break;
                }
                pending = Vec::with_capacity(64 * 1024);
                last_flush = std::time::Instant::now();
            }

            if had_nothing {
                std::thread::sleep(BETWEEN_READS);
            }
        }

        if !pending.is_empty() {
            counters.frames.fetch_add(1, Ordering::Relaxed);
            let _ = tx.blocking_send(pending);
        }

        let _ = child.wait();
    });
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
