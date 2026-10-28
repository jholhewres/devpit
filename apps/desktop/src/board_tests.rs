//! Reading the board's words back out of the database.
//!
//! Two of these are `match` statements with a catch-all arm, which is exactly
//! the shape that goes wrong silently: a word the schema allows and this
//! build spells differently falls into the default and the screen draws a
//! confident lie. The schema's `CHECK` constraints are the list; these assert
//! this file agrees with it.

use super::*;

/// The `kind` values `CREATE TABLE step` allows.
const KINDS: [&str; 3] = ["agent", "session", "command"];

/// The `state` values `CREATE TABLE run` allows.
const STATES: [&str; 4] = ["running", "ok", "failed", "cancelled"];

#[test]
fn every_step_kind_the_schema_allows_is_read_as_itself() {
    assert_eq!(kind_of("agent"), StepKind::Agent);
    assert_eq!(kind_of("session"), StepKind::Session);
    assert_eq!(kind_of("command"), StepKind::Command);

    // None of them lands in the catch-all, which is the point: the arm exists
    // for a word from a newer build, not for one of these.
    for word in KINDS.iter().filter(|word| **word != "agent") {
        assert_ne!(
            kind_of(word),
            StepKind::Agent,
            "{word} fell through to the default"
        );
    }
}

#[test]
fn every_run_state_the_schema_allows_is_read_as_itself() {
    assert_eq!(state_of("running"), RunState::Running);
    assert_eq!(state_of("ok"), RunState::Ok);
    assert_eq!(state_of("failed"), RunState::Failed);
    assert_eq!(state_of("cancelled"), RunState::Cancelled);

    for word in STATES.iter().filter(|word| **word != "running") {
        assert_ne!(
            state_of(word),
            RunState::Running,
            "{word} fell through to the default"
        );
    }
}

/// A word from a newer build has to land somewhere, and where matters.
///
/// `Agent` for a kind, because an unknown step is at worst one that takes no
/// terminal. `Running` for a state, because a run this build cannot name is
/// one it should not claim has finished.
#[test]
fn a_word_this_build_does_not_know_lands_on_the_cautious_answer() {
    assert_eq!(kind_of("something-newer"), StepKind::Agent);
    assert_eq!(state_of("something-newer"), RunState::Running);
    assert_eq!(state_of(""), RunState::Running);
}

/// #215: the board lists every card's runs without their output, which only
/// the card itself shows.
#[test]
fn the_board_leaves_a_runs_output_to_the_card() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = crate::projects::store().expect("store");
    let project = store.add_project(dir.path(), None).expect("project");
    store.ensure_board(&project).expect("board");
    let first = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &first, "a card", "")
        .expect("card");
    let step = store
        .create_step(&project, "command", "tests", "{}", false)
        .expect("step");
    let run = store.start_run(&card, &step, None).expect("run");
    store
        .finish_run(&run, "ok", Some("a long log"), None, None, Some(0))
        .expect("finish");

    let board = board_still_now(project.clone()).expect("board");
    let tile = board.cards.iter().find(|one| one.id == card).expect("tile");
    assert_eq!(tile.runs[0].output, None, "the board carried the output");
    let detail = crate::cards::detail_of(project, card).expect("detail");
    assert_eq!(detail.runs[0].output.as_deref(), Some("a long log"));
}
