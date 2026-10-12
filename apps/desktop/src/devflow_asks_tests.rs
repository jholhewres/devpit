use super::*;

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("store");
    (dir, store)
}

#[test]
fn the_click_is_kept_per_project_and_per_change() {
    let (_dir, store) = store();
    assert!(!made(&store, "prj_a", Ask::Enable));
    remember(&store, "prj_a", Ask::Enable, true).expect("kept");
    remember(&store, "prj_b", Ask::Enable, true).expect("kept");
    assert!(made(&store, "prj_a", Ask::Enable));
    assert!(!made(&store, "prj_a", Ask::Enforce));
    remember(&store, "prj_a", Ask::Enable, false).expect("forgotten");
    assert!(!made(&store, "prj_a", Ask::Enable));
    assert!(made(&store, "prj_b", Ask::Enable));
}

#[test]
fn devflow_is_offered_unless_switched_off_in_settings() {
    let (_dir, store) = store();
    assert!(globally_on(&store));
    store
        .set_preference_flag(preference::DEVFLOW, false)
        .expect("off");
    assert!(!globally_on(&store));
}

#[test]
fn a_change_asked_twice_waits_once() {
    raise(None, "prj_twice", "twice", Ask::Enable, false);
    raise(None, "prj_twice", "twice", Ask::Enable, false);
    raise(None, "prj_twice", "twice", Ask::Enforce, false);
    assert_eq!(waiting_in("prj_twice"), [Ask::Enable, Ask::Enforce]);
}

#[test]
fn what_the_person_dropped_is_not_raised_again_by_itself() {
    {
        let mut held = held().lock().expect("held");
        held.dropped
            .push(("prj_dropped".to_owned(), DevflowChange::Enable));
    }
    raise(None, "prj_dropped", "dropped", Ask::Enable, true);
    assert!(waiting_in("prj_dropped").is_empty());
    // An agent asking again is a new request, and the person sees it.
    raise(None, "prj_dropped", "dropped", Ask::Enable, false);
    assert_eq!(waiting_in("prj_dropped"), [Ask::Enable]);
}
