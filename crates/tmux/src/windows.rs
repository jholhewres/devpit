//! Making a window, and ending a project's session whole.

use std::path::Path;

use crate::{naming, Server, TmuxError};

impl Server {
    /// Kills a window and the client session that was pointed at it.
    ///
    /// Both, because `ensure_client_session` makes one grouped session per
    /// window: killing only the window leaves a session with nothing selected,
    /// and tmux keeps those around forever on a socket nobody else uses.
    ///
    /// A window that is already gone is not an error. Closing a pane whose
    /// shell exited a moment earlier is an ordinary thing to do, and a refusal
    /// would leave the leaf in the tree with no way to remove it.
    pub fn kill_window(&self, session: &str, window: &str) -> Result<(), TmuxError> {
        let client = naming::client_session(session, window);
        // Ungrouped, the client is the project's own session: never killed here.
        if naming::GROUPED && self.has_session(&client)? {
            let _ = self.run(&["kill-session", "-t", &client])?;
        }
        let _ = self.run(&["kill-window", "-t", &format!("{session}:{window}")])?;
        if !naming::GROUPED {
            self.end_when_empty(session);
        }
        Ok(())
    }

    /// psmux keeps its server, and the files under it, running with nothing
    /// in it — tmux leaves once its last session goes. With no terminal left
    /// there is nothing to come back to, so an empty server is ended here, and
    /// installing over devpit is not refused by a `tmux.exe` still running.
    fn end_when_empty(&self, session: &str) {
        let windows = self.list_windows(session).unwrap_or_default();
        if windows.is_empty() {
            let _ = self.run(&["kill-session", "-t", session]);
        }
        let left = self
            .run(&["list-sessions", "-F", "#{session_name}"])
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().is_empty())
            .unwrap_or(true);
        if left {
            let _ = self.run(&["kill-server"]);
        }
    }

    pub fn new_window(&self, session: &str, window: &str, cwd: &Path) -> Result<(), TmuxError> {
        self.open_window(session, window, cwd, None)
    }

    /// A window whose output is piped to `pipe` from its first byte.
    ///
    /// One tmux invocation, so the pipe is armed before the shell prints: a
    /// pipe armed after it missed the first prompt, and with it the folder
    /// the shell announced.
    pub fn new_window_piped(
        &self,
        session: &str,
        window: &str,
        cwd: &Path,
        pipe: &str,
    ) -> Result<(), TmuxError> {
        self.open_window(session, window, cwd, Some(pipe))
    }

    fn open_window(
        &self,
        session: &str,
        window: &str,
        cwd: &Path,
        pipe: Option<&str>,
    ) -> Result<(), TmuxError> {
        let cwd = cwd.to_string_lossy().into_owned();
        let mut argv: Vec<String> = ["new-window", "-t", session, "-n", window, "-c", &cwd]
            .iter()
            .map(|part| (*part).to_owned())
            .collect();
        argv.extend(cwd_env(&cwd));
        argv.extend(self.shell_args(window));
        let target = format!("{session}:{window}");
        let chained = crate::naming::GROUPED;
        if let (Some(pipe), true) = (pipe, chained) {
            argv.extend([";", "pipe-pane", "-t", &target, pipe].map(str::to_owned));
        }
        self.require(&argv.iter().map(String::as_str).collect::<Vec<_>>())?;
        // psmux takes one command per invocation; its shell is slow enough to
        // start that the pipe, armed right after, still hears the first prompt.
        if let (Some(pipe), false) = (pipe, chained) {
            self.require(&["pipe-pane", "-t", &target, pipe])?;
        }
        self.ensure_client_session(session, window)?;
        Ok(())
    }

    /// The session a window belongs to on this server — the project's, not
    /// a client's grouped on it — or nothing when no session holds it.
    pub fn session_of_window(&self, window: &str) -> Result<Option<String>, TmuxError> {
        let output =
            self.require(&["list-windows", "-a", "-F", "#{session_name} #{window_name}"])?;
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .find_map(|line| {
                let (session, name) = line.trim().split_once(' ')?;
                (name == window && !session.contains("__")).then(|| session.to_owned())
            }))
    }

    /// The window each pane on this server is in, by the pane's id (`%3`).
    ///
    /// A pane's id is the server's own and names one pane; a session's name
    /// does not, since every client session of a group shows every window.
    pub fn pane_windows(&self) -> Result<std::collections::HashMap<String, String>, TmuxError> {
        let output = self.require(&["list-panes", "-a", "-F", "#{pane_id} #{window_name}"])?;
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let (pane, window) = line.trim().split_once(' ')?;
                Some((pane.to_owned(), window.to_owned()))
            })
            .collect())
    }

    /// Every window of every session on this server, by name.
    pub fn all_windows(&self) -> Result<Vec<String>, TmuxError> {
        let output = self.require(&["list-windows", "-a", "-F", "#{window_name}"])?;
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect())
    }

    /// Ends a project's session and every client session grouped with it.
    ///
    /// All of them: the windows belong to the group, so killing only the
    /// project's own session left them running under the clients.
    pub fn kill_session(&self, session: &str) -> Result<(), TmuxError> {
        let listed = self.run(&["list-sessions", "-F", "#{session_name}"])?;
        let clients = format!("{session}__");
        for name in String::from_utf8_lossy(&listed.stdout).lines() {
            if name == session || name.starts_with(&clients) {
                // `=`: exactly this name, never one it happens to prefix.
                let _ = self.run(&["kill-session", "-t", &format!("={name}")]);
            }
        }
        Ok(())
    }
}

/// The folder a Windows shell is to stand in, handed over as a variable too:
/// a window made after the first came up in the person's home, and PowerShell
/// profiles often `cd` there themselves. Its startup file goes back to it.
pub(crate) fn cwd_env(cwd: &str) -> Vec<String> {
    if crate::naming::GROUPED {
        return Vec::new();
    }
    vec!["-e".to_owned(), format!("DEVPIT_CWD={cwd}")]
}
