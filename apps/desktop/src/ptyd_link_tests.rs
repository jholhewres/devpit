use super::*;
use devpit_rpc::{LayoutNode, SessionLayout, SplitDirection};

fn layout() -> SessionLayout {
    let tree = LayoutNode::leaf("leaf_a", "devpit-p:leaf_a")
        .split_leaf(
            "leaf_a",
            SplitDirection::Horizontal,
            LayoutNode::leaf("leaf_b", target("leaf_b")),
        )
        .expect("split");
    SessionLayout {
        project_id: "p".into(),
        focused_id: "leaf_a".into(),
        tree,
    }
}

#[test]
fn a_ptyd_leaf_is_told_apart_by_its_target() {
    let layout = layout();
    assert!(
        !leaf_is_ptyd(&layout, "leaf_a"),
        "a tmux window stays tmux's"
    );
    assert!(leaf_is_ptyd(&layout, "leaf_b"));
    assert!(!leaf_is_ptyd(&layout, "leaf_c"), "not in the layout");
    assert_eq!(ptyd_leaves(&layout), ["leaf_b"]);
}

#[test]
fn a_leaf_has_the_same_pane_number_on_every_run_and_never_zero() {
    assert_eq!(pane_number("leaf_01ABC"), pane_number("leaf_01ABC"));
    assert_ne!(pane_number("leaf_01ABC"), pane_number("leaf_01ABD"));
    assert_ne!(pane_number(""), 0);
}

#[test]
fn it_is_off_unless_the_preference_says_ptyd() {
    let dir = tempfile::tempdir().expect("dir");
    let store = Store::open(&dir.path().join("state.db")).expect("store");
    assert!(!wanted(&store), "off by default");
    store
        .set_preference(preference::TERMINAL_BACKEND, "tmux")
        .expect("set");
    assert!(!wanted(&store));
    store
        .set_preference(preference::TERMINAL_BACKEND, BACKEND)
        .expect("set");
    assert_eq!(wanted(&store), cfg!(unix));
}

/// With the preference on and no daemon to be had, a new terminal is made
/// in tmux as it always was: `in_ptyd` answers nothing, and that is the
/// fallback's whole contract with its callers.
#[cfg(unix)]
#[test]
fn with_no_daemon_a_new_terminal_falls_back_to_tmux() {
    let home = tempfile::tempdir().expect("home");
    let err = devpit_ptyd::launch::connect_or_start(
        None,
        &[],
        home.path(),
        &home.path().join("devpit-ptyd.sock"),
    )
    .expect_err("no daemon, nothing to start it from");
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

/// A daemon of this process's own, serving a home of its own, for as long
/// as it is held.
#[cfg(unix)]
pub(crate) struct Serving {
    pub(crate) home: tempfile::TempDir,
    socket: PathBuf,
}

#[cfg(unix)]
impl Drop for Serving {
    fn drop(&mut self) {
        // Under XDG_RUNTIME_DIR, not the home: it outlives the temp dir.
        let _ = std::fs::remove_file(&self.socket);
    }
}

#[cfg(unix)]
pub(crate) fn serving() -> Serving {
    let home = tempfile::tempdir().expect("home");
    let socket = socket_path(home.path());
    let listener = devpit_ptyd::socket::listen(&socket).expect("listen");
    let daemon = devpit_ptyd::daemon::Daemon::new();
    std::thread::spawn(move || daemon.serve(listener));
    Serving { home, socket }
}

/// The split's question, and every new pane's: nothing about the pane it
/// splits from or what it is for is asked, only whether ptyd is wanted.
#[cfg(unix)]
#[test]
fn a_new_pane_goes_to_ptyd_while_wanted_and_to_tmux_otherwise() {
    let daemon = serving();
    let home = daemon.home.path();
    let store = Store::open(&home.join("state.db")).expect("store");
    let in_tmux = || Ok("devpit_p:leaf_t".to_owned());
    assert_eq!(
        new_pane(&store, Some(home), "p", "leaf_t", home, in_tmux).expect("made"),
        "devpit_p:leaf_t",
        "off: tmux makes it"
    );

    store
        .set_preference(preference::TERMINAL_BACKEND, BACKEND)
        .expect("set");
    let made = new_pane(&store, Some(home), "p", "leaf_s", home, || {
        panic!("tmux was asked while the daemon answered")
    })
    .expect("made");
    assert_eq!(made, target("leaf_s"));
    assert!(running(home, pane_number("leaf_s")));
    kill(home, pane_number("leaf_s"));
}
