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

#[test]
fn a_folder_that_is_not_a_repository_is_refused_not_reported() {
    let dir = tempfile::tempdir().expect("tempdir");

    let refused = branch_of(dir.path()).expect_err("a plain folder has no branch");

    assert_eq!(refused.code, ErrorCode::Invalid);
}
