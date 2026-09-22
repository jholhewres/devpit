//! The environment a running server hands to every window it makes.
//!
//! A server takes its global environment from whoever started it, once. When
//! that was an AppImage devpit, every window since — and every window after an
//! update — starts with the AppImage's libraries and a mount that has moved
//! (see `devpit_pty::host_env`). Starting the server clean is not enough for a
//! server that is already running, so its global environment is cleaned too,
//! the first time this process reaches it.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use devpit_pty::host_env::{cleaned, mount_prefix, Change};

use crate::{Server, TmuxError};

/// Once per server per process: a window opened later asks nothing again.
pub(crate) fn clean_once(server: &Server) -> Result<(), TmuxError> {
    static DONE: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    let done = DONE.get_or_init(|| Mutex::new(HashSet::new()));
    if done
        .lock()
        .map(|seen| seen.contains(&server.socket))
        .unwrap_or(false)
    {
        return Ok(());
    }
    let listed = server.require(&["show-environment", "-g"])?;
    let changes = cleaned(
        global(&String::from_utf8_lossy(&listed.stdout)),
        &mount_prefix(),
    );
    if !changes.is_empty() {
        server.require(
            &one_line(&changes)
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        )?;
    }
    // A server started before devpit declared it also learns the client
    // draws true colour (`chrome::TRUE_COLOUR`).
    let _ = server.run(&crate::chrome::TRUE_COLOUR);
    if let Ok(mut seen) = done.lock() {
        seen.insert(server.socket.clone());
    }
    Ok(())
}

/// `show-environment -g` output as pairs. A line starting with `-` is a
/// variable marked for removal, which has no value to clean.
pub(crate) fn global(listed: &str) -> Vec<(String, String)> {
    listed
        .lines()
        .filter(|line| !line.starts_with('-'))
        .filter_map(|line| line.split_once('='))
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect()
}

/// Every change as one tmux invocation, commands separated by a bare `;`.
pub(crate) fn one_line(changes: &[Change]) -> Vec<String> {
    let mut argv = Vec::new();
    for (at, (name, value)) in changes.iter().enumerate() {
        if at > 0 {
            argv.push(";".to_owned());
        }
        argv.extend(["set-environment".to_owned(), "-g".to_owned()]);
        match value {
            Some(value) => argv.extend([name.clone(), value.clone()]),
            None => argv.extend(["-u".to_owned(), name.clone()]),
        }
    }
    argv
}

#[cfg(test)]
#[path = "environment_tests.rs"]
mod tests;
