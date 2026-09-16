//! The agent CLI, behind one boundary.
//!
//! Everything this product knows about driving an agent from the command line
//! lives here, and a guard in `xtask` fails if another crate reaches for the
//! binary directly. The surface is not documented by its vendor and will move;
//! when it does, it has to break in one place, loudly.
//!
//! What the CLI gives us, verified by running it:
//!
//! - `--bg` starts a session detached and prints a short id;
//! - `attach <id>` brings that session into the terminal you are already in,
//!   and leaving the attach does not end it;
//! - `agents --json` lists every session with `idle` or `busy`, and needs no
//!   TTY, which is what makes it usable as a status source;
//! - `-p --output-format stream-json` runs one turn and reports what it cost.

mod catalogue;
mod headless;
mod headless_stream;
mod hook_settings;
mod hooks;
mod schema;
mod session;
mod transcript;

pub use catalogue::{
    as_argument, read as read_agents, read_all as read_every_agent, source_of, Agent, Catalogue,
    Rejected,
};
pub use headless::{run_turn, run_turn_cancellable, Outcome, Turn};
pub use hook_settings::settings_json;
pub use hooks::{endpoint_file, read as read_hook, Event, Happening};
pub use schema::validates;
pub use sources::seed_sources;
// `start_background` and the argv builders live in this module.
pub use session::{AgentSession, Kind, Status};
pub use transcript::{read_cost, transcript_path, Cost};

use std::path::Path;
use std::process::Command;

/// The binary this crate drives. Overridable for tests and for anyone whose
/// install is not on the default path.
pub const PROGRAM: &str = "claude";

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("the agent CLI is not installed, or not on PATH")]
    NotInstalled,

    #[error("`{command}` failed: {stderr}")]
    Failed { command: String, stderr: String },

    #[error("could not read what the agent CLI returned: {0}")]
    Unreadable(String),
}

