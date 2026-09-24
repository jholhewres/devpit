//! A conversation whose process stays between turns.
//!
//! A turn that starts a process and lets it exit is deaf in between: the CLI
//! takes messages from the account's other sessions only while it runs, and a
//! session that is waited on can only say it finished to one that is still
//! there. So this process is started once and kept. Each person's message is a
//! line on its stdin; a message from another session wakes it, and the turn
//! that follows is read the same way.
//!
//! Measured on Claude Code 2.1.281: in stream-json input mode, `--print` stays
//! after its result while stdin is open, is listed as a live session, and an
//! idle one answers a message with a turn of its own.

use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;

use devpit_rpc::{Part, TurnEnd};

use crate::control::Control;
use crate::driver::Driver;
use crate::talk::{Said, Say};
use crate::talk_args::argv;
use crate::talk_stream::next_turn;
use crate::AgentError;

/// What the process says, a turn at a time.
pub enum Heard {
    Part(Part),
    /// A turn is over, whoever started it.
    Ended(Said),
    /// The process is gone: exited, killed, or its stream closed.
    Gone,
}

/// The process that stays, and its stdin.
pub struct Resident {
    control: Control,
    pid: u32,
}

impl Resident {
    /// Starts the process with `turn`'s flags — its prompt is not sent — and
    /// reads it on a thread of its own, handing everything to `on`.
    pub fn start(
        driver: Box<dyn Driver>,
        turn: &Say<'_>,
        on_session: Arc<dyn Fn(&str) + Send + Sync>,
        mut on: impl FnMut(Heard) + Send + 'static,
    ) -> Result<Self, AgentError> {
        let mut child = devpit_pty::host_env::command(turn.command)
            .args(argv(turn))
            .current_dir(turn.cwd)
            .envs(turn.env.iter().map(|(key, value)| (key, value)))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| AgentError::NotInstalled)?;
        let pid = child.id();
        let control = turn.control.cloned().unwrap_or_default();
        control.attach(child.stdin.take().ok_or(AgentError::NotInstalled)?);
        let stdout = child.stdout.take().ok_or(AgentError::NotInstalled)?;

        let reading = control.clone();
        std::thread::spawn(move || {
            let mut lines = BufReader::new(stdout).lines().map_while(Result::ok);
            loop {
                let started = Instant::now();
                let (heard, whole) = next_turn(
                    driver.as_ref(),
                    lines.by_ref(),
                    &reading,
                    Some(on_session.as_ref()),
                    |part| on(Heard::Part(part)),
                );
                if let Some(ending) = heard.ended {
                    on(Heard::Ended(Said {
                        end: TurnEnd {
                            turn_id: String::new(),
                            cost_usd: ending.cost_usd,
                            duration_ms: Some(started.elapsed().as_millis() as f64),
                            stop_reason: ending.stop_reason,
                            is_error: ending.is_error,
                            context: ending.context,
                        },
                        session_id: heard.session_id,
                        init: heard.init,
                        anchor: heard.anchor,
                    }));
                }
                if !whole {
                    break;
                }
            }
            let _ = child.wait();
            on(Heard::Gone);
        });

        Ok(Self { control, pid })
    }

    /// Hands it the person's next message. False when it no longer listens.
    pub fn say(&self, prompt: &str) -> bool {
        let message = serde_json::json!({
            "type": "user",
            "message": { "role": "user", "content": [{ "type": "text", "text": prompt }] }
        });
        self.control.write(&message.to_string())
    }

    /// Stops the turn it is on and keeps it running.
    pub fn interrupt(&self) -> bool {
        self.control.interrupt()
    }

    /// Lets it exit: with stdin closed the CLI finishes and goes.
    pub fn close(&self) {
        self.control.close();
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }
}

#[cfg(all(test, unix))]
#[path = "resident_tests.rs"]
mod tests;
