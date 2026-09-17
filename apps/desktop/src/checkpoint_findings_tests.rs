//! What a review found, read back through the command's own path.

use super::*;

use devpit_core::store::{Evidence, Ran};
use devpit_rpc::{Severity, Standing};

fn a_run() -> (tempfile::TempDir, Store, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let project = store.add_project(dir.path(), None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &column, "a card", "")
        .expect("card");
    let step = store
        .create_step(&project, "agent", "review", "{}", false)
        .expect("step");
    let run = store.start_run(&card, &step, None).expect("run");
    store
        .finish_run(&run, "ok", Some("done"), None, None, None)
        .expect("finish");
    (dir, store, run)
}

fn a_review_at(revision: Option<&str>) -> Evidence {
    let at = revision
        .map(|sha| format!("\"{sha}\""))
        .unwrap_or_else(|| "null".to_owned());
    Evidence {
        version: devpit_rpc::REVIEW_EVIDENCE,
        payload: format!(
            r#"{{"findings":[{{"file":"src/main.rs","line":12,"severity":"blocking","why":"the lock is never released"}}],"atRevision":{at},"rubric":"look for unreleased locks"}}"#
        ),
    }
}

#[test]
fn a_review_comes_back_as_findings_with_their_reasons() {
    let (_dir, store, run) = a_run();
    store
        .record_evidence(&run, &a_review_at(None))
        .expect("recorded");

    let found = findings(&store, &run).expect("read");
    assert_eq!(found.findings.len(), 1);
    assert_eq!(found.findings[0].file, "src/main.rs");
    assert_eq!(found.findings[0].line, Some(12));
    assert_eq!(found.findings[0].severity, Severity::Blocking);
    assert_eq!(found.rubric.as_deref(), Some("look for unreleased locks"));
}

/// A run that left no review is not a review that found nothing: one looked at
/// something and the other did not.
#[test]
fn a_run_that_left_no_review_says_so_rather_than_finding_nothing() {
    let (_dir, store, run) = a_run();
    let found = findings(&store, &run).expect("read");

    assert_eq!(found, devpit_rpc::Found::none());
    assert_eq!(found.standing, Standing::Unanchored);
}

/// A shape this build does not know is not one to read as though it were the
/// one it does.
#[test]
fn evidence_in_another_shape_is_not_read_as_a_review() {
    let (_dir, store, run) = a_run();
    store
        .record_evidence(
            &run,
            &Evidence {
                version: devpit_rpc::REVIEW_EVIDENCE + 99,
                payload: r#"{"findings":[]}"#.to_owned(),
            },
        )
        .expect("recorded");

    assert_eq!(
        findings(&store, &run).expect("read"),
        devpit_rpc::Found::none()
    );
}

/// The one the plan asks for: an edit afterwards does not silently re-point
/// the line. The review says which revision it meant, and the screen says it
/// is outdated.
///
/// Sabotage: answer `Current` whenever a revision was recorded and a finding
/// about line 12 of an old commit claims to be about line 12 of this one.
#[test]
fn a_review_of_another_revision_is_outdated_and_keeps_its_own_line() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let project = store.add_project(dir.path(), None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &column, "a card", "")
        .expect("card");
    let step = store
        .create_step(&project, "agent", "review", "{}", false)
        .expect("step");
    let run = store.start_run(&card, &step, None).expect("run");

    // A real repository, so `head_of` answers something to compare against.
    for args in [
        vec!["init", "--initial-branch=main", "-q"],
        vec!["config", "user.email", "test@example.invalid"],
        vec!["config", "user.name", "Test"],
        vec!["config", "commit.gpgsign", "false"],
    ] {
        std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(&args)
            .output()
            .expect("git");
    }
    std::fs::write(dir.path().join("one.txt"), "one\n").expect("write");
    for args in [vec!["add", "-A"], vec!["commit", "-q", "-m", "first"]] {
        std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(&args)
            .output()
            .expect("git");
    }

    store
        .record_what_ran(
            &run,
            &Ran {
                in_directory: Some(dir.path().display().to_string()),
                ..Ran::unknown()
            },
        )
        .expect("recorded");
    store
        .record_evidence(
            &run,
            &a_review_at(Some("0000000000000000000000000000000000000000")),
        )
        .expect("recorded");

    let found = findings(&store, &run).expect("read");
    assert_eq!(found.standing, Standing::Outdated);
    assert_eq!(
        found.findings[0].line,
        Some(12),
        "the line was re-pointed instead of the review being marked old"
    );
    // Both revisions, because a person reading an outdated review needs to
    // know which two.
    assert!(found.at_revision.is_some());
    assert!(found.now.is_some());
    assert_ne!(found.at_revision, found.now);
}

#[test]
fn a_run_nobody_knows_is_not_found() {
    let (_dir, store, _run) = a_run();
    assert_eq!(
        findings(&store, "run_nothing").expect_err("a refusal").code,
        ErrorCode::NotFound
    );
}
