use super::*;

/// The size a window last told a pane outlives the app: a screen drawn
/// again after a restart, with no window yet, is drawn at it.
#[test]
fn a_screen_is_drawn_at_the_size_last_told_even_after_a_restart() {
    let home = tempfile::tempdir().expect("home");
    let file = home.path().join(SIZES);
    let mut before = Screens::default();
    before.keep_at(file.clone());
    before.sized(7, 10, 40);

    let mut after = Screens::default();
    after.keep_at(file);
    after.output(7, "x".repeat(50).as_bytes());
    let shown = after.contents(7).expect("a screen");
    assert_eq!(shown.lines().next(), Some("x".repeat(40).as_str()));

    after.sized(7, 10, 80);
    after.output(7, b"\x1b[2J\x1b[H");
    after.output(7, "y".repeat(50).as_bytes());
    let shown = after.contents(7).expect("a screen");
    assert_eq!(shown.lines().next(), Some("y".repeat(50).as_str()));
}

#[test]
fn a_replay_starts_the_screen_over_and_output_goes_on_from_it() {
    let mut screens = Screens::default();
    screens.output(3, b"from before the daemon went\r\n");
    screens.replay(3, b"\x1b[?2004h");
    screens.replay(3, b"replayed\r\n");
    screens.output(3, b"live");
    let shown = screens.contents(3).expect("a screen");
    assert!(!shown.contains("from before"), "{shown}");
    assert!(shown.contains("replayed\nlive"), "{shown}");
    assert!(screens.modes(3).expect("modes").bracketed_paste);
}
