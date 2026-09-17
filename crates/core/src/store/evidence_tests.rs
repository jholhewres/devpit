//! What a run ran and what it proved, read back.

use super::*;

/// A store with one card and one step, and a run open on them.
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
        .create_step(&project, "command", "test", "{}", false)
        .expect("step");
    let run = store.start_run(&card, &step, None).expect("run");
    (dir, store, run)
}

fn a_snapshot() -> Ran {
    Ran {
        command: Some("make test".to_owned()),
        in_directory: Some("/w/devpit".to_owned()),
        declared_env: vec!["DEVPIT_BRANCH".to_owned(), "DEVPIT_CARD".to_owned()],
        base_revision: Some("2f0bee9".to_owned()),
        head_revision: Some("ed16baf".to_owned()),
        saw_changes: Some("a1b2c3".to_owned()),
        in_a_worktree: Some(true),
    }
}

#[test]
fn what_a_run_ran_comes_back_as_it_went_in() {
    let (_dir, store, run) = a_run();
    store
        .record_what_ran(&run, &a_snapshot())
        .expect("recorded");

    let read = store.what_ran(&run).expect("read").expect("a run");
    assert_eq!(read, a_snapshot());
    assert!(read.is_known());
}

/// A run from before migration 017 recorded none of this, and the answer is
/// `unknown` rather than a blank that reads like an answer.
#[test]
fn a_run_that_never_said_is_unknown_and_says_so() {
    let (_dir, store, run) = a_run();
    let read = store.what_ran(&run).expect("read").expect("a run");

    assert_eq!(read, Ran::unknown());
    assert!(!read.is_known(), "nothing recorded read as something known");
    assert_eq!(read.declared_env, Vec::<String>::new());
    assert_eq!(read.in_a_worktree, None, "a worktree was invented");
    assert_eq!(read.saw_changes, None, "a standing was invented");
}

#[test]
fn a_run_nobody_knows_has_no_snapshot_at_all() {
    let (_dir, store, _run) = a_run();
    assert_eq!(store.what_ran("run_nothing").expect("read"), None);
}

/// The names of the environment are kept and the values never are: a snapshot
/// of a step's environment is a snapshot of whatever secret was in it.
#[test]
fn only_the_names_of_the_environment_are_kept() {
    let (_dir, store, run) = a_run();
    store
        .record_what_ran(
            &run,
            &Ran {
                declared_env: vec!["DEVPIT_BRANCH".to_owned()],
                ..Ran::unknown()
            },
        )
        .expect("recorded");

    let kept: Option<String> = store
        .conn
        .query_row(
            "SELECT declared_env FROM run WHERE id = ?1",
            [&run],
            |row| row.get(0),
        )
        .expect("read");
    assert_eq!(kept.as_deref(), Some("DEVPIT_BRANCH"));
}

#[test]
fn evidence_comes_back_with_the_shape_it_was_written_in() {
    let (_dir, store, run) = a_run();
    let left = Evidence {
        version: 1,
        payload: r#"{"passed":29,"failed":0}"#.to_owned(),
    };
    store.record_evidence(&run, &left).expect("recorded");

    assert_eq!(store.evidence_of(&run).expect("read"), Some(left));
}

#[test]
fn a_run_that_left_no_evidence_answers_none() {
    let (_dir, store, run) = a_run();
    assert_eq!(store.evidence_of(&run).expect("read"), None);
    assert_eq!(store.evidence_of("run_nothing").expect("read"), None);
}

/// The ceiling is refused before the write, naming the size, rather than
/// truncated into a payload that parses and lies.
///
/// Sabotage: take the check out of `record_evidence` and this stores the lot.
#[test]
fn evidence_past_the_ceiling_is_refused_and_says_how_much() {
    let (_dir, store, run) = a_run();
    let too_much = Evidence {
        version: 1,
        payload: "x".repeat(MOST_EVIDENCE + 1),
    };

    assert_eq!(
        store.record_evidence(&run, &too_much),
        Err(EvidenceError::TooMuch {
            bytes: MOST_EVIDENCE + 1
        })
    );
    assert_eq!(
        store.evidence_of(&run).expect("read"),
        None,
        "the refused payload was written anyway"
    );

    // And what fits still fits: the ceiling is a ceiling, not a wall.
    let exactly = Evidence {
        version: 1,
        payload: "x".repeat(MOST_EVIDENCE),
    };
    assert_eq!(store.record_evidence(&run, &exactly), Ok(()));
    assert_eq!(store.evidence_of(&run).expect("read"), Some(exactly));
}

/// A store edited outside the app can hold more than this build would write.
/// The size is asked of SQLite before the payload is fetched, so the row is
/// refused rather than allocated.
///
/// Sabotage: drop the `length()` query from `evidence_of` and this returns a
/// megabyte and a byte the reader never agreed to hold.
#[test]
fn a_row_written_past_the_ceiling_by_something_else_is_not_read() {
    let (_dir, store, run) = a_run();
    store
        .conn
        .execute(
            "UPDATE run SET evidence = ?2, evidence_version = 1 WHERE id = ?1",
            rusqlite::params![run, "x".repeat(MOST_EVIDENCE + 1)],
        )
        .expect("written behind the app's back");

    assert_eq!(store.evidence_of(&run).expect("read"), None);
}
