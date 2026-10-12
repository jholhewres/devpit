//! A preference changed on one machine appears on the other: two devpit
//! homes of one account, the key carried by its recovery code, and a
//! stand-in for devpit.app between them. `two_homes_through_devpit_app.rs`
//! runs the same through devpit-app itself.

mod support;

use devpit_core::preference as key;
use devpit_sync::machines;
use devpit_sync::{files, SyncKey};
use support::{Home, Stand};

/// The first machine makes the key; the second is given it as the person
/// would type it, from the recovery code.
fn paired(a: &Home, b: &Home) {
    let made = SyncKey::generate().expect("a key");
    files::keep_key(&a.root(), &made).expect("keep");
    let typed = made.recovery_code().to_lowercase();
    files::keep_key(
        &b.root(),
        &SyncKey::from_recovery_code(&typed).expect("the code"),
    )
    .expect("keep");
}

#[tokio::test]
async fn a_theme_chosen_on_one_machine_is_the_theme_on_the_other() {
    let stand = Stand::default();
    let (mut a, mut b) = (Home::new("laptop"), Home::new("desktop"));
    paired(&a, &b);
    // The second machine had a theme of its own before it joined.
    b.store.set_preference(key::THEME, "dark").expect("set");

    a.store.set_preference(key::THEME, "light").expect("set");
    a.store
        .set_preference(key::CONFIRM_STOP, "false")
        .expect("set");
    a.store
        .set_preference(key::SHORTCUT_KEYS, r#"{"palette":["Ctrl+K"]}"#)
        .expect("set");
    let sent = a.sync(&stand).await.expect("a syncs");
    assert!(sent.sent.contains(&"prefs".to_owned()), "{sent:?}");

    let taken = b.sync(&stand).await.expect("b syncs");
    assert!(taken.written.contains(&"prefs".to_owned()), "{taken:?}");
    assert_eq!(
        b.pref(key::THEME).as_deref(),
        Some("light"),
        "joining takes what was settled"
    );
    assert_eq!(b.pref(key::CONFIRM_STOP).as_deref(), Some("false"));
    assert_eq!(
        b.pref(key::SHORTCUT_KEYS).as_deref(),
        Some(r#"{"palette":["Ctrl+K"]}"#)
    );

    // And back the other way, once both are in.
    b.store
        .set_preference(key::TERMINAL_CONTRAST, "7")
        .expect("set");
    b.store.set_preference(key::THEME, "system").expect("set");
    b.sync(&stand).await.expect("b syncs");
    a.sync(&stand).await.expect("a syncs");
    assert_eq!(a.pref(key::TERMINAL_CONTRAST).as_deref(), Some("7"));
    assert_eq!(a.pref(key::THEME).as_deref(), Some("system"));
}

#[tokio::test]
async fn a_round_with_nothing_new_sends_nothing() {
    let stand = Stand::default();
    let (mut a, b) = (Home::new("laptop"), Home::new("desktop"));
    paired(&a, &b);
    a.store.set_preference(key::THEME, "light").expect("set");
    a.sync(&stand).await.expect("first");
    let before = stand.sent.lock().unwrap().len();
    let again = a.sync(&stand).await.expect("again");
    assert!(again.sent.is_empty(), "{again:?}");
    assert_eq!(stand.sent.lock().unwrap().len(), before);
}

#[tokio::test]
async fn a_project_looks_the_same_on_every_machine_that_has_it() {
    let stand = Stand::default();
    let (mut a, mut b) = (Home::new("laptop"), Home::new("desktop"));
    paired(&a, &b);
    let origin = "git@github.com:someone/thing.git";
    let on_a = a
        .store
        .add_project(&a.root().join("thing"), Some(origin))
        .expect("add");
    a.store
        .edit_project(
            &on_a,
            "The Thing",
            Some("Work"),
            Some("rocket"),
            Some("#123abc"),
        )
        .expect("edit");
    a.store.install_plugin(&on_a, "notes").expect("plugin");
    a.sync(&stand).await.expect("a syncs");

    // The same repository, cloned over https on the other machine.
    let on_b = b
        .store
        .add_project(
            &b.root().join("elsewhere"),
            Some("https://github.com/someone/thing"),
        )
        .expect("add");
    b.sync(&stand).await.expect("b syncs");
    let row = b.store.project(&on_b).expect("read").expect("there");
    assert_eq!(row.name, "The Thing");
    assert_eq!(row.group.as_deref(), Some("Work"));
    assert_eq!(row.icon.as_deref(), Some("rocket"));
    assert_eq!(row.color.as_deref(), Some("#123abc"));
    assert_eq!(
        b.store.enabled_plugins(&on_b).expect("plugins"),
        vec!["notes"]
    );
    assert!(
        row.root_path.ends_with("elsewhere"),
        "where it lives on this machine is its own"
    );
}

#[tokio::test]
async fn each_machine_lists_the_others_by_name() {
    let stand = Stand::default();
    let (mut a, mut b) = (Home::new("laptop"), Home::new("desktop"));
    paired(&a, &b);
    a.sync(&stand).await.expect("a");
    b.sync(&stand).await.expect("b");
    a.sync(&stand).await.expect("a again");
    let listed = machines::listed(a.state().mirror("machines"));
    let names: Vec<&str> = listed.iter().map(|one| one.name.as_str()).collect();
    assert_eq!(listed.len(), 2, "{names:?}");
    assert!(names.contains(&"laptop") && names.contains(&"desktop"));
}
