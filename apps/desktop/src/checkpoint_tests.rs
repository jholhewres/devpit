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

/// Who asked and what did the work arrive as two answers, and a run that said
/// neither says neither.
#[test]
fn whose_a_run_was_arrives_as_two_answers() {
    let (_dir, store, run) = a_run_that("ok");
    store
        .record_whose_run(
            &run,
            &WhoseRun {
                asked: Asked::Checkpoint,
                asked_from: Some("tab_that_is_gone".to_owned()),
                carried: Carried::Agent {
                    profile: Some("glm".to_owned()),
                },
            },
        )
        .expect("recorded");

    let whose = checked(&store, &run).expect("read").whose;
    assert_eq!(whose.asked.as_deref(), Some("checkpoint"));
    assert_eq!(whose.carried.as_deref(), Some("agent"));
    assert_eq!(whose.profile.as_deref(), Some("glm"));
    // Kept as a reference, not looked up: the pane may be gone, and finding
    // another that shares its name would point at work that is not this run's.
    assert_eq!(whose.asked_from.as_deref(), Some("tab_that_is_gone"));
}

/// Sabotage: default `asked` to "board" and every run from before this was
/// recorded claims to have come from a drag nobody made.
#[test]
fn a_run_that_never_said_whose_it_was_says_nothing() {
    let (_dir, store, run) = a_run_that("ok");
    let whose = checked(&store, &run).expect("read").whose;

    assert_eq!(whose.asked, None);
    assert_eq!(whose.carried, None);
    assert_eq!(whose.profile, None);
    assert_eq!(whose.asked_from, None);
}

/// A local process carries no profile, and inventing one would name an account
/// that never touched this run.
#[test]
fn a_run_a_process_carried_out_names_no_account() {
    let (_dir, store, run) = a_run_that("ok");
    store
        .record_whose_run(
            &run,
            &WhoseRun {
                asked: Asked::Board,
                asked_from: None,
                carried: Carried::Process,
            },
        )
        .expect("recorded");

    let whose = checked(&store, &run).expect("read").whose;
    assert_eq!(whose.carried.as_deref(), Some("process"));
    assert_eq!(whose.profile, None);
}

/// A review with a blocking finding is a check that failed, and the panel says
/// `Failed` rather than the `Inconclusive` a command with no parser gets.
///
/// Sabotage: read a review's findings as a pass and an agent that found a bug
/// reports green.
#[test]
fn a_review_that_found_something_blocking_is_a_failure() {
    let (_dir, store, run) = a_run_that("ok");
    store
        .record_evidence(
            &run,
            &devpit_core::store::Evidence {
                version: devpit_rpc::REVIEW_EVIDENCE,
                payload: r#"{"findings":[{"file":"a.rs","line":1,"severity":"blocking","why":"x"}],"atRevision":null,"rubric":null}"#.to_owned(),
            },
        )
        .expect("recorded");

    assert_eq!(
        checked(&store, &run).expect("read").verdict,
        Verdict::Failed
    );
}

#[test]
fn a_review_that_found_nothing_blocking_is_a_pass() {
    let (_dir, store, run) = a_run_that("ok");
    store
        .record_evidence(
            &run,
            &devpit_core::store::Evidence {
                version: devpit_rpc::REVIEW_EVIDENCE,
                payload: r#"{"findings":[{"file":"a.rs","line":1,"severity":"noted","why":"x"}],"atRevision":null,"rubric":null}"#.to_owned(),
            },
        )
        .expect("recorded");

    assert_eq!(
        checked(&store, &run).expect("read").verdict,
        Verdict::Passed
    );
}

/// A shape this build does not know is not a shape to read as though it were
/// the one it does.
///
/// Sabotage: drop the version check and a payload from a later devpit is read
/// as a review, giving a verdict from fields that mean something else.
#[test]
fn evidence_in_a_shape_this_build_does_not_know_gives_no_verdict() {
    let (_dir, store, run) = a_run_that("ok");
    store
        .record_evidence(
            &run,
            &devpit_core::store::Evidence {
                version: devpit_rpc::REVIEW_EVIDENCE + 99,
                payload: r#"{"findings":[]}"#.to_owned(),
            },
        )
        .expect("recorded");

    assert_eq!(
        checked(&store, &run).expect("read").verdict,
        Verdict::Inconclusive
    );
}

/// A run that ended badly is not rescued by a review that read clean: the
/// process said something went wrong, and a review is not a contradiction of
/// that.
#[test]
fn a_review_does_not_rescue_a_run_whose_process_vanished() {
    let (_dir, store, run) = a_run_that("lost");
    store
        .record_evidence(
            &run,
            &devpit_core::store::Evidence {
                version: devpit_rpc::REVIEW_EVIDENCE,
                payload: r#"{"findings":[],"atRevision":null,"rubric":null}"#.to_owned(),
            },
        )
        .expect("recorded");

    assert_eq!(
        checked(&store, &run).expect("read").verdict,
        Verdict::Inconclusive
    );
}
