//! One attached client: the ends of its pty, and what it has printed.
//!
//! Apart from [`crate::claims`] because they answer different questions. This
//! is what a client *is*; that is who holds a pane. The ownership rule never
//! touches these handles — it only ever compares the id beside them — and
//! keeping them here is what makes that readable.

use std::io::Write;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use devpit_rpc::RpcError;
use portable_pty::{ChildKiller, MasterPty, PtySize};

pub(crate) struct Live {
    pub(crate) client_id: String,
    /// The webview whose page attached it: a reload of that page ends it.
    pub(crate) webview: String,
    pub(crate) ends: Ends,
    /// The scrollback this pane has produced, shared with the reader thread.
    ///
    /// Held here and not only inside the pty session so `session.scrollback`
    /// can answer while the frames are still draining — the drain owns the
    /// receiver, and a history nobody can ask for is a history nobody has.
    pub(crate) ring: Arc<Mutex<devpit_pty::RingBuffer>>,
    /// Output sent to a page that acknowledges it and not yet acknowledged.
    pub(crate) flow: Flow,
    /// How much of the daemon's history this attach holds: its replay and
    /// every output and older page since. What a page's next page is before.
    pub(crate) held: AtomicU64,
}

/// What an attach hands on to the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Out {
    /// Printed now.
    Output,
    /// Printed before the attach, sent ahead of the first live byte.
    Replay,
    /// A page of older history: `left u64`, then the bytes.
    Older,
}

/// Where an attach's bytes go: Tauri's channel, or a frame on the panes socket.
pub(crate) trait Sink: Send + Sync {
    /// False once nobody is there to take them.
    fn send(&self, out: Out, bytes: Vec<u8>) -> bool;
    /// Whether the page acknowledges what it parsed. One that does holds the
    /// pty back past [`Flow::MOST`]; one that does not is acknowledged as sent.
    fn acks(&self) -> bool;
}

/// A `tmux attach`'s frames into `sink` until either ends. A page that
/// acknowledges holds the pty back past the window: the bounded frames fill,
/// and the reader stops reading until it catches up.
pub(crate) async fn pump(
    frames: &mut tokio::sync::mpsc::Receiver<Vec<u8>>,
    live: &Live,
    sink: &dyn Sink,
) {
    let acks = sink.acks();
    while let Some(frame) = frames.recv().await {
        if acks {
            live.flow.sent(frame.len());
        }
        if !sink.send(Out::Output, frame) {
            break;
        }
        if acks {
            live.flow.room().await;
        }
    }
}

/// Bytes a page has not acknowledged yet, and a wait for room under the window.
#[derive(Default)]
pub(crate) struct Flow {
    unacked: AtomicUsize,
    acked: tokio::sync::Notify,
}

impl Flow {
    /// Past this, the pty is not read until the page catches up.
    pub(crate) const MOST: usize = 1024 * 1024;

    pub(crate) fn sent(&self, bytes: usize) {
        self.unacked.fetch_add(bytes, Ordering::AcqRel);
    }

    pub(crate) fn ack(&self, bytes: usize) {
        let _ = self
            .unacked
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |was| {
                Some(was.saturating_sub(bytes))
            });
        self.acked.notify_waiters();
    }

    /// Nobody left to acknowledge: nothing holds the pty.
    pub(crate) fn release(&self) {
        self.unacked.store(0, Ordering::Release);
        self.acked.notify_waiters();
    }

    /// Returns once what is unacknowledged is within the window.
    pub(crate) async fn room(&self) {
        loop {
            let acked = self.acked.notified();
            tokio::pin!(acked);
            acked.as_mut().enable();
            if self.unacked.load(Ordering::Acquire) <= Self::MOST {
                return;
            }
            acked.await;
        }
    }
}

/// What a client writes to and resizes: a `tmux attach` on a pty of the
/// desktop's own, or a connection to the pane devpit-ptyd holds.
pub(crate) enum Ends {
    Pty {
        writer: Mutex<Box<dyn Write + Send>>,
        master: Mutex<Box<dyn MasterPty + Send>>,
        killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
        /// What to insist to, when asking politely does not work.
        pid: Option<u32>,
    },
    #[cfg(unix)]
    Ptyd {
        pane: u32,
        /// The connection's writing half; the drain reads the other.
        stream: Mutex<std::os::unix::net::UnixStream>,
        size: Mutex<PtySize>,
    },
}

