//! One headless turn: run it, read the stream, report what it cost.
//!
//! This is the cheap half of the product. It takes no terminal, it declares a
//! spending cap before it starts, and the last line of its output says what it
//! actually spent — which is how a card stops saying "the agent is doing
//! something" and starts saying a number.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use serde::Deserialize;

use crate::{headless_argv, AgentError, PROGRAM};

/// What one turn produced.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// The final text, which is JSON when a schema was asked for.
    pub result: String,
    pub cost_usd: f64,
    pub duration_ms: i64,
    pub turns: i64,
    /// True when the CLI itself reported failure, budget included.
    pub is_error: bool,
    /// Why it ended: the CLI's own word, kept rather than mapped so a new one
    /// arrives intact instead of being flattened into "failed".
    pub stop_reason: Option<String>,
}

/// How a turn is configured.
pub struct Turn<'a> {
    pub prompt: &'a str,
    pub cwd: &'a Path,
    /// The `--agents` argument, when the step names an agent.
    pub agents: Option<&'a str>,
    /// A JSON Schema the answer has to satisfy.
    pub schema: Option<&'a str>,
    pub budget_usd: Option<f64>,
    pub model: Option<&'a str>,
    /// A settings file for this turn — the hooks that report what it is doing.
    pub settings: Option<&'a str>,
    /// Context the step declared, as environment variables.
    ///
    /// Variables and never interpolation: a branch named `fix;rm -rf /` has to
    /// become a value, not shell syntax.
    pub env: &'a [(String, String)],
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Line {
    #[serde(rename = "result")]
    Result {
        #[serde(default)]
        result: String,
        #[serde(default)]
        is_error: bool,
        #[serde(default)]
        total_cost_usd: f64,
        #[serde(default)]
        duration_ms: i64,
        #[serde(default)]
        num_turns: i64,
        #[serde(default)]
        stop_reason: Option<String>,
    },
    #[serde(other)]
    Other,
}

/// Runs the turn and reports the outcome.
///
/// `on_partial` is called with each assistant fragment as it arrives, so a
/// card can show work in progress rather than a spinner. A turn that produces
/// no `result` line is a failure with the stderr attached: it means the CLI
/// died rather than answered.
pub fn run_turn(turn: &Turn<'_>, on_partial: impl FnMut(&str)) -> Result<Outcome, AgentError> {
    run_turn_cancellable(turn, on_partial, |_| {})
}

/// The same, handing the caller a way to stop it.
///
/// `on_start` is given the process id the moment the turn begins. Without it a
/// turn that hangs is a run stuck at `running` for the life of the app, and a
/// card stuck behind it — a step you cannot stop is a step you learn not to
/// start.
pub fn run_turn_cancellable(
    turn: &Turn<'_>,
    mut on_partial: impl FnMut(&str),
    mut on_start: impl FnMut(u32),
) -> Result<Outcome, AgentError> {
    let argv = headless_argv(
        turn.agents,
        turn.schema,
        turn.budget_usd,
        turn.model,
        turn.settings,
    );
    let mut child = Command::new(PROGRAM)
        .args(&argv[1..])
        .current_dir(turn.cwd)
        .envs(turn.env.iter().map(|(key, value)| (key, value)))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| AgentError::NotInstalled)?;

    on_start(child.id());

    {
        let mut stdin = child.stdin.take().ok_or(AgentError::NotInstalled)?;
        let message = serde_json::json!({
            "type": "user",
            "message": { "role": "user", "content": [{ "type": "text", "text": turn.prompt }] }
        });
        writeln!(stdin, "{message}").map_err(|err| AgentError::Unreadable(err.to_string()))?;
    }

    let stdout = child.stdout.take().ok_or(AgentError::NotInstalled)?;
    let mut outcome = None;
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        match serde_json::from_str::<Line>(&line) {
            Ok(Line::Result {
                result,
                is_error,
                total_cost_usd,
                duration_ms,
                num_turns,
                stop_reason,
            }) => {
                outcome = Some(Outcome {
                    result,
                    cost_usd: total_cost_usd,
                    duration_ms,
                    turns: num_turns,
                    is_error,
                    stop_reason,
                });
            }
            Ok(Line::Other) => on_partial(&line),
            // A line we cannot read is not a reason to abandon the run: the
            // one that matters is `result`, and it comes last.
            Err(_) => {}
        }
    }

    let status = child
        .wait()
        .map_err(|err| AgentError::Unreadable(err.to_string()))?;

    outcome.ok_or_else(|| AgentError::Failed {
        command: "headless turn".to_owned(),
        stderr: format!("the run ended with {status} before reporting a result"),
    })
}

#[cfg(test)]
#[path = "headless_tests.rs"]
mod tests;
