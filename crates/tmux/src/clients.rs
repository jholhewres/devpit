//! The client sessions panes attach through, and the ones left behind.
//!
//! Each pane attaches through a session of its own, grouped with the
//! project's (`<session>__<window>`), so two panes can show two windows at
//! once. Closing a pane from the app kills both. A window that ends any other
//! way — its shell exits, somebody kills it in tmux — leaves its client
//! session behind, detached, showing whatever window the group falls back to.
//! Those accumulate and read as terminals nobody opened.
//!
//! So whenever a project's session is reached, a client session that is
//! detached and whose window is gone is removed. An attached one is left
//! alone: something is looking through it.

use std::collections::HashSet;

use crate::{naming, Server};

/// Removes the orphaned client sessions of `session`, given its windows.
/// Best effort: a tidy-up that fails leaves things as they were.
pub(crate) fn prune(server: &Server, session: &str, windows: &[String]) {
    let Ok(listed) = server.run(&["list-sessions", "-F", "#{session_name} #{session_attached}"])
    else {
        return;
    };
    for name in orphans(&String::from_utf8_lossy(&listed.stdout), session, windows) {
        let _ = server.run(&["kill-session", "-t", &name]);
    }
}

/// The client sessions in a `list-sessions` listing that have lost their
/// window and nothing is attached to.
pub(crate) fn orphans(listed: &str, session: &str, windows: &[String]) -> Vec<String> {
    let prefix = format!("{session}__");
    let wanted: HashSet<String> = windows
        .iter()
        .map(|window| naming::client_session(session, window))
        .collect();
    listed
        .lines()
        .filter_map(|line| {
            let (name, attached) = line.rsplit_once(' ')?;
            (name.starts_with(&prefix) && attached == "0" && !wanted.contains(name))
                .then(|| name.to_owned())
        })
        .collect()
}

#[cfg(test)]
#[path = "clients_tests.rs"]
mod tests;
