//! A remote viewer of one window: a client of its own, grouped with the
//! project's session like a pane's, that changes nothing on the desk.
//!
//! `ignore-size` keeps a phone from shrinking the window somebody is
//! looking at; `read-only` keeps a device that may only watch from typing,
//! enforced by tmux rather than by the page. Its own client session keeps
//! the desk's selected window where it was.

use crate::{naming, Server, TmuxError};

/// The client session a viewer watches through.
pub(crate) fn viewer_session(session: &str, window: &str, viewer: &str) -> String {
    let mut name = format!("{}__r{viewer}", naming::client_session(session, window));
    name.truncate(180);
    name
}

impl Server {
    /// The argv a pty spawns to watch `window` as viewer `viewer`, typing
    /// only when `typing`. Refused where sessions cannot be grouped.
    pub fn viewer_argv(
        &self,
        session: &str,
        window: &str,
        viewer: &str,
        typing: bool,
    ) -> Result<Vec<String>, TmuxError> {
        if !naming::GROUPED {
            return Err(TmuxError::Failed {
                command: "attach-session".to_owned(),
                stderr: "a remote viewer needs grouped sessions, which this tmux has not"
                    .to_owned(),
            });
        }
        let client = viewer_session(session, window, viewer);
        if !self.has_session(&client)? {
            self.require(&["new-session", "-d", "-t", session, "-s", &client])?;
        }
        self.require(&["select-window", "-t", &format!("{client}:{window}")])?;
        let flags = if typing {
            "ignore-size"
        } else {
            "read-only,ignore-size"
        };
        Ok(vec![
            naming::program(),
            "-S".to_owned(),
            self.socket.display().to_string(),
            "attach-session".to_owned(),
            "-t".to_owned(),
            format!("{client}:{window}"),
            "-f".to_owned(),
            flags.to_owned(),
        ])
    }

    /// A detached session `session` of its own running `argv`, unless there
    /// is one: how a terminal tmux does not hold is reached through tmux.
    /// Matched by its exact name, never as a prefix of another's.
    pub fn ensure_view(
        &self,
        session: &str,
        env: &[(String, String)],
        argv: &[String],
    ) -> Result<(), TmuxError> {
        if self.has_session(&format!("={session}"))? {
            return Ok(());
        }
        if let Some(parent) = self.socket.parent() {
            std::fs::create_dir_all(parent).map_err(|err| TmuxError::Failed {
                command: "mkdir".to_owned(),
                stderr: err.to_string(),
            })?;
        }
        let mut args: Vec<String> = ["new-session", "-d", "-s", session]
            .iter()
            .map(|part| (*part).to_owned())
            .collect();
        for (key, value) in env {
            args.extend(["-e".to_owned(), format!("{key}={value}")]);
        }
        args.push("--".to_owned());
        args.extend(argv.iter().cloned());
        self.require(&args.iter().map(String::as_str).collect::<Vec<_>>())?;
        Ok(())
    }

    /// Ends a viewer's client session. Gone already is not an error.
    pub fn end_viewer(&self, session: &str, window: &str, viewer: &str) {
        let _ = self.run(&[
            "kill-session",
            "-t",
            &viewer_session(session, window, viewer),
        ]);
    }
}
