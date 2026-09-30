//! A new window can be piped from its first byte, and a pane says where its
//! shell stands before the shell has said anything.
//!
//! Both are what a split needs: it opens in the folder of the pane it splits
//! from — a card's worktree, not the project root — and its tap hears the
//! first prompt, which is where the shell names its folder and so its branch.

use std::time::{Duration, Instant};

use devpit_tmux::Server;

fn until(what: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !what() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    what()
}

#[test]
fn a_window_made_piped_is_heard_from_its_first_byte() {
    if !Server::available() {
        eprintln!("skip: tmux not on PATH");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let server = Server::scratch(dir.path().join("tmux.sock"));
    let session = "devpit_test_split";
    server
        .ensure_session(session, "leaf_one", dir.path())
        .expect("ensure");
    let copy = dir.path().join("copy");
    let pipe = devpit_tmux::copy_to(&copy, true);

    server
        .new_window_piped(session, "leaf_two", dir.path(), &pipe)
        .expect("piped window");
    server
        .send_keys(&Server::target(session, "leaf_two"), "echo first-words")
        .expect("type");

    let heard = || std::fs::read_to_string(&copy).unwrap_or_default();
    assert!(
        until(|| heard().contains("first-words")),
        "nothing reached the pipe: {:?}",
        heard()
    );
}

#[test]
fn a_pane_says_the_folder_its_shell_stands_in() {
    if !Server::available() {
        eprintln!("skip: tmux not on PATH");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let worktree = dir.path().join("worktree");
    std::fs::create_dir(&worktree).expect("worktree");
    let server = Server::scratch(dir.path().join("tmux.sock"));
    let session = "devpit_test_where";
    server
        .ensure_session(session, "leaf_one", dir.path())
        .expect("ensure");
    server
        .send_keys(
            &Server::target(session, "leaf_one"),
            &format!("cd '{}'", worktree.display()),
        )
        .expect("cd");

    let wanted = worktree.canonicalize().expect("canonical");
    let stands = || {
        server
            .pane_path(session, "leaf_one")
            .and_then(|path| path.canonicalize().ok())
    };
    assert!(
        until(|| stands().as_ref() == Some(&wanted)),
        "the pane said {:?}",
        stands()
    );
    assert!(server
        .pane_pid(session, "leaf_one")
        .is_some_and(|pid| pid > 1));
}
