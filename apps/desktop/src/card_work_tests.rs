//! What the bell says when a card is played into a step with no undo.

use super::*;

/// The bell is the play button's, because the move stopped starting these.
///
/// `moving::what_runs` answers `None` for an irreversible step — the move
/// stands and the work waits for the button — so the ring that used to live on
/// the move could never fire. Sabotage: make this answer `Some` whatever the
/// step is, and the second half fails.
#[test]
fn a_step_with_no_undo_rings_and_one_that_can_be_undone_does_not() {
    assert_eq!(
        bell_for("deploy", true, "Ship the parser"),
        Some("deploy started on \u{201c}Ship the parser\u{201d}".to_owned()),
    );
    assert_eq!(bell_for("tests", false, "Ship the parser"), None);
}
