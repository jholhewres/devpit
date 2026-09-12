//! What one pane does, as opposed to how sessions are arranged.

use crate::{Server, TmuxError};

/// Sends a copy of everything a pane prints to `command`'s stdin.
///
/// The point is that it does not need a client: tmux keeps piping with
/// nobody attached, which is what lets a build be watched in a tab you are
/// not looking at. `-o` makes it a toggle, so arming a pane that is
/// already piped is not a second pipe.
///
/// Measured before relying on it: a reader that dies leaves the pane
/// running, and a reader that stalls does not stall the pane — 200,000
/// lines with nobody draining, and the pane still answered.
pub(crate) fn pipe_pane(server: &Server, target: &str, command: &str) -> Result<(), TmuxError> {
    server.require(&["pipe-pane", "-o", "-t", target, command])?;
    Ok(())
}

/// Stops the pipe. A pane that has none is not an error.
pub(crate) fn unpipe(server: &Server, target: &str) -> Result<(), TmuxError> {
    let _ = server.run(&["pipe-pane", "-t", target]);
    Ok(())
}

pub(crate) fn send_keys(server: &Server, target: &str, keys: &str) -> Result<(), TmuxError> {
    server.require(&["send-keys", "-t", target, keys, "Enter"])?;
    Ok(())
}

pub(crate) fn capture_pane(server: &Server, target: &str) -> Result<String, TmuxError> {
    let output = server.require(&["capture-pane", "-t", target, "-p"])?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
