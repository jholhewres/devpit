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
        let mut name = String::from("quockpit_");
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
        // A status bar inside a pane we already chrome is noise, and it steals
        // a row from the agent's TUI.
        let _ = self.require(&["set-option", "-t", session, "status", "off"]);
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
mod tests {
    use super::*;

    #[test]
    fn session_names_are_safe_for_tmux() {
        assert_eq!(Server::session_name("prj_01HXYZ"), "quockpit_prj_01HXYZ");
        assert_eq!(
            Server::session_name("prj/weird:name"),
            "quockpit_prj_weird_name"
        );
        assert!(Server::session_name("x").starts_with("quockpit_"));
    }

    #[test]
    fn availability_follows_the_program_exit() {
        assert!(Server::available_at(Path::new("/bin/true")));
        assert!(!Server::available_at(Path::new(
            "/path/that/does/not/contain/tmux"
        )));
    }

    #[test]
    fn a_session_survives_and_echoes_through_send_keys() {
        if !Server::available() {
            eprintln!("skip: tmux not on PATH");
            return;
        }

        let dir = tempfile::tempdir().expect("tempdir");
        let socket = dir.path().join("tmux.sock");
        let server = Server::new(socket);
        let session = "quockpit_test_session";
        let window = "leaf_test";

        server
            .ensure_session(session, window, dir.path())
            .expect("ensure");
        server
            .ensure_session(session, window, dir.path())
            .expect("ensure again");

        let windows = server.list_windows(session).expect("list");
        assert!(
            windows.iter().any(|name| name == window),
            "missing window: {windows:?}"
        );

        let target = Server::target(session, window);
        server
            .send_keys(&target, "echo quockpit-tmux-ok")
            .expect("send");

        // The shell needs a beat to print. A tight loop would flake on a
        // loaded machine; 200ms is well above a local echo and still a unit.
        std::thread::sleep(std::time::Duration::from_millis(200));
        let shot = server.capture_pane(&target).expect("capture");
        assert!(
            shot.contains("quockpit-tmux-ok"),
            "capture missed the echo: {shot:?}"
        );

        server
            .new_window(session, "leaf_two", dir.path())
            .expect("split window");
        let windows = server.list_windows(session).expect("list after split");
        assert!(windows.iter().any(|name| name == "leaf_two"));

        server.kill_server().expect("kill");
    }
}
