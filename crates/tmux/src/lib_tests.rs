//! The server's tests, kept beside it.

use super::*;

#[test]
fn session_names_are_safe_for_tmux() {
    assert_eq!(Server::session_name("prj_01HXYZ"), "devpit_prj_01HXYZ");
    assert_eq!(
        Server::session_name("prj/weird:name"),
        "devpit_prj_weird_name"
    );
    assert!(Server::session_name("x").starts_with("devpit_"));
}

/// A program that exits zero, wherever this system keeps it.
///
/// `/bin/true` is Linux's; macOS ships it as `/usr/bin/true` and has no
/// `/bin/true` at all, which is how this test failed a release on a Mac while
/// saying nothing about tmux.
fn a_program_that_exits_zero() -> &'static Path {
    ["/bin/true", "/usr/bin/true"]
        .iter()
        .map(Path::new)
        .find(|path| path.exists())
        .expect("no `true` on this system")
}

#[test]
fn availability_follows_the_program_exit() {
    assert!(Server::available_at(a_program_that_exits_zero()));
    assert!(!Server::available_at(Path::new(
        "/path/that/does/not/contain/tmux"
    )));
}

#[test]
fn a_session_survives_and_echoes_through_send_keys() {
    if !Server::available() {
        eprintln!("skip: tmux not on PATH");
        return;
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("tmux.sock");
    let server = Server::new(socket);
    let session = "devpit_test_session";
    let window = "leaf_test";

    server
        .ensure_session(session, window, dir.path())
        .expect("ensure");
    server
        .ensure_session(session, window, dir.path())
        .expect("ensure again");

    let windows = server.list_windows(session).expect("list");
    assert!(
        windows.iter().any(|name| name == window),
        "missing window: {windows:?}"
    );

    let target = Server::target(session, window);
    server
        .send_keys(&target, "echo devpit-tmux-ok")
        .expect("send");

    // The shell needs a beat to print. A tight loop would flake on a
    // loaded machine; 200ms is well above a local echo and still a unit.
    std::thread::sleep(std::time::Duration::from_millis(200));
    let shot = server.capture_pane(&target).expect("capture");
    assert!(
        shot.contains("devpit-tmux-ok"),
        "capture missed the echo: {shot:?}"
    );

    server
        .new_window(session, "leaf_two", dir.path())
        .expect("split window");
    let windows = server.list_windows(session).expect("list after split");
    assert!(windows.iter().any(|name| name == "leaf_two"));

    server.kill_server().expect("kill");
}

#[test]
fn a_window_with_no_shell_configured_is_started_as_tmux_always_did() {
    let server = Server::new(PathBuf::from("/tmp/s.sock"));
    /* Only the pane's name. A person who types `claude` into an unwrapped
    shell has to be as visible as one who picks it from the menu, and the
    environment is the only thing that reaches the agent's own hooks. */
    assert_eq!(
        server.shell_args("leaf_one"),
        ["-e", "DEVPIT_PANE=leaf_one", "-e", "COLORTERM=truecolor"]
    );
}

#[test]
fn a_wrapped_shell_rides_in_after_the_double_dash() {
    /* `--` is what stops tmux reading the shell's own flags as its own. */
    let server = Server::new(PathBuf::from("/tmp/s.sock")).with_shell(Shell {
        program: "/bin/bash".to_owned(),
        args: vec!["--rcfile".to_owned(), "/w/bash/rcfile".to_owned()],
        env: vec![("DEVPIT_SHELL_FEATURES".to_owned(), "marks".to_owned())],
    });
    assert_eq!(
        server.shell_args("leaf_one"),
        [
            "-e",
            "DEVPIT_PANE=leaf_one",
            "-e",
            "COLORTERM=truecolor",
            "-e",
            "DEVPIT_SHELL_FEATURES=marks",
            "--",
            "/bin/bash",
            "--rcfile",
            "/w/bash/rcfile",
        ]
    );
}

#[test]
fn a_pane_reports_its_window_its_terminal_and_its_program() {
    let seen = parse_running("leaf_a /dev/pts/8 zsh\nleaf_b /dev/pts/9 node\n");
    assert_eq!(seen[0].leaf_id, "leaf_a");
    assert_eq!(seen[0].tty, "/dev/pts/8");
    assert_eq!(seen[0].command, "zsh");
    assert_eq!(seen[1].leaf_id, "leaf_b");
    assert_eq!(seen[1].command, "node");
}

#[test]
fn a_line_it_cannot_read_is_dropped_rather_than_guessed_at() {
    /* tmux prints one line per pane and a session with none prints nothing;
    neither is an error worth failing the whole list over. */
    assert!(parse_running("\n  \nleaf_a\nleaf_b /dev/pts/1\n").is_empty());
}

/*
 * Which profile devpit started in a pane.
 *
 * `glm` and `claudin` are the same binary run with the same arguments; they
 * differ in environment alone. Nothing outside the process can tell them
 * apart without reading its environ, which is where its token lives. So the
 * answer is recorded at the moment devpit starts one — in tmux, because tmux
 * is what outlives the app.
 */

#[test]
fn a_pane_devpit_did_not_start_has_no_profile() {
    // tmux prints nothing at all for an unset user option, so the line is one
    // field shorter rather than malformed.
    let rows = crate::shell::parse_running("w1 /dev/pts/3 zsh");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].profile, "");
}

#[test]
fn a_pane_devpit_started_carries_the_profile_id() {
    let rows = crate::shell::parse_running("w1 /dev/pts/3 node 01JGLM");
    assert_eq!(rows[0].profile, "01JGLM");
    assert_eq!(rows[0].command, "node", "the rest still parses");
}

#[test]
fn the_format_asks_tmux_for_the_option() {
    // The parser and the format have to agree, and they are written apart.
    assert!(crate::shell::RUNNING_FORMAT.contains("@devpit_profile"));
}
