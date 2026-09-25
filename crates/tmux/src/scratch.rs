//! A tmux server that goes when it is dropped: for tests, whose servers used
//! to outlive them — one left per run, on a socket in a deleted temp folder,
//! until somebody noticed twenty of them eating memory.

use std::ops::Deref;
use std::path::PathBuf;

use crate::Server;

/// A [`Server`] killed on drop, whatever the test did — passed, failed or
/// panicked half way.
pub struct Scratch(Server);

impl Server {
    /// A server for a test, on `socket`, taken down when it goes out of scope.
    pub fn scratch(socket: impl Into<PathBuf>) -> Scratch {
        Scratch(Server::new(socket.into()))
    }
}

impl Deref for Scratch {
    type Target = Server;

    fn deref(&self) -> &Server {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // Nothing to take down is not a failure: the test may have ended it.
        let _ = self.0.kill_server();
    }
}
