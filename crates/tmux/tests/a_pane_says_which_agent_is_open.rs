//! What a terminal is running, measured on a real one.
//!
//! The whole reason the process table is asked at all: tmux answers
//! `#{pane_current_command}` with the *executable's* name, and every agent CLI
//! written in JavaScript runs as `node`. Claude Code, Codex, Gemini and
//! OpenCode all report `node`, so a sidebar reading only tmux showed nothing
//! recognisable while a conversation was happening in front of it.
//!
//! Read off a real pane rather than a fixture, because the fixture is the part
//! that was never in doubt. What was in doubt is whether tmux's tty and `ps`'s
//! tty are the same string, and whether the foreground marker survives a shell
//! starting a program.

use std::process::Command;
use std::time::{Duration, Instant};

/// A script named like an agent, run under `node` the way an agent is.
const AS_AN_AGENT: &str = "node ";

#[test]
fn a_pane_running_a_javascript_agent_is_named_after_the_script() {
    if !devpit_tmux::Server::available() {
        eprintln!("skipped: tmux is not installed");
        return;
    }
    if on_path("node").is_none() {
        eprintln!("skipped: node is not installed");
        return;
    }

    // A script called `claude`, because the name is what recognition reads.
    let home = tempfile::tempdir().expect("a directory");
    let script = home.path().join("claude");
    std::fs::write(&script, "setTimeout(function () {}, 900000)\n").expect("the script");

    let socket = std::env::temp_dir().join("devpit-running-test.sock");
    let _ = Command::new("tmux")
        .args(["-S", socket.to_str().expect("utf-8"), "kill-server"])
        .output();

    let server = devpit_tmux::Server::new(socket.clone());
    server
        .ensure_session("running_test", "leaf", home.path())
        .expect("the session");

    // The shell has to reach its prompt before it can be typed into. Measured
    // on this machine: a fresh tmux window took 1.3 seconds, and a line sent
    // before then was dropped without a trace. This is the same wait
    // `session.launch_agent` does, for the same reason.
    let idle = waiting_for_a_prompt(&server);
    assert!(
        devpit_pty::agents::idle_shell(&idle),
        "a shell at its prompt was read as {idle}"
    );

    let _ = server.send_keys(
        "running_test:leaf",
        &format!("{AS_AN_AGENT}{}", script.display()),
    );

    let running = waiting_for(&server, |pane| named(pane) == "claude");
    assert_eq!(
        running, "claude",
        "the pane was reported as `{running}`, which is the interpreter and not the agent"
    );

    let pane = one_pane(&server);
    let fronts = devpit_pty::looking(std::slice::from_ref(&pane.tty));
    let front = devpit_pty::front_on(&fronts, &pane.tty).expect("something in front of the pane");
    let found = devpit_pty::agents::recognise(&front.argv).expect("an agent");
    assert_eq!(found.id, "claude");
    assert_eq!(found.label, "Claude Code");

    // tmux on its own could not have said this, which is the point.
    assert_eq!(pane.command, "node");

    let _ = Command::new("tmux")
        .args(["-S", socket.to_str().expect("utf-8"), "kill-server"])
        .output();
}

/// The pane's one window, asked of tmux.
fn one_pane(server: &devpit_tmux::Server) -> devpit_tmux::Running {
    server
        .running("running_test")
        .expect("the panes")
        .into_iter()
        .next()
        .expect("one pane")
}

/// What the app would call what is in front of this pane.
fn named(pane: &devpit_tmux::Running) -> String {
    let fronts = devpit_pty::looking(std::slice::from_ref(&pane.tty));
    devpit_pty::front_on(&fronts, &pane.tty)
        .and_then(|front| devpit_pty::agents::program_of(&front.argv))
        .unwrap_or_else(|| pane.command.clone())
}

/// Polls until the pane is a shell waiting for a person.
///
/// The same rule `session.launch_agent` waits on, from the same place: a
/// prompt confirmed across consecutive looks. A copy of it here would be a
/// test that can pass while the product it stands for fails.
fn waiting_for_a_prompt(server: &devpit_tmux::Server) -> String {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut settling = devpit_pty::Settling::new();
    loop {
        let pane = one_pane(server);
        let fronts = devpit_pty::looking(std::slice::from_ref(&pane.tty));
        let front = devpit_pty::front_on(&fronts, &pane.tty);
        let at_prompt = front.is_some_and(devpit_pty::at_a_prompt);
        if settling.looked(at_prompt) || Instant::now() > deadline {
            // The name from the look that settled it, not from a fresh `ps`.
            // Asking twice is asking about two different moments, and a
            // process that started between them makes this a coin toss.
            return front
                .and_then(|seen| devpit_pty::agents::program_of(&seen.argv))
                .unwrap_or(pane.command);
        }
        std::thread::sleep(Duration::from_millis(90));
    }
}

/// Polls until the pane says what is expected, or gives up loudly.
///
/// A shell takes a moment to start a program, and a test that reads once reads
/// the shell. Two seconds is far longer than it takes and short enough that a
/// genuine failure is still a failure rather than a hang.
fn waiting_for(
    server: &devpit_tmux::Server,
    settled: impl Fn(&devpit_tmux::Running) -> bool,
) -> String {
    // Long enough for a heavy startup file. What it is waiting for takes
    // well over a second on a shell somebody actually uses.
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let pane = one_pane(server);
        if settled(&pane) || Instant::now() > deadline {
            return named(&pane);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn on_path(program: &str) -> Option<std::path::PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?).find_map(|dir| {
        let full = dir.join(program);
        full.is_file().then_some(full)
    })
}
