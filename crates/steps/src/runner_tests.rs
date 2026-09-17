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
    let ended = run(
        command,
        dir.path(),
        &context(),
        timeout,
        |_| {},
        |line| lines.push(line.to_owned()),
    )
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
    let (_, lines) = collected("echo \"$DEVPIT_BRANCH/$DEVPIT_CARD_TITLE\"", None);
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
        "echo \"$DEVPIT_BRANCH\" > /dev/null",
        dir.path(),
        &context,
        None,
        |_| {},
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
        |_| {},
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
        run("   ", dir.path(), &context(), None, |_| {}, |_| {}),
        Err(RunError::Empty)
    ));
}

/// A command run can be stopped, which means its pid has to come back.
///
/// Without this the card's stop and an update told to stop the work both
/// answered "not in flight here", and the run came back `lost` rather than
/// `cancelled`. Sabotage: stop calling `on_pid` and this fails on the `None`.
#[test]
fn the_shell_says_which_process_it_is_before_it_runs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut said: Option<u32> = None;
    let ended = run(
        "echo hello",
        dir.path(),
        &context(),
        None,
        |pid| said = Some(pid),
        |_| {},
    )
    .expect("ran");

    let pid = said.expect("the runner never said which process it started");
    assert!(pid > 1, "pid {pid} is not a process this run could stop");
    assert_eq!(ended.exit_code, Some(0));
}

/// A command that leaves a grandchild behind is the normal shape of a test
/// suite: `cargo test` is the shell's child and the test binary is its
/// grandchild. Killing only the shell leaves the work running under a card
/// that says it stopped — a "Cancel" that does that is worse than no button.
///
/// Sabotage: take `setsid` out of the spawn and the grandchild writes its
/// proof, and this fails.
#[test]
fn the_timeout_ends_the_grandchildren_too() {
    let dir = tempfile::tempdir().expect("tempdir");
    let outlived = dir.path().join("outlived");
    let command = format!("(sleep 2; echo alive > {}) & sleep 30", outlived.display());

    let ended = run(
        &command,
        dir.path(),
        &context(),
        Some(Duration::from_millis(300)),
        |_| {},
        |_| {},
    )
    .expect("ran");
    assert!(ended.timed_out, "it was allowed to keep running");

    // Past when the grandchild meant to write, so its silence is a fact
    // rather than a race the test happened to win.
    std::thread::sleep(Duration::from_secs(3));
    assert!(
        !outlived.exists(),
        "a grandchild outlived the timeout and wrote {}",
        outlived.display()
    );
}
