//! Arming a pane that is already piped leaves it piped.
//!
//! The tap arms every leaf each time a project is opened, and after the app
//! restarts the panes are still piped from its last run. `pipe-pane -o` is a
//! toggle, so that second arming closed the pipe and the pane went deaf.

use std::time::{Duration, Instant};

use devpit_tmux::Server;

#[test]
fn a_pane_piped_twice_is_still_piped() {
    if !Server::available() {
        eprintln!("skip: tmux not on PATH");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let server = Server::scratch(dir.path().join("tmux.sock"));
    let (session, window) = ("devpit_test_pipe", "leaf_pipe");
    server
        .ensure_session(session, window, dir.path())
        .expect("ensure");
    let target = Server::target(session, window);
    let copy = dir.path().join("copy");
    let pipe = format!("exec cat >> '{}'", copy.display());

    server.pipe_pane(&target, &pipe).expect("first arming");
    server.pipe_pane(&target, &pipe).expect("second arming");
    server.send_keys(&target, "echo still-heard").expect("type");

    let deadline = Instant::now() + Duration::from_secs(5);
    let heard = || std::fs::read_to_string(&copy).unwrap_or_default();
    while !heard().contains("still-heard") && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        heard().contains("still-heard"),
        "the second arming closed the pipe: {:?}",
        heard()
    );
}
