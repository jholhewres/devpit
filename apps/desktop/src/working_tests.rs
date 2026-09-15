use super::*;

#[test]
fn a_stopped_run_is_recorded_as_stopped_not_failed() {
    assert_eq!(end_state(true, false), "cancelled");
    assert_eq!(end_state(true, true), "cancelled");
    assert_eq!(end_state(false, true), "ok");
    assert_eq!(end_state(false, false), "failed");
}
