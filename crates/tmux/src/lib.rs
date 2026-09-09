//! A private tmux server that owns agent processes.
//!
//! Closing the window detaches the client. The session stays. That is the
//! whole reason this crate exists: writing a pty daemon is hundreds of files
//! and the same promise tmux already keeps.
//!
//! Each UI leaf is a tmux *window*, not a pane inside a split. The split tree
//! is drawn by the app (Orca's answer). A tmux split would show every leaf in
//! one client and fight the layout we persist.

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum TmuxError {
    #[error("tmux is not installed, or not on PATH")]
    Missing,

    #[error("tmux {command} failed: {stderr}")]
    Failed { command: String, stderr: String },
}

/// One tmux server, addressed by a socket we own.
pub struct Server {
    socket: PathBuf,
}

impl Server {
    pub fn new(socket: PathBuf) -> Self {
        Self { socket }
    }

    pub fn available() -> bool {
        Self::available_at(Path::new("tmux"))
    }

    fn available_at(program: &Path) -> bool {
        Command::new(program)
            .arg("-V")
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }

    /// Session names tmux will accept. Anything else is turned into `_`.
    pub fn session_name(project_id: &str) -> String {
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

    pub fn has_session(&self, session: &str) -> Result<bool, TmuxError> {
        let output = self.run(&["has-session", "-t", session])?;
        Ok(output.status.success())
    }

    /// Creates the session if needed, with status off, named window `window`.
    pub fn ensure_session(&self, session: &str, window: &str, cwd: &Path) -> Result<(), TmuxError> {
        if self.has_session(session)? {
            if !self
                .list_windows(session)?
                .iter()
                .any(|name| name == window)
            {
                self.new_window(session, window, cwd)?;
            }
            self.ensure_client_session(session, window)?;
            return Ok(());
        }

        if let Some(parent) = self.socket.parent() {
            std::fs::create_dir_all(parent).map_err(|err| TmuxError::Failed {
                command: "mkdir".to_owned(),
                stderr: err.to_string(),
            })?;
        }

        let cwd = cwd.to_string_lossy().into_owned();
        self.require(&["new-session", "-d", "-s", session, "-n", window, "-c", &cwd])?;
        self.quiet_chrome()?;
        self.ensure_client_session(session, window)?;
        Ok(())
    }

    pub fn new_window(&self, session: &str, window: &str, cwd: &Path) -> Result<(), TmuxError> {
        let cwd = cwd.to_string_lossy().into_owned();
        self.require(&["new-window", "-t", session, "-n", window, "-c", &cwd])?;
        self.ensure_client_session(session, window)?;
        Ok(())
    }

    /// Each visible leaf attaches to a grouped session with its own selected
    /// window. Grouped sessions share the processes but not the current-window
    /// pointer, so two xterms can display two project windows at once.
    fn ensure_client_session(&self, session: &str, window: &str) -> Result<(), TmuxError> {
        let client = Self::client_session(session, window);
        if !self.has_session(&client)? {
            self.require(&["new-session", "-d", "-t", session, "-s", &client])?;
        }
        self.require(&["select-window", "-t", &format!("{client}:{window}")])?;
        Ok(())
    }

    /// Everything tmux draws that this app draws better itself.
    ///
    /// Set on the server rather than per session, and that is the fix rather
    /// than a shortcut: a grouped session does not inherit another session's
    /// options, so setting them on the group left the client session — the one
    /// a pane actually attaches to — with a green tmux bar along the bottom.
    /// The server is ours, on our own socket, so a global here reaches every
    /// session including the ones made later.
    fn quiet_chrome(&self) -> Result<(), TmuxError> {
        for option in [
            // A status bar inside a pane we already chrome is noise, and it
            // steals a row from the agent's TUI.
            ["status", "off"],
            // Otherwise a shell's title escape renames the window under us, and
            // the layout is keyed by window name.
            ["allow-rename", "off"],
            ["automatic-rename", "off"],
            // A message that hangs around covers the last line of output.
            ["display-time", "1500"],
            // The default half-second swallows an Escape meant for the program
            // inside, which is most of them.
            ["escape-time", "10"],
        ] {
            let _ = self.require(&["set-option", "-g", option[0], option[1]]);
        }
        Ok(())
    }

    pub fn list_windows(&self, session: &str) -> Result<Vec<String>, TmuxError> {
        let output = self.require(&["list-windows", "-t", session, "-F", "#{window_name}"])?;
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect())
    }

    pub fn target(session: &str, window: &str) -> String {
        format!("{}:{window}", Self::client_session(session, window))
    }

    fn client_session(session: &str, window: &str) -> String {
        let mut name = format!("{session}__{window}");
        name.truncate(160);
        name
    }

    /// The argv a pty should spawn to attach as a client of this window.
    pub fn attach_argv(&self, session: &str, window: &str) -> Vec<String> {
        vec![
            "tmux".to_owned(),
            "-S".to_owned(),
            self.socket.display().to_string(),
            "attach-session".to_owned(),
            "-t".to_owned(),
            Self::target(session, window),
        ]
    }

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
        let client = Self::client_session(session, window);
        if self.has_session(&client)? {
            let _ = self.run(&["kill-session", "-t", &client])?;
        }
        let _ = self.run(&["kill-window", "-t", &format!("{session}:{window}")])?;
        Ok(())
    }

    pub fn send_keys(&self, target: &str, keys: &str) -> Result<(), TmuxError> {
        self.require(&["send-keys", "-t", target, keys, "Enter"])?;
        Ok(())
    }

    pub fn capture_pane(&self, target: &str) -> Result<String, TmuxError> {
        let output = self.require(&["capture-pane", "-t", target, "-p"])?;
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    pub fn kill_server(&self) -> Result<(), TmuxError> {
        let _ = self.run(&["kill-server"])?;
        Ok(())
    }

    fn require(&self, args: &[&str]) -> Result<std::process::Output, TmuxError> {
        let output = self.run(args)?;
        if output.status.success() {
            return Ok(output);
        }
        Err(TmuxError::Failed {
            command: args.join(" "),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        })
    }

    fn run(&self, args: &[&str]) -> Result<std::process::Output, TmuxError> {
        Command::new("tmux")
            .arg("-S")
            .arg(&self.socket)
            .args(args)
            .output()
            .map_err(|err| match err.kind() {
                std::io::ErrorKind::NotFound => TmuxError::Missing,
                _ => TmuxError::Failed {
                    command: args.join(" "),
                    stderr: err.to_string(),
                },
            })
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
