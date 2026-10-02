use super::on_at;

#[test]
fn a_pause_holds_until_its_time_and_not_after() {
    assert!(!on_at(0, 1_000), "not paused");
    assert!(on_at(2_000, 1_000));
    assert!(!on_at(2_000, 2_000), "it ends at its time");
    assert!(on_at(i64::MAX, 1_000), "until resumed");
}
