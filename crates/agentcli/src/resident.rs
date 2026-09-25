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

/// How long a closed process gets to finish on its own before it is ended.
const GRACE: std::time::Duration = std::time::Duration::from_secs(15);

/// Whether the process is still there and still this one: a zombie or a pid
/// already handed to someone else is not signalled.
fn still_running(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| {
            stat.rsplit(')')
                .next()
                .map(|rest| rest.trim_start().chars().next())
        })
        .flatten()
        .is_some_and(|state| state != 'Z')
}

/// What the process says, a turn at a time.
pub enum Heard {
    Part(Part),
    /// A turn is over, whoever started it.
    Ended(Said),
    /// The process is gone: exited, killed, or its stream closed.
    Gone,
    /// The CLI's answer to a control request, as it wrote it.
    Control(serde_json::Value),
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
        on: impl FnMut(Heard) + Send + 'static,
    ) -> Result<Self, AgentError> {
        // Shared by the turn reader and the line watcher below, which both
        // hand things on while one turn is being read.
        let on = std::sync::Arc::new(std::sync::Mutex::new(on));
        let tell = move |heard: Heard| {
            if let Ok(mut on) = on.lock() {
                on(heard);
            }
        };
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
            // Control answers are not turn frames; they are handed on as
            // they pass, and the turn reader never sees a difference.
            let watching = tell.clone();
            let mut lines = BufReader::new(stdout)
                .lines()
                .map_while(Result::ok)
                .inspect(move |line| {
                    if line.contains("\"control_response\"") {
                        if let Ok(said) = serde_json::from_str::<serde_json::Value>(line) {
                            watching(Heard::Control(said["response"].clone()));
                        }
                    }
                });
            loop {
                let started = Instant::now();
                let (heard, whole) = next_turn(
                    driver.as_ref(),
                    lines.by_ref(),
                    &reading,
                    Some(on_session.as_ref()),
                    |part| tell(Heard::Part(part)),
                );
                if let Some(ending) = heard.ended {
                    tell(Heard::Ended(Said {
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
            tell(Heard::Gone);
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

    /// Sends a control request under `id`; its answer comes back as
    /// [`Heard::Control`]. False when it no longer listens.
    pub fn control(&self, id: &str, request: serde_json::Value) -> bool {
        self.control.write(
            &serde_json::json!({ "type": "control_request", "request_id": id, "request": request })
                .to_string(),
        )
    }

    /// Stops the turn it is on and keeps it running.
    pub fn interrupt(&self) -> bool {
        self.control.interrupt()
    }

    /// Lets it exit: with stdin closed the CLI finishes and goes. One that
    /// is still there a while later — hung on a tool, a stalled retry — is
    /// ended, rather than left running for as long as devpit does.
    pub fn close(&self) {
        self.control.close();
        let pid = self.pid;
        std::thread::spawn(move || {
            std::thread::sleep(GRACE);
            if still_running(pid) {
                let _ = std::process::Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .status();
            }
        });
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }
}

#[cfg(all(test, unix))]
#[path = "resident_tests.rs"]
mod tests;
