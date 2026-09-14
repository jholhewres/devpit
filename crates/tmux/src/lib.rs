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

mod chrome;
mod naming;
mod pane;
mod running;
mod shell;
pub use shell::{parse_running, Running, Shell};

#[derive(Debug, thiserror::Error)]
pub enum TmuxError {
    #[error("tmux is not installed, or not on PATH")]
    Missing,

    #[error("tmux {command} failed: {stderr}")]
    Failed { command: String, stderr: String },
}

/// One tmux server, addressed by a socket we own.
pub struct Server {
    pub(crate) socket: PathBuf,
    shell: Option<Shell>,
}

impl Server {
    /// Session names tmux will accept. Anything else is turned into `_`.
    pub fn session_name(project_id: &str) -> String {
        naming::session_name(project_id)
    }

    /// `session:window` — how every tmux command is pointed at a leaf.
    pub fn target(session: &str, window: &str) -> String {
        naming::target(session, window)
    }

    pub fn new(socket: PathBuf) -> Self {
        Self {
            socket,
            shell: None,
        }
    }

    /// Whether tmux is on this machine.
    ///
    /// Asked once and remembered. It is asked on every command that touches a
    /// session — opening a terminal asks it four times, and the sidebar's
    /// two-second poll asks it again — and tmux does not get uninstalled
    /// while the window is open.
    pub fn available() -> bool {
        static FOUND: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *FOUND.get_or_init(|| Self::available_at(Path::new("tmux")))
    }

    fn available_at(program: &Path) -> bool {
        Command::new(program)
            .arg("-V")
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }

    pub fn has_session(&self, session: &str) -> Result<bool, TmuxError> {
        let output = self.run(&["has-session", "-t", session])?;
        Ok(output.status.success())
    }

    /// How the shell in a new window is started.
    ///
    /// tmux runs the login shell with no arguments by default, and a shell
    /// started that way says nothing about itself — no prompt boundary, no
    /// exit code, nothing for the OSC scanner to read. The caller hands the
    /// program, its arguments and the environment that turns the markers on.
    pub fn with_shell(mut self, shell: Shell) -> Self {
        self.shell = Some(shell);
        self
    }

    /// The trailing `-e KEY=VAL … -- program args…` a new window needs.
    ///
    /// `window` is the leaf the window is for, handed to whatever runs inside
    /// it as `DEVPIT_PANE`. That is the only thing joining an agent's own
    /// reports back to the pane a person is looking at: a hook fires in a
    /// process three levels below the shell, and the environment is what
    /// reaches down there.
    fn shell_args(&self, window: &str) -> Vec<String> {
        shell::window_args(self.shell.as_ref(), window)
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
        let mut argv: Vec<String> = ["new-session", "-d", "-s", session, "-n", window, "-c", &cwd]
            .iter()
            .map(|part| (*part).to_owned())
            .collect();
        argv.extend(self.shell_args(window));
        self.require(&argv.iter().map(String::as_str).collect::<Vec<_>>())?;
        self.quiet_chrome()?;
        self.ensure_client_session(session, window)?;
        Ok(())
    }

    pub fn new_window(&self, session: &str, window: &str, cwd: &Path) -> Result<(), TmuxError> {
        let cwd = cwd.to_string_lossy().into_owned();
        let mut argv: Vec<String> = ["new-window", "-t", session, "-n", window, "-c", &cwd]
            .iter()
            .map(|part| (*part).to_owned())
            .collect();
        argv.extend(self.shell_args(window));
        self.require(&argv.iter().map(String::as_str).collect::<Vec<_>>())?;
        self.ensure_client_session(session, window)?;
        Ok(())
    }

    /// Each visible leaf attaches to a grouped session with its own selected
    /// window. Grouped sessions share the processes but not the current-window
    /// pointer, so two xterms can display two project windows at once.
    fn ensure_client_session(&self, session: &str, window: &str) -> Result<(), TmuxError> {
        let client = naming::client_session(session, window);
        if !self.has_session(&client)? {
            self.require(&["new-session", "-d", "-t", session, "-s", &client])?;
        }
        self.require(&["select-window", "-t", &format!("{client}:{window}")])?;
        Ok(())
    }

    /// Everything tmux draws that this app draws better itself.
    fn quiet_chrome(&self) -> Result<(), TmuxError> {
        let _ = self.require(&chrome::one_line());
        Ok(())
    }

    /// What is running in each window of a session.
    pub fn running(&self, session: &str) -> Result<Vec<Running>, TmuxError> {
        let out = self.require(&[
            "list-panes",
            "-s",
            "-t",
            session,
            "-F",
            shell::RUNNING_FORMAT,
        ])?;
        Ok(shell::parse_running(&String::from_utf8_lossy(&out.stdout)))
    }

    /// Records which profile devpit started in a pane.
    pub fn name_pane(&self, target: &str, profile: &str) -> Result<(), TmuxError> {
        pane::name_pane(self, target, profile)
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

    /// The argv a pty should spawn to attach as a client of this window.
    pub fn attach_argv(&self, session: &str, window: &str) -> Vec<String> {
        naming::attach_argv(&self.socket, session, window)
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
        let client = naming::client_session(session, window);
        if self.has_session(&client)? {
            let _ = self.run(&["kill-session", "-t", &client])?;
        }
        let _ = self.run(&["kill-window", "-t", &format!("{session}:{window}")])?;
        Ok(())
    }

    /// A copy of everything a pane prints, to `command`'s stdin, with no
    /// client attached. See [`crate::pane`].
    pub fn pipe_pane(&self, target: &str, command: &str) -> Result<(), TmuxError> {
        pane::pipe_pane(self, target, command)
    }

    /// Stops the pipe. A pane that has none is not an error.
    pub fn unpipe(&self, target: &str) -> Result<(), TmuxError> {
        pane::unpipe(self, target)
    }

    pub fn send_keys(&self, target: &str, keys: &str) -> Result<(), TmuxError> {
        pane::send_keys(self, target, keys)
    }

    pub fn capture_pane(&self, target: &str) -> Result<String, TmuxError> {
        pane::capture_pane(self, target)
    }

    pub fn kill_server(&self) -> Result<(), TmuxError> {
        let _ = self.run(&["kill-server"])?;
        Ok(())
    }

    fn require(&self, args: &[&str]) -> Result<std::process::Output, TmuxError> {
        running::require(self, args)
    }

    fn run(&self, args: &[&str]) -> Result<std::process::Output, TmuxError> {
        running::run(self, args)
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
