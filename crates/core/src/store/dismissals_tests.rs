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
        .create_step(&project, "agent", "review", "{}", false)
        .expect("step");
    let run = store.start_run(&card, &step, None).expect("run");
    (dir, store, run)
}

/// Set aside, it stays set aside; brought back, it is not; twice is once.
#[test]
fn a_finding_set_aside_stays_so_until_it_is_brought_back() {
    let (_dir, store, run) = a_run();
    assert!(store.dismissed_findings(&run).expect("read").is_empty());
    store.dismiss_finding(&run, 2, true).expect("dismiss");
    store.dismiss_finding(&run, 2, true).expect("again");
    store.dismiss_finding(&run, 0, true).expect("another");
    assert_eq!(store.dismissed_findings(&run).expect("read"), vec![0, 2]);
    store.dismiss_finding(&run, 2, false).expect("keep");
    assert_eq!(store.dismissed_findings(&run).expect("read"), vec![0]);
}
