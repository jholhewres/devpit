//! A turn of conversation, read through a driver and reported as it arrives.
//!
//! `headless::run_turn` answers once, at the end — right for a board step,
//! wrong for a chat, where the point is watching it happen.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use devpit_rpc::{Part, TurnEnd};

use crate::driver::{Driver, Read};
use crate::AgentError;

/// What a turn needs to start.
pub struct Say<'a> {
    /// The binary this conversation's profile names. Two accounts of the
    /// same CLI differ here and nowhere else.
    pub command: &'a str,
    pub prompt: &'a str,
    pub cwd: &'a Path,
    pub model: Option<&'a str>,
    pub budget_usd: Option<f64>,
    /// Carried forward so the CLI continues the same session.
    pub session_id: Option<&'a str>,
    /// What the agent may do without asking. The CLI's own word for it.
    pub permission: Option<&'a str>,
    /// How hard to think. One of what the driver's `efforts` lists.
    pub effort: Option<&'a str>,
}

/// A turn, and the thread it belongs to on the CLI's side.
pub struct Said {
    pub end: TurnEnd,
    /// The CLI's id for this conversation, to resume it next turn.
    pub session_id: Option<String>,
}

/// Runs one turn, handing every part to `on_part` as it is read.
///
/// `on_start` receives the child's pid, which is what cancelling needs: the
/// turn has to be stoppable from another thread while this one is blocked on
/// the pipe.
pub fn say(
    driver: &dyn Driver,
    turn: &Say<'_>,
    mut on_part: impl FnMut(Part),
    mut on_start: impl FnMut(u32),
) -> Result<Said, AgentError> {
    let mut argv = vec![
        "--print".to_owned(),
        "--output-format".to_owned(),
        "stream-json".to_owned(),
        "--input-format".to_owned(),
        "stream-json".to_owned(),
        "--verbose".to_owned(),
    ];
    if let Some(model) = turn.model {
        argv.push("--model".to_owned());
        argv.push(model.to_owned());
    }
    if let Some(cap) = turn.budget_usd {
        argv.push("--max-cost".to_owned());
        argv.push(cap.to_string());
    }
    if let Some(session) = turn.session_id {
        argv.push("--resume".to_owned());
        argv.push(session.to_owned());
    }
    if let Some(mode) = turn.permission {
        argv.push("--permission-mode".to_owned());
        argv.push(mode.to_owned());
    }
    if let Some(effort) = turn.effort {
        argv.push("--effort".to_owned());
        argv.push(effort.to_owned());
    }

    let mut child = Command::new(turn.command)
        .args(&argv)
        .current_dir(turn.cwd)
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
    let started = std::time::Instant::now();
    let mut ended = None;
    let mut session_id = None;

    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        if session_id.is_none() {
            session_id = driver.session(&line);
        }
        match driver.read(&line) {
            Read::Parts(parts) => parts.into_iter().for_each(&mut on_part),
            Read::Ended {
                stop_reason,
                cost_usd,
                is_error,
            } => {
                ended = Some((stop_reason, cost_usd, is_error));
            }
            Read::Nothing => {}
        }
    }

    let status = child
        .wait()
        .map_err(|err| AgentError::Unreadable(err.to_string()))?;

    let (stop_reason, cost_usd, is_error) = ended.unwrap_or_else(|| {
        // No end frame means the turn was stopped, not that it finished. A
        // conversation that shows those the same way is lying about one.
        (Some("interrupted".to_owned()), None, !status.success())
    });

    Ok(Said {
        end: TurnEnd {
            turn_id: String::new(),
            cost_usd,
            duration_ms: Some(started.elapsed().as_millis() as f64),
            stop_reason,
            is_error,
        },
        session_id,
    })
}
