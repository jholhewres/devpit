//! A view session is a plain detached session running one command: asked for
//! twice, it is made once, and a session whose name merely starts with its
//! name does not count as it.

use devpit_tmux::Server;

#[cfg(unix)]
#[test]
fn a_view_session_is_made_once_and_runs_its_command_with_its_env() {
    if !Server::available() {
        eprintln!("skip: tmux not on PATH");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("tmux.sock");
    let server = Server::scratch(socket.clone());
    let marker = dir.path().join("ran");
    let argv = [
        "/bin/sh".to_owned(),
        "-c".to_owned(),
        "echo \"$VIEW_SAYS\" >> \"$VIEW_FILE\"; exec sleep 30".to_owned(),
    ];
    let env = [
        ("VIEW_SAYS".to_owned(), "viewed".to_owned()),
        ("VIEW_FILE".to_owned(), marker.display().to_string()),
    ];
    server
        .ensure_view("ptyd-view_leaf_1x", &env, &argv)
        .expect("a longer name");
    server
        .ensure_view("ptyd-view_leaf_1", &env, &argv)
        .expect("made");
    server
        .ensure_view("ptyd-view_leaf_1", &env, &argv)
        .expect("kept");

    let sessions = std::process::Command::new("tmux")
        .args([
            "-S",
            &socket.display().to_string(),
            "list-sessions",
            "-F",
            "#{session_name}",
        ])
        .output()
        .expect("list");
    let mut names: Vec<String> = String::from_utf8_lossy(&sessions.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    names.sort();
    assert_eq!(names, ["ptyd-view_leaf_1", "ptyd-view_leaf_1x"]);

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let ran = || std::fs::read_to_string(&marker).unwrap_or_default();
    while std::time::Instant::now() < deadline && ran().lines().count() < 2 {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert_eq!(
        ran(),
        "viewed\nviewed\n",
        "each session ran its command once"
    );
}
