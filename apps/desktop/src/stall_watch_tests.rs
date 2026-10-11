use super::*;

#[test]
fn a_clock_that_ran_lately_is_not_a_stall() {
    let mut clock = Clock::default();
    assert_eq!(clock.look(100, 100 - STALL_AFTER, true), None);
}

#[test]
fn a_stall_is_said_once_and_its_end_once() {
    let mut clock = Clock::default();
    let stuck_at = 100;
    assert_eq!(
        clock.look(stuck_at + STALL_AFTER + 1, stuck_at, true),
        Some(Said::Stuck(STALL_AFTER + 1))
    );
    // Still stuck two seconds later: already said.
    assert_eq!(clock.look(stuck_at + STALL_AFTER + 3, stuck_at, true), None);
    // It ran, forty seconds after it stopped.
    assert_eq!(
        clock.look(stuck_at + 41, stuck_at + 40, true),
        Some(Said::Back(40))
    );
    assert_eq!(clock.look(stuck_at + 43, stuck_at + 42, true), None);
}

#[test]
fn a_page_that_said_it_is_hidden_owes_no_beat() {
    let mut clock = Clock::default();
    assert_eq!(clock.look(1_000, 100, false), None);
}

#[test]
fn a_page_hidden_in_the_middle_of_a_stall_is_not_said_to_be_back() {
    let mut clock = Clock::default();
    assert!(matches!(clock.look(200, 100, true), Some(Said::Stuck(_))));
    assert_eq!(clock.look(202, 100, false), None);
    // And a later stall is said again, as a stall of its own.
    assert!(matches!(clock.look(400, 300, true), Some(Said::Stuck(_))));
}

#[test]
fn waking_from_a_suspend_is_not_a_stall() {
    assert!(!overslept(100 + LOOK_EVERY.as_secs(), 100));
    assert!(overslept(100 + 600, 100));
}
