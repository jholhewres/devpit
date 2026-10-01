use std::time::Duration;

use super::{fire, sleep_for, snooze, KIND};

#[test]
fn it_sleeps_until_the_next_reminder_and_no_longer_than_half_an_hour() {
    assert_eq!(sleep_for(1_000, None, true), None, "nothing to remind of");
    assert_eq!(sleep_for(1_000, Some(1_060), false), None, "turned off");
    assert_eq!(
        sleep_for(1_000, Some(1_060), true),
        Some(Duration::from_secs(60))
    );
    assert_eq!(
        sleep_for(1_000, Some(900), true),
        Some(Duration::ZERO),
        "already due"
    );
    assert_eq!(
        sleep_for(1_000, Some(1_000 + 6 * 3600), true),
        Some(Duration::from_secs(30 * 60)),
        "a machine that slept is looked at again"
    );
}

fn store_with_card() -> (tempfile::TempDir, devpit_core::Store, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = devpit_core::Store::open(&dir.path().join("state.db")).expect("store");
    let root = dir.path().join("project");
    std::fs::create_dir_all(&root).expect("root");
    let project = store.add_project(&root, None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &column, "Review the PR", "")
        .expect("card");
    (dir, store, card)
}

#[test]
fn a_reminder_goes_off_into_the_bell_once() {
    let (_dir, store, card) = store_with_card();
    store.set_card_due(&card, Some(1_000), true).expect("due");
    assert!(fire(&store, 999).is_empty());
    let fired = fire(&store, 1_000);
    assert_eq!(fired.len(), 1);
    assert!(fire(&store, 2_000).is_empty(), "once");
    let notices = store.notices(10).expect("notices");
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0].kind, KIND);
    assert_eq!(notices[0].title, "“Review the PR” is due");
    assert_eq!(store.reminders_pending().expect("pending").len(), 1);
}

#[test]
fn a_snooze_puts_it_off_and_reads_the_bell() {
    let (_dir, store, card) = store_with_card();
    store.set_card_due(&card, Some(1_000), true).expect("due");
    fire(&store, 1_000);
    assert!(
        snooze(&store, &card, 900.0, 1_000).is_err(),
        "not into the past"
    );
    snooze(&store, &card, 1_900.0, 1_000).expect("snoozed");
    assert!(store.reminders_pending().expect("pending").is_empty());
    assert_eq!(store.next_reminder().expect("next"), Some(1_900));
    assert_eq!(store.unread_notices().expect("unread"), 0);
    assert_eq!(fire(&store, 1_900).len(), 1, "it goes off again");
}
