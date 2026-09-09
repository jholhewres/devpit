//! tmux draws nothing the app draws itself.
//!
//! This exists because the bar came back: `set-option -t <session>` was set on
//! the group, and the client session a pane actually attaches to is a *grouped*
//! session, which does not inherit it. The result was a green tmux bar along
//! the bottom of the app's terminal — plumbing showing through the screen it is
//! meant to sit underneath.

use std::process::Command;

#[test]
fn no_session_on_our_server_draws_a_status_bar() {
    if !devpit_tmux::Server::available() {
        eprintln!("skipped: tmux is not installed");
        return;
    }

    // Short and fixed, for the reason `Store::root` documents: a unix socket
    // path is capped at ~108 bytes.
    let socket = std::env::temp_dir().join("devpit-chrome-test.sock");
    let _ = Command::new("tmux")
        .args(["-S", socket.to_str().expect("utf-8"), "kill-server"])
        .output();

    let server = devpit_tmux::Server::new(socket.clone());
    server
        .ensure_session("chrome_test", "leaf", &std::env::temp_dir())
        .expect("the session");

    let global = option(&socket, &["show-options", "-g", "status"]);

    // Every session, group and client alike. A session with no value of its
    // own inherits the global, which is the point of setting it there.
    let sessions = option(&socket, &["list-sessions", "-F", "#{session_name}"]);
    let overrides: Vec<String> = sessions
        .lines()
        .filter(|name| !name.trim().is_empty())
        .filter_map(|name| {
            let own = option(&socket, &["show-options", "-t", name.trim(), "status"]);
            (own.contains("status on")).then(|| name.trim().to_owned())
        })
        .collect();

    let _ = Command::new("tmux")
        .args(["-S", socket.to_str().expect("utf-8"), "kill-server"])
        .output();

    assert!(
        global.contains("status off"),
        "the server draws a status bar: {global}"
    );
    assert!(
        overrides.is_empty(),
        "these sessions turned the bar back on: {overrides:?}"
    );
}

fn option(socket: &std::path::Path, args: &[&str]) -> String {
    let mut all = vec!["-S", socket.to_str().expect("utf-8")];
    all.extend_from_slice(args);
    Command::new("tmux")
        .args(&all)
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .unwrap_or_default()
}
