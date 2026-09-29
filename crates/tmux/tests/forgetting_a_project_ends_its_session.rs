//! Ending a project's session takes its client sessions with it.
//!
//! Each leaf attaches through a client session grouped with the project's,
//! and the windows belong to the group: killing the project's session alone
//! left every window running under its clients, with nothing able to reach
//! them again once the project was forgotten.

use devpit_tmux::Server;

#[test]
fn a_projects_session_goes_with_every_client_and_no_other() {
    if !Server::available() {
        eprintln!("skip: tmux not on PATH");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let server = Server::scratch(dir.path().join("tmux.sock"));
    let (going, staying) = ("devpit_prj_going", "devpit_prj_going_too");
    for session in [going, staying] {
        server
            .ensure_session(session, "leaf_a", dir.path())
            .expect("ensure");
    }
    server
        .new_window(going, "leaf_b", dir.path())
        .expect("second window");

    server.kill_session(going).expect("kill");

    assert!(!server.has_session(going).expect("asked"));
    for client in ["leaf_a", "leaf_b"] {
        let name = format!("{going}__{client}");
        assert!(
            !server.has_session(&format!("={name}")).expect("asked"),
            "{name} outlived its project"
        );
    }
    assert!(
        server.has_session(&format!("={staying}")).expect("asked"),
        "a project whose name starts the same went too"
    );
    assert!(server
        .has_session(&format!("={staying}__leaf_a"))
        .expect("asked"));
}
