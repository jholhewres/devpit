//! The argv builders and the live round-trip, tested beside them.

use super::*;

#[test]
fn a_background_session_carries_only_what_it_was_given() {
    assert_eq!(background_argv(None, None, None, None), ["claude", "--bg"]);
    assert_eq!(
        background_argv(Some("uuid-1"), Some("fix-auth"), Some("opus"), None),
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

/// Recorded from the CLI, not imagined.
const BACKGROUNDED: &str = "backgrounded · fa35a378\n\
        \x20 claude agents             list sessions\n\
        \x20 claude attach fa35a378    open in this terminal\n\
        \x20 claude logs fa35a378      show recent output\n\
        \x20 claude stop fa35a378      stop this session\n";

#[test]
fn the_handle_is_read_off_the_attach_line() {
    assert_eq!(short_id_in(BACKGROUNDED).as_deref(), Some("fa35a378"));
}

/// The announcement carries it too, for a build that prints no help.
#[test]
fn the_announcement_alone_is_enough() {
    assert_eq!(
        short_id_in("backgrounded · abc123\n").as_deref(),
        Some("abc123")
    );
}

#[test]
fn output_with_no_handle_in_it_yields_none() {
    assert_eq!(short_id_in("could not start\n"), None);
    assert_eq!(short_id_in(""), None);
}

#[test]
fn attaching_takes_the_short_id() {
    assert_eq!(attach_argv("a1b2"), ["claude", "attach", "a1b2"]);
}

/// The spending cap is the contour the whole product turns on: a step that
/// cannot overspend is a step you can leave running.
#[test]
fn a_headless_turn_declares_its_cap() {
    let argv = headless_argv(None, None, Some(0.5), None, None);
    let cap = argv
        .iter()
        .position(|a| a == "--max-budget-usd")
        .expect("no cap in the line");
    assert_eq!(argv[cap + 1], "0.5");
}

#[test]
fn a_headless_turn_streams_in_and_out() {
    let argv = headless_argv(None, None, None, None, None);
    assert!(argv.contains(&"--input-format".to_owned()));
    assert!(argv.contains(&"--output-format".to_owned()));
    assert_eq!(argv.iter().filter(|a| *a == "stream-json").count(), 2);
}

/// The whole handle round-trip, against the installed CLI.
///
/// Opt-in, like the live turn: it starts a real session and costs money.
/// It is also the only test that would have caught the three things a
/// recorded fixture could not — the id is on the `attach` line, a
/// background session reports `state` rather than `status`, and the
/// worktree lands under the project.
#[test]
fn a_real_background_session_can_be_started_and_stopped() {
    if std::env::var_os("DEVPIT_LIVE_TURN").is_none() || !available() {
        eprintln!("skipped: set DEVPIT_LIVE_TURN=1 to start a real session");
        return;
    }

    let dir = tempfile::tempdir().expect("tempdir");
    for args in [
        vec!["init", "-q", "-b", "main"],
        vec!["config", "user.email", "t@example.com"],
        vec!["config", "user.name", "Test"],
    ] {
        Command::new("git")
            .args(&args)
            .current_dir(dir.path())
            .output()
            .expect("git");
    }
    std::fs::write(dir.path().join("a.txt"), "x\n").expect("write");
    for args in [vec!["add", "-A"], vec!["commit", "-qm", "first"]] {
        Command::new("git")
            .args(&args)
            .current_dir(dir.path())
            .output()
            .expect("git");
    }

    let short = start_background(dir.path(), None, None, None, None)
        .expect("the CLI started a session but no handle came back");
    assert!(is_handle(&short), "{short} is not a handle");

    // A session that has just been started may not have reported a state
    // yet. Waiting is the honest test; asserting immediately would be
    // asserting on a race.
    let mut status = Status::Unknown;
    for _ in 0..20 {
        if let Some(ours) = list(Some(dir.path()))
            .unwrap_or_default()
            .into_iter()
            .find(|session| session.short_id.as_deref() == Some(short.as_str()))
        {
            status = ours.status;
            if status != Status::Unknown {
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    assert_ne!(
        status,
        Status::Unknown,
        "a live session never named a state"
    );

    stop(&short).expect("stop");
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
