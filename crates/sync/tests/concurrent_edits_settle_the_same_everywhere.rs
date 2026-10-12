//! Two machines change the same things while apart. Whichever syncs first,
//! both end with the same values: per field, the later change, and between
//! two changes at the same moment, the same machine's every time.

mod support;

use devpit_core::preference as key;
use devpit_sync::clock::Clock;
use devpit_sync::{files, State, SyncKey};
use support::{now_ms, Home, Stand};

/// A machine with a name of its own for its stamps, so two runs compare.
fn home(id: &str, made: &SyncKey) -> Home {
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
    home
}

/// Sets a preference as if it had been changed `seconds` from `base`.
fn changed(home: &Home, name: &str, value: &str, base: i64, seconds: i64) {
    home.store.set_preference(name, value).expect("set");
    home.store
        .conn()
        .execute(
            "UPDATE preference SET updated_at = ?1 WHERE key = ?2",
            (base + seconds, name),
        )
        .expect("when");
}

type Values = Vec<Option<String>>;

const READ: [&str; 3] = [key::THEME, key::TERMINAL_CONTRAST, key::CONFIRM_STOP];

/// Both machines join, part, change things, and sync in `order` (0 is A).
async fn apart_then(order: &[usize]) -> (Values, Values, Option<String>, Option<String>) {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut homes = [home("m_a", &made), home("m_b", &made)];
    let origin = "git@github.com:jo/thing.git";
    let projects: Vec<String> = homes
        .iter()
        .map(|one| {
            one.store
                .add_project(&one.root().join("thing"), Some(origin))
                .expect("add")
        })
        .collect();
    for one in homes.iter_mut() {
        one.sync(&stand).await.expect("join");
    }
    let base = (now_ms() / 1000) as i64;
    // A chose light ten seconds from now, B dark at twenty: B's is later.
    changed(&homes[0], key::THEME, "light", base, 10);
    changed(&homes[1], key::THEME, "dark", base, 20);
    // The contrast the other way round: A's, at thirty, is later.
    changed(&homes[0], key::TERMINAL_CONTRAST, "5", base, 30);
    changed(&homes[1], key::TERMINAL_CONTRAST, "9", base, 25);
    // Both at the same second: the tie goes to the same machine everywhere.
    changed(&homes[0], key::CONFIRM_STOP, "true", base, 40);
    changed(&homes[1], key::CONFIRM_STOP, "false", base, 40);
    // Different fields of one project: both stay.
    homes[0]
        .store
        .edit_project(&projects[0], "thing", None, Some("rocket"), None)
        .expect("icon");
    homes[1]
        .store
        .edit_project(&projects[1], "thing", None, None, Some("#00ff00"))
        .expect("colour");
    for at in order {
        homes[*at].sync(&stand).await.expect("sync");
    }
    let values = |one: &Home| READ.iter().map(|name| one.pref(name)).collect::<Values>();
    let mark = |one: &Home, project: &str| {
        let row = one.store.project(project).expect("read").expect("there");
        Some(format!("{:?} {:?}", row.icon, row.color))
    };
    (
        values(&homes[0]),
        values(&homes[1]),
        mark(&homes[0], &projects[0]),
        mark(&homes[1], &projects[1]),
    )
}

#[tokio::test]
async fn concurrent_edits_settle_on_the_same_values_in_either_order() {
    let (a, b, a_mark, b_mark) = apart_then(&[0, 1, 0]).await;
    assert_eq!(a, b, "both machines agree");
    assert_eq!(a_mark, b_mark);
    assert_eq!(
        a,
        vec![
            Some("dark".to_owned()),
            Some("5".to_owned()),
            Some("false".to_owned())
        ],
        "the later change per field; the tie to m_b, the greater machine"
    );
    assert_eq!(
        a_mark.as_deref(),
        Some(r##"Some("rocket") Some("#00ff00")"##)
    );

    let (a2, b2, a2_mark, _) = apart_then(&[1, 0, 1]).await;
    assert_eq!(a2, a, "the other order lands on the same values");
    assert_eq!(b2, b);
    assert_eq!(a2_mark, a_mark);
}
