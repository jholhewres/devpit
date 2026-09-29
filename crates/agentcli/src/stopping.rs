//! Ending a background session, which only its own CLI can do.

use crate::{running, AgentError, PROGRAM};

/// Stops a background session through the CLI, which is the only thing that
/// can: its supervisor starts a killed process again, so a signal from here
/// had it back a moment later. The conversation is kept.
pub fn stop_background(runner: Option<&running::Runner>, id: &str) -> Result<(), AgentError> {
    let argv = stop_argv(runner, id);
    let output = devpit_pty::host_env::command(&argv[0])
        .args(&argv[1..])
        .envs(runner.map(|one| one.env.clone()).unwrap_or_default())
        .output()
        .map_err(|_| AgentError::NotInstalled)?;
    if output.status.success() {
        return Ok(());
    }
    Err(AgentError::Failed {
        command: argv.join(" "),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

/// `stop <id>`, under the profile's own program and arguments.
pub fn stop_argv(runner: Option<&running::Runner>, id: &str) -> Vec<String> {
    let mut argv = vec![runner
        .map_or(PROGRAM, |one| one.program.as_str())
        .to_owned()];
    argv.extend(runner.map(|one| one.args.clone()).unwrap_or_default());
    argv.extend(["stop".to_owned(), id.to_owned()]);
    argv
}
