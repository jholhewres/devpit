//! One attached client: the ends of its pty, and what it has printed.
//!
//! Apart from [`crate::claims`] because they answer different questions. This
//! is what a client *is*; that is who holds a pane. The ownership rule never
//! touches these handles — it only ever compares the id beside them — and
//! keeping them here is what makes that readable.

use std::io::Write;
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
                let mut size = size.lock().map_err(|_| RpcError::internal("size lock"))?;
                *size = wanted;
                Ok(wanted)
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
