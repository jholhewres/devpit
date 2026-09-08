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
mod session;
mod transcript;

pub use catalogue::{
    as_argument, read as read_agents, seed as seed_agents, Agent, Catalogue, Rejected,
};
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
    argv
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

/// Where to seed the first set of agents from.
///
/// Whatever the person already has installed, rather than a copy vendored
/// here: their set is the one they trust, and a vendored copy would be stale
/// the week after it was taken.
///
/// Every directory, not the best one. Picking a single "best" needs a rule for
/// which plugin wins, and any such rule is wrong for someone — one install had
/// a plugin with a single agent sorting above the set of nineteen. Seeding
/// from all of them and never overwriting gets the union with no rule at all.
pub fn seed_sources() -> Vec<std::path::PathBuf> {
    let Some(home) = dirs_home() else {
        return Vec::new();
    };
    let mut dirs = walk_agent_dirs(&home.join(".claude/plugins/cache"));
    let own = home.join(".claude/agents");
    if own.is_dir() {
        dirs.push(own);
    }
    dirs.sort();
    dirs
}

fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}

/// Every `agents/` directory under a plugin cache, at any depth.
fn walk_agent_dirs(root: &Path) -> Vec<std::path::PathBuf> {
    fn visit(dir: &Path, depth: usize, found: &mut Vec<std::path::PathBuf>) {
        if depth > 4 {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if path.file_name().is_some_and(|n| n == "agents") {
                found.push(path);
            } else {
                visit(&path, depth + 1, found);
            }
        }
    }
    let mut found = Vec::new();
    visit(root, 0, &mut found);
    found
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
mod tests {
    use super::*;

    #[test]
    fn a_background_session_carries_only_what_it_was_given() {
        assert_eq!(background_argv(None, None, None), ["claude", "--bg"]);
        assert_eq!(
            background_argv(Some("uuid-1"), Some("fix-auth"), Some("opus")),
            [
                "claude",
                "--bg",
                "--session-id",
                "uuid-1",
                "--worktree",
                "fix-auth",
                "--model",
                "opus"
            ]
        );
    }

    #[test]
    fn attaching_takes_the_short_id() {
        assert_eq!(attach_argv("a1b2"), ["claude", "attach", "a1b2"]);
    }

    /// The spending cap is the contour the whole product turns on: a step that
    /// cannot overspend is a step you can leave running.
    #[test]
    fn a_headless_turn_declares_its_cap() {
        let argv = headless_argv(None, None, Some(0.5), None);
        let cap = argv
            .iter()
            .position(|a| a == "--max-budget-usd")
            .expect("no cap in the line");
        assert_eq!(argv[cap + 1], "0.5");
    }

    #[test]
    fn a_headless_turn_streams_in_and_out() {
        let argv = headless_argv(None, None, None, None);
        assert!(argv.contains(&"--input-format".to_owned()));
        assert!(argv.contains(&"--output-format".to_owned()));
        assert_eq!(argv.iter().filter(|a| *a == "stream-json").count(), 2);
    }

    /// Runs against the installed binary, and steps aside when there is none
    /// so CI does not depend on it.
    #[test]
    fn the_installed_cli_answers_with_the_shape_we_decode() {
        if !available() {
            eprintln!("skipped: the agent CLI is not on PATH");
            return;
        }
        let sessions = list(None).expect("agents --json");
        for session in &sessions {
            assert!(!session.session_id.is_empty(), "a session with no id");
        }
    }
}
