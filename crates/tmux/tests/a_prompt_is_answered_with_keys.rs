//! Answering a prompt is keys pressed on its program: arrows to the choice,
//! then Enter, or Escape. `cat -v` shows what it was sent, so the assertion is
//! about the bytes the program got, not about how tmux was called.

use devpit_tmux::{Key, Server};

#[test]
fn a_prompt_is_answered_with_named_keys_only() {
    if !Server::available() {
        eprintln!("skip: tmux not on PATH");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let server = Server::new(dir.path().join("tmux.sock"));
    let (session, window) = ("devpit_test_keys", "leaf_keys");
    server
        .ensure_session(session, window, dir.path())
        .expect("ensure");
    let target = Server::target(session, window);
    server.send_keys(&target, "cat -v").expect("start cat");
    std::thread::sleep(std::time::Duration::from_millis(300));

    server
        .press(
            &target,
            &[Key::Down, Key::Down, Key::Up, Key::Escape, Key::Enter],
        )
        .expect("press");
    std::thread::sleep(std::time::Duration::from_millis(300));
    let shot = server.capture_pane(&target).expect("capture");
    assert!(
        shot.contains("^[[B^[[B^[[A^["),
        "cat was not sent the arrows and Escape: {shot:?}"
    );
}
