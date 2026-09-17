//! Whose a run was, read back.

use super::*;

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
        .create_step(&project, "command", "tests", "{}", false)
        .expect("step");
    let run = store.start_run(&card, &step, None).expect("run");
    (dir, store, run)
}

/// Asked by one thing and carried out by another: the shape a single "who"
/// column would have lost.
///
/// Sabotage: write both into one column and this fails on whichever was
/// written second.
#[test]
fn who_asked_and_what_did_the_work_are_two_answers() {
    let (_dir, store, run) = a_run();
    let whose = WhoseRun {
        asked: Asked::Checkpoint,
        asked_from: Some("tab_terminal_3".to_owned()),
        carried: Carried::Agent {
            profile: Some("glm".to_owned()),
        },
    };
    store.record_whose_run(&run, &whose).expect("recorded");

    assert_eq!(store.whose_run(&run).expect("read"), Some(whose));
}

#[test]
fn a_local_process_carries_no_profile() {
    let (_dir, store, run) = a_run();
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

    let read = store.whose_run(&run).expect("read").expect("a run");
    assert_eq!(read.carried, Carried::Process);
    assert_eq!(read.asked, Asked::Board);
}

/// Every run from before migration 019 said nothing, and nothing is `Unknown`
/// on both counts rather than a default that reads like an answer.
#[test]
fn a_run_that_never_said_is_unknown_on_both_counts() {
    let (_dir, store, run) = a_run();
    let read = store.whose_run(&run).expect("read").expect("a run");

    assert_eq!(read, WhoseRun::unknown());
    assert!(!read.is_known());
    assert_eq!(read.asked, Asked::Unknown);
    assert_eq!(read.carried, Carried::Unknown);
}

/// A surface a later build invents is a word this one does not know, and an
/// unknown word is not a meaning to guess at.
#[test]
fn a_surface_this_build_does_not_know_reads_as_unknown() {
    let (_dir, store, run) = a_run();
    store
        .conn
        .execute(
            "UPDATE run SET asked_by = 'from-the-future' WHERE id = ?1",
            [&run],
        )
        .expect("written");

    assert_eq!(
        store.whose_run(&run).expect("read").expect("a run").asked,
        Asked::Unknown
    );
}

/// The reference is kept as a reference. Nothing here looks up what it names,
/// so a terminal that has closed cannot be confused with another that shares
/// its name.
#[test]
fn the_reference_is_kept_and_never_resolved() {
    let (_dir, store, run) = a_run();
    store
        .record_whose_run(
            &run,
            &WhoseRun {
                asked: Asked::Card,
                asked_from: Some("tab_that_is_gone".to_owned()),
                carried: Carried::Process,
            },
        )
        .expect("recorded");

    assert_eq!(
        store
            .whose_run(&run)
            .expect("read")
            .expect("a run")
            .asked_from,
        Some("tab_that_is_gone".to_owned())
    );
}

#[test]
fn a_run_nobody_knows_has_nobody() {
    let (_dir, store, _run) = a_run();
    assert_eq!(store.whose_run("run_nothing").expect("read"), None);
}