/// Whether the CLI is reachable at all.
///
/// Every caller checks this before doing anything else, so the screen can say
/// "not installed" instead of showing an empty list that looks like "no
/// sessions".
pub fn available() -> bool {
    Command::new(PROGRAM)
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// Every live session, with its status.
///
/// `--json` is what makes this usable from a GUI: the interactive listing
/// refuses to run without a TTY.
pub fn list(cwd: Option<&Path>) -> Result<Vec<AgentSession>, AgentError> {
    let mut command = Command::new(PROGRAM);
    command.arg("agents").arg("--json");
    if let Some(path) = cwd {
        command.arg("--cwd").arg(path);
    }
    let output = command.output().map_err(|_| AgentError::NotInstalled)?;
    if !output.status.success() {
        return Err(AgentError::Failed {
            command: "agents --json".to_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    session::parse_list(&output.stdout)
}

/// The argv that starts a session in the background.
///
/// Returned rather than run so the caller can log it, and so a test can assert
/// the exact line without launching anything.
pub fn background_argv(
    session_id: Option<&str>,
    worktree: Option<&str>,
    model: Option<&str>,
    settings: Option<&str>,
) -> Vec<String> {
    let mut argv = vec![PROGRAM.to_owned(), "--bg".to_owned()];
    if let Some(id) = session_id {
        argv.push("--session-id".to_owned());
        argv.push(id.to_owned());
    }
    if let Some(name) = worktree {
        argv.push("--worktree".to_owned());
        argv.push(name.to_owned());
    }
    if let Some(model) = model {
        argv.push("--model".to_owned());
        argv.push(model.to_owned());
    }
    // The hooks matter more here than anywhere: a background session is the one
    // running where nobody is looking, and without them its card has nothing to
    // say between polls.
    if let Some(path) = settings {
        argv.push("--settings".to_owned());
        argv.push(path.to_owned());
    }
    argv
}

/// Starts a session in the background and returns the short id it printed.
///
/// The short id is what `attach`, `logs`, `stop` and `rm` all take, so it is
/// the handle worth keeping. It is not printed alone — see `short_id_in` for
/// the block it arrives in and why it is read off the `attach` line.
pub fn start_background(
    cwd: &Path,
    session_id: Option<&str>,
    worktree: Option<&str>,
    model: Option<&str>,
    settings: Option<&str>,
) -> Result<String, AgentError> {
    let argv = background_argv(session_id, worktree, model, settings);
    let output = Command::new(PROGRAM)
        .args(&argv[1..])
        .current_dir(cwd)
        .output()
        .map_err(|_| AgentError::NotInstalled)?;

    if !output.status.success() {
        return Err(AgentError::Failed {
            command: argv.join(" "),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }

    short_id_in(&String::from_utf8_lossy(&output.stdout)).ok_or_else(|| {
        AgentError::Unreadable(
            "the CLI started a session but printed no id to attach to".to_owned(),
        )
    })
}

/// The short id out of what `--bg` printed.
///
/// The output is a small help block, not a bare id:
///
/// ```text
/// backgrounded · fa35a378
///   claude agents             list sessions
///   claude attach fa35a378    open in this terminal
/// ```
///
/// Read off the `attach` line rather than the first: that line exists to be
/// copied, so its second-to-last token is the handle by construction. A guess
/// at "the last bare word" matched nothing here and failed at attach time,
/// where the cause is no longer on screen.
fn short_id_in(stdout: &str) -> Option<String> {
    let from_attach = stdout.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        (words.next()? == PROGRAM && words.next()? == "attach")
            .then(|| words.next())
            .flatten()
    });
    from_attach
        .or_else(|| {
            // Or the announcement itself, when the help block is not printed.
            stdout
                .lines()
                .find(|line| line.starts_with("backgrounded"))
                .and_then(|line| line.split_whitespace().next_back())
        })
        .filter(|id| is_handle(id))
        .map(ToOwned::to_owned)
}

fn is_handle(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= 64
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The argv that brings a background session into a terminal.
pub fn attach_argv(short_id: &str) -> Vec<String> {
    vec![PROGRAM.to_owned(), "attach".to_owned(), short_id.to_owned()]
}

/// One headless turn, as a line of arguments.
///
/// `--output-format stream-json` does not need `--verbose`, despite what is
/// often written: it was run without it and returned the whole stream.
pub fn headless_argv(
    agents: Option<&str>,
    schema: Option<&str>,
    budget_usd: Option<f64>,
    model: Option<&str>,
    settings: Option<&str>,
    session_id: Option<&str>,
) -> Vec<String> {
    let mut argv = vec![
        PROGRAM.to_owned(),
        "-p".to_owned(),
        "--input-format".to_owned(),
        "stream-json".to_owned(),
        "--output-format".to_owned(),
        "stream-json".to_owned(),
    ];
    if let Some(json) = agents {
        argv.push("--agents".to_owned());
        argv.push(json.to_owned());
    }
    if let Some(json) = schema {
        argv.push("--json-schema".to_owned());
        argv.push(json.to_owned());
    }
    if let Some(cap) = budget_usd {
        argv.push("--max-budget-usd".to_owned());
        argv.push(cap.to_string());
    }
    if let Some(model) = model {
        argv.push("--model".to_owned());
        argv.push(model.to_owned());
    }
    if let Some(path) = settings {
        argv.push("--settings".to_owned());
        argv.push(path.to_owned());
    }
    if let Some(id) = session_id {
        argv.push("--session-id".to_owned());
        argv.push(id.to_owned());
    }
    argv
}

/// The recent terminal output of a background session, without attaching.
///
/// This is how a card stays honest about a session nobody is watching.
pub fn logs(short_id: &str) -> Result<String, AgentError> {
    run(&["logs", short_id])
}

pub fn stop(short_id: &str) -> Result<(), AgentError> {
    run(&["stop", short_id]).map(|_| ())
}

pub fn remove(short_id: &str) -> Result<(), AgentError> {
    run(&["rm", short_id]).map(|_| ())
}

pub fn respawn(short_id: &str) -> Result<(), AgentError> {
    run(&["respawn", short_id]).map(|_| ())
}

fn run(args: &[&str]) -> Result<String, AgentError> {
    let output = Command::new(PROGRAM)
        .args(args)
        .output()
        .map_err(|_| AgentError::NotInstalled)?;
    if !output.status.success() {
        return Err(AgentError::Failed {
            command: args.join(" "),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

pub mod claude_lines;
pub mod cli_config;
pub mod control;
pub mod declaring;
pub mod driver;
#[cfg(test)]
mod driver_tests;
pub mod head;
#[cfg(test)]
mod head_tests;
pub mod history;
#[cfg(test)]
mod history_tests;
pub mod outside;
pub mod profile;
#[cfg(test)]
mod profile_tests;
pub mod running;
pub mod skills;
mod sources;
pub mod spend_prices;
pub mod spend_scan;
pub mod store;
#[cfg(test)]
mod store_tests;
pub mod talk;
mod talk_args;
mod talk_stream;
pub mod transcript_text;
