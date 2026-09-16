//! The argv builders and the live round-trip, tested beside them.

use super::*;

#[test]
fn a_background_session_carries_only_what_it_was_given() {
    assert_eq!(
        background_argv(None, None, None, None, None),
        ["claude", "--bg"]
    );
    assert_eq!(
        background_argv(None, Some("uuid-1"), Some("fix-auth"), Some("opus"), None),
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
    assert_eq!(attach_argv(None, "a1b2"), ["claude", "attach", "a1b2"]);
}

/// A session started under a profile is started by that profile's binary, with
/// its arguments, and its environment reaches the child — which is what says
/// whose account the session spends while it runs detached for days.
#[cfg(unix)]
#[test]
fn a_background_session_starts_under_its_profile() {
    use std::os::unix::fs::PermissionsExt;

    let named = crate::running::Runner {
        program: "/opt/claw/bin/claw".to_owned(),
        args: vec!["--profile".to_owned(), "work".to_owned()],
        env: Vec::new(),
    };
    let argv = background_argv(Some(&named), Some("s-1"), None, None, None);
    assert_eq!(argv[0], "/opt/claw/bin/claw");
    assert_eq!(argv[1..4], ["--profile", "work", "--bg"]);
    assert_eq!(attach_argv(Some(&named), "a1b2")[0], "/opt/claw/bin/claw");

    let dir = tempfile::tempdir().expect("tempdir");
    let cli = dir.path().join("fake-claude");
    std::fs::write(&cli, "#!/bin/sh\necho \"$0 attach $DEVPIT_ACCOUNT\"\n").expect("script");
    std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let stand_in = crate::running::Runner {
        program: cli.to_string_lossy().into_owned(),
        args: Vec::new(),
        env: vec![("DEVPIT_ACCOUNT".to_owned(), "theaccount".to_owned())],
    };

    // Same retry as the turn tests: a script written a moment ago can be "text
    // file busy" while another test forks with it open.
    let short = (0..5)
        .find_map(
            |_| match start_background(dir.path(), Some(&stand_in), None, None, None, None) {
                Err(AgentError::NotInstalled) => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    None
                }
                other => Some(other),
            },
        )
        .expect("the stand-in never started")
        .expect("a handle");

    assert_eq!(
        short, "theaccount",
        "the profile's environment did not reach the session"
    );
}

/// The spending cap is the contour the whole product turns on: a step that
/// cannot overspend is a step you can leave running.
#[test]
fn a_headless_turn_declares_its_cap() {
    let argv = headless_argv(None, None, Some(0.5), None, None, None);
    let cap = argv
        .iter()
        .position(|a| a == "--max-budget-usd")
        .expect("no cap in the line");
    assert_eq!(argv[cap + 1], "0.5");
}

#[test]
fn a_headless_turn_streams_in_and_out() {
    let argv = headless_argv(None, None, None, None, None, None);
    assert!(argv.contains(&"--input-format".to_owned()));
    assert!(argv.contains(&"--output-format".to_owned()));
    assert_eq!(argv.iter().filter(|a| *a == "stream-json").count(), 2);
    assert!(!argv.contains(&"--session-id".to_owned()));
}

/// A run names the session its turn speaks in, so the card can find it again.
#[test]
fn a_headless_turn_names_its_session() {
    let id = "0190f5a2-7b3c-4d1e-8f00-1234567890ab";
    let argv = headless_argv(None, None, None, None, None, Some(id));
    let at = argv
        .iter()
        .position(|a| a == "--session-id")
        .expect("no session id in the line");
    assert_eq!(argv[at + 1], id);
}
