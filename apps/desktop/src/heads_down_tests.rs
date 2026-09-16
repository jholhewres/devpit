//! What a stored focus reads back as.

use super::*;

/// The row is two fields in one string, so every way it can be wrong is a way
/// the app could open into a focus nobody is in.
#[test]
fn a_focus_reads_back_or_reads_as_none() {
    let read = parse(Some("prj_1:1700000000")).expect("a focus");
    assert_eq!(read.project_id, "prj_1");
    assert_eq!(read.since, 1_700_000_000.0);

    assert!(parse(None).is_none(), "no row at all");
    assert!(parse(Some("")).is_none(), "the row a focus is ended with");
    assert!(parse(Some("prj_1")).is_none(), "no clock");
    assert!(parse(Some(":1700000000")).is_none(), "no project");
    assert!(
        parse(Some("prj_1:soon")).is_none(),
        "a clock that is not one"
    );
}

/// The window and the store have to mean the same instant.
///
/// `focus_write` answered a fractional second and wrote a whole one, so the
/// window thought the focus began after the store did. A notice rung inside
/// that gap carried the whole second, compared as older than the focus, and
/// was never held — which is what the end-to-end test caught.
#[test]
fn the_second_a_focus_began_is_the_one_that_was_written() {
    let began = now();
    assert_eq!(
        began,
        began.trunc(),
        "a fraction of a second reached the window"
    );

    // And it survives the round trip through the row unchanged.
    let written = format!("prj_1:{}", began as i64);
    assert_eq!(parse(Some(&written)).expect("a focus").since, began);
}
