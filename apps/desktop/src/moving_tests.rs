//! The one rule a card landing on a column turns on.
//!
//! Beside `moving.rs` rather than in `board_tests.rs`, where it used to sit:
//! the rule moved out of `board.rs` and a test that stays behind is a test
//! about a file that no longer has the thing it tests.

use super::*;
use devpit_rpc::StepKind;

fn step(irreversible: bool) -> Step {
    Step {
        id: "step_1".to_owned(),
        kind: StepKind::Command,
        name: "deploy".to_owned(),
        config: String::new(),
        irreversible,
    }
}

/// A lane that runs nothing is a lane doing its job.
#[test]
fn a_column_without_a_step_starts_nothing() {
    assert!(what_runs(None).is_none());
}
#[test]
fn an_ordinary_step_runs_on_arrival() {
    assert!(what_runs(Some(&step(false))).is_some());
}
/// A deploy has no undo, so dropping a card on it is not enough — and nor is
/// "move it anyway", which answered a question about a run still going.
#[test]
fn an_irreversible_step_never_runs_from_a_move() {
    assert!(what_runs(Some(&step(true))).is_none());
}

/// A move onto a step an update would refuse is refused whole, before the
/// card is written: the board puts the card back, so the store must not move.
#[test]
fn a_move_an_update_refuses_writes_nothing() {
    let waiting = devpit_rpc::UpdateStatus::Waiting {
        runs: 1,
        turns: 0,
        since: 0.0,
    };
    assert!(refused_before_moving(Some(&step(false)), Some(&waiting)).is_some());
    // Nothing would start, so there is nothing to refuse.
    assert!(refused_before_moving(None, Some(&waiting)).is_none());
    assert!(refused_before_moving(Some(&step(true)), Some(&waiting)).is_none());
    assert!(
        refused_before_moving(Some(&step(false)), Some(&devpit_rpc::UpdateStatus::Idle)).is_none()
    );
    assert!(refused_before_moving(Some(&step(false)), None).is_none());
}

/// A reorder within the lane is not an arrival. Nothing starts, so no run is
/// filed as having come from the lane it never left.
#[test]
fn a_move_within_the_lane_starts_nothing() {
    assert!(what_a_move_runs(Some(&step(false)), true, false).is_none());
    assert!(what_a_move_runs(Some(&step(false)), false, false).is_some());
}

/// "Move it anyway" moves the card. A second run in the same checkout is what
/// `card.play` refuses, and a confirmed move must not get round it.
#[test]
fn a_confirmed_move_over_a_run_starts_no_second_run() {
    assert!(what_a_move_runs(Some(&step(false)), false, true).is_none());
}

/// A run that ends after its card was moved elsewhere on purpose chains from
/// nothing: the lane it stands in now ran a different step, or none.
#[test]
fn a_run_chains_only_from_the_lane_that_ran_it() {
    assert!(crate::chaining::ran_here(Some("step_a"), "step_a"));
    assert!(!crate::chaining::ran_here(Some("step_b"), "step_a"));
    assert!(!crate::chaining::ran_here(None, "step_a"));
}
