use super::*;

/*
 * The rate, which is the part a test can hold: everything else here is the
 * kernel answering, and `devpit_pty::usage` covers the reading of it.
 *
 * `tenths` and not a copy of it. The first version of this file reimplemented
 * the arithmetic here, which made every test below prove the test's own
 * arithmetic and nothing about the app — clippy noticed, because the module
 * import went unused.
 */

/// One second of one core, in ticks.
fn second() -> u64 {
    devpit_pty::usage::ticks_per_second()
}

#[test]
fn a_pane_asked_about_once_reports_nothing() {
    // A rate from one sample is not a rate. Dividing a long-running agent's
    // whole life by a second shows it at several hundred percent.
    assert_eq!(tenths(0, 0, 0.0), 0);
}

#[test]
fn one_second_of_cpu_over_one_second_is_one_core() {
    assert_eq!(tenths(0, second(), 1.0), 1000);
}

#[test]
fn half_a_second_of_cpu_over_one_second_is_half_a_core() {
    assert_eq!(tenths(0, second() / 2, 1.0), 500);
}

#[test]
fn two_cores_read_as_two_hundred_percent() {
    // Not clamped: an agent running four tools at once is doing four cores of
    // work, and saying 100% would hide the thing worth seeing.
    assert_eq!(tenths(0, second() * 2, 1.0), 2000);
}

#[test]
fn a_counter_that_went_backwards_reports_nothing() {
    // The pid was reused between samples. A negative rate is not a rate, and
    // an unsigned subtraction there would wrap to something enormous.
    assert_eq!(tenths(500, 100, 1.0), 0);
}

#[test]
fn two_samples_too_close_together_report_nothing() {
    // A few milliseconds apart, one tick of rounding is a whole core.
    assert_eq!(tenths(0, 100, 0.001), 0);
}

#[test]
fn an_idle_pane_reports_zero_rather_than_nothing_at_all() {
    assert_eq!(tenths(400, 400, 2.0), 0);
}
