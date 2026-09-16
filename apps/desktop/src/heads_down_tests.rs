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
