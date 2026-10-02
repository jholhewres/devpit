//! A remote viewer is a client of its own: watching a window from a small
//! screen does not shrink it for the desk, and a viewer that may only watch
//! is attached read-only by tmux itself.

use devpit_tmux::Server;

#[cfg(unix)]
#[test]
fn a_remote_viewer_is_read_only_and_ignores_its_size() {
    if !Server::available() {
        eprintln!("skip: tmux not on PATH");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let server = Server::scratch(dir.path().join("tmux.sock"));
    let (session, window) = ("devpit_test_viewer", "leaf_view");
    server
        .ensure_session(session, window, dir.path())
        .expect("ensure");

    let watching = server
        .viewer_argv(session, window, "1", false)
        .expect("argv");
    assert_eq!(
        watching[watching.len() - 2..],
        ["-f".to_owned(), "read-only,ignore-size".to_owned()]
    );
    assert!(watching
        .iter()
        .any(|arg| arg.ends_with(&format!("__r1:{window}"))));

    let typing = server
        .viewer_argv(session, window, "2", true)
        .expect("argv");
    assert_eq!(typing.last().map(String::as_str), Some("ignore-size"));

    // Its own client session, which ending removes.
    let sessions = || {
        let out = std::process::Command::new("tmux")
            .args([
                "-S",
                &dir.path().join("tmux.sock").display().to_string(),
                "list-sessions",
                "-F",
                "#{session_name}",
            ])
            .output()
            .expect("list");
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    assert!(sessions().contains("__r1"));
    server.end_viewer(session, window, "1");
    assert!(!sessions().contains("__r1"));
}
