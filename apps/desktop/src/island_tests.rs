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
