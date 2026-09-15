use super::*;

#[test]
fn a_stop_is_marked_until_the_run_is_forgotten() {
    let in_flight = InFlight::new();
    in_flight.watch("run_1", 42);
    assert!(!in_flight.was_cancelled("run_1"));
    in_flight.cancel("run_1");
    assert!(in_flight.was_cancelled("run_1"));
    assert!(!in_flight.was_cancelled("run_2"));
    in_flight.forget("run_1");
    assert!(!in_flight.was_cancelled("run_1"));

    // A stop that never reached the process takes its mark back.
    in_flight.cancel("run_3");
    in_flight.uncancel("run_3");
    assert!(!in_flight.was_cancelled("run_3"));
}
