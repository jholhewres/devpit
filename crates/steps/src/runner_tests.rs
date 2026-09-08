//! The runner's tests, kept beside it.

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
