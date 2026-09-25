//! The rule the product turns on, proved against tmux and the agent CLI.
//!
//! One target terminal per project, and a card's session reaches it by being
//! typed into that pane. Everything else about the board is arrangement; this
//! is the part that either works or the product does not exist.
//!
//! Opt-in behind `DEVPIT_LIVE_TURN`: it starts a real agent session, which
//! costs money, and a test that quietly bills someone is a test that gets
//! deleted.

use std::path::Path;
use std::process::Command;

fn skip(why: &str) -> bool {
    eprintln!("skipped: {why}");
    true
}

#[test]
fn a_session_typed_into_the_target_pane_attaches_to_it() {
    if std::env::var_os("DEVPIT_LIVE_TURN").is_none() {
        assert!(skip("set DEVPIT_LIVE_TURN=1 to start a real session"));
        return;
    }
    if !devpit_tmux::Server::available() || !devpit_agentcli::available() {
        assert!(skip("tmux or the agent CLI is not installed"));
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

    // A short socket path, for the reason `Store::root` documents: the kernel
    // caps a unix socket at ~108 bytes and a tempdir path is nowhere near it,
    // but a per-test directory under one would be.
    let socket = std::env::temp_dir().join("devpit-attach-test.sock");
    let server = devpit_tmux::Server::scratch(socket.clone());
    let session = devpit_tmux::Server::session_name("attach_test");
    let leaf = "leaf_attach";

    server
        .ensure_session(&session, leaf, dir.path())
        .expect("the target terminal");
    let target = devpit_tmux::Server::target(&session, leaf);

    let short = devpit_agentcli::start_background(dir.path(), None, None, None, None, None, None)
        .expect("start a session");

    // Exactly what the product types, built by the same function.
    let line = devpit_agentcli::attach_argv(None, &short).join(" ");
    server.send_keys(&target, &line).expect("send the attach");

    // What counts as arrival.
    //
    // A first attach in a folder the agent has not seen before opens its own
    // trust prompt instead of the session — which is the agent protecting the
    // machine, not a failure of ours, and it is just as much proof that the
    // attach landed in this pane. Both are accepted; anything else is not.
    let drew = |pane: &str| {
        pane.contains("auto mode")
            || pane.contains("context")
            || pane.contains("Accessing workspace")
    };

    // Polling rather than a fixed sleep: this is the one place a real UI is
    // being waited on, and a sleep is either flaky or slow.
    let mut pane = String::new();
    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        pane = capture(&socket, &target);
        if drew(&pane) {
            break;
        }
    }

    // The session goes with the server: this test runs its own, on its own
    // socket, and killing it takes every pane and every process in them.
    let _ = Command::new("tmux")
        .args(["-S", socket.to_str().expect("utf-8"), "kill-server"])
        .output();

    assert!(
        drew(&pane),
        "the agent never drew itself in the target pane. What the pane showed:\n{pane}"
    );
}

fn capture(socket: &Path, target: &str) -> String {
    Command::new("tmux")
        .args([
            "-S",
            socket.to_str().expect("utf-8"),
            "capture-pane",
            "-t",
            target,
            "-p",
        ])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .unwrap_or_default()
}
