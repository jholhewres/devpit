//! The switch as the app turns it: from the settings, at start, and for the
//! window's own errors.
//!
//! One test, because the switch is process-wide and a second one flipping it
//! at the same time would be testing the race.

use super::*;
use devpit_core::reports::ErrorLog;

#[test]
fn the_switch_follows_the_choice_the_start_and_the_window() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let store = Store::open(&root.join("state.db")).expect("store");
    let log = ErrorLog::at(root);

    // Never asked: the start leaves it off.
    resume_with(Store::open(&root.join("state.db")), root);
    from_window("before anyone chose", None);
    assert!(!log.path().exists(), "off unless chosen");

    // Chosen on: the window's errors are kept, as the window's.
    choose(&store, root, true).expect("on");
    assert_eq!(
        store
            .preference_flag(preference::ERROR_REPORTS)
            .expect("read"),
        Some(true)
    );
    from_window("TypeError: x is undefined", Some("at render"));
    let kept = log.read();
    let window = kept
        .iter()
        .find(|entry| entry.kind == "frontend")
        .expect("the window's error");
    assert_eq!(window.message, "TypeError: x is undefined");

    // Chosen off: at once, and the file goes.
    choose(&store, root, false).expect("off");
    assert!(!log.path().exists(), "off wipes");
    from_window("after", None);
    assert!(!log.path().exists(), "off keeps nothing");

    // A start with the choice on turns it back on.
    store
        .set_preference_flag(preference::ERROR_REPORTS, true)
        .expect("on in the store");
    resume_with(Store::open(&root.join("state.db")), root);
    from_window("after a restart", None);
    assert!(log.path().exists(), "on again after the start");

    reports::switch(root, false).expect("leave it off");
}
