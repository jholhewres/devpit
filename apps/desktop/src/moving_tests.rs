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
