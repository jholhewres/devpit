use super::*;

#[test]
fn a_stopped_run_is_recorded_as_stopped_not_failed() {
    assert_eq!(end_state(true, false), "cancelled");
    assert_eq!(end_state(true, true), "cancelled");
    assert_eq!(end_state(false, true), "ok");
    assert_eq!(end_state(false, false), "failed");
}

/// A run whose thread gets no store of its own ends as failed, with the reason
/// on it, instead of reading `running` for ever.
#[test]
fn a_run_that_cannot_open_the_store_ends_as_failed() {
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
    let run = store.start_run(&card, &step, None).expect("start");

    // A real refusal: the store's directory would have to be inside a file.
    let file = dir.path().join("a file");
    std::fs::write(&file, "").expect("file");
    let refused = Store::open(&file.join("state.db"));
    assert!(refused.is_err(), "the store opened under a file");

    assert!(store_for_thread(&store, &run, refused).is_none());
    assert_eq!(
        store.run_state(&run).expect("state").as_deref(),
        Some("failed")
    );
    let row = store.runs(&card).expect("runs").remove(0);
    assert!(row
        .output
        .unwrap_or_default()
        .contains("could not open the store"));
}
