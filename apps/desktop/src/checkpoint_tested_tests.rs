//! A run's test reports, read back through the command's own path and the
//! verdict it earns.

use super::*;

use devpit_core::store::Evidence;
use devpit_rpc::Verdict;

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
    store
        .finish_run(&run, state, Some("done"), None, None, Some(0))
        .expect("finish");
    (dir, store, run)
}

/// What `steps::tested::kept` writes for a report folder holding `report`.
fn kept(store: &Store, run: &str, report: &str) {
    let folder = tempfile::tempdir().expect("tempdir");
    std::fs::write(folder.path().join("report.json"), report).expect("write");
    crate::steps::tested::kept(store, run, Some(folder.path()), None, folder.path());
}

const PASSED: &str = r#"{"numPassedTests":2,"numFailedTests":0,"testResults":[]}"#;

#[test]
fn a_green_command_whose_tests_passed_is_a_pass() {
    let (_dir, store, run) = a_run_that("ok");
    kept(&store, &run, PASSED);

    let checked = crate::checkpoint::checked(&store, &run).expect("read");
    assert_eq!(checked.verdict, Verdict::Passed);
    assert_eq!(tested(&store, &run).expect("read").passed, 2);
}

#[test]
fn a_failed_test_is_a_failure_and_is_named() {
    let (_dir, store, run) = a_run_that("failed");
    kept(
        &store,
        &run,
        r#"{"numPassedTests":1,"numFailedTests":1,"testResults":[{"name":"/x/a.test.ts","status":"failed","assertionResults":[{"fullName":"adds","status":"failed","failureMessages":["expected 1 to be 2"]}]}]}"#,
    );

    let checked = crate::checkpoint::checked(&store, &run).expect("read");
    assert_eq!(checked.verdict, Verdict::Failed);
    let tested = tested(&store, &run).expect("read");
    assert_eq!(tested.failures[0].name, "adds");
    assert_eq!(
        tested.failures[0].message.as_deref(),
        Some("expected 1 to be 2")
    );
}

/// The runner matched nothing and said so in a report: still no result.
#[test]
fn a_report_of_zero_tests_is_not_a_pass() {
    let (_dir, store, run) = a_run_that("ok");
    kept(
        &store,
        &run,
        r#"{"numPassedTests":0,"numFailedTests":0,"testResults":[]}"#,
    );

    let checked = crate::checkpoint::checked(&store, &run).expect("read");
    assert_eq!(checked.verdict, Verdict::Inconclusive);
}

#[test]
fn a_run_that_left_no_report_reads_as_none() {
    let (_dir, store, run) = a_run_that("ok");
    assert!(tested(&store, &run).expect("read").read_from.is_empty());
}

/// A review's evidence is not a test report, and is not read as one.
#[test]
fn another_shape_of_evidence_is_not_read_as_tests() {
    let (_dir, store, run) = a_run_that("ok");
    store
        .record_evidence(
            &run,
            &Evidence {
                version: devpit_rpc::REVIEW_EVIDENCE,
                payload: PASSED.to_owned(),
            },
        )
        .expect("recorded");
    assert!(tested(&store, &run).expect("read").read_from.is_empty());
}

/// The folder a run is given is empty, and gone once read.
#[test]
fn a_run_gets_an_empty_folder_and_it_is_removed_after() {
    let (dir, store, run) = a_run_that("ok");
    let card = store.run_card(&run).expect("read").expect("a card");
    let root = dir.path().join("home");
    let folder = crate::steps::tested::fresh_folder(&store, &root, &card, &run).expect("folder");
    std::fs::write(folder.join("stale.json"), PASSED).expect("write");
    let again = crate::steps::tested::fresh_folder(&store, &root, &card, &run).expect("folder");
    assert_eq!(std::fs::read_dir(&again).expect("read").count(), 0);

    crate::steps::tested::kept(&store, &run, Some(&again), None, dir.path());
    assert!(!again.exists());
}

/// A failing test that printed a profile's secret does not keep it.
///
/// Sabotage: drop the `kept_out` pass in `steps::tested::kept` and the run's
/// evidence holds the token.
#[test]
fn a_profile_secret_does_not_reach_a_kept_failure() {
    let (_dir, store, run) = a_run_that("failed");
    crate::agent_profiles::save(
        &store,
        &[devpit_rpc::Declared {
            id: "glm".to_owned(),
            label: "GLM".to_owned(),
            base: "claude".to_owned(),
            command: "claude".to_owned(),
            args: vec![],
            models: Vec::new(),
            env: vec![devpit_rpc::EnvVar {
                name: "ANTHROPIC_AUTH_TOKEN".to_owned(),
                value: "sk-live-0123456789abcdef".to_owned(),
            }],
        }],
    )
    .expect("saved");
    kept(
        &store,
        &run,
        r#"{"numPassedTests":0,"numFailedTests":1,"testResults":[{"name":"/x/a.test.ts","status":"failed","assertionResults":[{"fullName":"auth","status":"failed","failureMessages":["sent sk-live-0123456789abcdef and got 401"]}]}]}"#,
    );

    let evidence = store.evidence_of(&run).expect("read").expect("kept");
    assert!(
        !evidence.payload.contains("sk-live-0123456789abcdef"),
        "{}",
        evidence.payload
    );
    assert!(evidence.payload.contains("[hidden by devpit]"));
}
