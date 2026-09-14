//! The ceilings, called rather than restated.
//!
//! Nothing else stops a read from pulling a whole file into memory and handing
//! it to a window that then has to draw it, so both the threshold and the
//! sentence it produces are worth a test.

use super::*;

#[test]
fn a_small_file_is_opened() {
    assert_eq!(past_the_ceiling("a.txt", 4_096), None);
}

/// Exactly at the ceiling still opens: the rule is "past it", and an
/// off-by-one here would refuse a file the message says is allowed.
#[test]
fn a_file_exactly_at_the_ceiling_is_opened() {
    assert_eq!(past_the_ceiling("a.txt", MOST_BYTES), None);
    assert_eq!(too_big_to_draw("a.png", MOST_MEDIA_BYTES), None);
}

#[test]
fn a_file_past_the_ceiling_is_refused_with_its_size() {
    let said = past_the_ceiling("big.bin", MOST_BYTES + 1).expect("refused");
    assert!(
        said.contains("big.bin"),
        "the sentence lost the path: {said}"
    );
    assert!(
        said.contains("2 MB"),
        "the sentence lost the ceiling: {said}"
    );
}

/// A picture travels as a data URL, which is a third bigger than the bytes it
/// carries and crosses the IPC boundary as a string — so its ceiling is its
/// own, and the sentence has to name the smaller number.
#[test]
fn a_picture_past_its_own_ceiling_is_refused_with_that_ceiling() {
    let said = too_big_to_draw("huge.png", MOST_MEDIA_BYTES + 1).expect("refused");
    assert!(said.contains("huge.png"), "the sentence lost the path");
    assert!(
        said.contains("8 MB"),
        "the sentence lost the ceiling: {said}"
    );
    assert!(
        said.contains("draws"),
        "the sentence is the wrong one: {said}"
    );
}

#[test]
fn something_that_is_not_a_file_says_so_by_name() {
    assert!(not_a_file("taps/leaf_1.fifo").starts_with("taps/leaf_1.fifo is not a file"));
}
