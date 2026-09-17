//! What a run proves, read back through the command's own path.

use super::*;

use devpit_rpc::{RunState, Validity, Verdict};

/// A store with a card, a step and a run that ended `state`.
fn a_run_that(state: &str) -> (tempfile::TempDir, Store, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let project = store.add_project(dir.path(), None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &column, "a card", "")
        .expect("card");
    let step = store
        .create_step(&project, "command", "tests", "{}", false)
        .expect("step");
    let run = store.start_run(&card, &step, None).expect("run");
    if state != "running" {
        store
            .finish_run(&run, state, Some("all good"), None, None, Some(0))
            .expect("finish");
    }
    (dir, store, run)
}

/// The one this whole module exists for: a command that exited zero and left
/// no report devpit can read is **not** a pass.
///
/// Sabotage: make `verdict` answer `Passed` for `Ok` with no report and this
/// fails, which is the green tick nobody should be able to ship.
#[test]
fn a_green_command_with_nothing_to_read_is_not_reported_as_passing() {
    let (_dir, store, run) = a_run_that("ok");
    let checked = checked(&store, &run).expect("read");

    assert_eq!(checked.state, RunState::Ok);
    assert_eq!(checked.verdict, Verdict::Inconclusive);
}

/// The three states are three fields. A screen cannot accidentally read one
/// off another if they arrive apart.
#[test]
fn the_three_states_arrive_separately() {
    let (_dir, store, run) = a_run_that("lost");
    let checked = checked(&store, &run).expect("read");

    assert_eq!(checked.state, RunState::Lost);
    assert_eq!(checked.verdict, Verdict::Inconclusive);
    assert_eq!(checked.validity, Validity::Unknown);
}

/// A run from before migration 017 recorded nothing about itself, and the
/// answer says nothing rather than a blank snapshot that reads like one.
#[test]
fn a_run_that_recorded_nothing_carries_no_snapshot() {
    let (_dir, store, run) = a_run_that("ok");
    let checked = checked(&store, &run).expect("read");

    assert_eq!(checked.ran, None);
    assert_eq!(checked.validity, Validity::Unknown);
    assert_eq!(checked.evidence_version, None);
}

/// What the run recorded comes back, with the environment by name only.
#[test]
fn a_run_that_said_what_it_ran_says_it_here() {
    let (dir, store, run) = a_run_that("ok");
    store
        .record_what_ran(
            &run,
            &Ran {
                command: Some("make test".to_owned()),
                in_directory: Some(dir.path().display().to_string()),
                declared_env: vec!["DEVPIT_BRANCH".to_owned()],
                ..Ran::unknown()
            },
        )
        .expect("recorded");

    let said = checked(&store, &run)
        .expect("read")
        .ran
        .expect("a snapshot");
    assert_eq!(said.command.as_deref(), Some("make test"));
    assert_eq!(said.declared_env, ["DEVPIT_BRANCH"]);
    assert_eq!(said.in_a_worktree, None, "a worktree was invented");
}

#[test]
fn a_run_nobody_knows_is_not_found() {
    let (_dir, store, _run) = a_run_that("ok");
    let refused = checked(&store, "run_nothing").expect_err("a refusal");
    assert_eq!(refused.code, ErrorCode::NotFound);
}
