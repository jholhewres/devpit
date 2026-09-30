//! What a session, a window and a leaf are called.
//!
//! Addressing, kept apart from operating: every tmux command is pointed at a
//! `session:window`, and the rules for building one — which characters tmux
//! accepts, how long a name may be, which session a client attaches to — are
//! a subject of their own.

/// Session names tmux will accept. Anything else is turned into `_`.
pub(crate) fn session_name(project_id: &str) -> String {
    let mut name = String::from("devpit_");
    for ch in project_id.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            name.push(ch);
        } else {
            name.push('_');
        }
    }
    name.truncate(80);
    name
}

pub(crate) fn target(session: &str, window: &str) -> String {
    format!("{}:{window}", client_session(session, window))
}

/// Whether a pane attaches through a session grouped with the project's.
///
/// psmux, the tmux of Windows, has no grouped sessions: a pane is attached
/// to the project's own session there, at its window.
pub(crate) const GROUPED: bool = !cfg!(windows);

pub(crate) fn client_session(session: &str, window: &str) -> String {
    if !GROUPED {
        return session.to_owned();
    }
    let mut name = format!("{session}__{window}");
    name.truncate(160);
    name
}

/// The argv a pty spawns to become a client of one window.
///
/// Here beside the other name-shaping because that is all it is: a socket
/// path and a `session:window` target, written out in the order tmux reads
/// them.
pub(crate) fn attach_argv(socket: &std::path::Path, session: &str, window: &str) -> Vec<String> {
    vec![
        program(),
        "-S".to_owned(),
        socket.display().to_string(),
        "attach-session".to_owned(),
        "-t".to_owned(),
        target(session, window),
    ]
}

/// The tmux to run, as a path where the name alone is not enough.
///
/// On Windows a pty is started with the environment Windows keeps for the
/// user, not this process's: the psmux devpit puts first on its own `PATH`
/// is not on that one. So it is found here, on this process's `PATH`, and
/// named whole.
pub fn program() -> String {
    #[cfg(windows)]
    {
        let path = std::env::var_os("PATH").unwrap_or_default();
        if let Some(found) = std::env::split_paths(&path)
            .map(|dir| dir.join("tmux.exe"))
            .find(|candidate| candidate.is_file())
        {
            return found.display().to_string();
        }
    }
    "tmux".to_owned()
}
