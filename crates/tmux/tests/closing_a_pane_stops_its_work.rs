//! Closing a terminal ends what was running in it.
//!
//! This exists because it did not. Killing the tmux window sends `SIGHUP`,
//! and a TUI agent installs a handler for exactly that so it can survive a
//! terminal that went away. Measured: a process that ignores `SIGHUP` outlived
//! `kill-window`, was reparented to init, and went on working with nothing
//! left in the app able to reach it.
//!
//! The close prompt says "closing this terminal will stop the agent's current
//! work". A prompt that says that and does not do it is worse than no prompt.

use std::process::Command;
use std::time::{Duration, Instant};

/// A process that will not leave when asked politely, which is the case.
///
/// `SIGHUP` and `SIGINT` both ignored, so nothing short of `SIGTERM` to the
/// group ends it. Real agents install these handlers for real reasons.
const STUBBORN: &str = "process.on('SIGHUP', () => {});\
                        process.on('SIGINT', () => {});\
                        setInterval(function () {}, 1000);";

#[test]
fn a_process_that_ignores_a_hangup_is_still_stopped() {
    if !devpit_tmux::Server::available() {
        eprintln!("skipped: tmux is not installed");
        return;
    }
    if which("node").is_none() {
        eprintln!("skipped: node is not installed");
        return;
    }

    let home = tempfile::tempdir().expect("a directory");
    let script = home.path().join("claude");
    std::fs::write(&script, STUBBORN).expect("the script");

    let socket = std::env::temp_dir().join("devpit-stopping-test.sock");
    let _ = tmux(&socket, &["kill-server"]);

    let server = devpit_tmux::Server::new(socket.clone());
    server
        .ensure_session("stopping_test", "leaf", home.path())
        .expect("the session");

    let tty = at_a_prompt(&server);
    let _ = server.send_keys("stopping_test:leaf", &format!("node {}", script.display()));

    let front = waiting_for_the_agent(&tty);
    let pgid = front.pgid;
    assert!(alive(pgid), "the agent never started");

    // What the app does when a tab is closed, in the order it does it.
    devpit_pty::stop_group(pgid, devpit_pty::GRACE);
    let _ = server.kill_window("stopping_test", "leaf");

    // Generous, because the point is that it stops at all — not how fast.
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline && alive(pgid) {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        !alive(pgid),
        "the agent outlived the tab that was closed to stop it"
    );

    let _ = tmux(&socket, &["kill-server"]);
}

/// The pane's tty, once its shell is waiting for a person.
fn at_a_prompt(server: &devpit_tmux::Server) -> String {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut settling = devpit_pty::Settling::new();
    loop {
        let pane = one_pane(server);
        let fronts = devpit_pty::looking(std::slice::from_ref(&pane.tty));
        let ready = devpit_pty::front_on(&fronts, &pane.tty).is_some_and(devpit_pty::at_a_prompt);
        if settling.looked(ready) || Instant::now() > deadline {
            return pane.tty;
        }
        std::thread::sleep(Duration::from_millis(90));
    }
}

fn waiting_for_the_agent(tty: &str) -> devpit_pty::Front {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let fronts = devpit_pty::looking(&[tty.to_owned()]);
        if let Some(front) = devpit_pty::front_on(&fronts, tty) {
            let named = devpit_pty::agents::program_of(&front.argv).unwrap_or_default();
            if named == "claude" || Instant::now() > deadline {
                return front.clone();
            }
        }
        if Instant::now() > deadline {
            panic!("the agent never reached the foreground");
        }
        std::thread::sleep(Duration::from_millis(90));
    }
}

fn one_pane(server: &devpit_tmux::Server) -> devpit_tmux::Running {
    server
        .running("stopping_test")
        .expect("the panes")
        .into_iter()
        .next()
        .expect("one pane")
}

/// Signal 0 asks whether a group is there without sending anything.
///
/// Through the syscall, not `/usr/bin/kill`: that binary takes a negative pid
/// as an option and exits zero having done nothing, so a test written on it
/// would report success no matter what.
fn alive(pgid: u32) -> bool {
    // SAFETY: `kill` reads no memory, and signal 0 sends nothing.
    unsafe { libc::kill(-(pgid as i32), 0) == 0 }
}

fn tmux(socket: &std::path::Path, args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new("tmux")
        .arg("-S")
        .arg(socket)
        .args(args)
        .output()
}

fn which(program: &str) -> Option<std::path::PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?).find_map(|dir| {
        let full = dir.join(program);
        full.is_file().then_some(full)
    })
}
