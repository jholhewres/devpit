use super::*;

#[test]
fn one_failing_check_is_failing_whatever_else_passed() {
    let said = [
        ("COMPLETED", "SUCCESS"),
        ("COMPLETED", "FAILURE"),
        ("IN_PROGRESS", ""),
    ];
    assert_eq!(rollup(said), Some("failing"));
}

#[test]
fn checks_still_going_are_running_and_all_done_are_passing() {
    assert_eq!(
        rollup([("COMPLETED", "SUCCESS"), ("QUEUED", "")]),
        Some("running")
    );
    assert_eq!(
        rollup([("COMPLETED", "SUCCESS"), ("COMPLETED", "SKIPPED")]),
        Some("passing")
    );
    // A status context says its state, not a conclusion.
    assert_eq!(rollup([("PENDING", "")]), Some("running"));
}

#[test]
fn nothing_run_is_nothing_to_say() {
    assert_eq!(rollup(std::iter::empty()), None);
}
