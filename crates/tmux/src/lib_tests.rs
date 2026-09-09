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

#[test]
fn availability_follows_the_program_exit() {
    assert!(Server::available_at(Path::new("/bin/true")));
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
