use super::*;
use devpit_tmux::{Running, Snapped};

#[test]
fn a_view_session_is_named_for_its_leaf_in_words_tmux_takes() {
    assert_eq!(view_session("leaf_01ABC"), "ptyd-view_leaf_01ABC");
    assert_eq!(
        view_session("a:b.c d"),
        "ptyd-view_a_b_c_d",
        "tmux reads : and . as a target"
    );
    assert_ne!(view_session("leaf_a"), view_session("leaf_b"));
}

/// The snapshot and the session list read a `devpit_` session as a project's:
/// a view of a pane must never be one, or its pane is listed twice.
#[test]
fn a_view_session_is_no_projects() {
    let session = view_session("leaf_01ABC");
    assert!(!session.starts_with("devpit_"));
    let row = Snapped {
        session,
        pane_id: "%9".into(),
        running: Running {
            leaf_id: "0".into(),
            command: "devpit".into(),
            tty: "/dev/pts/9".into(),
            profile: String::new(),
        },
    };
    assert!(crate::pane_snapshot::projects(&[row]).is_empty());
}

#[test]
fn the_attach_line_names_the_socket_and_quotes_only_what_needs_it() {
    assert_eq!(
        attach_line(Path::new("/home/me/.devpit/tmux.sock"), "ptyd-view_leaf_1"),
        "tmux -S /home/me/.devpit/tmux.sock attach -t ptyd-view_leaf_1"
    );
    assert_eq!(
        attach_line(
            Path::new("/home/my user/.devpit/tmux.sock"),
            "devpit_p__l:l"
        ),
        "tmux -S '/home/my user/.devpit/tmux.sock' attach -t devpit_p__l:l"
    );
    assert_eq!(
        attach_line(Path::new("/x/it's;rm"), "s"),
        r"tmux -S '/x/it'\''s;rm' attach -t s"
    );
}
