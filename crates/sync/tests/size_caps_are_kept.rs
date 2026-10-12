//! Nothing past its ceiling leaves the machine: a preference past 64 KiB
//! stays where it is, a namespace that seals past 1 MiB is not sent while
//! the others still are, and the client refuses such a blob before a byte
//! goes out — devpit-app refuses it too, and keeps each account to its room.

mod support;

use devpit_core::preference as key;
use devpit_sync::prefs::{Prefs, LONGEST};
use devpit_sync::space::{Here, Seen};
use devpit_sync::{files, round, Blobs, DevpitApp, Namespace, SyncError, SyncKey, MOST_BLOB};
use support::{now_ms, Home, Stand};

/// A namespace holding one field of `size` bytes.
struct Heavy {
    size: usize,
}

impl Namespace for Heavy {
    fn name(&self) -> &'static str {
        "heavy"
    }

    fn here(&self) -> Result<Here, String> {
        Ok(Here::from([(
            "x".to_owned(),
            [(
                "blob".to_owned(),
                Seen::of(Some("a".repeat(self.size)), None),
            )]
            .into(),
        )]))
    }

    fn write(&self, _: &str, _: &str, _: Option<&str>) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::test]
async fn a_preference_past_its_ceiling_stays_on_its_machine() {
    let stand = Stand::default();
    let (mut a, mut b) = (Home::new("laptop"), Home::new("desktop"));
    let made = SyncKey::generate().expect("key");
    files::keep_key(&a.root(), &made).expect("keep");
    files::keep_key(&b.root(), &made).expect("keep");
    let huge = format!(r#"{{"palette":["{}"]}}"#, "K".repeat(LONGEST));
    a.store
        .set_preference(key::SHORTCUT_KEYS, &huge)
        .expect("set");
    a.store.set_preference(key::THEME, "light").expect("set");
    a.sync(&stand).await.expect("a");
    b.sync(&stand).await.expect("b");
    assert_eq!(b.pref(key::THEME).as_deref(), Some("light"));
    assert_eq!(b.pref(key::SHORTCUT_KEYS), None);
}

#[tokio::test]
async fn a_namespace_past_a_blobs_ceiling_is_not_sent_and_the_others_are() {
    let stand = Stand::default();
    let home = Home::new("laptop");
    home.store.set_preference(key::THEME, "light").expect("set");
    let made = SyncKey::generate().expect("key");
    let mut state = files::state(&home.root());
    let prefs = Prefs { store: &home.store };
    let heavy = Heavy { size: MOST_BLOB };
    let spaces: [&dyn Namespace; 2] = [&heavy, &prefs];
    let outcome = round(&stand, &made, &mut state, &spaces, now_ms())
        .await
        .expect("the round goes on");
    assert_eq!(outcome.sent, vec!["prefs".to_owned()]);
    assert_eq!(outcome.problems.len(), 1, "{outcome:?}");
    assert!(
        outcome.problems[0].contains("at most 1024 KiB"),
        "{}",
        outcome.problems[0]
    );
    assert!(
        !stand.kept().contains_key(&made.blob_name("heavy")),
        "nothing past the ceiling reached devpit.app"
    );

    let light = Heavy { size: 1000 };
    let spaces: [&dyn Namespace; 1] = [&light];
    let fine = round(&stand, &made, &mut state, &spaces, now_ms())
        .await
        .expect("round");
    assert_eq!(fine.sent, vec!["heavy".to_owned()], "a size that fits goes");
}

#[tokio::test]
async fn the_client_refuses_a_blob_past_the_ceiling_before_sending_it() {
    // Nothing listens on the discard port: a request that went out would
    // fail as unreachable, not as too large.
    let app = DevpitApp::new("http://127.0.0.1:9", "a-session").expect("client");
    let refused = app
        .put(
            "0123456789abcdef0123456789abcdef",
            0,
            vec![0u8; MOST_BLOB + 1],
        )
        .await
        .expect_err("refused");
    assert!(matches!(refused, SyncError::TooLarge(_)), "{refused:?}");
}
