//! What devpit.app keeps is sealed: every blob a machine sent is captured
//! here and searched for the words that went into it, and opened with keys
//! that are not the account's. And what devpit.app could do with it — change
//! a byte, serve an older copy, start a second copy beside the first — stops
//! a round instead of reaching a machine.

mod support;

use devpit_core::preference as key;
use devpit_sync::seal::{open, Unsealed};
use devpit_sync::{files, SyncError, SyncKey};
use support::{holds, opened, Home, Stand};

const WORDS: [&str; 9] = [
    "light",
    "appearance.theme",
    "Quarterly Secret Project",
    "rocket",
    "origin:",
    "notes",
    "Jo's laptop",
    "machines",
    "prefs",
];

async fn sent_by_one_machine() -> (Stand, Home, SyncKey) {
    let stand = Stand::default();
    let mut a = Home::new("Jo's laptop");
    let made = SyncKey::generate().expect("key");
    files::keep_key(&a.root(), &made).expect("keep");
    a.store.set_preference(key::THEME, "light").expect("set");
    let project = a
        .store
        .add_project(&a.root().join("q"), Some("git@github.com:jo/quarterly.git"))
        .expect("add");
    a.store
        .edit_project(
            &project,
            "Quarterly Secret Project",
            None,
            Some("rocket"),
            None,
        )
        .expect("edit");
    a.store.install_plugin(&project, "notes").expect("plugin");
    a.sync(&stand).await.expect("sync");
    (stand, a, made)
}

#[tokio::test]
async fn a_captured_blob_carries_none_of_the_words_that_went_in() {
    let (stand, _a, made) = sent_by_one_machine().await;
    let sent = stand.sent.lock().unwrap().clone();
    assert_eq!(sent.len(), 3, "prefs, projects and machines");
    for (name, _, bytes) in &sent {
        assert_eq!(name.len(), 32, "a name drawn from the key: {name}");
        for word in WORDS {
            assert!(!holds(bytes, word), "{word:?} crossed in the clear");
            assert!(!name.contains(word), "{word:?} is in a blob's name");
        }
    }
    // The capture is the real thing: the account's key opens it to the words.
    let words = opened(&made, &stand.kept());
    for word in ["light", "Quarterly Secret Project", "rocket", "Jo's laptop"] {
        assert!(words.contains(word), "{word:?} was not synced at all");
    }
}

#[tokio::test]
async fn a_captured_blob_does_not_open_without_the_key() {
    let (stand, _a, _) = sent_by_one_machine().await;
    let stranger = SyncKey::generate().expect("another key");
    for (name, (version, bytes)) in stand.kept() {
        assert_eq!(
            open(&stranger, &name, version, &bytes),
            Err(Unsealed::OtherKey)
        );
        // Not even by claiming to be the key the blob names.
        let mut claimed = bytes.clone();
        claimed[4..12]
            .copy_from_slice(&devpit_sync::seal::sealed_by(&stranger_blob(&stranger)).expect("id"));
        assert_eq!(
            open(&stranger, &name, version, &claimed),
            Err(Unsealed::Tampered)
        );
    }
}

fn stranger_blob(key: &SyncKey) -> Vec<u8> {
    devpit_sync::seal::seal(key, "x", 1, b"{}").expect("seal")
}

#[tokio::test]
async fn a_changed_blob_stops_the_round_and_reaches_no_machine() {
    let (stand, a, made) = sent_by_one_machine().await;
    let mut b = Home::new("desktop");
    files::keep_key(&b.root(), &made).expect("keep");
    b.store.set_preference(key::THEME, "dark").expect("set");
    let prefs = made.blob_name("prefs");
    let (version, mut bytes) = stand.kept()[&prefs].clone();
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    stand.tamper(&prefs, version, bytes);
    let refused = b.sync(&stand).await.expect_err("refused");
    assert!(matches!(refused, SyncError::Unreadable(_)), "{refused:?}");
    assert_eq!(b.pref(key::THEME).as_deref(), Some("dark"));
    drop(a);
}

#[tokio::test]
async fn an_older_copy_served_after_a_newer_one_is_refused() {
    let (stand, mut a, made) = sent_by_one_machine().await;
    let prefs = made.blob_name("prefs");
    let older = stand.kept()[&prefs].clone();
    a.store.set_preference(key::THEME, "system").expect("set");
    a.sync(&stand).await.expect("newer");
    stand.tamper(&prefs, older.0, older.1);
    assert_eq!(a.sync(&stand).await, Err(SyncError::Rollback));
    assert_eq!(a.pref(key::THEME).as_deref(), Some("system"));
}

#[tokio::test]
async fn a_machine_with_another_key_does_not_start_a_second_copy() {
    let (stand, _a, _) = sent_by_one_machine().await;
    let mut c = Home::new("other");
    files::keep_key(&c.root(), &SyncKey::generate().expect("key")).expect("keep");
    let before = stand.kept();
    assert_eq!(c.sync(&stand).await, Err(SyncError::OtherKey));
    assert_eq!(
        stand.kept(),
        before,
        "nothing was written beside the account's blobs"
    );
}

#[tokio::test]
async fn a_machine_whose_blobs_are_gone_stops_rather_than_writing_them_again() {
    let (stand, mut a, _) = sent_by_one_machine().await;
    // Started over elsewhere: everything removed, a new key's blobs instead.
    use devpit_sync::Blobs;
    stand.clear().await.expect("clear");
    a.store.set_preference(key::THEME, "dark").expect("set");
    assert_eq!(a.sync(&stand).await, Err(SyncError::Reset));
    assert!(stand.kept().is_empty());
}
