//! Making a window, and ending a project's session whole.

use std::path::Path;

use crate::{Server, TmuxError};

impl Server {
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
        argv.extend(self.shell_args(window));
        if let Some(pipe) = pipe {
            let target = format!("{session}:{window}");
            argv.extend([";", "pipe-pane", "-t", &target, pipe].map(str::to_owned));
        }
        self.require(&argv.iter().map(String::as_str).collect::<Vec<_>>())?;
        self.ensure_client_session(session, window)?;
        Ok(())
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
