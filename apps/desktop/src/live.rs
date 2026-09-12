//! One attached client: the ends of its pty, and what it has printed.
//!
//! Apart from [`crate::claims`] because they answer different questions. This
//! is what a client *is*; that is who holds a pane. The ownership rule never
//! touches these handles — it only ever compares the id beside them — and
//! keeping them here is what makes that readable.

use std::io::Write;
use std::sync::{Arc, Mutex};

use devpit_rpc::RpcError;
use portable_pty::{ChildKiller, MasterPty};

pub(crate) struct Live {
    pub(crate) client_id: String,
    pub(crate) writer: Mutex<Box<dyn Write + Send>>,
    pub(crate) master: Mutex<Box<dyn MasterPty + Send>>,
    pub(crate) killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    /// The scrollback this pane has produced, shared with the reader thread.
    ///
    /// Held here and not only inside the pty session so `session.scrollback`
    /// can answer while the frames are still draining — the drain owns the
    /// receiver, and a history nobody can ask for is a history nobody has.
    pub(crate) ring: Arc<Mutex<devpit_pty::RingBuffer>>,
    /// What to insist to, when asking politely does not work.
    pub(crate) pid: Option<u32>,
}

impl Live {
    /// Ends this client, without waiting forever to be obeyed.
    ///
    /// `kill` on a pty child is `SIGHUP`, which is a request. A closed tab
    /// must not be able to hang the window because whatever was inside it
    /// decided to take its time.
    pub(crate) fn stop(&self) -> Result<(), RpcError> {
        let mut killer = self
            .killer
            .lock()
            .map_err(|_| RpcError::internal("killer lock"))?;
        devpit_pty::stop(self.pid, devpit_pty::GRACE, || {
            let _ = killer.kill();
        });
        Ok(())
    }
}
