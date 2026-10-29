//! One repository cloned over https on one machine and over ssh on the other
//! is one project. Its address settles on one spelling instead of each
//! machine sending its own back at every round.

mod support;

use devpit_core::origin::origin_hash;
use devpit_sync::clock::Clock;
use devpit_sync::{files, State, SyncKey};
use support::{Home, Stand};

fn home(id: &str, made: &SyncKey, origin: &str) -> Home {
    let home = Home::new(id);
    files::keep_key(&home.root(), made).expect("keep");
    files::keep_state(
        &home.root(),
        &State {
            machine: id.to_owned(),
            clock: Clock::new(id),
            spaces: Default::default(),
            synced_at: None,
        },
    )
    .expect("state");
    home.store
        .add_project(&home.root().join("thing"), Some(origin))
        .expect("add");
    home
}

#[tokio::test]
async fn two_spellings_of_one_origin_settle_and_stop_sending() {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut homes = [
        home("m_a", &made, "https://github.com/jo/thing.git"),
        home("m_b", &made, "git@github.com:jo/thing.git"),
    ];
    for _ in 0..3 {
        for one in homes.iter_mut() {
            one.sync(&stand).await.expect("round");
        }
    }

    for one in homes.iter_mut() {
        let outcome = one.sync(&stand).await.expect("round");
        assert!(
            !outcome.sent.iter().any(|space| space == "projects"),
            "still sending its own spelling: {outcome:?}"
        );
    }
    let kept: Vec<_> = homes
        .iter()
        .map(|one| one.store.synced_projects().expect("list")[0].clone())
        .collect();
    assert_eq!(kept[0].origin_url, kept[1].origin_url, "one spelling");
    assert_eq!(
        kept[0].origin_hash.as_deref(),
        Some(origin_hash("git@github.com:jo/thing.git").as_str()),
        "and the identity did not move"
    );
}
