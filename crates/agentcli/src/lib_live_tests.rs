//! The two tests that need the CLI itself installed, and one that spends.
//!
//! Apart from `lib_tests.rs` because those are pure argv assertions that run
//! anywhere, and these two step aside when the binary is missing.

use super::*;

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

    let short = start_background(dir.path(), None, None, None, None, None)
        .expect("the CLI started a session but no handle came back");
    assert!(is_handle(&short), "{short} is not a handle");

    // A session that has just been started may not have reported a state
    // yet. Waiting is the honest test; asserting immediately would be
    // asserting on a race.
    let mut status = Status::Unknown;
    for _ in 0..20 {
        if let Some(ours) = list(None, Some(dir.path()))
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

    // Straight to the CLI: devpit itself never stops a background session —
    // they outlive the window on purpose — so there is no wrapper to call.
    let _ = std::process::Command::new(PROGRAM)
        .args(["stop", &short])
        .output();
}

/// Runs against the installed binary, and steps aside when there is none
/// so CI does not depend on it.
#[test]
fn the_installed_cli_answers_with_the_shape_we_decode() {
    if !available() {
        eprintln!("skipped: the agent CLI is not on PATH");
        return;
    }
    let sessions = list(None, None).expect("agents --json");
    for session in &sessions {
        assert!(!session.session_id.is_empty(), "a session with no id");
    }
}
