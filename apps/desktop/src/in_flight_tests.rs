use super::*;

/// A run can only be stopped if somebody said which process it is.
///
/// This is the whole of S-6: only agent steps registered a pid, so `stop_run`
/// answered NotFound for a command run, the update waited its five seconds and
/// installed over it, and the run came back `lost` instead of `cancelled`. The
/// command step registers one now (`working.rs`), and the runner hands it over
/// (`crates/steps/src/runner.rs`).
#[test]
fn only_a_run_somebody_watched_can_be_stopped() {
    let in_flight = InFlight::new();
    in_flight.watch("run_1", 42);
    assert_eq!(in_flight.pid_of("run_1"), Some(42));
    assert_eq!(
        in_flight.pid_of("run_2"),
        None,
        "a run nobody watched is the one stop_run answers NotFound for"
    );

    // Forgotten when it ends: a pid outlives its process, and signalling a
    // number the kernel has since given to something else is how a stop kills
    // a stranger.
    in_flight.forget("run_1");
    assert_eq!(in_flight.pid_of("run_1"), None);
}

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
