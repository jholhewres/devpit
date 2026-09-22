//! Running a tmux command and reading what it said.
//!
//! The plumbing every verb goes through, kept apart from the verbs: one place
//! decides how the binary is found, how the socket is named on the line, and
//! what a non-zero exit turns into.

use crate::{Server, TmuxError};

pub(crate) fn require(server: &Server, args: &[&str]) -> Result<std::process::Output, TmuxError> {
    let output = run(server, args)?;
    if output.status.success() {
        return Ok(output);
    }
    Err(TmuxError::Failed {
        command: args.join(" "),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

pub(crate) fn run(server: &Server, args: &[&str]) -> Result<std::process::Output, TmuxError> {
    devpit_pty::host_env::command("tmux")
        .arg("-S")
        .arg(&server.socket)
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