impl Live {
    /// Bytes into the pane.
    pub(crate) fn write(&self, bytes: &[u8]) -> Result<(), RpcError> {
        match &self.ends {
            Ends::Pty { writer, .. } => {
                let mut writer = writer
                    .lock()
                    .map_err(|_| RpcError::internal("writer lock"))?;
                writer
                    .write_all(bytes)
                    .and_then(|_| writer.flush())
                    .map_err(|err| RpcError::internal(err.to_string()))
            }
            #[cfg(unix)]
            Ends::Ptyd { pane, stream, .. } => {
                use devpit_ptyd::frame::{self, Frame, Kind};
                let mut stream = stream
                    .lock()
                    .map_err(|_| RpcError::internal("ptyd stream lock"))?;
                frame::write(
                    &mut *stream,
                    &Frame::new(*pane, Kind::Input, bytes.to_vec()),
                )
                .map_err(|err| RpcError::internal(err.to_string()))
            }
        }
    }

    /// Resizes the pane, and answers the size it is at afterwards.
    pub(crate) fn resize(&self, rows: u16, cols: u16) -> Result<PtySize, RpcError> {
        let wanted = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        match &self.ends {
            Ends::Pty { master, .. } => {
                let master = master
                    .lock()
                    .map_err(|_| RpcError::internal("master lock"))?;
                master
                    .resize(wanted)
                    .map_err(|err| RpcError::internal(err.to_string()))?;
                // Asked back rather than echoed: the kernel is what decides,
                // and it is the only thing that can be asked honestly.
                master
                    .get_size()
                    .map_err(|err| RpcError::internal(err.to_string()))
            }
            #[cfg(unix)]
            Ends::Ptyd { pane, stream, size } => {
                use devpit_ptyd::frame::{self, Frame, Kind};
                let mut bytes = rows.to_le_bytes().to_vec();
                bytes.extend_from_slice(&cols.to_le_bytes());
                let mut stream = stream
                    .lock()
                    .map_err(|_| RpcError::internal("ptyd stream lock"))?;
                frame::write(&mut *stream, &Frame::new(*pane, Kind::Resize, bytes))
                    .map_err(|err| RpcError::internal(err.to_string()))?;
                crate::ptyd_tap::tap().sized(*pane, rows, cols);
                let mut size = size.lock().map_err(|_| RpcError::internal("size lock"))?;
                *size = wanted;
                Ok(wanted)
            }
        }
    }

    /// The size a ptyd pane was last told, by this window.
    pub(crate) fn ptyd_size(&self) -> Option<PtySize> {
        match &self.ends {
            Ends::Pty { .. } => None,
            #[cfg(unix)]
            Ends::Ptyd { size, .. } => size.lock().ok().map(|size| *size),
        }
    }

    /// Bytes the page parsed: room for the pty to read on.
    pub(crate) fn ack(&self, bytes: u32) {
        match &self.ends {
            Ends::Pty { .. } => self.flow.ack(bytes as usize),
            #[cfg(unix)]
            Ends::Ptyd { pane, stream, .. } => {
                use devpit_ptyd::frame::{self, Frame, Kind};
                if let Ok(mut stream) = stream.lock() {
                    let ack = Frame::new(*pane, Kind::Ack, bytes.to_le_bytes().to_vec());
                    let _ = frame::write(&mut *stream, &ack);
                }
            }
        }
    }

    /// Asks the daemon for the page before what this attach holds; the
    /// answer comes back on the attach's own stream. False where there is no
    /// history to page: a `tmux attach` redraws, it does not replay.
    pub(crate) fn ask_older(&self) -> bool {
        match &self.ends {
            Ends::Pty { .. } => false,
            #[cfg(unix)]
            Ends::Ptyd { pane, stream, .. } => {
                use devpit_ptyd::frame::{self, Frame, Kind};
                let have = self.held.load(Ordering::Acquire).to_le_bytes().to_vec();
                stream.lock().is_ok_and(|mut stream| {
                    frame::write(&mut *stream, &Frame::new(*pane, Kind::History, have)).is_ok()
                })
            }
        }
    }

    /// Ends this client, without waiting forever to be obeyed.
    ///
    /// `kill` on a pty child is `SIGHUP`, which is a request. A closed tab
    /// must not be able to hang the window because whatever was inside it
    /// decided to take its time.
    ///
    /// For a ptyd pane this is the connection only: the shell is the
    /// daemon's, and lives on until the pane is closed.
    pub(crate) fn stop(&self) -> Result<(), RpcError> {
        self.flow.release();
        match &self.ends {
            Ends::Pty { killer, pid, .. } => {
                let mut killer = killer
                    .lock()
                    .map_err(|_| RpcError::internal("killer lock"))?;
                devpit_pty::stop(*pid, devpit_pty::GRACE, || {
                    let _ = killer.kill();
                });
            }
            #[cfg(unix)]
            Ends::Ptyd { stream, .. } => {
                if let Ok(stream) = stream.lock() {
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                }
            }
        }
        Ok(())
    }
}
