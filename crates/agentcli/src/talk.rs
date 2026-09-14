//! A turn of conversation, read through a driver and reported as it arrives.
//!
//! `headless::run_turn` answers once, at the end — right for a board step,
//! wrong for a chat, where the point is watching it happen.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

use devpit_rpc::{Part, SessionInit, TurnEnd};

use crate::driver::{Driver, Read};
use crate::talk_args::argv;
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
    /// A message in that session to fork at instead of resuming its end.
    pub fork_at: Option<&'a str>,
    /// What the agent may do without asking. The CLI's own word for it.
    pub permission: Option<&'a str>,
    /// How hard to think. One of what the driver's `efforts` lists.
    pub effort: Option<&'a str>,
    /// Where the turn's stdin is kept while it runs, for control requests.
    pub control: Option<&'a crate::control::Control>,
}

/// A turn, and the thread it belongs to on the CLI's side.
pub struct Said {
    pub end: TurnEnd,
    /// The CLI's id for this conversation, to resume it next turn.
    pub session_id: Option<String>,
    /// The last self-description the CLI printed. A background completion can
    /// wake it for a second pass, which prints another.
    pub init: Option<SessionInit>,
    /// The last message the agent wrote, where a rewind to this turn forks.
    pub anchor: Option<String>,
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
    let argv = argv(turn);

    let mut child = Command::new(turn.command)
        .args(&argv)
        .current_dir(turn.cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| AgentError::NotInstalled)?;

    on_start(child.id());

    let stdin = child.stdin.take().ok_or(AgentError::NotInstalled)?;
    let control = turn.control.cloned().unwrap_or_default();
    control.attach(stdin);
    let message = serde_json::json!({
        "type": "user",
        "message": { "role": "user", "content": [{ "type": "text", "text": turn.prompt }] }
    });
    if !control.write(&message.to_string()) {
        return Err(AgentError::Unreadable(
            "could not send the prompt".to_owned(),
        ));
    }
    let mut running = crate::control::Running::default();
    let mut result_seen = false;

    let stdout = child.stdout.take().ok_or(AgentError::NotInstalled)?;
    let started = std::time::Instant::now();
    let mut ended = None;
    let mut session_id = None;
    let mut init = None;
    let mut anchor = None;

    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        // The last id seen, not the first: `/clear` starts a new session inside
        // the turn, and resuming the one it left would undo it.
        if let Some(seen) = driver.session(&line) {
            session_id = Some(seen);
        }
        anchor = driver.anchor(&line).or(anchor);
        match driver.read(&line) {
            Read::Parts(parts) => parts.into_iter().for_each(|part| {
                running.saw(&part);
                on_part(part)
            }),
            Read::Ended {
                stop_reason,
                cost_usd,
                is_error,
            } => {
                ended = Some((stop_reason, cost_usd, is_error));
                result_seen = true;
            }
            Read::Init(said) => init = Some(said),
            Read::Nothing => {}
        }
        if crate::control::may_close(result_seen, &running) {
            control.close();
        }
    }

    // Whatever ended the stream, stdin is not needed any more.
    control.close();
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
        init,
        anchor,
    })
}
