use devpit_rpc::IslandRect;

use super::*;

fn rect(x: f64, y: f64, width: f64, height: f64) -> IslandRect {
    IslandRect {
        x,
        y,
        width,
        height,
    }
}

/// A click on the island's very edge must reach the island, not whatever
/// window sits under the transparent glass around it.
#[test]
fn the_edge_of_the_island_still_takes_the_mouse() {
    let taken = taking(rect(100.0, 0.0, 200.0, 32.0), true);
    assert!(lands(taken, 100.0 - MARGIN + 1.0, 10.0));
    assert!(lands(taken, 300.0 + MARGIN - 1.0, 32.0 + MARGIN - 1.0));
    assert!(!lands(taken, 100.0 - MARGIN - 1.0, 10.0));
    assert!(!lands(taken, 200.0, 32.0 + MARGIN + 1.0));
}

/// The margin never reaches past the window's own corner.
#[test]
fn the_margin_stays_inside_the_window() {
    let taken = taking(rect(4.0, 0.0, 50.0, 20.0), true);
    assert_eq!((taken.x, taken.y), (0.0, 0.0));
}

#[test]
fn nothing_drawn_takes_nothing() {
    let taken = taking(rect(10.0, 10.0, 0.0, 0.0), true);
    assert!(!lands(taken, 10.0, 10.0));
    assert!(!lands(taken, 0.0, 0.0));
}

/// Hidden, only the strip wakes the island: a margin around it would stop
/// the top of every window under it from answering clicks.
#[test]
fn the_wake_strip_takes_no_margin() {
    let taken = taking(rect(260.0, 0.0, 240.0, 4.0), false);
    assert!(lands(taken, 300.0, 2.0));
    assert!(!lands(taken, 300.0, 5.0));
    assert!(!lands(taken, 259.0, 2.0));
}

#[test]
fn a_chat_is_found_by_the_cli_session_it_keeps() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("conv_A.json"),
        r#"{"sessionId":"s-1","title":"a"}"#,
    )
    .expect("a");
    std::fs::write(
        dir.path().join("conv_B.json"),
        r#"{"sessionId":"s-2","title":"b"}"#,
    )
    .expect("b");
    std::fs::write(dir.path().join("conv_B.jsonl"), "not the meta").expect("transcript");
    assert_eq!(
        conversation_of(dir.path(), "s-2").as_deref(),
        Some("conv_B")
    );
    assert_eq!(conversation_of(dir.path(), "s-9"), None);
}

/// Hidden, the watch costs nothing on Linux, where the strip that wakes the
/// island is its input shape; elsewhere it looks four times a second for it.
#[test]
fn the_watch_sleeps_while_the_island_is_hidden() {
    use std::time::Duration;
    assert_eq!(
        super::between_looks(true, true),
        Some(Duration::from_millis(33))
    );
    assert_eq!(
        super::between_looks(true, false),
        Some(Duration::from_millis(33))
    );
    assert_eq!(
        super::between_looks(false, false),
        Some(Duration::from_millis(250))
    );
    assert_eq!(super::between_looks(false, true), None);
}
