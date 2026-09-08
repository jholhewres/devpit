//! The board commands' tests, kept beside them.

use super::*;

fn step(irreversible: bool) -> Step {
    Step {
        id: "step_1".to_owned(),
        kind: StepKind::Command,
        name: "deploy".to_owned(),
        config: "{}".to_owned(),
        irreversible,
    }
}

/// A lane that runs nothing is a lane doing its job.
#[test]
fn a_column_without_a_step_starts_nothing() {
    assert!(what_runs(None, false).is_none());
    assert!(what_runs(None, true).is_none());
}

#[test]
fn an_ordinary_step_runs_on_arrival() {
    assert!(what_runs(Some(&step(false)), false).is_some());
}

/// A deploy has no undo, so dropping a card on it is not enough.
#[test]
fn an_irreversible_step_waits_to_be_asked() {
    assert!(what_runs(Some(&step(true)), false).is_none());
    assert!(what_runs(Some(&step(true)), true).is_some());
}
