//! Running a command step, and streaming what it prints.
//!
//! Output arrives in pieces while the command runs rather than in one block at
//! the end. A test suite that takes twenty minutes shows its first line
//! immediately; buffering it would make a working command indistinguishable
//! from a hung one for twenty minutes.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::Context;

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("the command could not be started: {0}")]
    NotStarted(String),

    #[error("no command to run")]
    Empty,
}

/// How a command run ended.
#[derive(Debug, Clone, PartialEq)]
pub struct Ended {
    /// `None` when the command was killed for running past its timeout.
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub duration_ms: i64,
}

/// Runs `command` in `cwd`, calling `on_line` with each line as it arrives.
///
/// The context reaches the command only through the environment. The command
/// string itself is never built from it, so a value containing shell syntax
/// stays a value.
pub fn run(
    command: &str,
    cwd: &Path,
    context: &Context,
    timeout: Option<Duration>,
    mut on_line: impl FnMut(&str),
) -> Result<Ended, RunError> {
    if command.trim().is_empty() {
        return Err(RunError::Empty);
    }

    let started = Instant::now();
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .envs(context.environment())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| RunError::NotStarted(err.to_string()))?;

    // stderr matters as much as stdout for a failing build, and interleaving
    // them keeps the order a person would have seen in a terminal.
    let (tx, rx) = mpsc::channel::<String>();
    for stream in [
        child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
        child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let tx = tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
    }
    drop(tx);

    let mut timed_out = false;
    loop {
        // A short wait rather than a blocking read, so a command that prints
        // nothing can still be timed out.
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => on_line(&line),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if let Some(limit) = timeout {
            if started.elapsed() > limit {
                let _ = child.kill();
                timed_out = true;
                break;
            }
        }
        if !timed_out {
            if let Ok(Some(_)) = child.try_wait() {
                // Drain whatever is still in flight before calling it done.
                while let Ok(line) = rx.try_recv() {
                    on_line(&line);
                }
                break;
            }
        }
    }

    let status = child.wait().ok();
    Ok(Ended {
        exit_code: if timed_out {
            None
        } else {
            status.and_then(|s| s.code())
        },
        timed_out,
        duration_ms: started.elapsed().as_millis() as i64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> Context {
        Context {
            branch: "main".to_owned(),
            card_title: "a card".to_owned(),
            ..Context::default()
        }
    }

    fn collected(command: &str, timeout: Option<Duration>) -> (Ended, Vec<String>) {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut lines = Vec::new();
        let ended = run(command, dir.path(), &context(), timeout, |line| {
            lines.push(line.to_owned())
        })
        .expect("the command ran");
        (ended, lines)
    }

    #[test]
    fn a_command_that_succeeds_reports_zero() {
        let (ended, lines) = collected("echo hello", None);
        assert_eq!(ended.exit_code, Some(0));
        assert!(!ended.timed_out);
        assert_eq!(lines, ["hello"]);
    }

    #[test]
    fn a_command_that_fails_reports_its_code() {
        let (ended, _) = collected("exit 3", None);
        assert_eq!(ended.exit_code, Some(3));
    }

    /// A failing build says why on stderr. Dropping it would leave the card
    /// with a red run and no reason on it.
    #[test]
    fn stderr_reaches_the_card_too() {
        let (_, lines) = collected("echo out; echo err 1>&2", None);
        assert!(lines.contains(&"out".to_owned()));
        assert!(lines.contains(&"err".to_owned()));
    }

    /// The context is readable, and only from the environment.
    #[test]
    fn the_command_reads_its_context_from_the_environment() {
        let (_, lines) = collected("echo \"$QUOCKPIT_BRANCH/$QUOCKPIT_CARD_TITLE\"", None);
        assert_eq!(lines, ["main/a card"]);
    }

    /// The class of bug this design removes, proved rather than argued.
    #[test]
    fn a_branch_full_of_shell_syntax_runs_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let proof = dir.path().join("proof");
        std::fs::write(&proof, "still here").expect("write");

        let context = Context {
            branch: format!("x; rm -f {}", proof.display()),
            ..Context::default()
        };
        run(
            "echo \"$QUOCKPIT_BRANCH\" > /dev/null",
            dir.path(),
            &context,
            None,
            |_| {},
        )
        .expect("ran");

        assert!(proof.exists(), "the branch name executed as a command");
    }

    /// A command that never ends has to end anyway.
    #[test]
    fn a_command_past_its_timeout_is_killed() {
        let (ended, _) = collected("sleep 30", Some(Duration::from_millis(300)));
        assert!(ended.timed_out, "it was allowed to keep running");
        assert_eq!(ended.exit_code, None);
        assert!(ended.duration_ms < 5_000, "{}ms", ended.duration_ms);
    }

    /// Output arrives while it works, not when it stops.
    #[test]
    fn the_first_line_arrives_before_the_command_ends() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut first: Option<Duration> = None;
        let started = Instant::now();
        run(
            "echo now; sleep 1; echo later",
            dir.path(),
            &context(),
            None,
            |_| {
                if first.is_none() {
                    first = Some(started.elapsed());
                }
            },
        )
        .expect("ran");

        let first = first.expect("nothing was streamed");
        assert!(
            first < Duration::from_millis(800),
            "the first line waited {first:?} for a command that ran for a second"
        );
    }

    #[test]
    fn an_empty_command_is_refused_rather_than_run() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(matches!(
            run("   ", dir.path(), &context(), None, |_| {}),
            Err(RunError::Empty)
        ));
    }
}
